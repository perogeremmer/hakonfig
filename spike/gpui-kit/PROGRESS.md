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

## Cara menggabungkan hasil agent

Agent bekerja di worktree terpisah (`/tmp/gpui-<nama>`, branch `agent/<nama>`)
dan hanya boleh menyentuh `src/views/<nama>.rs` + `progress/<nama>.md`.

```sh
cd spike/gpui-kit
./merge-agent.sh add        # salin 2 file, tolak kalau agent menyentuh file lain, lalu build
# kalau hijau: tambah entri di PROGRESS.md, commit di spike/gpui-kit
```

## Status ringkas

| Area | Selesai | Total |
|---|---|---|
| Shell & navigasi | 5 | 5 |
| Providers | 7 | 9 |
| Tambah provider | 7 | 7 |
| Diff | 6 | 6 |
| Riwayat & backup | 2 | 2 |
| Halaman harness | 3 | 4 |
| Pengaturan | 5 | 5 |
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

### 2026-11-23 02:40 · repo · halaman Providers bisa digulir
- Laporan: di harness `pi` halaman provider tidak bisa di-scroll. Penyebabnya
  hanya grid provider yang punya `overflow_y_scroll`; header + kartu berkas
  config + kartu notice memakan seluruh tinggi, grid menyusut sampai nol, dan
  sisanya terpotong tanpa jalan untuk menggulir.
- Perbaikan: seluruh halaman (header, config, notice, filter, grid, catatan
  read-only) masuk satu wadah gulir `providers-scroll`.
- Verifikasi roda mouse: 8 klik roda bawah → 10.487 piksel berubah; 25 klik
  lagi → tidak berubah (sudah mentok, jadi tak ada konten yang terpotong); dua
  klik roda atas → 11.275 piksel berubah.

### 2026-11-23 02:10 · repo · navigasi dipisah, panel pratinjau dipin
- Keluhan wajar: menu dan daftar harness sama-sama di kiri. Sekarang **tab layar
  pindah ke bar atas** (Providers · Tambah provider · Diff · Riwayat ·
  Pengaturan, plus petunjuk singkat di kanan); **sidebar khusus harness** dengan
  header "HARNESS · n".
- Ditemukan saat verifikasi: tombol "Pratinjau diff" terasa mati karena panel
  hasilnya ikut ter-render di dalam area scroll dan berada di bawah lipatan.
  Panel sekarang dipin di bawah daftar model (`when_some` + `flex_shrink_0`),
  jadi hasil selalu kelihatan.
- Verifikasi pakai piksel (bukan mata): tab aktif bergeser sesuai klik, isi
  layar berganti 5–11 ribu piksel per klik, panel pratinjau memunculkan blok
  peringatan `#fff1e0`, sidebar tidak lagi punya baris menu (hanya 4 harness +
  footer versi).

### 2026-11-23 01:15 · repo · angka final (CPU idle, release)
- Frame pertama berisi data **236 ms**, baca 4 config **29 ms**, jendela ter-map
  **17–19 ms**, RSS **128 MB**, binary **59 MB** (46 MB strip).
- Perbandingan Tauri di mesin sama: jendela 64–84 ms, RSS 144 MB, binary 6,3 MB
  + WebKitGTK sistem. Probe versi selesai 1,7 s di background.

### 2026-11-23 01:05 · agent/add, agent/diff, agent/settings + repo · tiga layar terakhir
- Kerja: tiga agent paralel (worktree terpisah) mengisi `add.rs` (1.168 baris),
  `diff.rs` (675 baris), `settings.rs` (479 baris). Pemilik repo menggabungkan,
  membetulkan import yang salah (`EditOp`/`KeyMode`/`ProviderDraft` ada di
  `engine::model`, bukan `api`), `px(11)` → `px(11.)`, dan `.disabled(...)` yang
  tidak ada di `Stateful<Div>` → `opacity` + guard di handler.
- Verifikasi manual tiap layar (screenshot di `docs/ui-*.png`): stepper 1-2-3
  dengan 61 preset, mode key inline/env (`GROQ_API_KEY`), picker model dengan
  context window + reasoning, ringkasan pratinjau ("0 file baru, 2 file diubah"
  + warning core), diff nyata `models.json` (+13 −0) dan `settings.json` (+2 −1),
  pengaturan dengan toggle tema yang me-repaint seluruh shell.
- Kendala: build dingin di tiga worktree sekaligus menghabiskan >15 menit/orang;
  agent diff menemukan bug porting nyata (slice JS inklusif vs Rust eksklusif,
  masking yang ikut menelan `max_tokens`).

### 2026-11-23 00:45 · repo · Ctrl+K gagal, dicatat sebagai blocker
- Kerja: coba pintasan keyboard lewat `cx.on_action(TypeId, window, ...)` +
  `cx.bind_keys([KeyBinding::new("ctrl-k", ...)])`.
- Hasil: **gagal**. `Window::on_action` hanya boleh dipanggil saat fase paint;
  memanggilnya di `App::new` maupun di `render` sama-sama panic dengan
  "this method can only be called during paint".
- Keputusan: fitur dicabut (tidak ada kode mati), dicatat sebagai F36 `[!]`.
  Yang perlu dicoba berikutnya: daftarkan action lewat elemen root milik sendiri
  (`div().on_action(...)`) atau lewat `Root` gpui-kit, bukan `Context::on_action`.

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
