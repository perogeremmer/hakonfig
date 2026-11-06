# PRD — Rekonfig

> **Rekonfig** — Aplikasi desktop untuk membaca, mengubah, dan menulis konfigurasi provider/model pada AI coding harness (OpenCode, Pi, Hermes, oh-my-pi, dll) tanpa harus "akrobat" edit file di file explorer.

- **Status:** Draft v1
- **Stack final:** Tauri v2 (Rust) + HTML + Tailwind CSS (tanpa frontend framework) + vanilla JS
- **Target platform:** macOS, Windows, Linux

---

## 1. Ringkasan Eksekutif

Mahasiswa/pengguna sering mendapat API key gratis (Groq, Gemini, DeepSeek, dll), tetapi kesulitan mengonfigurasinya ke dalam harness AI coding karena:

1. Setiap harness menyimpan konfigurasi di lokasi dan format yang berbeda.
2. Konfigurasi tersebar di **beberapa file** (bukan satu file): misal Pi memisahkan `models.json` (pakai referensi `$ENV_VAR`) dari `auth.json` (key asli).
3. Pengguna akhirnya mengedit file JSON/YAML secara manual, yang rawan merusak konfigurasi — dibuktikan dengan banyaknya file backup manual (`.bak`, `.bak2`, `.bak.peer-*`).

**Rekonfig** adalah GUI yang membaca konfigurasi saat ini, menampilkannya dalam bentuk yang mudah dipahami, dan menulis perubahan dengan aman (merge + backup otomatis).

---

## 2. Latar Belakang & Masalah

### 2.1 Kondisi aktual (hasil observasi)

| Harness | File model/provider | Tempat key disimpan | Catatan |
|---|---|---|---|
| Pi | `~/.pi/agent/models.json` | `~/.pi/agent/auth.json` (terpisah) | `models.json` pakai `$ENV_VAR` sebagai referensi key |
| OpenCode | `~/.config/opencode/opencode.json` | auth store milik OpenCode (`~/.local/share/opencode/`) | skema beda total |
| Hermes | *(perlu diteliti)* | *(perlu diteliti)* | fork OpenCode |
| oh-my-pi | *(perlu diteliti)* | *(perlu diteliti)* | *(perlu diteliti)* |

### 2.2 Pain point konkret

- Pengguna tidak tahu **di mana** file konfigurasi berada.
- Pengguna tidak tahu **skema/format** yang benar per harness.
- Pengguna tidak tahu bahwa key dan model bisa **terpisah di file berbeda** (Pi).
- Edit manual menghasilkan banyak file backup tidak terkelola dan konfigurasi rusak.
- Provider gratis sering "tidak terdeteksi" sehingga perlu menambah provider/model secara manual.

---

## 3. Tujuan & Sasaran

### 3.1 Tujuan

1. Mengurangi waktu dan kesalahan saat menambah/mengubah provider AI di harness.
2. Menyediakan satu GUI untuk **membaca** dan **menulis** konfigurasi berbagai harness.
3. Menjamin keamanan perubahan: **selalu backup**, **selalu merge** (tidak overwrite).
4. Memberikan preset provider gratis (Groq, Gemini, DeepSeek, dll) agar pengguna tinggal tempel API key.

### 3.2 Non-Goals (bukan ini)

- ❌ Bukan proxy/gateway agregator (tidak meniru LiteLLM / one-api / 9router).
- ❌ Bukan penyimpan key terpusat / vault / cloud sync. Key tetap tinggal di file harness masing-masing.
- ❌ Bukan installer/downloader harness.
- ❌ Bukan editor model catalog global yang meng-override data harness.
- ❌ Tidak menyimpan token di penyimpanan milik aplikasi (kecuali transien saat "test connection").

---

## 4. Target Pengguna

### Persona utama

- **Mahasiswa** yang baru mendapat API key gratis, awam soal konfigurasi.
- **Pengajar/dosen** yang ingin membagikan preset konfigurasi ke kelas.
- **Pengguna umum** yang gonta-ganti provider untuk coba-coba model.

### Asumsi tingkat keahlian

- Paham konsep "API key", tapi tidak nyaman mengedit file JSON/YAML manual.

---

## 5. Lingkup Fungsional (MoSCoW)

### 5.1 Must have (MVP)

| ID | Fitur | Deskripsi |
|---|---|---|
| F1 | Deteksi harness | Mendeteksi harness terpasang + menampilkan **path konfigurasi secara eksplisit**. |
| F2 | Baca konfigurasi | Membaca dan menampilkan daftar provider + model + default saat ini. |
| F3 | Tambah provider | Menambah provider baru dari **preset** (Groq, Gemini, DeepSeek, dll) cukup dengan menempel API key. |
| F4 | Ubah / hapus provider | Mengedit field provider (base URL, model, key) dan menghapus provider. |
| F5 | Tulis dengan aman | Menulis balik dengan **merge** (preserve field tak dikenal) + **backup otomatis** sebelum tulis. |
| F6 | Backup & restore | Snapshot otomatis + daftar backup + restore satu klik. |
| F7 | Preview sebelum simpan | Menampilkan diff (sebelum/sesudah) sebelum menulis ke disk. |

### 5.2 Should have (fase 2)

| ID | Fitur | Deskripsi |
|---|---|---|
| F8 | Test connection | Ping endpoint provider dengan key yang diinput; tampilkan status + error. |
| F9 | Validasi skema | Validasi terhadap `$schema` (JSON Schema) bila tersedia; tampilkan error inline. |
| F10 | Preset kustom | Pengguna bisa simpan preset sendiri; export/import preset (untuk dibagikan dosen ke kelas). |

### 5.3 Could have (fase 3)

| ID | Fitur | Deskripsi |
|---|---|---|
| F11 | Edit `settings.json`/default | Ubah default provider/model, enabled models, tema. |
| F12 | Mode kelas | Import preset massal dari file `.rekonfig` ke banyak mesin. |
| F13 | Editor mode | UI non-GUI (CLI) untuk otomasi. |

---

## 6. Dukungan Harness (Adapter)

Setiap harness = satu **adapter** yang bertanggung jawab atas: *discover path → parse → normalisasi → serialize → write*.

| Prioritas | Harness | Status |
|---|---|---|
| P0 | Pi | Path & format sudah diketahui (observasi langsung) |
| P0 | OpenCode | Path & format sudah diketahui (observasi langsung) |
| P1 | Hermes | Perlu riset path/format |
| P1 | oh-my-pi | Perlu riset path/format |

### 6.1 Kontrak Adapter (interface)

```rust
trait Adapter {
    fn discover(&self) -> Result<HarnessInfo>;                 // path per OS
    fn read(&self) -> Result<HarnessConfig>;                   // parse → bentuk normal
    fn write(&self, config: &HarnessConfig) -> Result<()>;     // serialize → merge → tulis
    fn backups(&self) -> Result<Vec<BackupInfo>>;
    fn restore(&self, backup: &BackupInfo) -> Result<()>;
}
```

### 6.2 Catatan penting per harness

- **Pi** — dua file wajib konsisten:
  - `models.json`: provider + model + referensi `$ENV_VAR`.
  - `auth.json`: `{ "<provider_id>": { "type": ..., "key": "<asli>" } }`.
  - Menambah provider di Pi berarti menulis ke **keduanya**.
- **OpenCode** — provider didefinisikan di `opencode.json`; autentikasi mengikuti mekanisme OpenCode sendiri (auth store / env). Adapter harus menghormati ini dan tidak menulis key ke tempat yang salah.

---

## 7. Preset Provider Gratis

Preset = data statis (nama, base URL, jenis API, daftar model + kemampuan) yang sudah terisi, pengguna hanya perlu menempel key.

Kandidat preset (fase 1):

| Provider | baseUrl | Catatan |
|---|---|---|
| Groq | `https://api.groq.com/openai/v1` | free tier, rate limit |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/` | free tier |
| DeepSeek | `https://api.deepseek.com` | murah |
| OpenRouter | `https://openrouter.ai/api/v1` | agregator, ada model gratis |
| Mistral | `https://api.mistral.ai/v1` | free tier |
| Cerebras | `https://api.cerebras.ai/v1` | cepat, free tier |
| OpenCode Go | `https://opencode.ai/inference/go/openai/v1` | dari observasi |

Setiap preset menyimpan **daftar model** dengan: `id`, `name`, `context_window`, `max_tokens`, `reasoning`, `input_modalities`, `cost` (bila relevan), dan `compat`/flags khusus harness.

> Sumber data model harus mudah di-update (file JSON terpisah, bukan hardcode di kode).

---

## 8. Arsitektur Teknis

### 8.1 Teknologi

| Lapisan | Teknologi | Alasan |
|---|---|---|
| Shell / runtime | **Tauri v2** | binary kecil, cross-platform, file-system access native |
| Backend engine | **Rust** (di `src-tauri`) | adapter file, parsing, merge — UI-agnostic |
| Frontend | **HTML + Tailwind CSS + vanilla JS** | tanpa framework; ringan, mudah di-maintain |
| Bridge UI↔Rust | `@tauri-apps/api` (`invoke`) | memanggil Tauri commands |
| Editor JSON/YAML | **CodeMirror 6** (opsional, vanilla) | ringan, support JSON+YAML |
| Parsing | `serde_json` + `serde_yaml` | JSON & YAML harness |
| Path lintas OS | crate `dirs` / `etcetera` | resolusi `~/.config`, `~/Library/Application Support`, `%APPDATA%` |

### 8.2 Struktur repo

```
rekonfig/
├── index.html               # UI (vanilla)
├── src/                     # frontend vanilla
│   ├── main.js              # boot + invoke commands
│   ├── app.js               # render provider table, editor pane, diff
│   ├── styles.css           # tailwind entry
│   └── lib/
│       ├── api.js           # wrapper invoke()
│       ├── table.js         # render DataTable sederhana
│       └── diff.js          # render preview diff
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── commands.rs      # Tauri #[command] entry
│       └── engine/
│           ├── mod.rs
│           ├── discover.rs  # resolve path per OS
│           ├── model.rs     # tipe data normal (HarnessConfig, Provider, Model)
│           ├── merge.rs     # merge config (preserve field tak dikenal)
│           ├── backup.rs    # snapshot/restore
│           ├── validate.rs  # validasi $schema (bila ada)
│           ├── preset.rs    # load preset provider
│           └── adapters/
│               ├── mod.rs
│               ├── pi.rs
│               └── opencode.rs
└── package.json
```

### 8.3 Model data normal (internal)

```rust
enum HarnessKind { Pi, OpenCode, Hermes, OhMyPi }

struct HarnessInfo {
    kind: HarnessKind,
    config_paths: Vec<PathBuf>,   // semua file terkait, tampil eksplisit ke user
}

struct HarnessConfig {
    kind: HarnessKind,
    providers: Vec<Provider>,
    defaults: Defaults,
}

struct Provider {
    id: String,
    name: String,
    base_url: Option<String>,
    api: ApiKind,                 // OpenAiCompletions | Anthropic | ...
    auth: AuthRef,                // EnvVar(String) | Inline | AuthStore(String)
    models: Vec<Model>,
    // field "lain" yang tidak dikenal tetap dipertahankan (round-trip)
    extra: serde_json::Value,
}

struct Model {
    id: String,
    name: String,
    context_window: usize,
    max_tokens: usize,
    reasoning: bool,
    input: Vec<String>,
    cost: Option<Cost>,
    compat: serde_json::Value,
}

struct Defaults {
    provider: Option<String>,
    model: Option<String>,
    enabled: Vec<String>,
}
```

### 8.4 Tauri Commands (API internal)

```rust
#[tauri::command] fn discover_harnesses() -> Vec<HarnessInfo>;
#[tauri::command] fn read_config(harness: HarnessKind) -> HarnessConfig;
#[tauri::command] fn write_config(harness: HarnessKind, config: HarnessConfig, backup: bool) -> WriteResult;
#[tauri::command] fn add_provider(harness: HarnessKind, preset: &str, api_key: &str) -> WriteResult;
#[tauri::command] fn update_provider(harness: HarnessKind, provider_id: &str, patch: Provider) -> WriteResult;
#[tauri::command] fn remove_provider(harness: HarnessKind, provider_id: &str) -> WriteResult;
#[tauri::command] fn test_connection(config: ProviderConfig, api_key: &str) -> TestResult;
#[tauri::command] fn list_backups(harness: HarnessKind) -> Vec<BackupInfo>;
#[tauri::command] fn restore_backup(harness: HarnessKind, backup_id: &str) -> WriteResult;
#[tauri::command] fn list_presets() -> Vec<ProviderPreset>;
#[tauri::command] fn validate_config(harness: HarnessKind, config: HarnessConfig) -> Vec<Issue>;
```

---

## 9. Alur Pengguna (User Flows)

### 9.1 Tambah provider gratis (alur inti)

1. Buka Rekonfig → otomatis **deteksi harness** dan tampilkan daftar + path.
2. Pilih harness (mis. Pi) → tampil daftar provider saat ini.
3. Klik **"Tambah Provider"** → pilih preset (mis. Groq) → tempel API key.
4. Klik **"Test connection"** (opsional) → status OK/error.
5. Klik **"Simpan"** → tampil **preview diff** (file & perubahan).
6. Konfirmasi → aplikasi: **backup** file lama → **merge** → tulis ke `models.json` + `auth.json` (Pi) → tampil "berhasil, file ditulis ke: …".

### 9.2 Ubah provider

1. Pilih provider → edit field → simpan → preview → backup + merge + tulis.

### 9.3 Restore backup

1. Menu backup → daftar snapshot (timestamp) → pilih → **restore** → tulis balik file lama.

---

## 10. UI/UX

### 10.1 Layout (satu jendela)

```
┌──────────────────────────────────────────────┐
│ Sidebar                  │  Panel konten     │
│  Harness terdeteksi      │  Provider list    │
│   - Pi      (path…)      │   [table]         │
│   - OpenCode(path…)      │  Tombol:          │
│   - Hermes  (path…)      │   [+ Tambah]      │
│  Backup                  │  Editor JSON/YAML │
│  Preset                  │  (CodeMirror)     │
└──────────────────────────────────────────────┘
```

### 10.2 Prinsip

- **Path selalu ditampilkan eksplisit** — menyelesaikan "ga tahu file di mana".
- **Baca dulu, ubah, preview, baru tulis** — tidak ada tulis tersembunyi.
- **Warn** sebelum operasi destruktif (hapus provider, restore).
- Bahasa UI: Indonesia (default), extensible.

---

## 11. Keamanan

- Token tetap disimpan **di file harness** (plaintext — memang begitu cara kerja harness), aplikasi tidak menyalin ke lokasi sendiri.
- Tidak menulis token ke **log** atau telemetri.
- File yang ditulis diberi permission ketat (`0600`) untuk `auth.json` bila memungkinkan.
- Token hanya dikirim saat `test_connection` ke endpoint provider terkait, tidak ke server Rekonfig.
- **Selalu backup sebelum menulis**; jangan menimpa file `.bak` milik pengguna (pakai folder backup sendiri: `.rekonfig/backups/<harness>/`).

---

## 12. Non-Functional Requirements

- **Cross-platform:** macOS, Windows, Linux. Bagian OS-specific hanya pada resolusi path (di engine), bukan UI.
- **Performa:** muat daftar model hingga ribuan baris tanpa lag (render tabel virtual bila perlu).
- **Ukuran:** target installer/binary < 15 MB (Tauri tipikal).
- **Offline:** seluruh fungsi inti (baca/tulis config) offline; hanya `test_connection` butuh jaringan.
- **Aksesibilitas:** label form, keyboard navigation dasar.
- **Idempotensi:** menulis dua kali hasilnya sama (merge deterministik).

---

## 13. Milestone & Roadmap

| Fase | Deliverable | Fitur |
|---|---|---|
| **M0 — Spike** | Core engine + CLI | `discover`, `read`, `write` (merge+backup) untuk Pi & OpenCode |
| **M1 — Shell** | Tauri + HTML + Tailwind | jendela, deteksi harness, **read-only viewer** |
| **M2 — Edit** | MVP | tambah/ubah/hapus provider, preset, backup/restore, preview |
| **M3 — Kualitas** | Validasi + test | test connection, validasi `$schema`, diff |
| **M4 — Ekspansi** | Adapter + distribusi | Hermes, oh-my-pi, bundler + installer 3 OS |

---

## 14. Metrik Keberhasilan

- Waktu "tambah provider" < 60 detik (vs. edit manual yang bisa 10+ menit + salah).
- 100% operasi tulis menghasilkan backup + merge (tidak ada konfigurasi yang hilang).
- Konfigurasi hasil tulis **valid** dan harness dapat memuat provider baru (uji smoke).
- ≥ 3 harness didukung pada rilis publik.

---

## 15. Risiko & Mitigasi

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Skema config berubah antar versi harness | Tulis salah/rusak | Lapisan adapter + validasi `$schema` + pin versi + round-trip field tak dikenal |
| Auth terpisah per harness (Pi: `auth.json`) | Key tertulis di tempat salah | Adapter bertanggung jawab penuh; uji integrasi per harness |
| Perbedaan path antar OS | Tidak ketemu file | `discover.rs` terpusat + crate `dirs` + CI matrix 3 OS |
| Kehilangan data saat tulis | Konfigurasi hilang | Backup otomatis + merge (bukan overwrite) + preview |
| Token plaintext | Bocor | Tidak log, permission ketat, edukasi di UI |
| Project muda (Tauri/Tailwind API berubah) | Maintenance | Tauri v2 stabil; Tailwind pin versi; engine UI-agnostic |

---

## 16. Open Questions

1. **oh-my-pi** dan **Hermes** — apa persisnya path & format konfigurasi? (perlu riset sebelum adapter dibuat)
2. OpenCode: di mana tepatnya key disimpan pada versi terbaru (auth store vs env)? (perlu verifikasi)
3. Apakah perlu dukungan **Windows credential manager** untuk key, atau cukup file plaintext seperti harness?
4. Preset model: siapa yang maintain daftar model per provider? (opsi: file JSON statis + update manual)
5. Bahasa UI: Indonesia saja, atau perlu i18n?
6. Nama produk final & branding.

---

## 17. Appendiks — Contoh Format Konfigurasi (diamati, key di-redaksi)

### 17.1 Pi `~/.pi/agent/models.json` (ringkas)

```jsonc
{
  "$schema": "https://pi.dev/schemas/models.schema.json",
  "providers": {
    "deepseek": {
      "name": "DeepSeek",
      "baseUrl": "https://api.deepseek.com",
      "api": "openai-completions",
      "apiKey": "$DEEPSEEK_API_KEY",
      "modelOverrides": { "...": "..." },
      "models": [
        {
          "id": "deepseek-flash",
          "name": "DeepSeek Flash",
          "reasoning": true,
          "input": ["text"],
          "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
          "contextWindow": 1048576,
          "maxTokens": 131072,
          "compat": { "supportsStore": false, "supportsDeveloperRole": false }
        }
      ]
    }
  }
}
```

### 17.2 Pi `~/.pi/agent/auth.json` (key di-redaksi)

```jsonc
{
  "deepseek": { "type": "...", "key": "REDACTED" }
}
```

### 17.3 Pi `~/.pi/agent/settings.json` (ringkas)

```jsonc
{
  "defaultProvider": "commandcode",
  "defaultModel": "deepseek/deepseek-v4.1-flash",
  "enabledModels": ["commandcode/*", "deepseek/*", "opencode-go/*"],
  "theme": "dark"
}
```

### 17.4 OpenCode `~/.config/opencode/opencode.json` (ringkas)

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    "commandcode": { "...": "..." }
  }
}
```

---

*Dokumen ini adalah living document; direvisi seiring riset adapter (terutama Hermes & oh-my-pi).*
