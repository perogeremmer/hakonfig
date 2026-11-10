// =============================================================================
// Ratchet skala UI: ukuran teks harus lewat token rem.
//
// px tidak mengikuti preferensi ukuran font sistem, rem ikut. Konversi 147
// kelas px tidak ada artinya kalau kelas baru boleh muncul lagi besok, jadi
// test ini gagal begitu `text-[13px]` kembali. Ukuran kotak (max-w, h) juga
// diminta rem untuk alasan yang sama.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const webFiles = readdirSync(here).filter((name) => name.endsWith(".js") && !name.endsWith(".test.js"));

test("no hardcoded text size in px", () => {
  const offenders = [];
  for (const name of webFiles) {
    const text = readFileSync(join(here, name), "utf8");
    for (const match of text.matchAll(/text-\[[\d.]+px\]/g)) {
      const line = text.slice(0, match.index).split("\n").length;
      offenders.push(`${name}:${line} ${match[0]}`);
    }
  }
  assert.deepEqual(offenders, [], `pakai token rem (text-xs, text-base, ...): ${offenders.join(", ")}`);
});

test("no hardcoded box size in px", () => {
  const offenders = [];
  for (const name of webFiles) {
    const text = readFileSync(join(here, name), "utf8");
    for (const match of text.matchAll(/\b(?:max-w|min-w|max-h|min-h|w|h)-\[[\d.]+px\]/g)) {
      const line = text.slice(0, match.index).split("\n").length;
      offenders.push(`${name}:${line} ${match[0]}`);
    }
  }
  assert.deepEqual(offenders, [], `pakai rem, mis. h-[9.25rem]: ${offenders.join(", ")}`);
});

test("styles source defines the rem type ramp", () => {
  const css = readFileSync(join(here, "styles.src.css"), "utf8");
  assert.match(css, /@theme\s*\{/, "ramp tipografi harus ada di @theme");
  for (const token of ["2xs", "xs", "sm", "base", "md", "lg", "xl"]) {
    assert.match(css, new RegExp(`--text-${token}: [\\d.]+rem`), `token --text-${token} harus rem`);
  }
  const pxFontSizes = css.match(/font-size: [\d.]+px/g) ?? [];
  assert.deepEqual(pxFontSizes, [], `font-size harus rem: ${pxFontSizes.join(", ")}`);
});
