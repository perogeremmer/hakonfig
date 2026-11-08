// =============================================================================
// Aplikasi harus jalan tanpa internet: semua aset (font, logo, ikon, CSS)
// dibundel lokal. Yang boleh memanggil jaringan hanya aksi eksplisit pengguna
// (Test koneksi, Sync model, Install tool).
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");

test("index.html tidak memuat aset dari jaringan", () => {
  const html = read("./index.html");
  const remote = [...html.matchAll(/(?:src|href)\s*=\s*"(https?:\/\/[^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(remote, [], `aset eksternal: ${remote.join(", ")}`);
});

test("CSS tidak mengimpor berkas dari jaringan", () => {
  const css = read("./styles.src.css");
  const remote = [...css.matchAll(/url\(\s*["']?(https?:\/\/[^)"']+)/g)].map((m) => m[1]);
  const imports = [...css.matchAll(/@import\s+[^;]*https?:\/\//g)].map((m) => m[0]);
  assert.deepEqual([...remote, ...imports], [], "CSS memuat sumber eksternal");
});

test("main.js tidak memuat gambar/script dari jaringan", () => {
  const js = read("./main.js");
  // Tautan dokumentasi (dibuka di browser lewat open_external) tidak dihitung.
  const remote = [...js.matchAll(/(?:<img[^>]+src|<script[^>]+src|<link[^>]+href)\s*=\s*["'](https?:\/\/[^"']+)/g)]
    .map((m) => m[1]);
  assert.deepEqual(remote, [], `aset eksternal di JS: ${remote.join(", ")}`);
});

test("font yang dipakai benar-benar ada di web/fonts", () => {
  const css = read("./styles.src.css");
  const faces = [...css.matchAll(/url\("\.\/fonts\/([^"]+)"\)/g)].map((m) => m[1]);
  assert.ok(faces.length >= 8, `hanya ${faces.length} font-face`);
  for (const file of faces) {
    const content = readFileSync(new URL(`./fonts/${file}`, import.meta.url));
    assert.ok(content.byteLength > 1000, `${file} terlalu kecil, mungkin placeholder`);
  }
});
