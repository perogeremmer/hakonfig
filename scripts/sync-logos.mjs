#!/usr/bin/env node
// =============================================================================
// Hasilkan daftar logo provider dari isi folder assets.
//
// UI perlu tahu id mana yang punya file logo (supaya provider custom langsung
// memakai mark OpenAI-compatible, bukan monogram). Daftar itu tidak boleh
// dirawat manual: setiap kali logo ditambah, jalankan skrip ini.
//
//   node scripts/sync-logos.mjs
// =============================================================================

import { readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dir = join(root, "web/assets/providers");

const files = readdirSync(dir).filter((f) => /\.(svg|png)$/i.test(f));

const ids = new Set();
const png = new Set();
const dual = new Set();

for (const file of files) {
  const [name, ext] = file.split(/\.(?=[^.]+$)/);
  if (name.endsWith("-dark")) {
    dual.add(name.slice(0, -"-dark".length));
    continue;
  }
  ids.add(name);
  if (ext.toLowerCase() === "png") png.add(name);
}

// Ikon yang punya varian terang/hitam untuk tema gelap tetap butuh id dasarnya.
for (const id of dual) ids.add(id);

const sorted = [...ids].sort();
const list = (values) => values.map((v) => `  ${JSON.stringify(v)},`).join("\n");

const output = `// Dibuat oleh scripts/sync-logos.mjs - jangan diedit manual.
// Regenerate: node scripts/sync-logos.mjs

/** Id yang punya file logo di web/assets/providers/. */
export const LOGO_IDS = new Set([
${list(sorted)}
]);

/** Id yang logonya PNG (sisanya SVG). */
export const LOGO_EXT_PNG = new Set([
${list([...png].sort())}
]);

/** Id yang punya varian "-dark" untuk tema gelap. */
export const LOGO_DARK_VARIANT = new Set([
${list([...dual].sort())}
]);
`;

writeFileSync(join(root, "web/assets/providers/logo-index.js"), output);
console.log(`logo: ${sorted.length} id, ${png.size} png, ${dual.size} varian gelap`);
