#!/usr/bin/env node
// Shim `hakonfig-cli` untuk package npm `hakonfig`.
//
// Binari semua platform ikut di dalam package ini (lihat scripts/publish-npm.mjs
// untuk alasan kenapa tidak dipecah per platform); shim ini memilih binari yang
// cocok dengan platform saat ini dan meneruskan seluruh argumen.
"use strict";

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

// Harus sama dengan BINARIES di scripts/publish-npm.mjs.
const BINARIES = {
  darwin: { file: "hakonfig-cli-darwin", arch: ["x64", "arm64"] },
  linux: { file: "hakonfig-cli-linux-x64", arch: ["x64"] },
  win32: { file: "hakonfig-cli-win32-x64.exe", arch: ["x64"] },
};

const target = BINARIES[process.platform];
if (!target) {
  console.error(
    `hakonfig: platform ${process.platform} tidak didukung. ` +
      `Pakai installer dari https://github.com/perogeremmer/hakonfig/releases`
  );
  process.exit(1);
}
if (!target.arch.includes(process.arch)) {
  console.error(
    `hakonfig: arsitektur ${process.platform}/${process.arch} belum didukung ` +
      `(tersedia: ${target.arch.join(", ")}). ` +
      `Pakai installer dari https://github.com/perogeremmer/hakonfig/releases`
  );
  process.exit(1);
}

const bin = join(__dirname, "bin", target.file);
if (!existsSync(bin)) {
  console.error(
    `hakonfig: binari ${target.file} tidak ada di package ini. ` +
      `Coba install ulang: npm i -g hakonfig`
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`hakonfig: gagal menjalankan binari: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
