#!/usr/bin/env node
// =============================================================================
// Satu sumber kebenaran versi: `package.json`.
//
// Skrip ini menyebarkan nomor versi ke seluruh tempat yang membacanya:
//   * package.json            (sumber)
//   * src-tauri/Cargo.toml    ([package] version -> dipakai binary & bundle)
//   * src-tauri/crates/*/Cargo.toml (crate core + cli ikut versi app)
//   * src-tauri/tauri.conf.json (version -> nama installer)
//   * web/version.js          (APP_VERSION -> ditampilkan di Settings)
//
// Pemakaian:
//   node scripts/version.mjs show
//   node scripts/version.mjs check
//   node scripts/version.mjs bump patch|minor|major
//   node scripts/version.mjs bump 1.2.3
//
// `check` dipakai CI: gagal kalau ada file yang tertinggal.
// =============================================================================

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const FILES = {
  package: join(root, "package.json"),
  cargo: join(root, "src-tauri/Cargo.toml"),
  core: join(root, "src-tauri/crates/core/Cargo.toml"),
  cli: join(root, "src-tauri/crates/cli/Cargo.toml"),
  tauri: join(root, "src-tauri/tauri.conf.json"),
  web: join(root, "web/version.js"),
  lock: join(root, "src-tauri/Cargo.lock"),
};

/** Versi pertama (`[package]`) dari sebuah Cargo.toml. */
function cargoVersion(file) {
  try {
    return /^version = "([^"]+)"/m.exec(readFileSync(file, "utf8"))?.[1] ?? "";
  } catch {
    return "";
  }
}

/** Versi sebuah package di dalam Cargo.lock ("" bila tidak ada). */
function lockVersion(text, name) {
  const pattern = new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`);
  return pattern.exec(text)?.[1] ?? "";
}

const SEMVER = /^\d+\.\d+\.\d+$/;

function readPackageVersion() {
  return JSON.parse(readFileSync(FILES.package, "utf8")).version;
}

/** Versi yang tertulis di tiap file, dalam bentuk yang mudah dibandingkan. */
function collect() {
  const pkg = readPackageVersion();
  const tauri = JSON.parse(readFileSync(FILES.tauri, "utf8")).version;
  const web = /APP_VERSION = "([^"]+)"/.exec(readFileSync(FILES.web, "utf8"))?.[1];
  let lockText = "";
  try {
    lockText = readFileSync(FILES.lock, "utf8");
  } catch {
    // tanpa Cargo.lock, lewati pemeriksaan ini
  }
  return {
    package: pkg,
    cargo: cargoVersion(FILES.cargo),
    core: cargoVersion(FILES.core),
    cli: cargoVersion(FILES.cli),
    tauri,
    web,
    lock: lockVersion(lockText, "hakonfig"),
    lockCore: lockVersion(lockText, "hakonfig-core"),
    lockCli: lockVersion(lockText, "hakonfig-cli"),
  };
}

function bump(current, kind) {
  if (SEMVER.test(kind)) return kind;
  const [major, minor, patch] = current.split(".").map(Number);
  if (kind === "major") return `${major + 1}.0.0`;
  if (kind === "minor") return `${major}.${minor + 1}.0`;
  if (kind === "patch") return `${major}.${minor}.${patch + 1}`;
  throw new Error(`argumen bump tidak dikenal: ${kind} (pakai patch|minor|major|x.y.z)`);
}

function write(next) {
  if (!SEMVER.test(next)) throw new Error(`versi tidak valid: ${next}`);

  // package.json: pertahankan urutan key apa adanya, ganti hanya versinya.
  const pkgText = readFileSync(FILES.package, "utf8");
  writeFileSync(
    FILES.package,
    pkgText.replace(/"version":\s*"[^"]+"/, `"version": "${next}"`),
  );

  // Cargo.toml: hanya `version` pertama (milik [package], bukan dependency).
  for (const manifest of [FILES.cargo, FILES.core, FILES.cli]) {
    let text;
    try {
      text = readFileSync(manifest, "utf8");
    } catch {
      continue;
    }
    writeFileSync(manifest, text.replace(/^version = "[^"]+"/m, `version = "${next}"`));
  }

  // tauri.conf.json
  const tauriText = readFileSync(FILES.tauri, "utf8");
  writeFileSync(FILES.tauri, tauriText.replace(/"version":\s*"[^"]+"/, `"version": "${next}"`));

  // Cargo.lock: entri package milik aplikasi ini ikut berubah saat versi naik.
  try {
    const lockPath = join(root, "src-tauri/Cargo.lock");
    const lock = readFileSync(lockPath, "utf8");
    let patched = lock;
    for (const name of ["hakonfig", "hakonfig-core", "hakonfig-cli"]) {
      patched = patched.replace(
        new RegExp(`(\\[\\[package\\]\\]\\nname = "${name}"\\nversion = ")[^"]+(")`),
        `$1${next}$2`,
      );
    }
    if (patched !== lock) writeFileSync(lockPath, patched);
  } catch {
    // Cargo.lock belum ada (checkout bersih): tidak masalah.
  }

  // web/version.js
  writeFileSync(
    FILES.web,
    `// Dibuat oleh scripts/version.mjs - jangan diedit manual.\nexport const APP_VERSION = "${next}";\n`,
  );
}

const [command, argument] = process.argv.slice(2);

if (command === "show") {
  console.log(readPackageVersion());
} else if (command === "check") {
  const versions = collect();
  // Entri kosong = file belum ada (mis. Cargo.lock di checkout bersih).
  const values = Object.values(versions).filter(Boolean);
  const ok = values.every((value) => value === versions.package);
  if (!ok) {
    console.error("Versi tidak konsisten:");
    for (const [file, version] of Object.entries(versions)) {
      console.error(`  ${file.padEnd(8)} ${version ?? "(tidak ditemukan)"}`);
    }
    console.error("\nJalankan: node scripts/version.mjs bump " + versions.package);
    process.exit(1);
  }
  console.log(`versi konsisten: ${versions.package}`);
} else if (command === "bump") {
  if (!argument) {
    console.error("Butuh argumen: patch|minor|major|x.y.z");
    process.exit(1);
  }
  const current = readPackageVersion();
  const next = bump(current, argument);
  write(next);
  console.log(`versi ${current} -> ${next}`);
  console.log(`lanjut: git commit -am "chore: release v${next}" && git tag v${next} && git push --follow-tags`);
} else {
  console.error("Pemakaian: node scripts/version.mjs show|check|bump <patch|minor|major|x.y.z>");
  process.exit(1);
}
