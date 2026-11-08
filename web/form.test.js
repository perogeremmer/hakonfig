// =============================================================================
// Test struktural: handler form tidak boleh membaca DOM setelah form ditutup.
//
// Bug yang dijaga: `close()` memanggil `render()`, yang mengganti isi #content.
// Setelah itu `field("api")` mengembalikan null dan simpan gagal dengan
// "TypeError: null is not an object". Semua nilai harus dibaca sebelum close().
// Dijalankan tanpa DOM/Tauri: hanya membaca sumber web/main.js.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("./main.js", import.meta.url), "utf8");

/**
 * Ambil blok handler mulai dari `marker` sampai kurung kurawal penutupnya
 * seimbang. Cukup untuk file ini karena handler memakai `() => { ... }`.
 */
function handlerBlock(marker) {
  const start = source.indexOf(marker);
  assert.notEqual(start, -1, `marker tidak ditemukan: ${marker}`);
  let depth = 0;
  let seenBrace = false;
  for (let i = start; i < source.length; i += 1) {
    const char = source[i];
    if (char === "{") {
      depth += 1;
      seenBrace = true;
    } else if (char === "}") {
      depth -= 1;
      if (seenBrace && depth === 0) return source.slice(start, i + 1);
    }
  }
  assert.fail(`blok tidak tertutup: ${marker}`);
}

const formSave = handlerBlock('[data-save]", overlay).addEventListener("click", async () => {');

test("provider save handler reads every field before close()", () => {
  const closeAt = formSave.indexOf("close();");
  assert.notEqual(closeAt, -1, "handler harus memanggil close()");

  const afterClose = formSave.slice(closeAt + "close();".length);
  const lateReads = [...afterClose.matchAll(/field\("(\w+)"\)/g)].map((match) => match[1]);

  assert.deepEqual(
    lateReads,
    [],
    `field dibaca setelah close() (akan null): ${lateReads.join(", ")}`,
  );
});

test("upsert payload uses copies of values, not direct DOM reads", () => {
  // Nilai endpoint diambil lewat helper, karena provider dari katalog tidak
  // merender field baseUrl/api/name sama sekali.
  assert.match(formSave, /const api = endpointApi\(\);/, "api harus lewat endpointApi()");
  assert.match(formSave, /const baseUrl = endpointUrl\(\);/, "baseUrl harus lewat endpointUrl()");
  assert.match(formSave, /const name = providerName\(\);/, "name harus lewat providerName()");
  assert.doesNotMatch(formSave, /api: field\(/, "payload tidak boleh memanggil field()");
  assert.doesNotMatch(formSave, /key: field\(/, "payload tidak boleh memanggil field()");
});

test("model draft is read before the form closes", () => {
  const closeAt = formSave.indexOf("close();");
  const builtAt = formSave.indexOf("readModels()");
  assert.notEqual(builtAt, -1, "handler harus memanggil readModels()");
  assert.ok(builtAt < closeAt, "readModels() harus dipanggil sebelum close()");
});

test("provider id input does not use data-field=\"id\"", () => {
  // Baris model juga memakai data-field="id". Kalau input id provider memakai
  // nama yang sama, `field("id")` bisa mengambil model pertama (lihat bug
  // variabel environment CLAUDE_SONNET_5_5_API_KEY) dan id provider bisa salah
  // terbaca saat menyimpan.
  assert.match(source, /data-field="providerId"/, "input id provider harus pakai providerId");
  assert.doesNotMatch(source, /field\("id"\)/, "jangan query field(\"id\") tanpa scope baris model");
});
