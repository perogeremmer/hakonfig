// =============================================================================
// i18n - English default, Bahasa Indonesia opsional (fallback ke en).
// Tidak ada auto-detect dari locale OS: default selalu "en" agar dokumentasi
// dan screenshot konsisten.
// =============================================================================

import en from "./en.js";
import id from "./id.js";

const DICTS = { en, id };
export const LANGS = ["en", "id"];

let current = "en";

/** Ganti bahasa aktif. Mengembalikan true bila berubah. */
export function setLang(lang) {
  const next = LANGS.includes(lang) ? lang : "en";
  const changed = next !== current;
  current = next;
  document.documentElement.setAttribute("lang", next);
  return changed;
}

export function getLang() {
  return current;
}

/**
 * Terjemahkan `key`, ganti `{var}` dengan nilai dari `vars`.
 * Key yang tidak ada di bahasa aktif jatuh ke English; bila dua-duanya tidak
 * ada, kembalikan key-nya supaya kelihatan saat QA (tidak pernah crash).
 */
export function t(key, vars) {
  const dict = DICTS[current] ?? en;
  let text = dict[key] ?? en[key] ?? key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.replaceAll(`{${name}}`, String(value ?? ""));
    }
  }
  return text;
}

/** Tanggal mengikuti bahasa aktif. */
export function fmtDate(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return String(iso ?? "");
  return date.toLocaleString(current === "id" ? "id-ID" : "en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

/** Semua key yang tersedia - dipakai test paritas en/id. */
export const EN_KEYS = Object.keys(en);
export const ID_KEYS = Object.keys(id);
