# Riset Harness - kandidat adapter berikutnya

Status: **belum di-commit** (untuk direview).
Disusun: 7 November 2026.

Tujuan: menentukan harness mana yang layak ditambahkan ke Hakonfig setelah
**Pi**, **oh-my-pi**, **OpenCode**, dan **Hermes**.

Kriteria penilaian sebuah harness layak dijadikan adapter:

1. **Lazim dipakai untuk vibecoding** - dipakai sehari-hari untuk ngoding, bukan
   sebagai bot/otomasi chat (itu sebabnya OpenClaw, bot Telegram/Discord, dan
   sejenisnya tidak masuk daftar).
2. **Konfigurasi berbasis file** - bisa dibaca & ditulis tanpa GUI/keychain.
3. **Mendukung provider OpenAI-compatible** - inti nilai Hakonfig.
4. **API key bisa ditulis ke file** - bukan disimpan di OS keychain (kalau di
   keychain, adapter cuma bisa setengah jalan).

Tingkat keyakinan ditandai eksplisit: **✅ terverifikasi** (saya baca
dokumentasi/sumber resminya) vs **⚠️ perlu verifikasi** (belum saya buktikan).

---

## Ringkasan

| # | Harness | Lazim vibecode | Config | Key di file? | Layak | Prioritas |
|---|---|---|---|---|---|---|
| 1 | **Claude Code** | ✅ sangat | `~/.claude/settings.json` (`env`) | ✅ ya | ⚠️ satu endpoint, bukan daftar | **P1** |
| 2 | **Codex CLI** | ✅ sangat | `~/.codex/config.toml` | ⚠️ `env_key` → butuh env/auth | ✅ | **P1** |
| 3 | **Cline** | ✅ sangat | VS Code `globalStorage` | ✅ ya | ⚠️ | P2 |
| 4 | **Roo Code / Kilo Code** | ✅ tinggi | VS Code `globalStorage` | ✅ ya | ⚠️ | P2 |
| 5 | **Gemini CLI** | ✅ tinggi | `~/.gemini/settings.json` | ⚠️ | ⚠️ single provider | P2 |
| 6 | **Aider** | ✅ tinggi (klasik) | `~/.aider.conf.yml` / `.env` | ✅ ya | ✅ mudah | **P1** |
| 7 | **Continue** | ✅ sedang-tinggi | `~/.continue/config.yaml` | ✅ ya (`apiKey`) | ✅ mudah | **P1** |
| 8 | **Crush** (Charm) | ✅ sedang | `crush.json` (deprecated) / `crushrc` | ✅ ya | ⚠️ format pindah ke DSL | P3 |
| 9 | **Qwen Code** | ✅ sedang | `~/.qwen/settings.json` | ⚠️ | ⚠️ fork Gemini CLI | P3 |
| 10 | **Goose** (Block) | ✅ sedang | `~/.config/goose/config.yaml` | ❌ keyring | ⚠️ | P3 |
| 11 | **Copilot CLI** | ✅ sedang | `~/.config/github-copilot/` | ❌ OAuth | ❌ | - |
| 12 | **Amp / Droid / OpenHands** | ⚠️ niche | bervariasi | ⚠️ | belum diteliti | - |
| - | **Zed** | ✅ tinggi | `settings.json` (`language_models`) | ❌ **keychain** | ❌ | - |
| - | **OpenClaw** | ❌ bukan vibecode (bot) | - | - | ❌ (sesuai arahan) | - |

**Rekomendasi urutan kerja:** Aider → Continue → Codex → Claude Code, lalu
Cline/Roo/Kilo kalau mau menyasar pengguna VS Code.

---

## Detail per harness

### 1. Claude Code (Anthropic) - ✅ terverifikasi

- Sangat lazim dipakai untuk vibecoding; sering jadi standar di banyak tutorial.
- Config: `~/.claude/settings.json` (dan `settings.local.json` per proyek).
- Bentuknya **blok `env`**, bukan daftar provider:

  ```jsonc
  {
    "env": {
      "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic",
      "ANTHROPIC_AUTH_TOKEN": "sk-...",
      "ANTHROPIC_MODEL": "glm-4.6"
    }
  }
  ```

- **Artinya:** Claude Code tidak punya konsep "banyak provider" - ia selalu satu
  endpoint Anthropic-compatible. Ini justru pola yang dipakai orang untuk
  menyambungkan GLM/Kimi/DeepSeek ke Claude Code.
- Konsekuensi UI: halaman harness ini butuh bentuk lain - **satu kartu
  "endpoint aktif"** (base URL + token + model), bukan grid provider. Kalau
  tetap dipaksa grid, datanya cuma satu.
- Adapter: mudah secara teknis (JSON, key inline), sedang secara UI (butuh mode
  "single-endpoint"). Layak, tapi jangan dipaksa ke model mental provider-list.

### 2. Codex CLI (OpenAI) - ✅ terverifikasi

- Config: `~/.codex/config.toml` (bisa dialihkan lewat `$CODEX_HOME`).
  Ada juga `requirements.toml` dan profil di `$CODEX_HOME/profile`.
- Provider kustom berada di tabel `model_providers.<id>`. Key yang tersedia
  (dibaca dari config reference resmi):

  ```
  model_providers.<id>.name
  model_providers.<id>.base_url
  model_providers.<id>.env_key
  model_providers.<id>.env_key_instructions
  model_providers.<id>.wire_api          # HANYA "responses"
  model_providers.<id>.http_headers
  model_providers.<id>.env_http_headers
  model_providers.<id>.query_params
  model_providers.<id>.requires_openai_auth
  model_providers.<id>.experimental_bearer_token   # "discouraged"
  model_providers.<id>.capabilities
  model_providers.<id>.request_max_retries / stream_max_retries / stream_idle_timeout_ms
  model_providers.<id>.auth.{command,args}         # bearer token via command
  ```

- **Batasan penting:** `wire_api` hanya mendukung `"responses"`. Jadi provider
  yang cuma punya endpoint chat-completions (`/chat/completions`) **tidak**
  otomatis jalan di Codex. Ini harus ditampilkan sebagai peringatan di UI.
- Key: lewat `env_key` (nama variabel environment), atau `auth.command`,
  atau `experimental_bearer_token` (didorong untuk tidak dipakai).
  ⚠️ Perlu verifikasi: apakah `~/.codex/auth.json` masih jalur yang benar untuk
  kredensial (dokumen `authentication.md` di repo hanya berupa pointer).
- Adapter: perlu crate TOML yang mempertahankan komentar (`toml_edit`), karena
  `config.toml` pengguna biasanya sudah berisi banyak setelan.

### 3. Cline (VS Code) - ⚠️ perlu verifikasi

- Ekstensi VS Code paling populer untuk vibecoding. Mendukung "OpenAI
  Compatible" sebagai provider (di dokumen resminya frasa ini muncul puluhan
  kali).
- ⚠️ Yang belum saya verifikasi: **path pasti** penyimpanan setelan. Biasanya di
  VS Code `globalStorage` (mis. `.../User/globalStorage/saoudrizwan.claude-dev/`),
  tapi nama folder & bentuk JSON-nya perlu dicek langsung di mesin yang punya
  Cline terpasang.
- Risiko: VS Code menulis ulang file state saat ekstensi aktif → butuh penanganan
  "jangan tulis saat VS Code jalan" atau minimal peringatan.

### 4. Roo Code / Kilo Code - ⚠️ perlu verifikasi

- Turunan Cline, pola penyimpanan mirip (VS Code `globalStorage`, nama ekstensi
  berbeda). Karena satu basis kode, satu adapter bisa menutup dua-duanya kalau
  formatnya sama - perlu dicek.
- Di mesin ini ada paket npm `@kilocode/*`; bisa dipakai sebagai sumber
  verifikasi berikutnya.

### 5. Gemini CLI (Google) - ⚠️ perlu verifikasi

- Sangat lazim untuk vibecoding gratis. Config: `~/.gemini/settings.json`.
- Mayoritas pengguna memakai OAuth Google, bukan API key. Dukungan endpoint
  OpenAI-compatible ada lewat jalur tambahan, bukan bawaan inti.
- Di mesin ini `~/.gemini/` ada tetapi **tanpa** `settings.json` (memakai akun
  Google), jadi belum bisa saya konfirmasi bentuk skemanya.
- Kesimpulan sementara: nilai tambah untuk Hakonfig lebih kecil, karena
  "provider" di Gemini CLI bukan daftar.

### 6. Aider - ✅ terverifikasi

- Klasik, masih dipakai luas untuk vibecoding di terminal.
- Config: `.aider.conf.yml` (proyek) dan `~/.aider.conf.yml` (global), plus `.env`.
- Kunci yang relevan (dikonfirmasi ada di dokumen opsi resmi):
  `openai-api-base`, `openai-api-key`, `api-key`.
- Key bisa inline di YAML → cocok dengan model "tulis key ke file" Hakonfig.
- **Paling mudah** diimplementasikan: YAML, satu model mental mirip Pi
  (`openai-api-base` + `openai-api-key` + `model`).

### 7. Continue - ✅ terverifikasi

- Config: `~/.continue/config.yaml` - berbentuk daftar `models:` dengan
  `apiBase` dan `apiKey` per entri. Mendukung banyak provider sekaligus.
- Ini **paling cocok** dengan model mental Hakonfig (daftar provider + toggle
  aktif), dan bentuknya YAML yang sudah kita tangani.
- Prioritas tinggi: rasio nilai/effort terbaik setelah Aider.

### 8. Crush (Charmbracelet) - ⚠️ ada jebakan

- `crush.json` mendukung `providers.<id>` dengan `type`, `base_url`, `api_key`,
  `models[]` - tapi **statusnya deprecated**.
- Format baru: **`crushrc`**, yaitu DSL mirip shell (`provider add ...`,
  `model add ...`). Bukan JSON/YAML.
- Catatan keamanan dari dokumennya sendiri: `crushrc` dieksekusi sebagai shell,
  dan `$(...)` di `crush.json` dijalankan saat load. Artinya menulis file ini
  secara programatik punya risiko lebih tinggi daripada config biasa.
- Rekomendasi: kerjakan belakangan, dan kalau dikerjakan, **hanya** menulis
  `crush.json` (bukan `crushrc`), dengan peringatan bahwa format itu deprecated.

### 9. Qwen Code - ⚠️ perlu verifikasi

- Fork Gemini CLI dari Qwen; config di `~/.qwen/settings.json` mengikuti induknya.
  Populer di kalangan pengguna model Qwen/Qwen-Coder (banyak yang gratis).
- Perlu verifikasi skema + apakah mendukung endpoint OpenAI-compatible.

### 10. Goose (Block) - ⚠️ keyring

- Config: `~/.config/goose/config.yaml` (provider + model).
- Tapi API key disimpan di **keyring OS**, bukan di YAML. Ini pola yang sama
  dengan Zed → adapter hanya bisa setengah (provider bisa, key tidak).
- Karena itu saya sarankan **tidak** dikerjakan kecuali ada cara resmi menulis
  kredensial lewat file.

### Tidak layak (dengan alasan)

| Harness | Alasan |
|---|---|
| **Zed** | `language_models` ada di `settings.json`, tetapi API key disimpan di **keychain OS** → tidak bisa ditulis dari file. |
| **Copilot CLI** | Autentikasi OAuth GitHub; tidak ada "provider + API key" untuk diatur. |
| **OpenClaw** | Sesuai arahan: lebih tepat dipakai sebagai bot, bukan harness vibecoding. |
| **Amp, Droid, OpenHands** | Belum diteliti; kalau mau disasar, perlu pass verifikasi yang sama. |

---

## Yang perlu diverifikasi sebelum menulis kode

Untuk tiap kandidat, cek **di mesin yang benar-benar punya harness-nya**
(bukan menebak):

1. Path config persis (termasuk variasi Windows/macOS).
2. Bentuk skema: nama field, tipe, apakah array atau map.
3. Tempat API key: file mana, nama field apa.
4. Apakah ada daftar "disabled/active providers" (menentukan toggle bisa
   reversible atau tidak, seperti temuan di oh-my-pi & OpenCode).
5. Pertahankan komentar? (TOML/YAML pengguna sering penuh komentar - ini masih
   batasan Hakonfig saat ini.)

Cara tercepat: jalankan harness-nya, tambahkan satu provider kustom lewat UI
bawaannya, lalu **diff file config sebelum/sesudah**. Itu bukti terkuat, lebih
kuat dari dokumentasi.

---

## Perkiraan usaha per adapter

Berdasarkan 4 adapter yang sudah ada (Pi, oh-my-pi, OpenCode, Hermes):

| Kategori | Contoh | Perkiraan |
|---|---|---|
| JSON/YAML datar, key inline | Aider, Continue | ~1 file adapter + test, cepat |
| JSON/YAML dengan struktur khas | Codex (TOML), Cline | adapter + crate TOML + test |
| Single-endpoint (bukan daftar) | Claude Code, Gemini CLI | perlu **mode UI baru**, bukan cuma adapter |

Poin terakhir itu penting: Claude Code dan Gemini CLI bukan "tambah satu file
adapter" - mereka menuntut Hakonfig punya tampilan khusus untuk harness yang
hanya punya satu endpoint. Itu keputusan produk, bukan keputusan teknis.
