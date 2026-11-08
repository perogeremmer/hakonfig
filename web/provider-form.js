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
