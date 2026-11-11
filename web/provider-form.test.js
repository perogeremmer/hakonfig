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
  keyEnvName,
  keyFromEnv,
  keyModeFor,
  normalizeValue,
  registeredApi,
} from "./provider-form.js";

const groq = { id: "groq", name: "Groq", baseUrl: "https://api.groq.com/openai/v1", api: "openai-completions" };

test("catalog provider is recognized even when renamed", () => {
  // Bug yang dijaga: dulu `name` ikut dibandingkan, sehingga mengganti nama
  // tampilan memunculkan kembali field URL/ID/Jenis API.
  assert.equal(
    isRegisteredProvider(groq, { ...groq, name: "Groq (punyaku)" }),
    true,
    "nama tampilan tidak boleh memengaruhi deteksi",
  );
});

test("manually changed endpoint is treated as a custom provider", () => {
  assert.equal(
    isRegisteredProvider(groq, { ...groq, baseUrl: "https://proxy.example.com/v1" }),
    false,
  );
});

test("empty api still counts as a match", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, api: "" }), true);
  assert.equal(isRegisteredProvider(groq, { ...groq, api: undefined }), true);
});

test("different api is treated as a custom provider", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, api: "anthropic-messages" }), false);
});

test("id missing from the catalog is not a registered provider", () => {
  assert.equal(isRegisteredProvider(null, groq), false);
  assert.equal(isRegisteredProvider({ ...groq, id: "lain" }, groq), false);
});

test("small URL shape differences do not break detection", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "https://api.groq.com/openai/v1/" }), true);
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "  https://API.groq.com/openai/v1  " }), true);
  assert.equal(normalizeValue("HTTPS://A.test/v1//"), "https://a.test/v1");
});

test("provider without a baseUrl is not a registered provider", () => {
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: "" }), false);
  assert.equal(isRegisteredProvider(groq, { ...groq, baseUrl: undefined }), false);
});

test("API type on save uses the catalog value when empty", () => {
  assert.equal(registeredApi(groq, { ...groq, api: "" }), "openai-completions");
  assert.equal(registeredApi(groq, { ...groq, api: "google-generative-ai" }), "google-generative-ai");
  assert.equal(registeredApi(null, {}), "openai-completions");
});

// ---------------------------------------------------------------- env var

test("variable name follows the provider id", () => {
  assert.equal(envVarName("opencode-go"), "OPENCODE_GO_API_KEY");
  assert.equal(envVarName("commandcode"), "COMMANDCODE_API_KEY");
  assert.equal(envVarName("my.endpoint"), "MY_ENDPOINT_API_KEY");
  assert.equal(envVarName("  spaced id  "), "SPACED_ID_API_KEY");
});

test("variable follows the provider id, not the presets envVar", () => {
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

test("env var is never taken from a model id", () => {
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

test("custom provider uses the id being typed", () => {
  assert.equal(envVarFor({ provider: null, typedId: "endpointku", managed: false }), "ENDPOINTKU_API_KEY");
  // id masih kosong: tampilkan pola, bukan "_API_KEY"
  assert.equal(envVarFor({ provider: null, typedId: "", managed: false }), "\u2039provider-id\u203a_API_KEY");
});

// ------------------------------------------------------- mode key (env/file)

test("env key preselects the environment option, not inline", () => {
  const provider = { keyMode: "env", keyEnv: "COMMANDCODE_API_KEY", keyDisplay: "$COMMANDCODE_API_KEY" };
  assert.equal(keyModeFor(provider), "env");
  assert.equal(keyFromEnv(provider), true);
  assert.equal(keyEnvName(provider), "COMMANDCODE_API_KEY");
});

test("stored key preselects the inline option", () => {
  const provider = { keyMode: "stored", keyDisplay: "sk-…abcd" };
  assert.equal(keyModeFor(provider), "inline");
  assert.equal(keyFromEnv(provider), false);
});

test("older summaries without keyMode still work through keyDisplay", () => {
  assert.equal(keyModeFor({ keyDisplay: "$GROQ_API_KEY" }), "env");
  assert.equal(keyModeFor({ keyDisplay: "${GROQ_API_KEY} (not set)" }), "env");
  assert.equal(keyEnvName({ keyDisplay: "${GROQ_API_KEY} (not set)" }), "GROQ_API_KEY");
  assert.equal(keyModeFor({ keyDisplay: "sk-…1234" }), "inline");
  assert.equal(keyModeFor({}), "inline");
  assert.equal(keyEnvName({}), null);
});
