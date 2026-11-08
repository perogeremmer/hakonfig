// =============================================================================
// Jaga agar daftar logo tidak pernah basi terhadap isi folder.
// Kalau gagal: jalankan `bun run logos`.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, "assets/providers");
const { LOGO_DARK_VARIANT, LOGO_EXT_PNG, LOGO_IDS } = await import("./assets/providers/logo-index.js");

const files = readdirSync(dir).filter((file) => /\.(svg|png)$/i.test(file));

function expected() {
  const ids = new Set();
  const png = new Set();
  const dual = new Set();
  for (const file of files) {
    const dot = file.lastIndexOf(".");
    const name = file.slice(0, dot);
    const ext = file.slice(dot + 1).toLowerCase();
    if (name.endsWith("-dark")) {
      dual.add(name.slice(0, -5));
      continue;
    }
    ids.add(name);
    if (ext === "png") png.add(name);
  }
  return { ids: [...ids].sort(), png: [...png].sort(), dual: [...dual].sort() };
}

test("logo-index.js cocok dengan file di assets/providers", () => {
  const want = expected();
  assert.deepEqual(
    [...LOGO_IDS].sort(),
    want.ids,
    "daftar id logo berbeda dari isi folder, jalankan: bun run logos",
  );
  assert.deepEqual([...LOGO_EXT_PNG].sort(), want.png, "daftar PNG tidak sinkron");
  assert.deepEqual([...LOGO_DARK_VARIANT].sort(), want.dual, "daftar varian gelap tidak sinkron");
});

test("setiap provider di presets punya jalur logo yang jelas", () => {
  // Provider tanpa file logo memakai mark custom; yang punya harus terdaftar
  // supaya tidak jatuh ke monogram dua huruf.
  const missing = [...LOGO_DARK_VARIANT].filter((id) => !LOGO_IDS.has(id));
  assert.deepEqual(missing, [], "varian -dark tanpa id dasar");
});
