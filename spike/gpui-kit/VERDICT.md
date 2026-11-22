# Putusan: migrasi ke GPUI Kit?

Ringkasan untuk keputusan, berdasarkan spike di folder ini. Semua angka diukur di
mesin ini (Linux, X11, Rust 1.95.0, release tanpa LTO), bukan perkiraan.

## Yang terbukti

1. **`hakonfig_core` benar-benar bebas UI.** Seluruh spike memakai
   `api::list_harnesses`, `read_harness`, `preview_edit`, `list_backups`,
   `list_notices`, `harness_version` langsung — nol baris core diubah, nol IPC,
   nol DTO serde, nol `capabilities`. Ini keuntungan struktural: migrasi UI
   tidak menyentuh aset paling berharga (engine + 198 test).
2. **Tampilan bisa setara.** Font bundel, palet hangat, kartu provider, dark mode,
   dialog, toast, tabel diff — semuanya native, tanpa webview.
3. **Distribusi Linux jauh lebih ringan** dari sisi dependency: tidak ada
   GTK3/WebKitGTK. Binary 54 MB (42 MB strip) vs 6,3 MB + WebKitGTK 2.50 sistem.

## Yang TIDAK terbukti

1. **"Lebih cepat karena native."** Tidak. Bottleneck startup adalah
   `harness_version()` (spawn `<bin> --version`, ±1.000 ms dari total ±1,3 s),
   dan itu identik di Tauri. Setelah dipindah ke background: frame pertama
   ±340 ms — perbaikan itu bisa dilakukan di GUI Tauri sekarang juga.
2. **"Lebih hemat memori."** Hanya 119 MB vs 144 MB (−17%).
3. **"Iterasi lebih enak."** Tidak. Ubah CSS = simpan; ubah Rust = recompile.
   Untuk aplikasi yang isinya form/list/diff, HTML+Tailwind lebih cepat
   dikerjakan.

## Biaya yang harus dibayar

| Pos | Perkiraan |
|---|---|
| Port 5 layar (providers, tambah, diff, riwayat, pengaturan) | 6–8 ribu baris Rust |
| Setara 5.400 baris JS/CSS/i18n sekarang | 1–2 bulan kerja fokus |
| Perawatan 0.x | rilis besar tiap ±2 minggu, pin `gpui-pre` exact snapshot |
| Kehilangan | devtools, preview browser, ekosistem JS, CSS |

## Rekomendasi

**Jangan port sekarang.** Urutannya:

1. Perbaiki `harness_version` di kode yang ada (cache + lazy). Dapat ±1 detik
   startup tanpa migrasi. Ini sudah dibuktikan di spike ini.
2. Kalau keluhan utama adalah user Linux yang harus memasang WebKitGTK, ukur
   dulu berapa banyak: itu satu-satunya argumen kuat untuk pindah.
3. Kalau tetap mau pindah: pakai spike ini sebagai dasar (lihat `FEATURES.md`
   untuk 38 butir + status, `README.md` untuk jebakan API gpui-kit yang sudah
   ditemukan), dan port per layar sambil Tauri tetap jadi rilis utama.

## Kalau pindah, apa yang paling mahal

Bukan renderer, bukan data. Yang mahal adalah **detail interaksi** yang gratis
di HTML: fokus Tab, seleksi teks, copy-paste, IME, scroll halus, resize,
aksesibilitas. Di GPUI semuanya harus ditulis dan diuji sendiri per layar.
Itu sebabnya 596 baris pertama terasa mudah, dan 5.000 baris berikutnya tidak.
