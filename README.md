# Rekonfig

Aplikasi desktop untuk **membaca, mengubah, dan menulis konfigurasi provider/model** pada harness AI coding — Pi, oh-my-pi, OpenCode, dan Hermes — tanpa mengedit file manual di file explorer.

Mahasiswa sering dapat API key gratis (Groq, Gemini, Cerebras, …) tapi tidak tahu harus menaruhnya di mana, dalam format apa, dan bahwa API key kadang disimpan di **file berbeda** dari katalog model. Rekonfig menyelesaikan itu: pilih harness → pilih preset → tempel key → pratinjau diff → tulis.

Rekonfig **bukan** proxy/gateway dan **tidak** menyimpan key di tempat sendiri. Ia hanya menulis ke file yang memang dibaca harness kamu.

---

## Status

MVP selesai dan teruji (unit + integrasi + uji UI manual).

| Harness | Status | File yang dikelola |
|---|---|---|
| **Pi** | stabil | `~/.pi/agent/models.json` (JSONC) + `auth.json` + `settings.json`¹ |
| **oh-my-pi** | stabil | `~/.omp/agent/models.yml` + `config.yml`¹ |
| **OpenCode** | stabil | `~/.config/opencode/opencode.json` (+ `.jsonc`) + `~/.local/share/opencode/auth.json` |
| **Hermes** | eksperimental | `~/.hermes/config.yaml` + `~/.hermes/.env` |

¹ dibaca untuk menampilkan default provider/model; tidak pernah ditulis.

---

## Fitur

- **Deteksi harness + path eksplisit** — menyelesaikan "filenya di mana?".
- **Baca konfigurasi saat ini** sebagai tabel provider: base URL, API, status key (disamarkan), daftar model.
- **Tambah/ubah/hapus provider** dengan **preset siap pakai**: Groq, Gemini, Cerebras, DeepSeek, OpenRouter, Mistral, Together, NVIDIA NIM, OpenCode Go, Ollama. Ada opsi **Custom** untuk endpoint apa pun.
- **Pratinjau diff sebelum menulis** — tidak ada penulisan tersembunyi.
- **Backup otomatis** setiap kali menulis + panel **pulihkan** (restore).
- **Test koneksi** ke endpoint provider (validasi key + latency + contoh model).
- **Tulis aman**: merge (field tak dikenal dipertahankan), penulisan atomik (tmp + rename), permission `0600` untuk file berisi kredensial.
- **Nilai rahasia disamarkan** pada seluruh pratinjau.

---

## Prasyarat

- Node.js/Bun (untuk build CSS Tailwind)
- Rust toolchain ≥ 1.82
- Dependensi sistem Tauri v2:
  - **Linux**: `webkit2gtk-4.1`, `gtk3` (Debian/Ubuntu: `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`)
  - **macOS**: Xcode Command Line Tools
  - **Windows**: WebView2 (sudah ada di Windows 11) + MSVC build tools

---

## Menjalankan

```bash
bun install          # sekali saja
bun run dev          # build CSS + cargo tauri dev
```

Build produksi:

```bash
bun run build        # hanya CSS (Tailwind, minified)
cargo tauri build    # bundel installer
```

Hasil bundel ada di `src-tauri/target/release/bundle/`.

---

## Struktur repo

```
rekonfig/
├── web/                     # frontend: HTML + Tailwind + vanilla JS (tanpa framework)
│   ├── index.html
│   ├── main.js
│   ├── styles.src.css       # sumber Tailwind
│   └── styles.css           # hasil build (di-gitignore)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs      # jembatan UI ↔ engine
│   │   └── engine/
│   │       ├── model.rs     # tipe data bersama
│   │       ├── paths.rs     # resolusi path per OS (satu-satunya tempat OS-specific)
│   │       ├── doc.rs       # dokumen: parse / mutasi / render / redaksi
│   │       ├── backup.rs    # snapshot + restore + manifest
│   │       ├── write.rs     # penulisan atomik + permission
│   │       ├── preset.rs    # pemuat preset
│   │       ├── tests.rs     # test integrasi sandbox
│   │       └── adapters/    # satu file per harness
│   │           ├── pi.rs
│   │           ├── omp.rs
│   │           ├── opencode.rs
│   │           └── hermes.rs
│   ├── presets/providers.json   # data preset (mudah diperbarui tanpa sentuh kode)
│   ├── capabilities/
│   └── tauri.conf.json
├── docs/UI-CONTRACT.md      # kontrak command & tipe antara UI dan Rust
├── PRD.md                   # dokumen produk
└── scripts/make-demo-sandbox.sh
```

---

## Test

```bash
cd src-tauri && cargo test
```

Test berjalan di **sandbox** (`REKONFIG_ROOT`), jadi config asli tidak pernah tersentuh. Cakupan:

- merge tidak menghapus field tak dikenal (`modelOverrides`, `thinking`, `custom`, …)
- file kredensial dibuat dengan permission `0600`
- komentar `.env` tetap awet (pengeditan berbasis baris)
- pratinjau tidak pernah membocorkan API key
- hapus provider yang tidak ada tidak mengubah apa pun
- backup & restore
- validasi input (slug, base URL, minimal satu model)

Untuk mencoba UI tanpa menyentuh config asli:

```bash
./scripts/make-demo-sandbox.sh /tmp/rk-demo
REKONFIG_ROOT=/tmp/rk-demo bun run dev
```

`REKONFIG_ROOT` mengalihkan seluruh direktori harness ke bawah root tersebut.

---

## Model keamanan

- Key tetap tinggal di file harness (memang begitu cara harness bekerja). Rekonfig tidak menyalin key ke penyimpanannya sendiri.
- Key tidak pernah ditulis ke log.
- Pratinjau menyamarkan nilai rahasia (API key, token, password) — termasuk saat menampilkan diff.
- Selalu **backup dulu** sebelum menulis; backup disimpan di `<dir harness>/.rekonfig/backups/<timestamp>/` beserta `manifest.json`.
- Penulisan atomik: file sementara di direktori yang sama, lalu `rename`.

---

## Batasan yang diketahui

1. **Komentar hilang pada file JSON/JSONC/YAML.** Menulis ulang `models.json`, `models.yml`, `opencode.json`, atau `config.yaml` menghapus komentar di dalamnya. Rekonfig **memberi peringatan** dan selalu membuat backup. File `.env` **tidak** terkena masalah ini (diedit baris-per-baris, komentar tetap).
2. **Hermes masih eksperimental.** Hanya provider + API key yang ditulis; daftar model diambil Hermes sendiri dari endpoint.
3. **oh-my-pi tidak punya file auth terpisah yang stabil**, jadi key ditulis sebagai `apiKey` plaintext di `models.yml` (dengan permission `0600`).
4. **Path OpenCode di macOS/Windows** memakai beberapa kandidat (XDG, `Library/Application Support`, `%APPDATA%`). Bila instalasi kamu memakai lokasi tak lazim, Rekonfig mungkin perlu penyesuaian di `engine/paths.rs`.
5. **Preset model bersifat statis.** Provider sering mengganti/menghapus model gratis. Perbarui `src-tauri/presets/providers.json` bila ada model yang sudah tidak tersedia.
6. Skema config harness bisa berubah antar versi. Selalu periksa diff dan gunakan panel backup bila hasilnya tidak sesuai.

---

## Menambah harness baru

1. Buat `src-tauri/src/engine/adapters/<nama>.rs` yang mengimplementasikan `Adapter`:
   `specs()`, `backup_base()`, `snapshot()`, `build()`.
2. Daftarkan di `adapters::all()` (`src-tauri/src/engine/adapters/mod.rs`).
3. Tambahkan test di `engine/tests.rs`.

UI tidak perlu diubah — daftar harness, tabel provider, dan form dirender dari data backend.
