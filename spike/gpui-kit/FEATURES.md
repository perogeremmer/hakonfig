# Fitur yang perlu dibuat di GPUI Kit

Daftar kerja port GUI Hakonfig dari Tauri (HTML/Tailwind) ke GPUI Kit.
Sumber kebenaran perilaku adalah GUI web sekarang: `web/main.js` (3.554 baris),
`web/index.html`, `web/diff.js`, `web/provider-form.js`, `web/i18n/*`.

Legenda status: `[x]` selesai · `[~]` sedang dikerjakan · `[ ]` belum · `[!]` diblokir.

Aturan tetap untuk semua pekerjaan di spike ini:

1. **Tulis hanya lewat core.** Penulisan config memakai `api::apply_edit`
   (backup otomatis dibuat core); jangan menulis file config langsung.
   `api::restore_backup` dan `api::fix_notice` masih belum dipakai.
2. **Core tidak disentuh.** Semua data lewat `hakonfig_core::api`. Kalau sebuah
   fitur butuh data yang belum ada di core, catat di bagian "Butuh di core".
3. **Satu view satu file.** `src/views/<nama>.rs` — jangan menyunting file view
   lain, `src/app.rs`, atau `src/main.rs` (itu seam milik pemilik repo).
4. **Bahasa UI Indonesia**, istilah teknis (provider, model, key, diff) tetap.
5. **`cargo build` harus hijau setiap kali selesai.**

---

## A. Shell & navigasi

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F0a | Top bar: brand, statistik tool/provider/model, ikon cog → Pengaturan | [x] | `src/app.rs` |
| F0b | Sidebar: daftar harness + jumlah provider/key, penanda aktif; kosong saat instalasi baru | [x] | `src/app.rs` |
| F0g | Instalasi baru: sidebar kosong + tombol "Tambah harness baru" → modal Pi/OMP/OpenCode/Hermes; pilihan disimpan di `~/.config/hakonfig-spike/harnesses` | [x] | `src/app.rs`, `src/state.rs` |
| F0c | Navigasi layar: kotak AKSI di Providers (Tambah/Diff/Riwayat) + cog (Pengaturan) + tombol kembali `‹ Providers` | [x] | `src/app.rs`, `src/views/providers.rs` |
| F0e | Kotak AKSI: tiga baris berikon dengan judul + keterangan singkat, hover, klik membuka layar | [x] | `src/views/providers.rs` |
| F0f | Tangkapan layar instalasi baru (HOME kosong) terang + gelap, lewat `./shot.sh --fresh` / `--dark` | [x] | `shot.sh`, `docs/fresh-install/` |
| F0d | Font bundel (Plus Jakarta Sans, JetBrains Mono) + palet terang/gelap | [x] | `src/palette.rs` |
| F0e | Load config di background supaya frame pertama < 200 ms | [x] | `src/app.rs` |

## B. Providers (layar utama)

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F1 | Kartu provider: nama, id mono, base URL, jumlah model | [x] | `views/providers.rs` |
| F2 | Penanda key: `✓ key tersimpan` / `env · NAMA` / `tanpa key` | [x] | `views/providers.rs` |
| F3 | Pencarian provider (nama + id, langsung per ketikan) | [x] | `views/providers.rs` |
| F4 | Toggle aktif/nonaktif per provider | [x] menulis via `EditOp::SetEnabled` | `views/providers.rs` |
| F5 | Menu `…` per kartu: edit, kelola model, default, toggle, lupakan key, hapus | [x] | `views/providers.rs`, `src/app.rs` |
| F6 | Filter cepat: semua / punya key / nonaktif | [ ] | `views/providers.rs` |
| F7 | Kartu default model + picker model (cari, tandai default aktif) → `EditOp::SetDefault` | [x] | `views/providers.rs`, `src/app.rs` |
| F8 | Masking key di semua tempat (`•••• 1a2b`), tidak pernah mentah | [x] | `views/providers.rs` |
| F9 | Empty state (belum ada provider / hasil pencarian kosong) | [x] | `views/providers.rs` |

## C. Tambah provider (tiga langkah)

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F10 | Picker preset: 61 provider dari `api::list_presets()` + kartu Custom, dengan pencarian | [x] | `views/add.rs` |
| F11 | Langkah key: mode simpan vs env (nama otomatis `<ID>_API_KEY`), input tersamarkan | [x] | `views/add.rs` |
| F12 | "Test & find models": panggil endpoint provider, tampilkan ok/gagal + latensi + contoh model | [x] | `views/add.rs` |
| F13 | Picker model: multi-pilih, cari, tandai context window + reasoning, pilih semua/kosongkan | [x] | `views/add.rs` |
| F14 | Form lanjutan untuk Custom: id, display name, base URL, api type + validasi | [x] | `views/add.rs` |
| F15 | Ringkasan + tombol "Pratinjau diff" yang memanggil `api::preview_edit` | [x] | `views/add.rs` |
| F16 | Stepper 1-2-3, tombol kembali, state tidak hilang saat pindah langkah | [x] | `views/add.rs` |
| F16b | Mode edit provider: field terisi data yang ada, key kosong = pertahankan key lama, model terpasang terpilih | [x] | `views/add.rs`, `src/app.rs` |
| F16c | Tombol Terapkan + konfirmasi overlay → `api::apply_edit`, lalu muat ulang + toast | [x] | `views/add.rs`, `src/app.rs` |

## D. Diff (pratinjau sebelum tulis)

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F17 | Tab per file + jumlah baris +/- per file | [x] | `views/diff.rs` |
| F18 | Render unified diff: warna tambah/hapus, nomor baris, konteks, monospace | [x] | `views/diff.rs` |
| F19 | Secret dimasking di dalam diff | [x] | `views/diff.rs` |
| F20 | Empty state "tidak ada perubahan" | [x] | `views/diff.rs` |
| F21 | Sumber diff read-only: `api::preview_edit` dari EditOp demo | [x] | `views/diff.rs` |
| F22 | Tombol Apply ada tapi nonaktif + alasan ("spike read-only") | [x] | `views/diff.rs` |

## E. Riwayat & backup

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F23 | Daftar backup per harness (`api::list_backups`): waktu, id, berkas | [x] | `views/history.rs` |
| F24 | Restore dengan dialog konfirmasi (eksekusi sengaja nonaktif) | [x] | `views/history.rs` |

## F. Halaman harness

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F25 | Panel berkas config: path, ada/tidak, sensitif, buka folder | [x] | `views/providers.rs` |
| F26 | Notices: daftar masalah config (aksi perbaikan belum aktif) | [x] | `views/providers.rs` |
| F27 | Install/uninstall tool dengan log streaming + progress | [ ] | `views/harness.rs` |
| F28 | Deteksi versi tool di background, tidak di jalur frame pertama | [x] | `src/data.rs` |

## G. Pengaturan

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F29 | Tema: terang / gelap / ikut sistem | [x] | `views/settings.rs` |
| F30 | Bahasa: Indonesia / English (port subset `web/i18n`) | [x] | `views/settings.rs` |
| F31 | Skala tampilan: catat bahwa zoom CSS tidak relevan di native, pakai DPI OS | [x] | `views/settings.rs` |
| F32 | Diagnostik: timing baca config vs probe versi, jumlah harness/provider/model, RSS | [x] | `views/settings.rs` |
| F33 | Tentang: versi, lisensi, tautan repo | [x] | `views/settings.rs` |

## H. Lintas fitur

| ID | Fitur | Status | File |
|----|-------|--------|------|
| F34 | Toast/notifikasi (gpui-kit `notification`) untuk hasil aksi | [ ] | siapa pun yang butuh |
| F35 | Dialog konfirmasi standar (judul, isi, Batal/OK, ESC, fokus awal) | [~] contoh ada di `src/app.rs` | pemilik repo |
| F36 | Keyboard: `Esc` tutup dialog jalan; `Ctrl+K` diblokir API (`on_action` hanya saat paint) | [!] | `src/app.rs` |
| F37 | Label aksesibilitas untuk switch/tombol ikon (AccessKit) | [ ] | per view |
| F38 | Semua string lewat satu modul i18n (tidak ada teks hard-coded) | [ ] | `src/i18n.rs` |

---

## Butuh di core (bukan pekerjaan UI)

- `api::list_presets()` sudah ada; pastikan bentuknya memuat logo/kategori untuk picker.
- Tes koneksi provider: cari fungsi yang dipakai GUI web (`api.rs`), atau tambah
  `api::test_provider(preset, key)` supaya view tidak memanggil HTTP sendiri.
- `api::list_backups()` mengembalikan `BackupInfo`; butuh ukuran file untuk UI riwayat.

## Kontrak view (dipakai agent, jangan diubah tanpa memperbarui dokumen ini)

```rust
// src/views/<nama>.rs
pub struct XView { /* state milik view */ }

impl XView {
    // App memanggil ini; window bisa dipakai membuat InputState.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
    // Dipanggil App setiap harness terpilih berubah (boleh no-op).
    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>);
}

impl Render for XView { /* ... */ }
```

Fakta core yang sering dibutuhkan (jangan cari ulang di repo):

- `api::list_presets() -> Vec<ProviderPreset>`; `ProviderPreset { id, name, base_url, api, env_var: Option<String>, docs_url: Option<String>, key_required: bool, badge: Option<String>, notes: String, models: Vec<ModelSummary> }`.
- `ModelSummary { id, name: Option<String>, context_window: Option<u64>, max_tokens: Option<u64>, reasoning: bool }`.
- `EditOp::Upsert(ProviderDraft)`, `EditOp::SetModels { provider_id, models }`, `EditOp::SetKey { provider_id, key, key_mode }`, `EditOp::SetDefault { provider_id, model_id }`, `EditOp::SetEnabled { provider_id, enabled }`, `EditOp::Remove { provider_id }`, `EditOp::ForgetKey { provider_id }`.
- `ProviderDraft { id, name, base_url, api, key: Option<String>, key_mode: Option<KeyMode>, models: Vec<ModelDraft> }`; `ModelDraft { id, name: Option<String>, context_window: Option<u64>, max_tokens: Option<u64>, reasoning: bool }`; `KeyMode::{Inline, Env}`.
- `api::preview_edit(harness_id: String, op: EditOp) -> Result<PreviewResult, String>`; `PreviewResult { files: Vec<FileDiff>, warnings, will_create, will_modify }`; `FileDiff { path, role, sensitive, before: Option<String>, after: Option<String>, changed: bool }` (isi file utuh, sudah diredaksi core untuk file sensitif).
- `api::test_connection` async; pakai pembungkus blocking `crate::net::test_connection(base_url, api, key, harness_id: Option<String>, provider_id: Option<String>) -> Result<TestResult, String>`; `TestResult { ok, status: Option<u16>, message, latency_ms: Option<u64>, models_sample: Vec<String> }`.
- `api::list_backups(harness_id) -> Result<Vec<BackupInfo>, String>`; `BackupInfo { id, created_at, dir, files: Vec<BackupFile> }`.

## Catatan hasil spike (fakta terukur, jangan diubah tanpa mengukur ulang)

- Baca config 4 harness = 2 ms. `harness_version()` = ±1.000 ms karena spawn proses.
- Frame pertama 167 ms, jendela tampil ±110 ms, RSS 119 MB, binary 54 MB (42 MB strip).
- Tauri di mesin yang sama: jendela 64–84 ms, RSS 144 MB, binary 6,3 MB + WebKitGTK.
