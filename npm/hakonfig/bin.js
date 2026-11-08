#!/usr/bin/env node
// Shim `hakonfig-cli` untuk package npm `hakonfig`.
//
// Binary asli hidup di package platform (hakonfig-darwin, hakonfig-linux-x64,
// hakonfig-win32-x64) sebagai optionalDependencies; shim ini meneruskan
// seluruh argumen ke binary yang cocok dengan platform saat ini.
"use strict";

const { spawnSync } = require("node:child_process");

const PLATFORMS = {
  darwin: { pkg: "hakonfig-darwin", ext: "" },
  linux: { pkg: "hakonfig-linux-x64", ext: "" },
  win32: { pkg: "hakonfig-win32-x64", ext: ".exe" },
};

const target = PLATFORMS[process.platform];
if (!target) {
  console.error(
    `hakonfig: platform ${process.platform}/${process.arch} tidak didukung.`
  );
  process.exit(1);
}

let bin;
try {
  bin = require.resolve(`${target.pkg}/bin/hakonfig-cli${target.ext}`);
} catch {
  console.error(
    `hakonfig: binary untuk platform ini tidak ditemukan (${target.pkg}).` +
      ` Coba install ulang: npm i -g hakonfig`
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`hakonfig: gagal menjalankan binary: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
