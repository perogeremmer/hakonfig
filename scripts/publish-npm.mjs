// Rakit dan publish package npm `hakonfig` + package platform-nya.
//
// Dipakai oleh release workflow, tapi bisa dijalankan manual:
//   node scripts/publish-npm.mjs <path-binary-darwin> <path-binary-linux> <path-binary-win.exe>
//   node scripts/publish-npm.mjs --dry-run <...>   # rakit tanpa publish
//
// Struktur:
//   hakonfig                              -> shim bin.js + optionalDependencies
//   @perogeremmer/hakonfig-darwin         -> binary universal (Apple Silicon + Intel)
//   @perogeremmer/hakonfig-linux-x64      -> binary linux x86_64
//   @perogeremmer/hakonfig-win32-x64      -> binary windows x86_64
//
// Binary diambil dari argumen (di CI: hasil unduhan aset release), jadi
// script ini tidak pernah membangun Rust sendiri.
//
// Kenapa package platform scoped? npm menolak nama unscoped berpola seperti
// `hakonfig-win32-x64` dengan `403 Package name triggered spam detection`
// (proteksi nama mirip package auto-generated dari akun baru) — sudah dicoba
// berkali-kali, termasuk jeda dan retry berjam-jam, dan selalu ditolak.
// Package scoped tidak terkena heuristik itu: ini pola yang dipakai esbuild
// (`@esbuild/linux-x64`) dan swc (`@swc/core-linux-x64-gnu`). Scope diambil
// dari username/org npm yang punya token; pindah ke org nanti cukup ganti
// SCOPE di bawah (mis. `@hakonfig`) plus nama package di npm/hakonfig/bin.js.
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, copyFileSync, writeFileSync, chmodSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const REPO = "perogeremmer/hakonfig";
const SCOPE = "@perogeremmer";
const ROOT = new URL("..", import.meta.url).pathname;
const SPACING_MS = 30_000; // jeda antar publish
const RETRY_MS = 60_000; // tunggu sebelum mencoba ulang
const RETRIES = 3;

const argv = process.argv.slice(2);
const dryRun = argv[0] === "--dry-run";
const binaries = (dryRun ? argv.slice(1) : argv).map((p) => resolve(p));
if (binaries.length !== 3) {
  console.error(
    "pakai: node scripts/publish-npm.mjs [--dry-run] <darwin-bin> <linux-bin> <win-bin.exe>"
  );
  process.exit(1);
}

const version = execFileSync("node", [join(ROOT, "scripts/version.mjs"), "show"], {
  encoding: "utf8",
}).trim();

const PLATFORMS = [
  {
    pkg: `${SCOPE}/hakonfig-darwin`,
    os: ["darwin"],
    cpu: ["x64", "arm64"],
    ext: "",
    bin: binaries[0],
  },
  {
    pkg: `${SCOPE}/hakonfig-linux-x64`,
    os: ["linux"],
    cpu: ["x64"],
    ext: "",
    bin: binaries[1],
  },
  {
    pkg: `${SCOPE}/hakonfig-win32-x64`,
    os: ["win32"],
    cpu: ["x64"],
    ext: ".exe",
    bin: binaries[2],
  },
];

const tmp = mkdtempSync(join(tmpdir(), "hakonfig-npm-"));
const npm = (args, cwd) =>
  execFileSync("npm", args, { cwd, encoding: "utf8", stdio: "pipe" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Sudah terpublish? Supaya re-run release tidak gagal di langkah yang sama. */
function isPublished(name, atVersion) {
  try {
    npm(["view", `${name}@${atVersion}`, "version"]);
    return true;
  } catch {
    return false;
  }
}

/** Publish dengan retry: 403 spam detection biasanya hilang setelah jeda. */
async function publish(dir, name) {
  for (let attempt = 1; attempt <= RETRIES; attempt++) {
    try {
      npm(["publish", "--access", "public"], dir);
      console.log(`terpublish: ${name}@${version}`);
      return;
    } catch (error) {
      const output = String(error.stderr || error.message);
      if (attempt === RETRIES) throw error;
      console.error(
        `percobaan ${attempt}/${RETRIES} gagal untuk ${name}; mencoba ulang ` +
          `${RETRY_MS / 1000} detik lagi.\n${output.split("\n").slice(-4).join("\n")}`
      );
      await sleep(RETRY_MS);
    }
  }
}

/**
 * Pastikan package scoped bisa dibaca siapa saja. `npm publish --access public`
 * tidak selalu cukup: package scoped bisa tersimpan sebagai restricted, dan
 * gejalanya halus — publish sukses, tapi `GET /@scope/nama` menjawab 404 untuk
 * pengunjung anonim sehingga `npm i -g hakonfig` tidak mendapat binary-nya.
 * Best-effort: kegagalan tidak boleh menggagalkan rilis.
 */
function ensurePublic(name) {
  try {
    npm(["access", "set", "status=public", name]);
    console.log(`publik: ${name}`);
  } catch (error) {
    console.log(`lewati set public ${name}: ${String(error.message).split("\n")[0]}`);
  }
}

/**
 * Tandai artefak lama yang sudah tidak dipakai sebagai deprecated, supaya
 * yang menginstall versi itu dapat peringatan. Best-effort: kegagalan di sini
 * tidak boleh menggagalkan rilis.
 */
function deprecate(spec, message) {
  try {
    npm(["deprecate", spec, message]);
    console.log(`deprecated: ${spec}`);
  } catch (error) {
    console.log(`lewati deprecate ${spec}: ${String(error.message).split("\n")[0]}`);
  }
}

// Package platform versi lama memakai nama unscoped, yang kini tidak dipakai
// lagi dan hanya berisi 0.4.0 (win32-nya bahkan tidak pernah lolos publish).
const LEGACY_PLATFORM_PACKAGES = [
  "hakonfig-darwin",
  "hakonfig-linux-x64",
  "hakonfig-win32-x64",
];

function packageJson(name, extra) {
  return JSON.stringify(
    {
      name,
      version,
      description: "hakonfig CLI: manage AI coding harness provider configs",
      license: "MIT",
      repository: {
        type: "git",
        url: `https://github.com/${REPO}.git`,
        directory: "npm",
      },
      homepage: `https://github.com/${REPO}`,
      // Scoped package defaultnya restricted; tanpa ini
      // `npm i -g hakonfig` di mesin orang lain tidak menemukan binary-nya.
      publishConfig: { access: "public" },
      ...extra,
    },
    null,
    2
  ) + "\n";
}

/** Siapkan direktori package dan kembalikan path-nya. */
function buildPackage(name, files, extra) {
  // Nama scoped berisi "/": buat nama direktori yang datar.
  const dir = join(tmp, name.replace("/", "__"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "package.json"), packageJson(name, extra));
  for (const [src, dest] of files) {
    const target = join(dir, dest);
    mkdirSync(join(target, ".."), { recursive: true });
    copyFileSync(src, target);
    chmodSync(target, 0o755);
  }
  return dir;
}

// ------------------------------------------------------------------ utama
// Package utama tetap unscoped (`hakonfig`) supaya perintah install-nya
// pendek; hanya package platform yang scoped.
const mainDir = buildPackage("hakonfig", [
  [join(ROOT, "npm", "hakonfig", "bin.js"), "bin.js"],
], {
  bin: { "hakonfig-cli": "bin.js" },
  files: ["bin.js"],
  engines: { node: ">=18" },
  optionalDependencies: Object.fromEntries(
    PLATFORMS.map((platform) => [platform.pkg, version])
  ),
});

// ------------------------------------------------------------------ platform
const platformDirs = PLATFORMS.map((platform) => {
  if (!existsSync(platform.bin)) {
    console.error(`binary tidak ada: ${platform.bin}`);
    process.exit(1);
  }
  const binName = `hakonfig-cli${platform.ext}`;
  const dir = buildPackage(platform.pkg, [[platform.bin, join("bin", binName)]], {
    os: platform.os,
    cpu: platform.cpu,
    files: ["bin"],
  });
  return { name: platform.pkg, dir };
});

// ------------------------------------------------------------------ publish
if (dryRun) {
  npm(["pack", "--dry-run"], mainDir);
  console.log(`[dry-run] hakonfig@${version} valid.`);
  for (const { name, dir } of platformDirs) {
    npm(["pack", "--dry-run"], dir);
    console.log(`[dry-run] ${name}@${version} valid.`);
  }
} else {
  if (isPublished("hakonfig", version)) {
    console.log(`hakonfig@${version} sudah ada, dilewati.`);
  } else {
    await publish(mainDir, "hakonfig");
    await sleep(SPACING_MS);
  }

  for (const { name, dir } of platformDirs) {
    if (isPublished(name, version)) {
      console.log(`${name}@${version} sudah ada, dilewati.`);
      continue;
    }
    await publish(dir, name);
    ensurePublic(name);
    await sleep(SPACING_MS);
  }

  deprecate(
    "hakonfig@0.4.0",
    `Versi ini tidak bisa dipakai di Windows (paket binary win32 gagal publish). Pakai hakonfig@${version} atau lebih baru.`
  );
  for (const legacy of LEGACY_PLATFORM_PACKAGES) {
    deprecate(
      `${legacy}@0.4.0`,
      `Nama package ini sudah diganti ${SCOPE}/${legacy} sejak ${version}. Pakai hakonfig@${version}.`
    );
  }
}

console.log(`\nselesai${dryRun ? " (dry-run)" : ""}; artefak di ${tmp}`);
