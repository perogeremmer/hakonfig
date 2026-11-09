# Kontrak UI ↔ Backend (frozen v1)

Semua komunikasi lewat `window.__TAURI__.core.invoke(namaCommand, args)`.
`withGlobalTauri: true` aktif, jadi **tidak ada bundler / import package**.

Argumen dari JS memakai **camelCase**; Rust menerima `snake_case` (Tauri melakukan konversi otomatis).
Contoh: parameter Rust `harness_id` dipanggil dari JS sebagai `harnessId`.

Semua command mengembalikan `Promise`. Bila error, promise reject dengan **string** pesan error.

---

## Tipe data

```ts
type ConfigFileInfo = {
  path: string;      // path absolut
  role: string;      // "models" | "auth" | "settings" | "env" | "config"
  exists: boolean;
  sensitive: boolean; // true = berisi API key
};

type HarnessInfo = {
  id: "pi" | "omp" | "opencode" | "hermes";
  name: string;          // "Pi", "Oh My Pi", "OpenCode", "Hermes"
  installed: boolean;    // minimal satu file config sudah ada
  experimental: boolean;
  files: ConfigFileInfo[];
  notes: string;         // catatan singkat untuk ditampilkan
  providerCount: number;   // jumlah provider di config harness ini
  connectedCount: number;  // jumlah provider yang sudah punya API key
  readError: string | null; // terisi bila config ada tapi gagal dibaca
};

type ModelSummary = {
  id: string;
  name: string | null;
  contextWindow: number | null;
  maxTokens: number | null;
  reasoning: boolean;
};

type ProviderSummary = {
  id: string;
  name: string;
  baseUrl: string | null;
  api: string | null;
  hasKey: boolean;
  keyDisplay: string | null; // mis. "$GROQ_API_KEY", "•••• 1234", "oauth", null
  models: ModelSummary[];
  enabled: boolean;    // false bila dinonaktifkan lewat daftar disable harness
  canToggle: boolean;  // true = nonaktif reversible; false = nonaktif berarti hapus
};

type HarnessSnapshot = {
  harness: string;
  files: ConfigFileInfo[];
  providers: ProviderSummary[];
  defaultProvider: string | null;
  defaultModel: string | null;
  warnings: string[];
};

type ProviderPreset = {
  id: string;
  name: string;
  baseUrl: string;
  api: string;          // "openai-completions" | "anthropic-messages" | ...
  envVar: string | null;
  docsUrl: string | null;
  keyRequired: boolean;
  notes: string;
  models: ModelSummary[];
};

type ModelDraft = {
  id: string;
  name?: string | null;
  contextWindow?: number | null;
  maxTokens?: number | null;
  reasoning?: boolean;
};

type ProviderDraft = {
  id: string;            // slug, dipakai sebagai key provider
  name: string;
  baseUrl: string;
  api: string;
  key?: string | null;   // plaintext, hanya jika user mengisi
  keyMode?: "inline" | "env"; // default "inline"
  models: ModelDraft[];
};

type EditOp =
  | ({ kind: "upsert" } & ProviderDraft)
  | { kind: "remove"; providerId: string }
  | { kind: "set_enabled"; providerId: string; enabled: boolean }
  | { kind: "set_default"; providerId: string; modelId: string };

type FileDiff = {
  path: string;
  role: string;
  sensitive: boolean;
  before: string | null; // null = file belum ada
  after: string | null;  // null = file akan dihapus (tidak dipakai saat ini)
  changed: boolean;
};

type PreviewResult = {
  files: FileDiff[];
  warnings: string[];
  willCreate: string[];
  willModify: string[];
};

type ApplyResult = {
  filesWritten: string[];
  backupDir: string | null;
  warnings: string[];
  // Config tertulis tetapi harness masih bisa menyembunyikannya; lihat Notice.
  notices: Notice[];
};

// Sesuatu yang membuat provider/model yang sudah ditulis tidak terlihat harness.
type Notice = {
  code: string; // "pi.scope" | "omp.disabled" | "opencode.disabled" | "harness.builtin-id"
  harness: string;
  provider: string;
  message: string;
  fix: string | null; // null = hanya bisa dibereskan manual
};

type BackupInfo = {
  id: string;
  createdAt: string; // ISO-8601
  dir: string;
  files: { path: string; role: string; exists: boolean }[];
};

type TestResult = {
  ok: boolean;
  status: number | null;
  message: string;
  latencyMs: number | null;
  modelsSample: string[]; // maks 12 id model dari endpoint /models
};
```

---

## Commands

| Command | Args | Return |
|---|---|---|
| `list_harnesses` | – | `HarnessInfo[]` |
| `read_harness` | `{ harnessId: string }` | `HarnessSnapshot` |
| `list_presets` | – | `ProviderPreset[]` |
| `preview_edit` | `{ harnessId: string, op: EditOp }` | `PreviewResult` |
| `apply_edit` | `{ harnessId: string, op: EditOp }` | `ApplyResult` |
| `fix_notice` | `{ harnessId: string, code: string, provider: string }` | `ApplyResult` |
| `list_backups` | `{ harnessId: string }` | `BackupInfo[]` |
| `restore_backup` | `{ harnessId: string, backupId: string }` | `ApplyResult` |
| `test_connection` | `{ baseUrl: string, api: string, key: string, harnessId?: string, providerId?: string }` | `TestResult` |
| `open_in_file_manager` | `{ path: string }` | `null` |
| `list_harness_catalog` | – | `HarnessCatalogEntry[]` |
| `harness_version` | `{ harnessId: string }` | `string \| null` |
| `install_harness` | `{ harnessId: string }` | `InstallResult` |
| `uninstall_harness` | `{ harnessId: string }` | `InstallResult` |
| `fetch_models` | `{ baseUrl: string, api: string, key: string, harnessId?: string, providerId?: string }` | `ModelOption[]` |
| `open_external` | `{ url: string }` | `null` |

```ts
type HarnessCatalogEntry = {
  id: string;
  name: string;
  kind: "adapter" | "planned"; // planned = belum ada adapter, baru bisa dipasang
  description: string;
  docsUrl: string | null;
  installUnix: string | null;
  installWindows: string | null;
  installCommand: string | null; // sudah dipilih sesuai OS; null = install manual
  verified: boolean;             // false = perintah belum diverifikasi
};

type ModelOption = { id: string; contextWindow?: number };

type InstallResult = {
  ok: boolean;
  command: string;
  output: string; // stdout+stderr, maks ~4000 karakter terakhir
};
```

### Catatan pemakaian

- **Alur simpan wajib 2 langkah**: `preview_edit` → tampilkan diff → user konfirmasi → `apply_edit`.
- `apply_edit` **selalu** membuat backup otomatis sebelum menulis.
- `preview_edit` tidak menyentuh disk (murni in-memory).
- `open_in_file_manager` membuka folder induk dari `path`.
- **`PreviewResult.files` hanya mengisi `before`/`after` untuk file yang berubah**; file lain tetap dikirim dengan `changed: false` dan isi `null` (agar payload tidak membengkak oleh katalog model yang besar).
- Untuk file dengan `sensitive: true`, isi `before`/`after` **sudah disamarkan backend**: nilai field `key`, `apiKey`, `token`, `access`, `refresh`, `secret`, `password`, `authorization` diganti `••••••••`. UI hanya menampilkan, tidak perlu menyamarkan sendiri.
- `set_enabled` **berbeda** dari `remove`:
  - Oh My Pi & OpenCode menulis ke daftar disable milik harness
    (`disabledProviders` / `disabled_providers`).
  - Pi & Hermes **mengomentari** entri provider di file yang sama (penanda
    `hakonfig:disabled <id>` … `hakonfig:end`). Data tidak hilang dan bisa
    diaktifkan kembali.
  - `remove` tetap berarti menghapus sungguhan, termasuk membuang blok komentar.
- `set_default` menulis default provider/model ke tempat masing-masing harness:

  | Harness | Ditulis ke |
  |---|---|
  | Pi | `settings.json` → `defaultProvider` + `defaultModel` (`providerId/modelId`) |
  | Oh My Pi | `config.yml` → `modelRoles.default` (`providerId/modelId`) |
  | OpenCode | `opencode.json` → `model` (`providerId/modelId`) |
  | Hermes | `config.yaml` → `model.provider` (`custom:<id>`) + `model.default` + `model.base_url` |
- `install_harness` / `uninstall_harness` memancarkan event `package-progress`
  `{ harnessId, line }` per baris output; UI menampilkan progress bar + log.
- `fetch_models` mengambil `GET {baseUrl}/models` dan mengembalikan **semua** id
  (bukan sampel), plus context window bila endpoint menyediakannya.
- `open_external` hanya menerima URL `http(s)` dan membukanya di browser default
  (`xdg-open` / `open` / `start`). URL lain ditolak.
- `ProviderPreset` kini punya `badge` (`"free"` / `"local"`) untuk chip di pemilih
  provider, dan `HarnessCatalogEntry` punya `descriptionId` (deskripsi Bahasa
  Indonesia; `description` = English).
- `harness_version` menjalankan `<bin> --version` lewat shell login (timeout 4 s)
  dan mengembalikan `null` bila tidak terdeteksi - UI harus tetap jalan tanpa versi.
- `install_harness` menjalankan perintah dari katalog (bukan input pengguna) dengan
  timeout 420 s. UI **wajib** menampilkan perintahnya lebih dulu untuk dikonfirmasi.
- `test_connection` / `fetch_models` memakai `key` yang dikirim UI; bila `key`
  kosong dan `harnessId` + `providerId` diisi, backend membaca key yang
  **tersimpan** di config harness lewat `Adapter::stored_key` (per adapter:
  Pi `auth.json` → `models.json.apiKey`, Oh My Pi `models.yml.apiKey`,
  OpenCode `auth.json` → `options.apiKey` → `env[0]`, Hermes `key_env` → `.env`).
  Referensi environment (`$VAR` / `${VAR}`) di-resolve backend. Key mentah tidak
  pernah dikirim ke UI, jadi UI cukup mengosongkan `key` bila
  `ProviderSummary.hasKey` bernilai true.
- `apply_edit` boleh dipanggil walau tidak ada perubahan; ia mengembalikan `filesWritten: []` tanpa membuat backup.
- `apply_edit` sudah merapikan sendiri hal yang bisa dirapikan: scope `enabledModels` pi dilebarkan dan provider yang ditulis dikeluarkan dari daftar disable omp/OpenCode, tampil sebagai diff biasa pada file settings. `notices` yang tersisa (mis. id bentrok dengan provider bawaan) tetap dikembalikan; notice dengan `fix` bukan null wajib ditawarkan sebagai aksi (tombol "Perbaiki" memanggil `fix_notice`), bukan hanya dicetak sebagai teks.
- `fix_notice` hanya menerima notice yang masih ada; notice yang sudah beres ditolak dengan error, jadi tombolnya aman ditekan dua kali.
- Form provider: untuk provider yang id-nya ada di `ProviderPreset` (katalog Hakonfig) **dan**
  nilai `baseUrl`/`api`/`name` masih sama dengan katalog, form hanya menampilkan API key +
  model. Endpoint ditampilkan sebagai kartu info (read-only) dan payload `upsert` tetap
  mengirim nilai katalog. Provider custom (id tidak dikenal) tetap memakai form lengkap.
- Logo provider dibaca dari `web/assets/providers/logo-index.js` (hasil `bun run logos`),
  bukan daftar manual. Menambah logo = taruh file, jalankan skrip, selesai.
- `ProviderPreset.models` boleh kosong; UI wajib membiarkan pengguna mengisi lewat
  **Sync model** (atau menambah model manual) sebelum menyimpan.
- Logo: id yang tidak terdaftar di `LOGO_IDS` memakai mark `custom.svg` (OpenAI-compatible),
  bukan monogram. Menambah logo = taruh file + daftarkan id-nya.
- Mode env memakai nama variabel `<PROVIDER_ID>_API_KEY` (aturan `env_name_from` di
  backend). UI wajib menampilkan nama itu, bukan `envVar` dari preset, supaya hint dan
  isi config tidak pernah berbeda.
- `upsert` **tanpa** `key` berarti "jangan sentuh kredensial": `apiKey`/`options.apiKey`/`env[]`/`.env`
  yang sudah ada dibiarkan apa adanya. Hanya `upsert` **dengan** `key` yang menulis ulang
  kredensial (dan membersihkan salinan lama di tempat lain). Jadi mengganti nama atau
  daftar model tidak pernah menghapus API key pengguna.

---

## Preferensi frontend (localStorage)

Disimpan di key `hakonfig.prefs` - **bukan** di backend.

| Field | Nilai | Default | Efek |
|---|---|---|---|
| `lang` | `"en"` \| `"id"` | `"en"` | Bahasa UI; `id.js` fallback ke `en.js` |
| `theme` | `"light"` \| `"dark"` | `"light"` | Kelas `dark` di `<html>` |
| `confirmDelete` | `boolean` | `true` | Dialog konfirmasi sebelum hapus provider |

Tidak ada auto-detect bahasa dari locale OS: default selalu English agar
dokumentasi dan screenshot konsisten.

## i18n

- `web/i18n/en.js` = sumber kebenaran; `web/i18n/id.js` wajib punya key yang sama.
- `t(key, vars)` mengganti `{var}`; key yang hilang jatuh ke English.
- `setLang(lang)` juga memperbarui `<html lang>`; UI di-render ulang tanpa restart.
- Paritas key dijaga test `web/i18n.test.js` (jalan tanpa DOM/jaringan):
  key hilang, nilai kosong, placeholder berbeda, dan terjemahan yang lupa
  diterjemahkan (kecuali kata serapan yang didaftarkan eksplisit).

## Tema

- Token didefinisikan sebagai CSS variable di `@theme` (Tailwind v4); `.dark`
  hanya menimpa nilainya, sehingga markup memakai kelas yang sama di kedua tema.
- Kontras terukur: `ink`/surface 16.4:1 · putih di `brand` 7.9:1 (terang) dan
  5.8:1 (gelap) · teks link `brand-text` 9.5:1 / 8.0:1.
