// =============================================================================
// Ratchet urutan resolusi modal konfirmasi.
//
// `openModal` memanggil `onClose` di dalam `close()`, dan `confirmDialog`
// memakai `onClose: () => resolve(false)`. Promise hanya memakai resolusi
// pertama, jadi kalau handler `[data-confirm]` memanggil `close()` sebelum
// `resolve(true)`, setiap konfirmasi dibaca sebagai batal — tombolnya seolah
// tidak berfungsi. Itu bug nyata yang membuat "Forget key" diam saja, dan test
// ini menjaga urutannya.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, "main.js"), "utf8");

test("confirm handler resolves before closing the modal", () => {
  const start = source.indexOf('$("[data-confirm]", overlay)');
  assert.ok(start > 0, "handler [data-confirm] harus ada");

  const block = source.slice(start, start + 600);
  const resolveAt = block.indexOf("resolve(true)");
  const closeAt = block.indexOf("close();");

  assert.ok(resolveAt > 0, "handler harus resolve(true)");
  assert.ok(closeAt > 0, "handler harus menutup modal");
  assert.ok(
    resolveAt < closeAt,
    "resolve(true) harus dipanggil SEBELUM close(): close() memicu onClose -> resolve(false)",
  );
});

test("confirmDialog maps onClose to a cancel", () => {
  assert.ok(
    /onClose:\s*\(\)\s*=>\s*resolve\(false\)/.test(source),
    "onClose harus me-resolve false supaya Escape/klik luar dihitung batal",
  );
});
