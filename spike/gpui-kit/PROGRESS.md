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
| Providers | 6 | 9 |
| Tambah provider | 0 | 7 |
| Diff | 0 | 6 |
| Riwayat & backup | 0 | 2 |
| Halaman harness | 0 | 4 |
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
