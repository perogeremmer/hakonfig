// =============================================================================
// Model di wizard "Add provider".
//
// Endpoint `/models` sering lebih baru dan lebih lengkap daripada katalog
// preset (Command Code: 105 model di endpoint vs 3 di preset). Karena itu
// metadata hasil sync harus ikut sampai ke draft, bukan dibuang setelah daftar
// ditampilkan. Angka endpoint diutamakan; preset hanya cadangan.
//
// Harness Pi dan Oh My Pi memakai `contextWindow` apa adanya dan jatuh ke
// 128000 bila field itu tidak ditulis, jadi kehilangan angka di sini terlihat
// seperti batas context yang salah.
//
// Dipisah dari main.js supaya bisa diuji tanpa DOM.
// =============================================================================

/** Pretty-print id model untuk ditampilkan sebagai nama: gpt-oss-120b → GPT OSS 120B */
export function prettyModel(id) {
  return String(id)
    .replace(/[._/]/g, "-")
    .split("-")
    .filter(Boolean)
    .map((part) => (/^[a-z]/.test(part) ? part.charAt(0).toUpperCase() + part.slice(1) : part))
    .join(" ");
}

/** Map id → entri model dari katalog preset, sudah membawa context/max token. */
export function addFlowEntries(preset) {
  const entries = new Map();
  for (const model of preset?.models ?? []) {
    entries.set(model.id, {
      selected: true,
      label: model.name || prettyModel(model.id),
      contextWindow: model.contextWindow ?? null,
      maxTokens: model.maxTokens ?? null,
    });
  }
  return entries;
}

/**
 * Gabungkan satu model hasil sync ke daftar entri. Endpoint menang untuk
 * context window; nilai preset dipertahankan bila endpoint tidak mengirim
 * angka. Status centang pengguna tidak diubah.
 */
export function mergeSyncedModel(entries, model) {
  const prev = entries.get(model.id);
  entries.set(model.id, {
    selected: prev?.selected ?? true,
    label: prev?.label || prettyModel(model.id),
    contextWindow: model.contextWindow ?? prev?.contextWindow ?? null,
    maxTokens: prev?.maxTokens ?? null,
  });
}

/** Payload `models` untuk upsert: entri hasil sync dulu, preset sebagai cadangan. */
export function addFlowDrafts(entries, preset) {
  const known = new Map((preset?.models ?? []).map((model) => [model.id, model]));
  const drafts = [];
  for (const [id, entry] of entries) {
    if (!entry.selected) continue;
    const base = known.get(id);
    drafts.push({
      id,
      name: base?.name ?? prettyModel(id),
      contextWindow: entry.contextWindow ?? base?.contextWindow ?? null,
      maxTokens: entry.maxTokens ?? base?.maxTokens ?? null,
      reasoning: Boolean(base?.reasoning),
    });
  }
  return drafts;
}
