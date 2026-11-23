// =============================================================================
// Test wizard "Add provider": metadata model dari endpoint tidak boleh hilang.
//
// Kasus nyata: Command Code punya 105 model di `/models` (76 di antaranya
// ≥ 1 juta token), tetapi preset hanya mendaftar 3. Kalau context window dari
// endpoint tidak ikut ke draft, Pi menulis model tanpa `contextWindow` dan
// memakai default 128000.
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";

import { addFlowDrafts, addFlowEntries, mergeSyncedModel, prettyModel } from "./add-models.js";

const preset = {
  id: "commandcode",
  name: "Command Code",
  baseUrl: "https://api.commandcode.ai/provider/v1",
  api: "openai-completions",
  models: [
    { id: "gpt-6-astra", name: "GPT-6 Astra", contextWindow: 1050000 },
    { id: "old-model", name: "Old Model", contextWindow: 128000, maxTokens: 8192, reasoning: true },
  ],
};

test("preset entries carry their context window and max tokens", () => {
  const entries = addFlowEntries(preset);
  assert.deepEqual(entries.get("gpt-6-astra"), {
    selected: true,
    label: "GPT-6 Astra",
    contextWindow: 1050000,
    maxTokens: null,
  });
  assert.equal(entries.get("old-model").maxTokens, 8192);
});

test("endpoint context window wins over the preset", () => {
  const entries = addFlowEntries(preset);
  mergeSyncedModel(entries, { id: "old-model", contextWindow: 262144 });
  assert.equal(entries.get("old-model").contextWindow, 262144);
  assert.equal(entries.get("old-model").label, "Old Model");
});

test("a model missing from the preset keeps the endpoint context window", () => {
  const entries = addFlowEntries(preset);
  mergeSyncedModel(entries, { id: "claude-sonnet-5-5", contextWindow: 1000000 });
  const drafts = addFlowDrafts(entries, preset);
  const draft = drafts.find((model) => model.id === "claude-sonnet-5-5");
  assert.equal(draft.contextWindow, 1000000);
  assert.equal(draft.name, "Claude Sonnet 5 5");
  assert.equal(draft.reasoning, false);
});

test("endpoint without a context window does not erase the preset value", () => {
  const entries = addFlowEntries(preset);
  mergeSyncedModel(entries, { id: "old-model", contextWindow: null });
  assert.equal(entries.get("old-model").contextWindow, 128000);
  assert.equal(entries.get("old-model").maxTokens, 8192);
});

test("no number anywhere stays null so the harness default is explicit", () => {
  const entries = addFlowEntries({ models: [] });
  mergeSyncedModel(entries, { id: "mystery-model", contextWindow: null });
  const [draft] = addFlowDrafts(entries, preset);
  assert.equal(draft.contextWindow, null);
  assert.equal(draft.maxTokens, null);
});

test("sync keeps the user's checkbox state", () => {
  const entries = addFlowEntries(preset);
  entries.get("gpt-6-astra").selected = false;
  mergeSyncedModel(entries, { id: "gpt-6-astra", contextWindow: 1050000 });
  const ids = addFlowDrafts(entries, preset).map((model) => model.id);
  assert.deepEqual(ids, ["old-model"]);
});

test("prettyModel formats ids for display", () => {
  assert.equal(prettyModel("zai-org/GLM-5.3"), "Zai Org GLM 5 3");
});
