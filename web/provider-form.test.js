// =============================================================================
// Test logika "provider terdaftar" (dipakai form edit provider).
// Dijalankan tanpa DOM: hanya fungsi murni dari web/provider-form.js.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";

import {
  envVarFor,
  envVarName,
  isRegisteredProvider,
  normalizeValue,
  registeredApi,
} from "./provider-form.js";

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

// ---------------------------------------------------------------- env var

test("nama variabel mengikuti id provider", () => {
  assert.equal(envVarName("opencode-go"), "OPENCODE_GO_API_KEY");
  assert.equal(envVarName("commandcode"), "COMMANDCODE_API_KEY");
  assert.equal(envVarName("my.endpoint"), "MY_ENDPOINT_API_KEY");
  assert.equal(envVarName("  spaced id  "), "SPACED_ID_API_KEY");
});

test("variabel mengikuti id provider, bukan envVar di presets", () => {
  // Backend memakai `env_name_from(id)`; kalau UI memakai nama lain (mis.
  // OPENCODE_API_KEY untuk id opencode-go), variabel yang disuruh export
  // bukan yang benar-benar dibaca harness.
  assert.equal(
    envVarFor({ provider: { id: "opencode-go" }, managed: true }),
    "OPENCODE_GO_API_KEY",
  );
  assert.equal(envVarFor({ provider: { id: "commandcode" }, managed: true }), "COMMANDCODE_API_KEY");
  assert.equal(envVarFor({ provider: { id: "huggingface" }, managed: true }), "HUGGINGFACE_API_KEY");
});

test("env var tidak pernah diambil dari id model", () => {
  // Bug yang dijaga: form provider terdaftar tidak punya input id provider,
  // sehingga query pertama `[data-field="id"]` adalah baris model pertama
  // (mis. claude-sonnet-5-5) dan variabelnya jadi CLAUDE_SONNET_5_5_API_KEY.
  const preset = { id: "commandcode", name: "Command Code", baseUrl: "https://x.test/v1", api: "openai-completions" };
  const provider = { ...preset };
  assert.equal(
    envVarFor({ provider, managed: true }),
    "COMMANDCODE_API_KEY",
    "harus dari id provider, bukan id model",
  );
});

test("provider custom memakai id yang sedang diketik", () => {
  assert.equal(envVarFor({ provider: null, typedId: "endpointku", managed: false }), "ENDPOINTKU_API_KEY");
  // id masih kosong: tampilkan pola, bukan "_API_KEY"
  assert.equal(envVarFor({ provider: null, typedId: "", managed: false }), "\u2039provider-id\u203a_API_KEY");
});
