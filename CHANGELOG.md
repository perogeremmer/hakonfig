# Changelog

Format mengikuti [Keep a Changelog](https://keepachangelog.com/) dan
[Semantic Versioning](https://semver.org/). Nomor versi disebarkan ke seluruh
file oleh `scripts/version.mjs`.

## [0.4.2]

### Fixed

- npm: binari semua platform kini ikut di dalam satu paket `hakonfig`
  (≈5 MB terkompresi), menggantikan rencana paket per-platform. Dua percobaan
  sebelumnya gagal karena alasan di luar kode kita: nama unscoped
  `hakonfig-win32-x64` ditolak permanen dengan `403 Package name triggered spam
  detection`, lalu paket scoped `@perogeremmer/hakonfig-*` tersimpan
  *restricted* (packument 404 untuk pengunjung anonim) dan token rilis tidak
  boleh mengubah visibilitasnya. Dengan binari gabungan, `npm i -g hakonfig`
  bekerja di macOS, Linux, dan Windows tanpa `optionalDependencies` sama sekali.
- Rilis: draft release sekarang benar-benar di-publish (`gh release edit
  --draft=false --latest`) di akhir workflow. Sebelumnya tidak ada langkah ini,
  jadi setiap rilis tertinggal sebagai draft dan tautan unduhan 404.
- Rilis: kegagalan publish crates.io (mis. email akun crates.io belum
  diverifikasi — kasus yang terjadi pada 0.4.1) tidak lagi menggagalkan
  workflow yang sama dengan aset desktop; sebagai gantinya muncul warning.
- npm: `scripts/publish-npm.mjs` memverifikasi hasil publish lewat registry
  tanpa autentikasi, jadi publish yang tersimpan *private* langsung ketahuan
  alih-alih baru ketahuan saat pengguna meng-install.

## [0.4.1]

Dirilis untuk memperbaiki distribusi npm: paket platform dipindah ke scope
`@perogeremmer/` setelah nama unscoped kena spam detection. Ternyata paket
scoped tersimpan *restricted*, jadi 0.4.1 tetap tidak terpakai lewat npm —
digantikan 0.4.2 (binari gabungan). Aset desktop 0.4.1 sendiri normal.

### Fixed

- npm: paket binary platform dipindah ke scope `@perogeremmer/`
  (`@perogeremmer/hakonfig-darwin`, `-linux-x64`, `-win32-x64`). npm menolak
  publish nama unscoped `hakonfig-win32-x64` dengan `403 Package name
  triggered spam detection` — proteksi terhadap nama berpola auto-generated
  dari akun baru — sehingga `npm i -g hakonfig` di Windows gagal karena
  optional dependency-nya tidak pernah ada. Pola scoped ini sama dengan
  distribusi esbuild/swc. Paket utama tetap `hakonfig` (tanpa scope), jadi
  perintah install tidak berubah.
- npm: `hakonfig@0.4.0` serta dua paket platform unscoped ditandai
  *deprecated* supaya pengguna versi lama dapat peringatan saat install.

## [0.4.0]

### Added

- Crate CLI `hakonfig-cli`: kelola config harness tanpa GUI, memakai jalur
  kode yang sama dengan aplikasi desktop (`hakonfig_core::api`). Perintah:
  `list`, `show`, `providers`, `paths`, `add` (wizard interaktif atau
  non-interaktif penuh untuk skrip/server), dengan `--json` untuk output
  yang stabil. Dokumentasi ada di `docs/CLI.md`.
- Instalasi CLI satu perintah: `install.sh` (macOS/Linux) dan `install.ps1`
  (Windows) yang mengunduh binary release, memverifikasi `SHA256SUMS`, dan
  memasang ke `~/.local/bin`. Jalur lain: Homebrew tap
  (`brew install perogeremmer/tap/hakonfig-cli`), npm (`npm i -g hakonfig`,
  paket `hakonfig` dengan binary per-platform), dan crates.io
  (`cargo install hakonfig-cli`).
- Release workflow kini mengunggah binary `hakonfig-cli` per platform
  (macOS universal, Linux, Windows) plus `SHA256SUMS` ke draft release,
  lalu mem-publish ke npm dan crates.io (dilewati dengan pesan jelas bila
  secret-nya belum di-set). Workflow `homebrew-tap.yml` (manual, per tag)
  menulis ulang formula di `perogeremmer/homebrew-tap`.
- CI kini menjalankan `cargo clippy -D warnings` dan `cargo fmt --check`.

### Changed

- CLI: `--test` eksplisit yang gagal kini menggagalkan perintah
  (exit non-zero, tidak ada yang ditulis), karena automation
  memintanya sebagai gerbang sebelum menulis. Tanpa `--test`/`--no-test`,
  mode non-interaktif tetap tidak mencoba endpoint sama sekali.
- CLI: `--json` dan stdout yang dipipe kini memaksa mode non-interaktif,
  sehingga wizard tidak pernah tercampur dengan output JSON.
- CLI: kuota potongan diff dibagi merata antara baris yang dihapus dan
  ditambah (sisa satu sisi diberikan ke sisi lain), bukan dihabiskan pada
  baris yang dihapus lebih dulu.
- String warning/error di `hakonfig-core` kini seluruhnya English (sebelumnya
  Indonesia). Nama semua test Rust dan JS kini juga English.

### Notes

- String warning engine yang English berlaku juga di aplikasi desktop:
  pengguna desktop yang memakai Bahasa Indonesia akan melihat warning dalam
  bahasa Inggris sampai string engine masuk katalog i18n yang sama.

## [0.3.2] - 2025

### Fixed

- Katalog: nama yang benar untuk DeepSeek Harness (bukan "DeepSeek chat").

## [0.3.1] - 2025

### Fixed

- Default model benar-benar diterapkan di Pi dan Oh My Pi.

## [0.3.0] - 2025

### Added

- Katalog tool: web tools dan OpenHands.

## [0.2.1] - 2025

### Changed

- README: "Get started" dipindah ke atas dan logo brand ditambahkan.

## [0.2.0] - 2025

### Fixed

- CI: `Cargo.lock` tetap sinkron dengan versi aplikasi.

## [0.1.4] - 2025

### Added

- 37 preset provider baru dengan logo lokal; semua aset sepenuhnya offline.

## [0.1.3] - 2025

### Changed

- UI: checkbox digambar sendiri dengan sudut membulat yang lembut.

## [0.1.2] - 2025

### Added

- Brand Hakonfig: mark, app icon, dan design token.

## [0.1.1] - 2025

### Fixed

- UI: nama variabel env diambil dari provider, bukan dari baris model.

## [0.1.0]

Rilis pertama.

- Empat adapter harness: Pi, Oh My Pi, OpenCode, Hermes (eksperimental).
- 24 preset provider OpenAI-compatible dengan logo lokal.
- Pratinjau diff sebelum menulis, backup otomatis (maksimal 5 per harness),
  dan riwayat pemulihan.
- Panduan awal saat aplikasi pertama kali dibuka: deteksi harness + model.
- Bilingual (English / Bahasa Indonesia), tema terang dan gelap.
- Bundle: macOS (dmg), Windows (NSIS), Linux (deb).
