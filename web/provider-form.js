// =============================================================================
// Logika form provider: apakah provider ini berasal dari katalog Hakonfig?
//
// Dipisah dari main.js supaya bisa diuji tanpa DOM. Aturannya sengaja longgar
// pada nama tampilan: pengguna sering mengganti nama provider ("Groq" jadi
// "Groq kantor") dan itu bukan alasan untuk memunculkan kembali field endpoint.
// =============================================================================

/** Samakan bentuk URL/tipe API sebelum dibandingkan. */
export function normalizeValue(value) {
  return String(value ?? "")
    .trim()
    .replace(/\/+$/, "")
    .toLowerCase();
}

/**
 * True bila provider ini dikelola katalog: id-nya ada di preset dan endpoint
 * serta tipe API-nya masih sama dengan katalog.
 *
 * `api` yang kosong dianggap cocok (banyak config lama tidak menulisnya); nilai
 * `name` tidak ikut dibandingkan.
 */
export function isRegisteredProvider(preset, provider) {
  if (!preset || !provider) return false;
  if (preset.id !== provider.id) return false;
  if (!normalizeValue(provider.baseUrl)) return false;
  if (normalizeValue(preset.baseUrl) !== normalizeValue(provider.baseUrl)) return false;

  const storedApi = normalizeValue(provider.api);
  if (!storedApi) return true;
  return normalizeValue(preset.api) === storedApi;
}

/** Tipe API yang ditulis saat menyimpan provider terdaftar. */
export function registeredApi(preset, provider) {
  return provider?.api || preset?.api || "openai-completions";
}

/**
 * Nama variabel environment untuk mode env, mis. `opencode-go` -> `OPENCODE_GO_API_KEY`.
 * Salinan aturan Rust `env_name_from` (`src-tauri/src/engine/doc.rs`).
 */
export function envVarName(id) {
  const upper = String(id ?? "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "_")
    .replace(/^_+|_+$/g, "");
  return `${upper}_API_KEY`;
}

/**
 * Variabel yang ditampilkan pada mode env.
 *
 * Selalu diturunkan dari id provider, karena backend memakai aturan yang sama
 * (`env_name_from`) saat menulis `$VAR` ke config. Memakai nama lain di UI
 * hanya akan menyesatkan: variabel yang benar-benar dibaca bukan itu.
 */
export function envVarFor({ provider, typedId, managed }) {
  const id = managed ? provider?.id : typedId ?? provider?.id;
  const trimmed = String(id ?? "").trim();
  return trimmed ? envVarName(trimmed) : "\u2039provider-id\u203a_API_KEY";
}
