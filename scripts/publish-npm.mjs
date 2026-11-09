// Rakit dan publish package npm `hakonfig`.
//
// Dipakai oleh release workflow, tapi bisa dijalankan manual:
//   node scripts/publish-npm.mjs <darwin-bin> <linux-bin> <win-bin.exe>
//   node scripts/publish-npm.mjs --dry-run <...>   # rakit tanpa publish
//
// Bentuk package: satu package, binari ikut di dalamnya.
//   hakonfig
//     bin.js                        shim yang memilih binari sesuai platform
//     bin/hakonfig-cli-darwin       universal (Apple Silicon + Intel)
//     bin/hakonfig-cli-linux-x64
//     bin/hakonfig-cli-win32-x64.exe
//
// Kenapa tidak pakai package platform terpisah (pola esbuild)? Dua kali gagal
// di npm:
//   1. nama unscoped `hakonfig-win32-x64` ditolak permanen dengan
//      `403 Package name triggered spam detection` (heuristic nama
//      auto-generated dari akun baru), walau darwin dan linux lolos;
//   2. package scoped `@perogeremmer/hakonfig-*` berhasil di-publish tapi
//      tersimpan sebagai restricted — `GET /@perogeremmer%2Fhakonfig-linux-x64`
//      menjawab 404 untuk pengunjung anonim, dan token rilis tidak boleh
//      mengubah visibilitas (`POST /-/package/.../access` → 403 karena
//      pembatasan token bypass-2FA). Akibatnya `npm i -g hakonfig` tidak
//      mendapat binari apa pun.
// Binari gabungan hanya ~11 MB (≈5 MB terkompresi) dan menghilangkan seluruh
// ketergantungan pada optionalDependencies, visibilitas scope, maupun
// heuristic nama. Ini juga yang membuat install di Windows akhirnya berhasil.
//
// Binari diambil dari argumen (di CI: hasil unduhan aset release), jadi script
// ini tidak pernah membangun Rust sendiri.
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, copyFileSync, writeFileSync, chmodSync, existsSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const REPO = "perogeremmer/hakonfig";
const PACKAGE_NAME = "hakonfig";
const ROOT = new URL("..", import.meta.url).pathname;
const RETRY_MS = 60_000; // tunggu sebelum mencoba publish ulang
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

// Nama file di dalam package; harus sama dengan peta di npm/hakonfig/bin.js.
const BINARIES = [
  { source: binaries[0], name: "hakonfig-cli-darwin" },
  { source: binaries[1], name: "hakonfig-cli-linux-x64" },
  { source: binaries[2], name: "hakonfig-cli-win32-x64.exe" },
];

// Package platform versi lama: unscoped (0.4.0) dan scoped (0.4.1). Keduanya
// tidak dipakai lagi — ditandai deprecated supaya yang menginstall versi itu
// dapat peringatan, bukan bingung karena binarinya tidak ketemu.
const LEGACY_PACKAGES = [
  "hakonfig-darwin@0.4.0",
  "hakonfig-linux-x64@0.4.0",
  "@perogeremmer/hakonfig-darwin@0.4.1",
  "@perogeremmer/hakonfig-linux-x64@0.4.1",
  "@perogeremmer/hakonfig-win32-x64@0.4.1",
];

const npm = (args, cwd) =>
  execFileSync("npm", args, { cwd, encoding: "utf8", stdio: "pipe" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Sudah terpublish? Supaya re-run release tidak gagal di langkah yang sama. */
function isPublished(atVersion) {
  try {
    npm(["view", `${PACKAGE_NAME}@${atVersion}`, "version"]);
    return true;
  } catch {
    return false;
  }
}

/**
 * Publish dengan retry: 403 dari npm (rate limit / spam detection) biasanya
 * hilang setelah jeda.
 */
async function publish(dir) {
  for (let attempt = 1; attempt <= RETRIES; attempt++) {
    try {
      npm(["publish", "--access", "public"], dir);
      console.log(`terpublish: ${PACKAGE_NAME}@${version}`);
      return;
    } catch (error) {
      const output = String(error.stderr || error.message);
      if (attempt === RETRIES) throw error;
      console.error(
        `percobaan ${attempt}/${RETRIES} gagal; mencoba ulang ` +
          `${RETRY_MS / 1000} detik lagi.\n${output.split("\n").slice(-4).join("\n")}`
      );
      await sleep(RETRY_MS);
    }
  }
}

/**
 * Pastikan package benar-benar bisa dibaca publik. Publish bisa melaporkan
 * sukses sementara packument tetap 404 untuk pengunjung anonim — persis
 * kegagalan yang membuat rilis 0.4.1 tidak terpakai — jadi ini diverifikasi
 * sebagai pengunjung tanpa token.
 */
async function verifyPublic(atVersion) {
  const attempts = 10;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    let status = "tidak ada respons";
    try {
      const res = await fetch(
        `https://registry.npmjs.org/${PACKAGE_NAME}?cb=${Date.now()}`,
        { headers: { "Cache-Control": "no-cache" } }
      );
      if (res.ok) {
        const doc = await res.json();
        if (doc.versions?.[atVersion]) {
          console.log(`terverifikasi publik: ${PACKAGE_NAME}@${atVersion}`);
          return;
        }
        status = "versi belum terindeks";
      } else {
        status = `HTTP ${res.status}`;
      }
    } catch (error) {
      status = String(error.message);
    }
    if (attempt === attempts) {
      throw new Error(
        `${PACKAGE_NAME}@${atVersion} tidak bisa diakses publik (${status}); ` +
          `cek visibilitas package di npmjs.com`
      );
    }
    console.log(`verifikasi ${attempt}/${attempts} belum lolos (${status}); ulangi 15 detik.`);
    await sleep(15_000);
  }
}

/**
 * Tandai artefak lama sebagai deprecated. Best-effort: kegagalan di sini tidak
 * boleh menggagalkan rilis.
 */
function deprecate(spec, message) {
  try {
    npm(["deprecate", spec, message]);
    console.log(`deprecated: ${spec}`);
  } catch (error) {
    const first = String(error.stderr || error.message).split("\n").slice(-3).join(" ").trim();
    console.log(`lewati deprecate ${spec}: ${first.slice(0, 160)}`);
  }
}

function packageJson(extra) {
  return JSON.stringify(
    {
      name: PACKAGE_NAME,
      version,
      description: "hakonfig CLI: manage AI coding harness provider configs",
      license: "MIT",
      repository: {
        type: "git",
        url: `https://github.com/${REPO}.git`,
        directory: "npm",
      },
      homepage: `https://github.com/${REPO}`,
      // Package ini unscoped, tapi tetap ditulis eksplisit: kalau suatu saat
      // dipindah ke scope, default-nya restricted dan install akan rusak.
      publishConfig: { access: "public" },
      ...extra,
    },
    null,
    2
  ) + "\n";
}

// ------------------------------------------------------------------ rakit
const tmp = mkdtempSync(join(tmpdir(), "hakonfig-npm-"));
const dir = join(tmp, PACKAGE_NAME);
mkdirSync(join(dir, "bin"), { recursive: true });
writeFileSync(
  join(dir, "package.json"),
  packageJson({
    bin: { "hakonfig-cli": "bin.js" },
    files: ["bin.js", "bin"],
    engines: { node: ">=18" },
  })
);
copyFileSync(join(ROOT, "npm", "hakonfig", "bin.js"), join(dir, "bin.js"));
for (const binary of BINARIES) {
  if (!existsSync(binary.source)) {
    console.error(`binary tidak ada: ${binary.source}`);
    process.exit(1);
  }
  const target = join(dir, "bin", binary.name);
  copyFileSync(binary.source, target);
  chmodSync(target, 0o755);
  console.log(`+ ${binary.name} (${statSync(target).size} byte)`);
}

// ------------------------------------------------------------------ publish
if (dryRun) {
  npm(["pack", "--dry-run"], dir);
  console.log(`[dry-run] ${PACKAGE_NAME}@${version} valid.`);
} else {
  if (isPublished(version)) {
    console.log(`${PACKAGE_NAME}@${version} sudah ada, dilewati.`);
  } else {
    await publish(dir);
  }
  await verifyPublic(version);

  for (const spec of LEGACY_PACKAGES) {
    deprecate(
      spec,
      `Tidak dipakai lagi sejak ${version}: binari semua platform sekarang ada di dalam paket ${PACKAGE_NAME}. Pakai hakonfig@${version} atau lebih baru.`
    );
  }
  deprecate(
    "hakonfig@0.4.0",
    `Versi ini tidak punya binari Windows. Pakai hakonfig@${version} atau lebih baru.`
  );
}

console.log(`\nselesai${dryRun ? " (dry-run)" : ""}; artefak di ${tmp}`);
