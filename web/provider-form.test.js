// =============================================================================
// Test logika "provider terdaftar" (dipakai form edit provider).
// Dijalankan tanpa DOM: hanya fungsi murni dari web/provider-form.js.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";

import { isRegisteredProvider, normalizeValue, registeredApi } from "./provider-form.js";

const groq = { id: "groq", name: "Groq", baseUrl: "https://api.groq.com/openai/v1", api: "openai-completions" };

test("provider dari katalog dikenali walau namanya diganti", () => {
  // Bug yang dijaga: dulu `name` ikut dibandingkan, sehingga mengganti nama
  // tampilan memunculkan kembali field URL/ID/Jenis API.
  assert.equal(
    isRegisteredProvider(groq, { ...groq, name: "Groq (punyaku)" }),
    true,
    "nama tampilan tidak boleh memengaruhi deteksi",
  );
});

test("endpoint yang diubah manual dianggap provider custom", () => {
  assert.equal(
    isRegisteredProvider(groq, { ...groq, baseUrl: "https://proxy.example.com/v1" }),
    false,
  );
});

test("api kosong tetap dianggap cocok", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, api: "" }), true);
  assert.equal(isRegisteredProvider(groq, { ...groq, api: undefined }), true);
});

test("api yang berbeda dianggap provider custom", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, api: "anthropic-messages" }), false);
});

test("id yang tidak ada di katalog bukan provider terdaftar", () => {
  assert.equal(isRegisteredProvider(null, groq), false);
  assert.equal(isRegisteredProvider({ ...groq, id: "lain" }, groq), false);
});

test("perbedaan bentuk URL kecil tidak membatalkan deteksi", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "https://api.groq.com/openai/v1/" }), true);
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "  https://API.groq.com/openai/v1  " }), true);
  assert.equal(normalizeValue("HTTPS://A.test/v1//"), "https://a.test/v1");
});

test("provider tanpa baseUrl bukan provider terdaftar", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "" }), false);
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: undefined }), false);
});

test("tipe API saat menyimpan memakai nilai katalog bila kosong", () => {
  assert.equal(registeredApi(groq, { ...groq, api: "" }), "openai-completions");
  assert.equal(registeredApi(groq, { ...groq, api: "google-generative-ai" }), "google-generative-ai");
  assert.equal(registeredApi(null, {}), "openai-completions");
});
