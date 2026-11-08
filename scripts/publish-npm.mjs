// Rakit dan publish package npm `hakonfig` + package platform-nya.
//
// Dipakai oleh release workflow, tapi bisa dijalankan manual:
//   node scripts/publish-npm.mjs <path-binary-darwin> <path-binary-linux> <path-binary-win.exe>
//   node scripts/publish-npm.mjs --dry-run <...>   # rakit tanpa publish
//
// Struktur:
//   hakonfig            -> shim bin.js + optionalDependencies per platform
//   hakonfig-darwin     -> binary universal (Apple Silicon + Intel)
//   hakonfig-linux-x64  -> binary linux x86_64
//   hakonfig-win32-x64  -> binary windows x86_64
//
// Binary diambil dari argumen (di CI: hasil unduhan aset release), jadi
// script ini tidak pernah membangun Rust sendiri.
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, copyFileSync, writeFileSync, chmodSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const REPO = "perogeremmer/hakonfig";
const ROOT = new URL("..", import.meta.url).pathname;

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
    pkg: "hakonfig-darwin",
    os: ["darwin"],
    cpu: ["x64", "arm64"],
    ext: "",
    bin: binaries[0],
  },
  {
    pkg: "hakonfig-linux-x64",
    os: ["linux"],
    cpu: ["x64"],
    ext: "",
    bin: binaries[1],
  },
  {
    pkg: "hakonfig-win32-x64",
    os: ["win32"],
    cpu: ["x64"],
    ext: ".exe",
    bin: binaries[2],
  },
];

const tmp = mkdtempSync(join(tmpdir(), "hakonfig-npm-"));
const npm = (args, cwd) =>
  execFileSync("npm", args, { cwd, encoding: "utf8", stdio: "pipe" });

/** Sudah terpublish? Supaya re-run release tidak gagal di langkah yang sama. */
function isPublished(name, atVersion) {
  try {
    npm(["view", `${name}@${atVersion}`, "version"]);
    return true;
  } catch {
    return false;
  }
}

function packageJson(name, extra) {
  return JSON.stringify(
    {
      name,
      version,
      description: "hakonfig CLI: manage AI coding harness provider configs",
      license: "MIT",
      repository: { type: "git", url: `github:${REPO}` },
      homepage: `https://github.com/${REPO}`,
      ...extra,
    },
    null,
    2
  ) + "\n";
}

// ------------------------------------------------------------------ platform
for (const platform of PLATFORMS) {
  if (!existsSync(platform.bin)) {
    console.error(`binary tidak ada: ${platform.bin}`);
    process.exit(1);
  }
  const dir = join(tmp, platform.pkg);
  mkdirSync(join(dir, "bin"), { recursive: true });
  writeFileSync(join(dir, "package.json"), packageJson(platform.pkg, {
    os: platform.os,
    cpu: platform.cpu,
    files: ["bin"],
  }));
  const binName = `hakonfig-cli${platform.ext}`;
  copyFileSync(platform.bin, join(dir, "bin", binName));
  chmodSync(join(dir, "bin", binName), 0o755);

  if (isPublished(platform.pkg, version)) {
    console.log(`${platform.pkg}@${version} sudah ada, dilewati.`);
    continue;
  }
  if (dryRun) {
    npm(["pack", "--dry-run"], dir);
    console.log(`[dry-run] ${platform.pkg}@${version} valid.`);
  } else {
    npm(["publish", "--access", "public"], dir);
    console.log(`terpublish: ${platform.pkg}@${version}`);
  }
}

// ------------------------------------------------------------------ utama
const mainDir = join(tmp, "hakonfig");
mkdirSync(mainDir, { recursive: true });
writeFileSync(join(mainDir, "package.json"), packageJson("hakonfig", {
  bin: { "hakonfig-cli": "bin.js" },
  files: ["bin.js"],
  engines: { node: ">=18" },
  optionalDependencies: Object.fromEntries(
    PLATFORMS.map((platform) => [platform.pkg, version])
  ),
}));
copyFileSync(join(ROOT, "npm", "hakonfig", "bin.js"), join(mainDir, "bin.js"));
chmodSync(join(mainDir, "bin.js"), 0o755);

if (isPublished("hakonfig", version)) {
  console.log(`hakonfig@${version} sudah ada, dilewati.`);
} else if (dryRun) {
  npm(["pack", "--dry-run"], mainDir);
  console.log(`[dry-run] hakonfig@${version} valid.`);
} else {
  npm(["publish", "--access", "public"], mainDir);
  console.log(`terpublish: hakonfig@${version}`);
}

console.log(`\nselesai${dryRun ? " (dry-run)" : ""}; artefak di ${tmp}`);
