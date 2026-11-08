// =============================================================================
// Test paritas terjemahan: setiap key di en.js harus ada di id.js.
// Dijalankan dengan `node --test web/` (tanpa DOM, tanpa Tauri).
// =============================================================================

import test from "node:test";
import assert from "node:assert/strict";

// i18n/index.js menyentuh `document` saat setLang dipanggil, tapi import saja aman
// karena aksesnya lazy. Tetap beri stub minimal agar jelas.
globalThis.document = globalThis.document ?? {
  documentElement: { setAttribute() {}, classList: { toggle() {} } },
};

const en = (await import("./i18n/en.js")).default;
const id = (await import("./i18n/id.js")).default;

test("id.js has every key that en.js has", () => {
  const missing = Object.keys(en).filter((key) => !(key in id));
  assert.deepEqual(missing, [], `key belum diterjemahkan: ${missing.join(", ")}`);
});

test("id.js has no foreign keys", () => {
  const extra = Object.keys(id).filter((key) => !(key in en));
  assert.deepEqual(extra, [], `key tidak dipakai di en.js: ${extra.join(", ")}`);
});

test("every translation is non-empty", () => {
  const empty = Object.entries({ en, id })
    .flatMap(([lang, dict]) =>
      Object.entries(dict)
        .filter(([, value]) => !String(value).trim())
        .map(([key]) => `${lang}:${key}`),
    );
  assert.deepEqual(empty, [], `nilai kosong: ${empty.join(", ")}`);
});

test("placeholders {var} match in both languages", () => {
  const vars = (text) => [...String(text).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  const mismatched = Object.keys(en).filter(
    (key) => JSON.stringify(vars(en[key])) !== JSON.stringify(vars(id[key])),
  );
  assert.deepEqual(mismatched, [], `placeholder berbeda: ${mismatched.join(", ")}`);
});

test("UI copy does not use an em dash", () => {
  // Aturan copy: pakai titik, koma, atau titik dua. Em dash (U+2014) dilarang
  // karena bikin kalimat terasa seperti teks tempelan.
  const offenders = Object.entries({ en, id })
    .flatMap(([lang, dict]) =>
      Object.entries(dict)
        .filter(([, value]) => String(value).includes("\u2014"))
        .map(([key]) => `${lang}:${key}`),
    );
  assert.deepEqual(offenders, [], `masih memakai em dash: ${offenders.join(", ")}`);
});

test("no translation is identical because it was left untranslated", () => {
  // Kata serapan yang memang sama di kedua bahasa: sengaja tidak diterjemahkan.
  const intentionallySame = new Set([
    "harness.uninstall",
    "install.title",
    "install.run",
    "uninstall.title",
    "uninstall.run",
    "file.env",
    "status.providersOne",
    "provider.modelsOne",
    "default.provider",
    "default.model",
    "advanced.sectionEndpoint",
    "advanced.sectionKey",
    "advanced.baseUrl",
    "advanced.modelContext",
    "advanced.modelMax",
    "advanced.addManual",
    "advanced.modelReasoning",
    "docs",
    "install.uninstallShort",
    "cap.vision",
    "cap.reasoning",
    "cap.text",
    "advanced.modelMax",
    "advanced.addManual",
    "advanced.modelReasoning",
    // nama produk dan merek browser
    "web.badge",
    "web.pwa.chrome.title",
  ]);

  const untouched = Object.keys(en).filter(
    (key) => en[key] === id[key] && !intentionallySame.has(key),
  );
  assert.deepEqual(untouched, [], `belum diterjemahkan: ${untouched.join(", ")}`);
});
