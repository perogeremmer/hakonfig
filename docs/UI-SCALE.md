# Skala tampilan (HiDPI)

Laptop resolusi tinggi bisa membuat seluruh UI terasa kecil. Ada tiga penyebab
berbeda, dan Hakonfig menangani ketiganya tanpa menebak.

## Kenapa zoom, bukan hanya ukuran font

Seluruh tampilan memakai ratusan nilai px. Mengubah `font-size` saja tidak ikut
membesarkan tombol, padding, atau ikon. Zoom webview menskalakan semuanya
sekaligus, jadi satu angka di `Settings → Skala tampilan` cukup.

## Cara kerjanya

1. **Auto.** `zoom = skala monitor / devicePixelRatio`. Di Windows/macOS/Wayland
   keduanya sama sehingga hasilnya 1.0 (tidak ada yang diubah). Di WebKitGTK
   (Linux) sering terjadi `devicePixelRatio = 1` padahal monitor 2x — di situlah
   UI dirender separuh, dan auto menambalnya. Rasio di bawah 1.1 dianggap cocok,
   supaya pembulatan skala pecahan tidak memicu zoom yang tidak perlu.
2. **Manual.** Pilihan 90/100/110/125/150% disimpan di preferensi aplikasi dan
   menang atas auto. Perhitungannya di
   `hakonfig_core::engine::ui_scale::resolve` (murni, ada unit test-nya).
3. **Cache sebelum paint.** Zoom hasil hitungan ditulis ke
   `<app config dir>/ui-scale.json` dan diterapkan saat `setup`, sebelum halaman
   pertama dilukis. Tanpa itu, pengguna 150% melihat satu frame UI kecil setiap
   membuka aplikasi.
4. **Ctrl + `+` / `-` / `0`.** Naik/turun satu langkah, `Ctrl+0` kembali ke auto.
   Panel Settings ikut tersegarkan supaya angka di layar tidak pernah berbeda
   dari yang berlaku.

## Jendela harus muat di layar

`minWidth 940` (config lama) lebih besar dari ruang CSS laptop 1366x768 @150%
(911 px), jadi jendela tidak bisa dikecilkan sampai muat. Sekarang:

- ukuran jendela dibatasi ke 90% work area monitor saat pertama dibuka;
- minimum jendela diturunkan ke min(config, work area) — di layar sempit minimum
  ikut mengecil, bukan memaksa;
- default config `720x480`.

## Diagnosa

Panel Settings menampilkan angka mentah untuk laporan bug:

```
monitor 2.00x · webview 1x · viewport 800x546 css px · zoom 2.00
```

- `monitor` — `Window::current_monitor().scale_factor()`.
- `webview` — `devicePixelRatio` dari halaman.
- `viewport` — ukuran halaman dalam CSS px.
- `zoom` — nilai yang sedang berlaku.

Kalau `monitor` > `webview`, itu kasus WebKitGTK dan auto seharusnya sudah
menanganinya. Kalau keduanya sama tetapi masih kecil, penyebabnya skala OS
(100% di layar rapat) dan pilihannya manual.

## Tipografi

Ukuran teks dan kotak memakai rem, bukan px:

| Token | Nilai | px lama |
|---|---|---|
| `text-2xs` | 0.625rem | 10 |
| `text-xs` | 0.6875rem | 11 |
| `text-sm` | 0.75rem | 12 |
| `text-base` | 0.8125rem | 13 |
| `text-md` | 0.875rem | 14 |
| `text-lg` | 0.9375rem | 15 |
| `text-xl` | 1.0625rem | 17 |
| `text-2xl` | 1.125rem | 18 |
| `text-3xl` | 1.25rem | 20 |
| `text-4xl` | 1.625rem | 26 |

Alasannya: px tidak mengikuti preferensi ukuran font sistem, rem ikut. Jadi
pengguna yang menaikkan ukuran teks OS tetap mendapat teks lebih besar walau
tidak menyentuh zoom.

`web/scale.test.js` menjaga ini: `text-[13px]` atau `h-[148px]` baru akan
menggagalkan test.

## Batas yang diketahui

- Radius (`rounded-[14px]`), border 1px, dan shadow tetap px: nilai itu tidak
  mengikuti ukuran teks, dan mengubahnya ke rem membuat hairline jadi pecahan.
- Ukuran/posisi jendela belum dipersist. Window manager sudah mengingatnya di
  sebagian besar desktop.
- Skala pecahan di X11 tanpa compositor tetap bergantung pada laporan driver;
  panel diagnosa ada untuk memastikan, bukan menebak.
