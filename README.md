# Hakonfig

> **Hakonfig** = **Ha**rness **Konfig**uration — aplikasi desktop untuk mengelola konfigurasi provider AI di harness coding agent, tanpa mengedit file JSON/YAML manual.

Aplikasi desktop untuk **membaca, mengubah, dan menulis konfigurasi provider AI** pada harness AI coding — **Pi**, **oh-my-pi**, **OpenCode**, dan **Hermes** — tanpa mengedit file JSON/YAML secara manual.

Masalah yang dipecahkan: mahasiswa/pengguna punya API key gratis (Groq, Gemini, Cerebras, …) tapi tidak tahu **di mana** harus menaruhnya, **dalam format apa**, dan bahwa API key kadang disimpan di **file berbeda** dari katalog model. Hakonfig menyelesaikannya: pilih harness → pilih preset → tempel key → lihat diff → tulis.

Hakonfig **bukan** proxy/gateway dan **tidak** menyimpan key di tempat sendiri. Ia hanya menulis ke file yang memang dibaca harness kamu.

- **Stack:** Tauri v2 (Rust) + HTML + Tailwind CSS v4 + vanilla JS — tanpa framework frontend
- **Ukuran:** biner 5,3 MB · paket `.deb` 2,4 MB
- **Test:** 56 test (36 Rust + 20 JS), tanpa jaringan

---

## Daftar isi

- [Fitur](#fitur)
- [Harness yang didukung](#harness-yang-didukung)
- [Prasyarat](#prasyarat)
- [Menjalankan (mode dev)](#menjalankan-mode-dev)
- [Build untuk distribusi](#build-untuk-distribusi)
- [Test](#test)
- [Cara pakai](#cara-pakai)
- [Struktur repo](#struktur-repo)
- [Konfigurasi lewat `HAKONFIG_ROOT`](#konfigurasi-lewat-hakonfig_root)
- [Menambah harness baru](#menambah-harness-baru)
- [Model keamanan](#model-keamanan)
- [Batasan yang diketahui](#batasan-yang-diketahui)
- [Troubleshooting](#troubleshooting)
- [Lisensi](#lisensi)

---

## Fitur

- **Deteksi harness + path eksplisit** — menjawab "filenya di mana?" dan ada tombol buka folder.
- **Baca konfigurasi saat ini** sebagai tabel: provider, base URL, API, status key (disamarkan), jumlah model.
- **Tambah / ubah / hapus provider** dengan **preset siap pakai**: Groq, Gemini, Cerebras, DeepSeek, OpenRouter, Mistral, Together, NVIDIA NIM, OpenCode Go, Ollama — plus opsi **Custom** untuk endpoint apa pun.
- **Pratinjau diff sebelum menulis** — tidak ada penulisan tersembunyi.
- **Backup otomatis** setiap penulisan + panel **pulihkan**.
- **Test koneksi** ke endpoint provider (validasi key, latency, contoh model).
- **Tulis aman**: merge (field tak dikenal dipertahankan), penulisan atomik (`tmp` + `rename`), permission `0600` untuk file berisi kredensial.
- **Nilai rahasia disamarkan** di seluruh pratinjau (API key, token, password).
- **Bahasa UI:** Indonesia.

## Harness yang didukung

| Harness | Status | File yang dikelola |
|---|---|---|
| **Pi** | stabil | `~/.pi/agent/models.json` (JSONC) + `auth.json` + `settings.json`¹ |
| **oh-my-pi** | stabil | `~/.omp/agent/models.yml` + `config.yml`¹ |
| **OpenCode** | stabil | `~/.config/opencode/opencode.json` (atau `.jsonc`) + `~/.local/share/opencode/auth.json` |
| **Hermes** | eksperimental | `~/.hermes/config.yaml` + `~/.hermes/.env` |

¹ hanya dibaca untuk menampilkan default provider/model; tidak pernah ditulis.

---

## Prasyarat

### Semua platform

| Kebutuhan | Versi | Cek |
|---|---|---|
| Rust (rustup) | ≥ 1.82 | `cargo --version` |
| Bun | ≥ 1.1 (Node ≥ 20 juga bisa) | `bun --version` |

CLI Tauri **tidak perlu** dipasang global — sudah menjadi devDependency (`@tauri-apps/cli`).

### Linux

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev libxdo-dev

# Fedora
sudo dnf install -y webkit2gtk4.1-devel openssl-devel curl wget file \
  librsvg2-devel libxdo-devel

# Arch
sudo pacman -S --needed base-devel curl wget file openssl webkit2gtk-4.1 librsvg libxdo
```

Verifikasi:

```bash
pkg-config --modversion webkit2gtk-4.1   # harus keluar nomor versi
```

### macOS

```bash
xcode-select --install
```

### Windows

1. **Visual Studio Build Tools** dengan workload *Desktop development with C++* (MSVC + Windows SDK).
2. **WebView2 Runtime** — sudah ada di Windows 11; Windows 10 bisa unduh dari situs Microsoft.
3. Rust toolchain MSVC:

```powershell
rustup default stable-msvc
```

---

## Menjalankan (mode dev)

```bash
bun install     # sekali saja
bun run dev     # build CSS lalu jalankan aplikasi
```

`bun run dev` menjalankan `bun run css` lebih dulu (Tailwind → `web/styles.css`), lalu `tauri dev`.

### Siklus edit yang perlu diketahui

Tauri **menyematkan aset `web/` saat compile**, jadi tidak ada hot reload:

| Yang kamu ubah | Yang perlu dilakukan |
|---|---|
| File Rust (`src-tauri/src/**`) | otomatis rebuild & restart oleh `tauri dev` |
| `web/*.html`, `web/*.js` | hentikan dev lalu `bun run dev` lagi (rebuild ±3 detik) |
| `web/styles.src.css` | `bun run css` (atau `bun run css:watch` di terminal lain), lalu restart dev |

Watcher `tauri dev` hanya memantau `src-tauri`, jadi perubahan di `web/` tidak memicu restart otomatis. Ini konsekuensi memilih tanpa bundler/dev-server.

Kalau ingin mencoba tanpa menyentuh config asli, lihat [HAKONFIG_ROOT](#konfigurasi-lewat-hakonfig_root).

---

## Build untuk distribusi

```bash
bun run build          # = bun run css && tauri build
```

Hasilnya:

```
src-tauri/target/release/hakonfig                      # biner (~5,3 MB)
src-tauri/target/release/bundle/                       # installer
```

Bundel yang dihasilkan tergantung OS tempat kamu build:

| OS build | Artefak |
|---|---|
| Linux | `.deb`, `.rpm`, `.AppImage` |
| macOS | `.app`, `.dmg` |
| Windows | `.msi`, `.exe` (NSIS) |

Membuat satu jenis bundel saja (lebih cepat):

```bash
bunx tauri build --bundles deb     # Linux
bunx tauri build --bundles dmg     # macOS
bunx tauri build --bundles nsis    # Windows
```

> **Catatan AppImage:** pembuatan AppImage mengunduh `linuxdeploy`/`appimagetool` dari internet saat pertama kali. Ukurannya juga jauh lebih besar (**~84 MB**) karena membawa pustaka GTK/WebKit di dalamnya, sedangkan `.deb`/`.rpm` hanya **~2,4 MB** karena memakai pustaka sistem. Kalau offline atau butuh paket kecil, pakai `--bundles deb`.

Contoh hasil `bun run build` di Linux:

```
bundle/deb/Hakonfig_0.1.0_amd64.deb          2,4 MB
bundle/rpm/Hakonfig-0.1.0-1.x86_64.rpm       2,4 MB
bundle/appimage/Hakonfig_0.1.0_amd64.AppImage 84 MB
```

> **Catatan cross-compile:** Tauri **tidak** bisa cross-compile. Untuk rilis 3 OS, pakai matriks CI (mis. GitHub Actions dengan runner `ubuntu-latest`, `macos-latest`, `windows-latest`), atau build di tiap OS.

### Menandatangani aplikasi (opsional, untuk rilis publik)

- macOS: butuh sertifikat Apple Developer + notarization; tanpa itu pengguna harus `Klik kanan → Open` pertama kali.
- Windows: tanpa sertifikat code signing, SmartScreen akan menampilkan peringatan.

Untuk pemakaian kelas/kelas internal, aplikasi unsigned biasanya cukup.

---

## Test

```bash
bun run test        # 36 test Rust + 20 test JS
bun run test:rust   # cargo test
bun run test:js     # node --test web/diff.test.js
```

Semua test berjalan **offline** dan tidak menyentuh config asli (Rust memakai sandbox `HAKONFIG_ROOT`).

| Lapisan | Jumlah | Cakupan |
|---|---|---|
| Unit — `engine/doc.rs` | 16 | parse JSONC (komentar + trailing comma), YAML (kosong/null/rusak), deteksi komentar, `mask_key`, `is_slug`, `env_name_from`, redaksi rahasia, edit `.env`, `is_changed` |
| Unit — `engine/backup.rs` | 4 | backup kosong, roundtrip create→list→restore, manifest rusak, urutan terbaru |
| Unit — `engine/write.rs` | 4 | direktori induk, overwrite, tanpa sisa `.hakonfig-tmp`, permission `0644` vs `0600` |
| Unit — `engine/paths.rs` | 2 | override `HAKONFIG_ROOT`, pemilihan `opencode.jsonc` vs `.json` |
| Unit — `web/diff.js` | 20 | `lcsDiff`, pemotongan konteks, file kosong, blok raksasa (anti-hang), escaping XSS, pemotongan output |
| Integrasi — `engine/tests.rs` | 5 | merge field tak dikenal, key ke file yang benar, mode `$ENV`, remove, no-op remove, backup/restore, redaksi pratinjau, validasi input |

---

## Cara pakai

1. Buka Hakonfig — harness yang terpasang langsung terdeteksi beserta path-nya.
2. Pilih harness di sidebar (mis. **Pi**).
3. Klik **+ Tambah Provider** → pilih preset (mis. **Groq**) → tempel API key → klik **Test koneksi** (opsional).
4. **Simpan** → muncul **pratinjau diff** → klik **Tulis ke disk**.

Selesai. Kalau ada yang salah, buka **Backup** → **Pulihkan**.

---

## Struktur repo

```
hakonfig/
├── web/                          # frontend: HTML + Tailwind + vanilla JS (tanpa framework)
│   ├── index.html
│   ├── main.js                   # UI + alur kerja
│   ├── diff.js                   # logika diff (dipisah agar bisa diuji)
│   ├── diff.test.js              # unit test diff
│   ├── styles.src.css            # sumber Tailwind
│   └── styles.css                # hasil build (gitignored)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs           # jembatan UI ↔ engine (9 command)
│   │   └── engine/
│   │       ├── model.rs          # tipe data bersama
│   │       ├── paths.rs          # resolusi path per OS (satu-satunya tempat OS-specific)
│   │       ├── doc.rs            # dokumen: parse / mutasi / render / redaksi
│   │       ├── backup.rs         # snapshot + restore + manifest
│   │       ├── write.rs          # penulisan atomik + permission
│   │       ├── preset.rs         # pemuat preset
│   │       ├── tests.rs          # test integrasi (lewat layer commands)
│   │       ├── testutil.rs       # sandbox + mutex untuk test
│   │       └── adapters/         # satu file per harness
│   │           ├── pi.rs
│   │           ├── omp.rs
│   │           ├── opencode.rs
│   │           └── hermes.rs
│   ├── presets/providers.json    # data preset (update tanpa sentuh kode)
│   ├── capabilities/
│   ├── icons/
│   └── tauri.conf.json
├── docs/UI-CONTRACT.md           # kontrak command & tipe antara UI dan Rust
├── scripts/make-demo-sandbox.sh  # sandbox demo untuk uji UI
├── PRD.md                        # dokumen produk
└── README.md
```

Engine (`src-tauri/src/engine/`) **tidak mengimpor Tauri sama sekali** — ia pustaka murni. UI juga tanpa framework. Artinya keduanya bisa dipakai ulang bila suatu saat shell-nya diganti.

---

## Konfigurasi lewat `HAKONFIG_ROOT`

Untuk pengembangan dan pengujian, seluruh direktori harness bisa dialihkan ke satu root:

```bash
./scripts/make-demo-sandbox.sh /tmp/rk-demo
HAKONFIG_ROOT=/tmp/rk-demo bun run dev
```

Pemetaan: `$HAKONFIG_ROOT/pi`, `/omp`, `/hermes`, `/opencode/config`, `/opencode/data`.

Ini dipakai oleh test Rust sehingga config asli di mesin tidak pernah tersentuh.

---

## Menambah harness baru

1. Buat `src-tauri/src/engine/adapters/<nama>.rs` yang mengimplementasikan trait `Adapter`:
   `specs()`, `backup_base()`, `snapshot()`, `build()` — dan opsional `experimental()` / `notes()`.
2. Daftarkan di `adapters::all()` (`src-tauri/src/engine/adapters/mod.rs`).
3. Tambahkan test integrasi di `engine/tests.rs`.

UI **tidak perlu diubah**: daftar harness, tabel provider, dan form semuanya dirender dari data backend.

---

## Model keamanan

- Key tetap tinggal di file harness (memang begitu cara harness bekerja). Hakonfig tidak menyalin key ke penyimpanannya sendiri.
- Key tidak pernah ditulis ke log.
- Pratinjau menyamarkan nilai rahasia — termasuk saat menampilkan diff.
- Selalu **backup dulu** sebelum menulis: `<dir harness>/.hakonfig/backups/<timestamp>/` + `manifest.json`.
- Penulisan atomik: file sementara di direktori yang sama, lalu `rename`.
- File berisi kredensial ditulis dengan permission `0600`.

---

## Batasan yang diketahui

1. **Komentar hilang pada file JSON/JSONC/YAML.** Menulis ulang `models.json`, `models.yml`, `opencode.json`, atau `config.yaml` menghapus komentar di dalamnya. Hakonfig **memberi peringatan** dan selalu membuat backup. File `.env` **tidak** terkena (diedit baris-per-baris, komentar tetap).
2. **Hermes masih eksperimental.** Hanya provider + API key yang ditulis; daftar model diambil Hermes sendiri dari endpoint.
3. **oh-my-pi tidak punya file auth terpisah yang stabil**, jadi key ditulis sebagai `apiKey` plaintext di `models.yml` (permission `0600`).
4. **Path OpenCode di macOS/Windows** memakai beberapa kandidat (XDG, `Library/Application Support`, `%APPDATA%`). Instalasi dengan lokasi tak lazim mungkin perlu penyesuaian di `engine/paths.rs`.
5. **Preset model bersifat statis.** Provider sering mengganti/menghapus model gratis — perbarui `src-tauri/presets/providers.json`.
6. **Tidak ada hot reload** untuk `web/` (tanpa bundler/dev-server).
7. Skema config harness bisa berubah antar versi. Selalu periksa diff dan gunakan panel backup bila hasilnya tidak sesuai.

---

## Troubleshooting

| Gejala | Penyebab & solusi |
|---|---|
| `The system library webkit2gtk-4.1 was not found` | Dependensi Linux belum ada → lihat [Prasyarat](#prasyarat). Cek: `pkg-config --modversion webkit2gtk-4.1` |
| `error: linker cc not found` (Linux) | `sudo apt install build-essential` |
| `link.exe not found` / error MSVC (Windows) | Install Visual Studio Build Tools + `rustup default stable-msvc`, buka shell baru |
| `Gtk-Message: Failed to load module "xapp-gtk3-module"` | **Aman diabaikan** — hanya modul tema Linux Mint/Cinnamon yang tidak ada |
| Jendela putih / UI kosong | Aset `web/` belum di-build ulang → `bun run css && bun run dev` |
| `styles.css` 404 / tampilan tanpa gaya | `bun run css` belum dijalankan |
| AppImage gagal dibuat | Butuh unduh tool dari internet → pakai `--bundles deb` |
| `identifier` berakhiran `.app` | Sudah diperbaiki (`dev.hudya.hakonfig`); identifier tidak boleh berakhiran `.app` karena bentrok dengan bundle macOS |
| Provider tidak muncul di harness | Cek path yang ditampilkan di sidebar; pastikan harness sudah pernah dijalankan minimal sekali |
| `file config tidak bisa diparse` | Hakonfig sengaja gagal keras daripada menimpa file rusak. Perbaiki JSON/YAML-nya dulu (backup tersedia) |
| Ingin coba tanpa risiko | Pakai `HAKONFIG_ROOT=/tmp/rk-demo` |

---

## Kontribusi

Pull request, issue, dan adapter harness baru sangat diterima. Untuk menambah
harness, lihat [Menambah harness baru](#menambah-harness-baru) — cukup satu file
adapter, UI tidak perlu diubah.

Sebelum mengirim PR:

```bash
bun run test     # 36 test Rust + 20 test JS harus hijau
```

---

## Lisensi

[MIT](LICENSE) © 2026 perogeremmer dan kontributor.

Artinya: siapa pun bebas memakai, menyalin, mengubah, menggabungkan,
menerbitkan, mendistribusikan, mensublisensikan, dan/atau menjual salinan
perangkat lunak ini, dengan tetap menyertakan pemberitahuan hak cipta dan izin
di atas. Perangkat lunak diberikan **tanpa jaminan** apa pun.
