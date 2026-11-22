# Progres port GPUI Kit

Catatan kerja kronologis. **Setiap sesi kerja wajib menambah satu entri** di
bawah, termasuk yang gagal — kegagalan yang tidak dicatat akan diulang.

## Cara mencatat

1. Tambah entri baru di bagian **Log** (paling baru di atas), format:
   ```
   ### YYYY-MM-DD HH:MM · <siapa> · <ringkas>
   - Kerja: ...
   - File: ...
   - Verifikasi: `cargo build` hijau · screenshot `docs/xxx.png` · angka: ...
   - Kendala/next: ...
   ```
2. Perbarui tabel **Status** di `FEATURES.md` (ubah `[ ]` → `[~]` → `[x]`).
3. Kalau ada temuan soal GPUI Kit (API aneh, bug, jebakan), tulis di
   **Temuan** — itu yang paling mahal untuk ditemukan ulang.
4. Jangan menulis rencana panjang di sini. Rencana ada di `FEATURES.md`.

Sesi paralel menulis ke `progress/<nama>.md` masing-masing, lalu pemilik repo
menggabungkannya ke sini agar tidak ada konflik tulis.

## Status ringkas

| Area | Selesai | Total |
|---|---|---|
| Shell & navigasi | 5 | 5 |
| Providers | 7 | 9 |
| Tambah provider | 0 | 7 |
| Diff | 0 | 6 |
| Riwayat & backup | 2 | 2 |
| Halaman harness | 3 | 4 |
| Pengaturan | 0 | 5 |
| Lintas fitur | 1 | 5 |

## Verifikasi cepat

```sh
cd spike/gpui-kit
cargo build                                  # harus hijau
cargo run --release                          # jendela terbuka
./bench.sh --no-build                        # startup + RSS + ukuran binary
./window-startup.sh ./target/release/hakonfig-spike "spike gpui-kit" 3
# screenshot (butuh ImageMagick + xdotool, X11):
xdotool search --name "spike gpui-kit" | tail -1 | xargs -I{} import -window {} docs/ui-light.png
```

## Log

### 2026-11-23 00:37 · repo · menu aksi provider + toast
- Kerja: tombol `…` per kartu provider membuka dialog berisi 6 aksi (Edit,
  Kelola model, Jadikan default, (Non)aktifkan, Lupakan kredensial, Hapus),
  masing-masing menyebut `EditOp` yang akan dipakai. Semua sengaja tidak
  mengeksekusi apa pun.
- Verifikasi: build hijau; screenshot interaksi manual (dialog tampil benar).
- Catatan: dialog gpui-kit memanggil closure-nya lebih dari sekali, jadi elemen
  harus dibangun DI DALAM closure (`Div` tidak Clone) — ini penyebab error
  pertama saat menulis menu ini.

### 2026-11-23 00:25 · repo · berkas config + notice di layar Providers
- Kerja: kartu "Berkas config" (role, path mono, badge kredensial, ada/belum,
  klik = buka folder lewat `xdg-open`/`open`/`explorer`), dan kartu temuan
  dari `api::list_notices` (amber, destructive ditandai merah).
- File: `src/views/providers.rs`, `src/data.rs`.
- Verifikasi: build hijau; `docs/ui-light.png`; uji pindah harness ke Hermes
  (v0.20.1, eksperimental, 2 berkas, auth ditandai kredensial).
- Catatan: `list_notices` membaca ulang berkas yang sama (±5 ms/harness), jadi
  digabung ke `HarnessEntry::load` alih-alih panggilan terpisah.

### 2026-11-23 00:10 · repo · startup dipangkas 3x + layar Riwayat
- Kerja: (1) `harness_version()` dipindah ke background, config dibaca sinkron
  (40–52 ms). (2) View `Riwayat` baru: daftar backup nyata dari
  `api::list_backups`, dialog konfirmasi restore (eksekusi nonaktif).
  (3) Seam view jadi `new(window, cx)` supaya view bisa membuat `InputState`.
- File: `src/app.rs`, `src/data.rs`, `src/views/history.rs`, `src/views/mod.rs`,
  `src/views/{add,diff,settings}.rs` (signature saja), `src/net.rs`.
- Verifikasi: build hijau tanpa warning; screenshot `docs/ui-history.png`,
  `docs/ui-dark.png`.
- Angka: frame pertama **307–387 ms** dengan data sudah tampil (sebelumnya
  1.178 ms dengan layar kosong "Memuat…"), probe versi selesai ±1,9 s di
  background. Catatan: run pertama setelah build bisa 1,2 s karena cache disk.
- Kendala: `Theme::update` butuh `&mut App`, jadi view tidak bisa memakainya
  lewat `cx.listener` (yang memberi `&mut Context<Self>`); handler tombol
  gpui-kit menerima `&mut App` sehingga tombol tema di view harus pakai
  closure mentah, bukan `cx.listener`.

### 2026-11-22 23:20 · repo · UI dipoles ke bahasa visual Hakonfig
- Kerja: font bundel (Plus Jakarta Sans + JetBrains Mono, `add_fonts`), palet
  hangat dari `web/styles.src.css`, kartu provider bergaya GUI web, switch
  aktif/nonaktif, dark mode, brand button.
- File: `src/main.rs`, `src/palette.rs`, `assets/fonts/*`
- Verifikasi: build hijau; `docs/ui-light.png`, `docs/ui-dark.png`.
- Temuan: `Button::primary()` mengambil token `button_primary` yang dibangun dari
  tema bawaan, bukan `ThemeColor` yang ditimpa — hasilnya hitam, bukan maroon.
  Solusi sementara: `palette::brand_button()`.

### 2026-11-22 23:55 · repo · spike dipecah jadi modul
- Kerja: `src/{app,data,palette}.rs` + `src/views/{providers,add,diff,settings}.rs`.
  Placeholder untuk tiga layar disiapkan supaya bisa dikerjakan paralel.
- File: lihat `src/`.
- Verifikasi: build hijau; screenshot ulang `docs/ui-light.png`.

### 2026-11-22 22:30 · repo · spike pertama (layar Providers)
- Kerja: 596 baris Rust, data langsung dari `hakonfig_core::api`, load di
  `cx.background_spawn`, metrik startup dicetak ke stderr.
- Verifikasi: jendela 110 ms, frame pertama 167 ms, RSS 119 MB, binary 54 MB.
- Temuan: `h_flex()` = row + `items_center`; butuh `items_stretch()` untuk layout
  penuh. gpui-kit menuntut Rust ≥ 1.95 (`std::hint::cold_path`). Linux butuh
  `libxkbcommon-x11` (dev) untuk link; `build.rs` menyediakan taut lokal.
