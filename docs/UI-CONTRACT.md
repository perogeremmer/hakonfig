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
  name: string;          // "Pi", "oh-my-pi", "OpenCode", "Hermes"
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
  | { kind: "set_enabled"; providerId: string; enabled: boolean };

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
| `list_backups` | `{ harnessId: string }` | `BackupInfo[]` |
| `restore_backup` | `{ harnessId: string, backupId: string }` | `ApplyResult` |
| `test_connection` | `{ baseUrl: string, api: string, key: string }` | `TestResult` |
| `open_in_file_manager` | `{ path: string }` | `null` |

### Catatan pemakaian

- **Alur simpan wajib 2 langkah**: `preview_edit` → tampilkan diff → user konfirmasi → `apply_edit`.
- `apply_edit` **selalu** membuat backup otomatis sebelum menulis.
- `preview_edit` tidak menyentuh disk (murni in-memory).
- `open_in_file_manager` membuka folder induk dari `path`.
- **`PreviewResult.files` hanya mengisi `before`/`after` untuk file yang berubah**; file lain tetap dikirim dengan `changed: false` dan isi `null` (agar payload tidak membengkak oleh katalog model yang besar).
- Untuk file dengan `sensitive: true`, isi `before`/`after` **sudah disamarkan backend**: nilai field `key`, `apiKey`, `token`, `access`, `refresh`, `secret`, `password`, `authorization` diganti `••••••••`. UI hanya menampilkan, tidak perlu menyamarkan sendiri.
- `set_enabled` hanya berlaku untuk harness dengan daftar disable (oh-my-pi:
  `disabledProviders`, OpenCode: `disabled_providers`); harness lain menolak
  lewat error, jadi UI harus memakai alur `remove` (dengan konfirmasi).
- `apply_edit` boleh dipanggil walau tidak ada perubahan; ia mengembalikan `filesWritten: []` tanpa membuat backup.
