# Plan Revamp Tampilan & Alur - Hakonfig

> Status: **P0 + P1 SUDAH DIIMPLEMENTASIKAN** (lihat §13 untuk catatan pelaksanaan).
> Dokumen ini tetap disimpan sebagai spesifikasi desain + rujukan angka kontras.
> Tanggal: 2026-11-06 · Branch: `main` (perubahan working tree belum di-commit)
> Stack: **Tauri v2 + HTML + Tailwind v4 + vanilla JS** (tanpa framework - tetap)
> Revisi 2: menetapkan **maroon** sebagai brand, **bilingual EN default + ID**, tone **casual-friendly**, target **pemula** (diff tertutup).

---

## 0. Ringkasan Keputusan

| Aspek | Kondisi sekarang | Rencana |
|---|---|---|
| Bahasa UI | Indonesia (semua string hardcode) | **Bilingual EN + ID, default English**. Switcher di Settings, pilihan disimpan di `hakonfig.prefs.lang` |
| Font | `ui-sans-serif / system-ui` - netral & kaku | **Plus Jakarta Sans** (bundel lokal, ramah) + **JetBrains Mono** untuk ID/path |
| Ukuran teks terkecil | 10px (25×), 11px (57×) | **Minimum 12px**, body 15px, heading 20–28px |
| Lebar konten | `max-w-5xl` (1024px), konten menggantung di kiri | **960px terpusat**, kartu provider 3 kolom |
| Alur tambah provider | Form panjang 7 field + tabel model manual | **2 langkah**: pilih logo → tempel key → Test & Find models → centang |
| Bentuk | Radius 8–16px, kartu kecil | Radius 12/14/18/20px, tombol 44px, input 44px, sudut lebih lembut |
| Warna | Hijau `#1EAF5D`, garis dingin `#F1F1F3` | **Maroon** (`brand-600 #8C3345`) di atas oat/krem hangat, border `#E8E1D6`, status terpisah (hijau/amber/biru/merah) |
| Uninstall | Fungsi ada di backend+frontend tapi **tidak pernah tampil di UI** | P0 - tombol + confirm dialog + progress |

**Prinsip utama:** satu layar = satu aksi utama. Tidak ada jargon tanpa penjelasan. Tidak ada teks kecil. Tidak ada langkah yang bisa ditebak sendiri.

**Keputusan desain yang sudah dikonfirmasi:**

| # | Keputusan | Alasan |
|---|---|---|
| 1 | **Font: Plus Jakarta Sans** | Humanist-geometris, x-height besar, beda weight jelas; OFL 1.1 (boleh dibundel) |
| 2 | **Brand: maroon** (`#8C3345`) | Hangat & dewasa; beda jelas dari aplikasi AI lain yang serba hijau/biru |
| 3 | **Tone: casual-friendly** | Target pemula; kalimat pendek, kata kerja langsung, tanpa "silakan" |
| 4 | **Target: pemula** → diff mentah **tertutup** default | Ringkasan bahasa manusia di atas, JSON di balik disclosure |
| 5 | **Bilingual EN + ID, default English** | EN untuk audiens global, ID untuk pengguna lokal |

> ⚠️ **Konsekuensi maroon** (dijelaskan di §2.3): brand **tidak lagi** bisa merangkap sebagai warna sukses. Hijau harus jadi token sendiri (`ok`), dan tombol destruktif butuh perlakuan khusus karena merah berdekatan dengan maroon.

---

## 1. Hasil Audit (berbasis kode + 12 tangkapan layar)

Audit dilakukan dengan membaca `web/main.js` (1776 baris), `web/index.html`, `web/styles.src.css`, `web/diff.js`, dan memotret tiap layar di Xvfb (`/tmp/rk-audit/01..12.png`).

### 1.1 Temuan fungsional (bukan estetika)

| # | Temuan | Bukti | Dampak |
|---|---|---|---|
| F1 | **Uninstall tidak bisa diakses.** `uninstallHarness()` ada di `main.js:818` tapi tidak ada elemen `[data-uninstall]` yang pernah dirender. | grep `uninstall` → hanya definisi + pemanggilan API | Fitur yang sudah dikerjakan tidak terlihat pengguna |
| F2 | **Modal tidak mengunci fokus.** `openModal()` hanya fokus elemen pertama; Tab bisa keluar ke halaman di belakang. | `main.js:190-240` | Salah klik bisa menulis ke config yang salah |
| F3 | **Toggle langsung menulis ke disk**, sedangkan tambah/hapus lewat "Tulis ke disk". Dua pola berbeda untuk aksi yang setara. | `toggleProvider()` vs `showPreview()` | Otak pengguna: "kenapa yang ini langsung berubah?" |
| F4 | **Dropdown model default kosong** walau default sudah ada: "- belum ada model di config -" padahal header menampilkan `deepseek/deepseek-chat`. | screenshot 06 | User bingung: "modelnya ada tapi dropdown kosong?" |
| F5 | **Halaman harness menampilkan kalimat teknis** di header: "Katalog di models.json, API key di auth.json. Menonaktifkan provider = mengomentarinya di models.json." | screenshot 02 | Persis yang complained user: "terlalu banyak akrobat" |
| F6 | **Teks 10px dipakai 25× dan 11px 57×.** Tidak ada satu pun teks body di atas 15px. | `grep -o "text-\[[0-9]*px\]"` | Di layar 1200×820, teks effective ~8px. Tidak terbaca |
| F7 | **Form "custom" punya dropdown "Preset provider"** padahal alur baru sudah punya picker terpisah - jalan buntu dobel. | screenshot 07 | Dua tempat untuk hal yang sama |
| F8 | **Tombol Hapus/Ubah hanya 10px** dan menempel di kanan bawah kartu. | screenshot 02 | klik salah → hapus provider |
| F9 | **Konten menggantung ke kiri** di dalam wrapper `max-w-5xl`, jendela 1200px menyisakan ~90px kanan kosong tanpa alasan. | screenshot 01/02 | Terasa tidak centered, tidak rapi |
| F10 | **Install/uninstall untuk Hermes & beberapa entry tidak punya perintah** - kartu fallback "Install manual" tanpa penjelasan. | `harnesses.json` | User klik mati |

### 1.2 Temuan visual per layar

| Layar | Masalah | Detail |
|---|---|---|
| **Home** | Terlalu banyak badge per kartu | "terkonfigurasi" + "eksperimental" + versi + jumlah provider = 4 baris info |
| | Judul "Belum terpasang" menggabung 2 konsep | (a) harness didukung tapi config belum ada, (b) harness belum didukung adapter - beda treatment |
| | Tombol "Install" + "Docs" sejajar warna sama | Install = brand, Docs = teks tipis; tidak jelas hierarki |
| **Harness** | Kartu provider terlalu kecil (avatar 36px, nama 13px) | Brand provider tidak terbaca |
| | Badge "openai-compatible" = jargon tanpa nilai | Semua preset pakai `openai-completions`, jadi badge ini tidak informatif |
| | Card action "Ubah"/"Hapus" 10px | (lihat F8) |
| | Panel "File konfigurasi" menampilkan path absolut `/tmp/rk-demo/pi/agent/models.json` sebagai label utama | Path bukan informasi; yang penting "Model catalog ✓" |
| **Picker provider** | Avatar = monogram GR/GE/CE | Tidak bisa dibedakan sekilas; harus baca teks |
| | Tanpa pencarian | 10 item masih ok, tapi akan painful di 20+ |
| | "OpenAI Compatible (custom)" dismissive | Orang awam merasa "__mungkin__ bukan buat saya" - padahalaitoalah yangPRODUCT |
| **Form preset** | Strip nama provider mengulang judul | "Tambah Groq" + baris "Groq (free tier) https://..." |
| | "Tes koneksi" dan "Temukan model" dua tombol setara | User harus tahu urutannya; User minta: paste key → test → sync → centang |
| | Daftar model cuma ID monospace, semua tercentang | Untuk 100+ model dari sync, mustahil dicentang manual |
| | Counter "5 dari 6 dipakai" |UMEUM: "5 selected" lebih natural |
| **Form custom** | 7 field + tabel 6 kolom | Overload total untuk orang awam |
| | "R = reasoning" | Jargon |
| **Konfirmasi hapus** | Copy panjang 2 kalimat | "Provider "deepseek" akan dihapus dari konfigurasi harness ini. Backup otomatis tetap dibuat, jadi perubahan masih bisa dipulihkan." → 1 kalimat + poin |
| **Pratinjau diff** | Raw JSON dengan +/− | Berguna untuk advanced, menakutkan untuk pemula |
| | Tidak ada ringkasan bahasa manusia | |
| **Install** | Hanya command | Tidak ada penjelasan "ini akan menginstall di komputermu" |
| **Backup** | "Belum ada backup" tanpa menjelaskan kenapa itu penting | Tidak ada entry point "Riwayat perubahan" |

---

## 2. Design System Baru

### 2.1 Font

**Utama: Plus Jakarta Sans** (OFL 1.1, Tokotype, Jakarta)

Kenapa font ini untuk target pemula:
- **Sudut membulat & counter terbuka** - terasa ramah, tidak seperti font sistem yang kaku.
- **Tinggi x-height besar** - tetap terbaca di 12–13px (penting karena kita banyak label kecil).
- **Weight 500/600 jelas beda dari 400** - hierarki bisa dibangun tanpa garis tebal di mana-mana.
- **Angka & huruf pembeda jelas** (`1/l/I`, `0/O`) - penting saat pengguna membaca ID model & API key.
- Kebetulan **buatan Indonesia** - resonansi dengan basis pengguna utama.

**Apakah cocok dengan maroon?** Ya: geometri humanist + maroon gelap menghasilkan kesan "hangat & dewasa"; sans-serif geometris (mis. Poppins) akan terasa terlalu bulat/manis, sedangkan grotesk (mis. Inter) terlalu korporat untuk tone casual.

**Weight yang dipakai:** 400 (body) · 500 (label sekunder) · 600 (tombol, label form) · 700 (judul) · 800 (display). **Tidak** memakai italic sama sekali (mengurangi bundle & tidak dipakai di UI).

**Mono: JetBrains Mono** (OFL 1.1) - hanya untuk ID model, path file, command shell, JSON.

**Cara supplying (WAJIB lokal - app desktop bisa offline):**

```
web/fonts/
├── PlusJakartaSans-Regular.woff2
├── PlusJakartaSans-Medium.woff2
├── PlusJakartaSans-SemiBold.woff2
├── PlusJakartaSans-Bold.woff2
├── PlusJakartaSans-ExtraBold.woff2
├── JetBrainsMono-Regular.woff2
├── JetBrainsMono-SemiBold.woff2
└── LICENSES.md          ← catatan lisensi OFL, wajib ikut repo
```

```css
/* web/styles.src.css */
@font-face {
  font-family: "Plus Jakarta Sans";
  src: url("./fonts/PlusJakartaSans-Regular.woff2") format("woff2");
  font-weight: 400; font-style: normal; font-display: swap;
}
/* …-Medium 500, -SemiBold 600, -Bold 700, -ExtraBold 800 */

@font-face {
  font-family: "JetBrains Mono";
  src: url("./fonts/JetBrainsMono-Regular.woff2") format("woff2");
  font-weight: 400; font-display: swap;
}
```

> Catatan: `font-display: swap` + `preload` link di `index.html` untuk 2 weight utama (400, 700) supaya tidak ada flash font fallback.

### 2.2 Skala Tipografi

Token baru di `@theme` (Tailwind v4):

```css
--font-sans: "Plus Jakarta Sans", ui-sans-serif, system-ui, sans-serif;
--font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace;
```

| Token | Ukuran | Line-height | Weight | Pemakaian |
|---|---|---|---|---|
| `text-display` | 28px | 1.2 | 800 | Judul layar utama ("Choose your tool") |
| `text-title` | 22px | 1.25 | 700 | Judul section, nama harness |
| `text-heading` | 18px | 1.3 | 700 | Judul modal, nama provider |
| `text-body` | 15px | 1.55 | 400/500 | Teks utama, label form |
| `text-small` | 13px | 1.5 | 500 | Teks sekunder, caption |
| `text-micro` | 12px | 1.45 | 600 | Badge, counter, label grup |
| `text-code` | 13px | 1.5 | 400 mono | Path, ID model, command |

**Aturan keras:** tidak boleh ada `text-[10px]` lagi. Semua teks minimal 12px. Label form selalu 13px/600 (bukan 11px).

### 2.3 Palet Warna

**Netral hangat** (menggantikan abu-abu dingin `#F1F1F3`):

| Token | Hex | Pemakaian |
|---|---|---|
| `canvas` | `#FAF7F2` | Latar aplikasi (oat hangat) |
| `surface` | `#FFFFFF` | Kartu, modal, input |
| `surface-muted` | `#F5F1EA` | Header baris, chip, area inset, hover |
| `surface-sunken` | `#EFE9DF` | Track switch off, track progress |
| `border` | `#E8E1D6` | Garis kartu, garis input |
| `border-strong` | `#D9D0C2` | Border input saat hover, track switch |
| `ink` | `#221F1B` | Teks utama (hangat, bukan `#0A0A0A` yang keras) |
| `ink-secondary` | `#6B6459` | Teks sekunder |
| `ink-muted` | `#9C9488` | Placeholder, teks disabled |
| `on-brand` | `#FFFFFF` | Teks di tombol brand |

**Brand - maroon (10 langkah, semua angka kontras terukur):**

| Token | Hex | Putih di atasnya | Sebagai teks di putih | Pemakaian |
|---|---|---|---|---|
| `brand-50` | `#FBF3F4` | 1.09:1 | 1.09:1 | Background banner lembut, hover baris |
| `brand-100` | `#F7E5E8` | 1.21:1 | 1.21:1 | Chip brand, tint badge, avatar provider utama |
| `brand-200` | `#EDCBD1` | 1.49:1 | 1.49:1 | Border chip brand |
| `brand-300` | `#DCA3AC` | 2.13:1 | 2.13:1 | **Dekoratif saja** - garis, ilustrasi, disabled |
| `brand-400` | `#C37583` | 3.39:1 | 3.39:1 | Ikon besar ≥ 24px, grafik |
| `brand-500` | `#A34758` | 5.83:1 | 5.83:1 | Ring fokus (`/25`), border input fokus, switch on |
| `brand-600` | `#8C3345` | **7.85:1** | 7.85:1 | **Tombol primary** (bg), teks putih aman semua ukuran |
| `brand-700` | `#7A2A3B` | 9.45:1 | 9.45:1 | Hover tombol primary, **teks link di atas putih** |
| `brand-800` | `#63202F` | 11.81:1 | 11.81:1 | Teks brand di atas `brand-100`, logo app |
| `brand-900` | `#4A1824` | 14.51:1 | 14.51:1 | Teks di atas `brand-50/100`, heading di banner |

> **Kenapa maroon aman:** pada hijau lama, `#17A96B` hanya 3.2:1 di atas putih sehingga tidak boleh jadi teks. Maroon `brand-600` sudah **7.85:1** - satu token bisa dipakai untuk tombol *dan* teks, aturan lebih sederhana: `brand-600` untuk permukaan aksi, `brand-700` untuk hover & teks link, `brand-800/900` untuk teks di atas tint.

**⚠️ Tiga aturan khusus karena brand = maroon:**

1. **Brand ≠ sukses.** Maroon tidak boleh dipakai untuk "Ready / Saved / Connected". Semua status sukses memakai token `ok` (hijau) di bawah. Tanpa ini, "provider siap" dan "tombol simpan" tampak sama.
2. **Danger ≠ maroon.** Merah destruktif (`#B42318`) berdekatan hue dengan maroon (`#7A2A3B`). Mitigasi:
   - Tombol destruktif **tidak boleh** tampil berdampingan dengan tombol primary dalam satu baris footer.
   - Destruktif selalu di dalam menu `⋯` (dipisah garis) atau modal konfirmasi tersendiri.
   - Tombol destruktif **selalu** pakai ikon (`trash`) + label, bukan warna saja.
   - Warna destruktif digeser ke merah-oranye `#B42318` (bukan merah murni) agar tidak terbaca sebagai "maroon versi terang".
3. **Avatar provider** memakai tint maroon untuk provider pertama/preset utama, sisanya tetap beragam (biru, amber, violet, hijau, rose) supaya 10 kartu tidak jadi lautan maroon.

**Netral hangat** (menggantikan abu-abu dingin `#F1F1F3`):

| Token | Hex | Kontras teks | Pemakaian |
|---|---|---|---|
| `canvas` | `#FAF7F2` | - | Latar aplikasi (oat hangat) |
| `surface` | `#FFFFFF` | - | Kartu, modal, input |
| `surface-muted` | `#F5F1EA` | - | Header baris, chip, area inset, hover |
| `surface-sunken` | `#EFE9DF` | - | Track switch off, track progress |
| `border` | `#E8E1D6` | - | Garis kartu, garis input |
| `border-strong` | `#D9D0C2` | - | Border input saat hover, track switch |
| `ink` | `#221F1B` | 16.4:1 di surface · 15.4:1 di canvas | Teks utama (hangat, bukan `#0A0A0A` yang keras) |
| `ink-secondary` | `#6B6459` | 5.85:1 di surface · 5.47:1 di canvas | Teks sekunder, helper form |
| `ink-muted` | `#9C9488` | 3.0:1 | **Hanya placeholder & disabled** - bukan informasi |
| `on-brand` | `#FFFFFF` | - | Teks di tombol brand |

> Maroon di atas krem: `brand-600` di `canvas` = **7.35:1**, jadi tombol primary tetap terbaca baik di kartu putih maupun latar krem.

**Status & aksen (5 tint, semua lolos AA untuk teks 12–13px):**

| Token | Background | Teks | Kontras fg/bg | Pemakaian |
|---|---|---|---|---|
| `ok` | `#E7F6EE` | `#0B5C3B` | **7.21:1** | "Key saved", "Ready", "Connected", "Installed" |
| `info` | `#E9F0FF` | `#1D4ED8` | **5.86:1** | Banner info, "OpenAI style" (detail teknis) |
| `warn` | `#FFF1E0` | `#8A4A08` | **6.17:1** | "No key yet", "Free tier", "Not verified" |
| `neutral` | `#F1EDE5` | `#6B6459` | **5.01:1** | "Experimental", "Manual setup", "Disabled" |
| `danger` | `#FDECEA` | `#B42318` | **5.75:1** | Error, konfirmasi hapus/uninstall |
| `danger-600` | - | bg `#B42318` + teks putih | 6.57:1 | Tombol destructive (jarang, selalu di modal) |

**Tabel kontras lengkap (hasil hitung, bukan perkiraan):**

| Kombinasi | Rasio | Status |
|---|---|---|
| `ink` di `surface` | 16.41:1 | AAA |
| `ink` di `canvas` | 15.35:1 | AAA |
| `ink-secondary` di `surface` | 5.85:1 | AA |
| `ink-secondary` di `canvas` | 5.47:1 | AA |
| `ink-muted` di `surface` | 3.00:1 | ⚠️ dekoratif/disabled saja |
| putih di `brand-600` | 7.85:1 | AAA - tombol primary |
| putih di `brand-700` (hover) | 9.45:1 | AAA |
| `brand-600` di `canvas` | 7.35:1 | AA+ |
| `brand-700` di `surface` | 9.45:1 | AAA - teks link |
| `brand-900` di `brand-100` | ~11.9:1 | AAA - teks di chip |
| `ok` fg/bg | 7.21:1 | AAA |
| `info` fg/bg | 5.86:1 | AA |
| `warn` fg/bg | 6.17:1 | AA |
| `neutral` fg/bg | 5.01:1 | AA |
| `danger` fg/bg | 5.75:1 | AA |
| `danger-600` + putih | 6.57:1 | AA |

### 2.4 Bentuk, Radius, Shadow, Spacing

**Radius:**

| Token | Nilai | Pemakaian |
|---|---|---|
| `r-xs` | 6px | Checkbox, chip kecil |
| `r-sm` | 10px | Baris dalam daftar, badge |
| `r-md` | 12px | Input, tombol, select |
| `r-lg` | 14px | Kartu inner, daftar model |
| `r-xl` | 18px | Kartu provider / harness tile |
| `r-2xl` | 20px | Modal, sheet |
| `r-full` | 999px | Pill, switch, avatar bulat |

**Shadow (hangat, lembut):**

```css
--shadow-xs: 0 1px 2px rgba(34, 31, 27, 0.05);
--shadow-sm: 0 1px 2px rgba(34,31,27,.04), 0 4px 12px -6px rgba(34,31,27,.10);
--shadow-md: 0 2px 4px rgba(34,31,27,.04), 0 12px 28px -12px rgba(34,31,27,.18);
--shadow-lg: 0 4px 8px rgba(34,31,27,.05), 0 24px 48px -20px rgba(34,31,27,.24);
```

**Spacing:** base 4px. Padding kartu 16px (kecil) / 20px (besar). Gap grid 16px. Section spacing 32px. Konten page padding 24px horizontal, 32px vertical.

### 2.5 Bentuk komponen (spesifikasi)

**Tombol**

| Varian | Height | Padding | Font | Radius | Warna |
|---|---|---|---|---|---|
| Primary | 44px | 0 20px | 14px/600 | 12px | bg `brand-600`, teks putih, shadow-sm |
| Secondary | 40px | 0 16px | 14px/600 | 12px | bg `surface`, border `border`, teks `ink` |
| Tertiary/Ghost | 36px | 0 12px | 13px/600 | 10px | transparan, teks `ink-secondary` |
| Danger | 44px | 0 20px | 14px/600 | 12px | bg `danger-600`, teks putih |
| Icon-only | 36×36 | - | - | 10px | bg `surface`, border `border` |

Semua tombol: `active:scale-[.98]`, `focus-visible:ring-[3px] ring-brand-500/25`, `disabled:opacity-45 disabled:cursor-not-allowed`, transition 120ms.

**Input**

| Elemen | Height | Font | Radius | Border | Focus |
|---|---|---|---|---|---|
| Input teks | 44px | 15px | 12px | 1px `border` | border `brand-500` + ring 3px `brand-500/20` |
| Input API key | 48px | 14px mono | 12px | 1px `border-strong` | sama; tombol Show/Hide **di dalam** field (kanan, 36×36) |
| Select | 44px | 15px | 12px | 1px `border` | chevron custom di kanan |
| Checkbox | 20×20 | - | 6px | 1px `border-strong` | centang `brand-600` |
| Radio | 20×20 | - | full | 1px `border-strong` | |
| Switch | 44×26 | - | full | - | track on `brand-500`, knob 20px putih + shadow |
| Search | 40px | 14px | 10px | 1px `border` |ikon magnifier di kiri |

Label form: 13px/600 `ink`, jarak ke input 6px, helper text 12px/500 `ink-secondary` (jarak 4px).

**Kartu**

- Harness tile: radius 18px, bg `surface`, border `border`, padding 20px, tinggi min 190px, `hover:-translate-y-1 hover:shadow-md hover:border-brand-500/40`.
- Provider card: radius 18px, bg `surface`, border `border`, padding 16px, min-height 172px, layout: baris atas (logo 40px + switch), nama 15px/600, sub 13px, chip status, footer menu.
- Section card (default model, config files): radius 16px, bg `surface`, border `border`, padding 18px.

**Badge / chip**

- Tinggi 22px, radius `r-full`, padding 0 10px, font 12px/600.
- Success `brand-100`/`brand-900` · Info `#E9F0FF`/`#1D4ED8` · Warn `#FFF1E0`/`#B45309` · Neutral `#F1EDE5`/`#6B6459` · Danger `#FDECEA`/`#B42318`.

**Modal**

| Jenis | Lebar | Radius | Padding body | Footer |
|---|---|---|---|---|
| Confirm | 480px | 20px | 24px | sticky, tombol 44px |
| Tambah provider | 640px | 20px | 24px | sticky |
| Advanced | 880px | 20px | 24px | sticky |
| Diff | 960px | 20px | 24px | sticky |
| Install/Progress | 560px | 20px | 24px | sticky |

- Overlay: `bg-[rgba(34,31,27,.35)]` + `backdrop-blur-[2px]`.
- Header: judul 18px/700 + subtitle 13px `ink-secondary`, border-bottom `border`, tinggi minimal 72px, tombol close 36×36 di kanan atas.
- Footer: border-top, tinggi 68px, tombol rata kanan, jarak 8px.
- **Focus trap** (F2): Tab/Shift+Tab terjebak di dalam modal; Esc menutup; fokus kembali ke elemen pemicu saat modal ditutup.

**Toast**

- Pojok kanan bawah, lebar 360px, radius 14px, border kiri 4px berwarna status, `surface` background + shadow-lg, auto-dismiss 4s (error 8s), tombol **Undo** opsional (untuk toggle & delete).

### 2.6 Ikon & Logo

- Ganti emoji (🔍 🧩 ✕) dengan **satu set SVG inline** gaya Lucide (ISC): stroke 1.75, round cap/join, 20px default.
  - `web/icons.js` exporting object of SVG path strings.
- **Logo provider asli** (bukan monogram): bundel SVG di `web/assets/providers/`:
  - OpenAI, Anthropic, Google Gemini, Groq, DeepSeek, Mistral, Cerebras, Together AI, OpenRouter, NVIDIA, Ollama, OpenCode.
  - Sumber: Simple Icons (CC0-1.0) + brand mark resmi. Wajib: `web/assets/providers/LICENSES.md`.
  - Fallback: monogram 2 huruf dengan warna tint (tetap ada kalau file hilang).
- Logo harness (Pi, oh-my-pi, OpenCode, Hermes) juga dibundel.

### 2.7 Grid & Lebar Konten

| Zona | Lebar | Grid |
|---|---|---|
| App shell | 100% | header sticky 64px, konten scroll |
| Konten | **960px** terpusat (`--container-app: 60rem`) | padding horizontal 24px |
| Harness tile grid | - | 4 kolom ≥ 1180px · 3 kolom ≥ 900px · 2 kolom < 700px |
| Provider card grid | - | 3 kolom (960px → card ~296px) · 4 kolom ≥ 1280px · 2 kolom < 760px |
| Preset picker | - | 4 kolom di modal 640px (card ~140px) · 2 kolom < 560px |
| Modal | lihat tabel 2.5 | - |

Perubahan dari sekarang: `max-w-5xl` → `max-w-[960px] mx-auto`, dan **headbar ikut** memakai container yang sama supaya logo app sejajar dengan isi.

---

## 3. Alur & Langkah (Step-by-Step)

### 3.1 Home - "Choose your tool"

Tujuan: pengguna awam menjawab 2 pertanyaan - *"tool apa yang saya pakai?"* dan *"tool saya sudah siap?"*

```
┌────────────────────────────────────────────────────────────────┐
│ [logo] Hakonfig                    Set up your AI coding tools  │  header 64px, container 960
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Choose your tool                                    (20px/700) │
│  Pick the one you use. Hakonfig sets up the providers for it.  │
│                                                                │
│  Installed (2)                                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │  [PI]    │ │  [OM]    │ │  [OC]    │ │  [HE]    │            │
│  │  Pi      │ │  oh-my-pi│ │  OpenCode│ │  Hermes  │            │
│  │  Ready   │ │  Needs a │ │  Ready   │ │  1 setup │            │
│  │  2 prov. │ │  key     │ │  1 prov. │ │  Experim.│            │
│  │  v1.5.0  │ │          │ │          │ │          │            │
│  │  ⋯       │ │          │ │          │ │          │            │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘            │
│                                                                │
│  Get another tool (6)                                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │  [CL]    │ │  [CX]    │ │  [GM]    │ │  [AI]    │            │
│  │  Claude  │ │  Codex   │ │  Gemini  │ │  Aider   │            │
│  │  Code    │ │  CLI     │ │  CLI     │ │          │            │
│  │  Coding  │ │  OpenAI  │ │  Google  │ │  Pair …  │            │
│  │  agent   │ │          │ │          │ │          │            │
│  │ [Install]│ │ [Install]│ │[Install] │ │[Install] │            │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘            │
└────────────────────────────────────────────────────────────────┘
```

Perubahan konkret:

1. **Hapus badge "terkonfigurasi"** - ganti dengan 1 baris status polos: `Ready · 2 providers` / `Needs an API key` / `1 provider · no key`.
2. **Badge "eksperimental"** jadi chip kecil di pojok kanan atas kartu (abu, bukan kuning mencolok).
3. **Versi** (`v1.5.0`) jadi 12px `ink-muted` di bawah nama - bukan mono besar.
4. **Kartu yang belum didukung** (Claude Code, dll): TIDAK lagi pakai badge "belum didukung" yang besar. Ganti jadi: nama + 1 kalimat deskripsi + tombol **Install** (primary, brand) + link **Docs**. Badge "belum didukung" dihapus dari kartu, dipindah ke tooltip pada tombol Install: *"Hakonfig can install the tool, but can't edit its provider config yet."*
5. **Kartu tanpa perintah install otomatis** (Hermes, atau entry `installUnix: null`): tombol utama berubah jadi **"Manual setup"** (secondary) + di bawahnya teks 12px: *"No automatic install yet - see the setup guide →"*. Tidak ada tombol mati.
6. **Menu "⋯" per kartu terpasang** (klik kanan / klik ikon): `Open`, `Reload`, `Uninstall…` (danger, di bawah separator). **Ini yang membuat Uninstall bisa diakses (F1).**
7. **Section title** bilingual-friendly: "Installed" / "Not installed" - tiap section punya 1 kalimat penjelasan (12px), misal "Ready to go." / "Install one to start configuring providers."

### 3.2 Harness page - "Pi"

```
┌────────────────────────────────────────────────────────────────┐
│ [←] [PI] Pi                            [History] [⋯] [+ Add provider]│
│      3 providers · 1 needs a key                               │
├────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Default model                          [Change]          │  │  ← 1 baris, bukan 2 kolom
│  │ [DE] DeepSeek   deepseek-chat                            │  │
│  │ Used when you start Pi.                                   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  ▸ Config files (3)                          [Show in folder]  │  ← disclosure, default tertutup
│                                                                │
│  Providers (3)                                    [+ Add]      │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐                  │
│  │ [DE]    ◉  │ │ [GE]    ◉  │ │ [GR]    ◉  │                  │
│  │ DeepSeek   │ │ Google Ge… │ │ Groq       │                  │
│  │ 2 models   │ │ 3 models   │ │ 6 models   │                  │
│  │ ✓ Key saved│ │ ⚠ No key   │ │ ✓ Key saved│                  │
│  │ ⋯          │ │ ⋯          │ │ ⋯          │                  │
│  └────────────┘ └────────────┘ └────────────┘                  │
└────────────────────────────────────────────────────────────────┘
```

Perubahan konkret:

1. **Header**: subtitle kalimat teknis → kalimat manusia: `3 providers · 1 needs a key`. Kalimat teknis dipindah ke tooltip ikon "?" di sebelah nama harness.
2. **Header action**: 1 tombol primary (`+ Add provider`, 44px) + 2 secondary (`History`, `⋯` menu berisi `Reload`, `Open config folder`, `Uninstall…`).
3. **"Default aktif" + "File konfigurasi" tidak lagi 2 kolom.** Default jadi kartu full-width (paling penting, paling sering dipakai). File konfigurasi jadi **disclosure tertutup** - dibuka saat memang perlu.
4. **Isi disclosure "Config files"** (bukan path mentah di depan):
   ```
   Model catalog      ~/.pi/agent/models.json      ✓
   Credentials        ~/.pi/agent/auth.json         ✓
   Preferences        ~/.pi/agent/settings.json     ✓
   ```
   Path dalam mono 12px `ink-muted` + ikon folder di kanan per baris. Path absolut hanya muncul saat hover (tooltip).
5. **Kartu provider**:
   - Logo asli 40px (bukan monogram).
   - Nama 15px/600, sub-line `2 models` 13px `ink-secondary`.
   - **Ganti badge "openai-compatible"** dengan chip status yang informatif: `✓ Key saved` (hijau) / `⚠ No key` (kuning) / `⏸ Disabled` (abu). Badge "openai-compatible" pindah ke modal edit (detail teknis).
   - **Switch** tetap di kanan atas (pola yang sudah bagus) tapi diperbesar ke 44×26 dan diberi `aria-label` + tooltip.
   - **Ganti "Ubah"/"Hapus"** link 10px dengan **ikon `⋯` 36×36** di kanan bawah → menu `Edit`, `Duplicate`, `Delete` (danger, atas pemisah).
6. **Empty state** (belum ada provider): ikon 48px + judul "No providers yet" + 1 kalimat ("Groq, Gemini and Cerebras have free tiers.") + tombol primary besar "Add your first provider".
7. **Toggle** (F3): tetap langsung menulis, TAPI:
   - Toast sukses muncul dengan **tombol Undo**: `DeepSeek disabled` · `Undo`.
   - Pesan menjelaskan efeknya dalam bahasa manusia, bukan "Menonaktifkan provider = mengomentarinya".
   - Bila harness tidak mendukung toggle → toast `This harness doesn't support pausing a provider yet.` (dalam bahasa manusia).

### 3.3 Tambah Provider - alur utama (⭐ inti perubahan)

#### Langkah 1 - Pilih provider (modal 640px)

```
┌──────────────────────────────────────────────────────────────┐
│  Add a provider                                         [×]  │
│  Pick one to start. You'll only need your API key.           │
├──────────────────────────────────────────────────────────────┤
│  [🔍 Search providers…                                      ]  │  ← baru (opsional)
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                │
│  │ [logo] │ │ [logo] │ │ [logo] │ │ [logo] │                │
│  │ Groq   │ │ Gemini │ │Cerebras│ │DeepSeek│                │
│  │ Free   │ │ Free   │ │ Free   │ │        │                │
│  └────────┘ └────────┘ └────────┘ └────────┘                │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                │
│  │ OpenRouter │ Mistral │Together│  NVIDIA │                │
│  └────────┘ └────────┘ └────────┘ └────────┘                │
│  ┌────────┐ ┌────────┐                                       │
│  │OpenCode Go│ Ollama │                                      │
│  └────────┘ └────────┘                                       │
│  ─────────────────────────────────────────────────────────── │
│  Have your own endpoint?  Use a custom OpenAI-compatible API →│
└──────────────────────────────────────────────────────────────┘
```

- Kartu: logo asli (40px) + nama (14px/600) + chip "Free tier" hanya jika memang free.
- Baris bawah: **link** (bukan kartu dashed yang terlihat seperti opsi utama): `Use a custom OpenAI-compatible API →` - terbuka ke Advanced.
- Hapus tombol "Batal" di footer (Esc / klik luar / × suffice) - footer cukup `Cancel` kecil di kanan? Tetap: cukup 1 tombol `Cancel` di kiri footer.

#### Langkah 2 - Tempel key + test + sync model (modal 640px)

```
┌──────────────────────────────────────────────────────────────┐
│  Add Groq                                              [×]   │
│  Just paste your API key - we handle the rest.               │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │  YOUR API KEY                                         │  │
│  │  ┌──────────────────────────────────┐  ┌────────────┐  │  │
│  │  │ gsk_••••••••••••••••••••••••    │  │ │ 👁 Show    │  │  │
│  │  └──────────────────────────────────┘  └────────────┘  │  │
│  │  Need a key?  Get one at console.groq.com/keys  ↗      │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ ✓ Connected · 142 ms                                 │  │  ← muncul setelah test
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  MODELS (24 FOUND)                        [Select all] [Clear]│
│  ┌────────────────────────────────────────────────────────┐  │
│  │ [🔍 Filter models…]                                   │  │
│  │ ┌────────────────────────────────────────────────────┐ │  │
│  │ │ ☑ Llama 3.3 70B                    llama-3.3-70b   │ │  │
│  │ │ ☑ Llama 3.1 8B                     llama-3.1-8b    │ │  │
│  │ │ ☑ GPT OSS 120B                     gpt-oss-120b    │ │  │
│  │ │ ☐ Qwen 3 32B                       qwen3-32b       │ │  │
│  │ └────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
│                              5 of 24 selected                 │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]      [ Add 5 models ]    │
└──────────────────────────────────────────────────────────────┘
```

Perubahan konkret:

1. **Satu tombol utama**: `Test & find models` (bukan dua tombol terpisah). Urutan otomatis: test koneksi → sync model. Status ditampilkan inline sebagai 2 baris terpisah:
   - Baris 1 (test): `✓ Connected · 142 ms` hijau / `✕ Invalid API key - check the key and try again` merah.
   - Baris 2 (sync): `24 models found` / `Couldn't load models - you can pick from the built-in list` (amber).
2. **Tombol Save berubah label dinamis**: `Add 5 models` (jumlah terpilih). Disabled sampai key teruji (state machine jelas).
3. **Daftar model** menampilkan **nama model** (dari metadata preset / slug yang di prettify) sebagai teks utama 14px/500, dengan **ID asli mono 12px muted** di kanan. Untuk model hasil sync yang tidak punya nama, pretty-print ID (`llama-3.3-70b-versatile` → `Llama 3.3 70B Versatile`) - vanilla, tanpa dependensi.
4. **Filter box** untuk daftar panjang + `Select all` / `Clear`.
5. **Grouping** (opsional, tahap 2): `Recommended` (3 model bawaan preset) di atas, lalu `All models`.
6. **Hapus blok strip provider** yang mengulang judul.
7. **Link "Get one at console.groq.com/keys"** menjadi tombol link dengan ikon ↗ (membuka browser). Di desktop Tauri, `shell.open` via backend (perlu command baru `open_external` - dicatat di §7).
8. **Show/Hide** jadi tombol di dalam field (kanan), bukan tombol terpisah di luar - mengurangi noise.

### 3.4 Advanced / custom OpenAI-compatible (modal 880px)

- Berpindah ke **mode "Advanced"**, ditandai chip `Advanced` di kanan atas modal.
- Layout: **accordion 4 bagian** (semua tertutup default kecuali "Endpoint"):
  1. **Endpoint** - `Base URL` (input mono, 13px), `API type` (select), `Key storage` (radio: `Save in the config file (easiest)` / `Use an environment variable`).
  2. **API key** - sama seperti alur utama (48px, Show/Hide).
  3. **Models** - **ganti tabel 6 kolom** menjadi daftar baris kartu:
     ```
     ┌───────────────────────────────────────────────────────┐
     │ model id *                [✕]                        │
     │ display name              context 128K   max 8K      │
     │                           [ ] reasoning              │
     └───────────────────────────────────────────────────────┘
     [+ Add model]
     ```
     Field wajib ditandai `*`; helper "Model id is what the provider calls it."
  4. **Advanced JSON** - textarea JSON mentah untuk yang sudah tahu (opsional, tersembunyi).
- **Hapus dropdown "Preset provider"** (F7) - sudah ada picker terpisah; ganti dengan link kecil: `Looking for a preset? Back to provider list`.
- Label 13px/600, helper 12px, jargon diberi tooltip: `context window` → "How much the model can read at once".

### 3.5 Set default model (modal 560px)

```
┌──────────────────────────────────────────────────────────────┐
│  Default model                                         [×]   │
│  This is the model Pi starts with.                          │
├──────────────────────────────────────────────────────────────┤
│  PROVIDER                                                  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ [DE] DeepSeek                                  ▾      │  │
│  └────────────────────────────────────────────────────────┘  │
│  MODEL                                                     │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ deepseek-chat                                    ▾      │  │
│  └────────────────────────────────────────────────────────┘  │
│  (dropdown diisi dari model milik provider - tidak boleh kosong)│
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]           [Save]        │
└──────────────────────────────────────────────────────────────┘
```

- **Perbaiki F4**: dropdown model wajib terisi dari `provider.models`; jika provider tidak punya daftar model, tampilkan input teks dengan placeholder `model id` + helper `This provider doesn't list models - type the id.`
- Kolom provider menampilkan logo + nama, bukan `DeepSeek (deepseek)`.
- Helper satu kalimat: "This is the model Pi starts with."

### 3.6 Hapus provider

```
┌──────────────────────────────────────────────────────────────┐
│  Delete DeepSeek?                                      [×]   │
├──────────────────────────────────────────────────────────────┤
│  DeepSeek will be removed from Pi's config.                 │
│  • A backup is saved first, so you can restore it.           │
│  • Other providers are not affected.                         │
│                                                               │
│  ☐ Don't ask me again                                       │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]        [ Delete ]       │
└──────────────────────────────────────────────────────────────┘
```

- 1 kalimat + 2 bullet (bukan paragraf).
- Setelah konfirmasi → layar **Review changes** (lihat 3.7) → `Apply changes`.
- Opsi "Don't ask again" tetap ada, sekarang berbunyi jelas: "Don't ask me again" (Inggris).

### 3.7 Review changes / Diff (modal 960px)

```
┌──────────────────────────────────────────────────────────────┐
│  Review changes                                       [×]   │
│  Nothing has been written to disk yet.                       │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ✅ Adds Groq to Pi                                   │  │  ← ringkasan bahasa manusia
│  │  ✅ Creates auth.json with your API key (hidden)       │  │
│  │  💾 A backup is saved automatically                    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  ▸ Technical details                                        │  ← disclosure, default tertutup
│  ┌────────────────────────────────────────────────────────┐  │
│  │  + "groq": {                                          │  │
│  │  +   "baseUrl": "https://api.groq.com/openai/v1"       │  │
│  │  …                                                      │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]      [ Apply changes ]  │
└──────────────────────────────────────────────────────────────┘
```

- **Ringkasan bahasa manusia selalu di atas** (dihasilkan backend dari jenis op).
- Diff mentah di bawah dalam `<details>` **tertutup default** - reducir Meeting to meet user awam; advanced bisa buka.
- Tombol: `Apply changes` (bukan "Tulis ke disk").
- Badge `Secrets hidden` pindah jadi catatan kecil di bawah ringkasan: `🔒 API keys are hidden in this preview.`

### 3.8 Install / Uninstall harness

**Install** (klik `Install` di kartu "Get another tool"):

```
┌──────────────────────────────────────────────────────────────┐
│  Install Claude Code                                  [×]   │
├──────────────────────────────────────────────────────────────┤
│  This installs Claude Code on your computer using npm.       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ $ npm install -g @anthropic-ai/claude-code             │  │
│  └────────────────────────────────────────────────────────┘  │
│  ⓘ  Hakonfig can install the tool, but can't edit its       │
│     provider config yet.                                     │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]          [ Install ]     │
└──────────────────────────────────────────────────────────────┘
```

**Progress** (setelah klik Install):

```
┌──────────────────────────────────────────────────────────────┐
│  Installing Claude Code                                [×]   │
├──────────────────────────────────────────────────────────────┤
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │
│  Installing… this can take a minute.                         │
│                                                               │
│  ▸ Show log                                                  │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ npm WARN deprecated …                                  │  │
│  │ added 42 packages in 3s                               │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                                          (tombol nonaktif)    │
└──────────────────────────────────────────────────────────────┘
```

- Progress bar **indeterminate** (maroon `brand-500`, animasi sweep) + teks status yang jujur: "Installing… this can take a minute." Tidak ada angka persen palsu.
- Log **tertutup default**, dibuka dengan `▸ Show log`.
- Selesai → modal ditutup + toast `Claude Code installed` (**`ok` hijau**, bukan maroon) + kartu di home auto-refresh jadi "Installed".

**Uninstall** (klik `⋯` → `Uninstall…` di kartu terpasang):

```
┌──────────────────────────────────────────────────────────────┐
│  Uninstall Claude Code                                  [×]   │
├──────────────────────────────────────────────────────────────┤
│  This removes Claude Code from your computer.                │
│  ✅ Your provider settings stay untouched.                   │
│  ⓘ  You can install it again anytime.                       │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel]      [ Uninstall ]      │
└──────────────────────────────────────────────────────────────┘
```

- Konfirmasi **wajib** (sesuai permintaan), lalu progress modal yang sama dengan install (label "Uninstalling…").
- Setelah selesai → toast `Claude Code uninstalled` + kartu kembali ke section "Not installed".
- Konfirmasi untuk uninstall **tidak** punya opsi "don't ask again" (hanya 1 klik ekstra, tidak otomatis berbahaya).

### 3.9 Settings & History

- **Settings** (modal 480px), tiga bagian:

  | Bagian | Kontrol | Default |
  |---|---|---|
  | **Language** | Segmented control 2 pilihan: `English` · `Bahasa Indonesia` | English |
  | **Confirmations** | Switch `Ask before deleting a provider` | ON |
  | **About** | Versi app, `verifikasi update` (link repo), `Setup guide` (link docs) | - |

  Perilaku language switcher:
  - Klik → `setLang("id")` → simpan ke `localStorage["hakonfig.prefs"].lang` → **re-render seluruh layar saat itu** (tanpa restart, tanpa kehilangan state modal).
  - `<html lang>` ikut berubah; `Intl` (format tanggal backup) ikut berubah.
  - Modal Settings sendiri juga langsung berubah bahasa - jadi pengguna melihat efeknya seketika.
  - Toast konfirmasi: `Language set to English` / `Bahasa diatur ke Indonesia`.
- **History / Backups** (modal 560px, diubah dari "Backup"):
  - Judul `History` + subtitle `Every change is backed up automatically.`
  - Timeline: `Today 14:32 - Added Groq` → `Restore` button per entri (ikon jam + teks), bukan path directory mentah.
  - Path directory lengkap hanya di tooltip.

---

## 4. Form & Input - Spesifikasi Konsisten

| Konteks | Maks. field sekaligus | Height input | Helper text | Placeholder |
|---|---|---|---|---|
| Tambah provider (preset) | **1** (API key) | 48px | Ya (link docs) | `Paste your API key` |
| Tambah provider (advanced) | 4 (accordion, 1 terbuka) | 44px | Ya | kontekstual |
| Set default | 2 (select) | 44px | 1 baris | - |
| Konfirmasi | 0 | - | - | - |
| Install/Uninstall | 0 (command display) | - | 1 kalimat | - |

Aturan tambahan:

- **Tidak ada input yang wajib tanpa tanda.** Field wajib pakai label + `*` merah, bukan placeholder sebagai satu-satunya petunjuk.
- **Placeholder bukan label.** Contoh: `Paste your API key` (placeholder) vs label `Your API key` (tetap ada).
- **Format key**: mono 14px, `autocomplete="off"`, `spellcheck="false"`, tidak pernah di-log.
- **Validasi inline** di bawah field (bukan toast) untuk: key kosong, URL tidak valid, id duplikat. Toast hanya untuk hasil aksi.
- **Otomatis fokus** field pertama; **Tab** mengikuti urutan visual; **Cmd/Ctrl+Enter** = tombol primary.
- **Tombol Show/Hide** di dalam field API key (ikon mata, 36×36, `aria-label`).
- Semua input punya `label` (bukan placeholder-only) → aksesibilitas.

---

## 5. Copy & Bahasa (English default + Bahasa Indonesia)

### 5.0 Strategi bilingual (EN default + ID)

**Default: English.** Pengguna Indonesia bisa pindah ke Bahasa Indonesia lewat Settings.

| Aspek | Keputusan |
|---|---|
| Deteksi awal | Selalu **English** di run pertama - tidak ada auto-detect dari locale OS (menghindari kejutan & memudahkan screenshot/dokumentasi) |
| Switcher | Settings → `Language` dengan 2 pilihan: **English** (default) · **Bahasa Indonesia**. Perubahan **langsung** re-render, tanpa restart |
| Penyimpanan | `localStorage["hakonfig.prefs"].lang = "en" \| "id"` (menyatu dengan prefs yang sudah ada) |
| Fallback | Key yang tidak ada di `id.js` → jatuh ke `en.js`. Tidak pernah menampilkan key mentah |
| Atribut `<html lang>` | Ikut berubah: `lang="en"` / `lang="id"` |
| Format tanggal/angka | `Intl` mengikuti bahasa aktif (`en-US` / `id-ID`) |
| Placeholder & aria-label | **Wajib** ikut diterjemahkan (sering terlewat) |
| Nama provider/model/harness | **Tidak** diterjemahkan (nama diri) |
| Pesan error dari backend Rust | Backend mengirim **kode** + parameter, frontend yang memilih kalimat. Contoh: `{ code: "invalidApiKey", status: 401 }` → EN `Invalid API key - check the key and try again.` / ID `API key tidak valid - periksa lagi key-nya.` |

**Struktur file:**

```
web/i18n/
├── index.js      → t(key, vars), setLang(lang), getLang(), re-render hook
├── en.js         → sumber kebenaran (semua key)
└── id.js         → terjemahan; boleh tidak lengkap (fallback ke en)
```

**Contoh isi:**

```js
// web/i18n/en.js
export default {
  "app.tagline": "Set up your AI coding tools",
  "home.title": "Choose your tool",
  "home.subtitle": "Pick the one you use - Hakonfig sets up its providers.",
  "home.section.installed": "Installed",
  "home.section.notInstalled": "Not installed",
  "status.ready": "Ready",
  "status.needsSetup": "Needs setup",
  "status.providers": "{count} providers",
  "status.noKey": "no API key yet",
  "harness.installed": "{name} installed",
  "harness.uninstalled": "{name} uninstalled",
  "provider.add": "Add a provider",
  "provider.addSubtitle": "Pick one to start. You'll only need your API key.",
  "provider.apiKey": "Your API key",
  "provider.pasteKey": "Paste your API key",
  "provider.getKey": "Need a key? Get one at {url}",
  "provider.testAndFind": "Test & find models",
  "provider.testing": "Testing…",
  "provider.connected": "Connected · {ms} ms",
  "provider.invalidKey": "Invalid API key - check the key and try again.",
  "provider.modelsFound": "{count} models found",
  "provider.modelsFailed": "Couldn't load models - pick from the built-in list instead.",
  "provider.selected": "{selected} of {total} selected",
  "provider.addModels": "Add {count} models",
  "provider.selectAll": "Select all",
  "provider.clear": "Clear",
  "provider.filterModels": "Filter models…",
  "provider.noProviders": "No providers yet",
  "provider.noProvidersHint": "Groq, Gemini and Cerebras have free tiers - start there.",
  "provider.saveKey": "Key saved",
  "provider.noKey": "No key yet",
  "action.edit": "Edit",
  "action.delete": "Delete",
  "action.cancel": "Cancel",
  "action.save": "Save",
  "action.close": "Close",
  "action.reload": "Reload",
  "action.history": "History",
  "action.settings": "Settings",
  "action.install": "Install",
  "action.uninstall": "Uninstall",
  "action.apply": "Apply changes",
  "delete.title": "Delete {name}?",
  "delete.body": "{name} will be removed from {harness}'s config.",
  "delete.bulletBackup": "A backup is saved first, so you can restore it.",
  "delete.bulletOthers": "Other providers are not affected.",
  "delete.dontAsk": "Don't ask me again",
  "uninstall.title": "Uninstall {name}",
  "uninstall.body": "This removes {name} from your computer.",
  "uninstall.bulletConfig": "Your provider settings stay untouched.",
  "uninstall.bulletReinstall": "You can install it again anytime.",
  "review.title": "Review changes",
  "review.subtitle": "Nothing has been written to disk yet.",
  "review.technicalDetails": "Technical details",
  "review.secretsHidden": "API keys are hidden in this preview.",
  "progress.installing": "Installing… this can take a minute.",
  "progress.uninstalling": "Uninstalling…",
  "progress.showLog": "Show log",
  "settings.language": "Language",
  "settings.confirmDelete": "Ask before deleting a provider",
  "settings.about": "About",
  "empty.noTools": "No coding tools found",
  "empty.noToolsHint": "Run one of them once, then reload.",
  "error.generic": "Something went wrong - try again.",
};
```

```js
// web/i18n/id.js (contoh, tidak harus lengkap)
export default {
  "app.tagline": "Atur provider AI coding kamu",
  "home.title": "Pilih tool kamu",
  "home.subtitle": "Pilih yang kamu pakai - Hakonfig menyiapkan provider-nya.",
  "home.section.installed": "Terpasang",
  "home.section.notInstalled": "Belum terpasang",
  "status.ready": "Siap",
  "status.needsSetup": "Perlu disiapkan",
  "provider.add": "Tambah provider",
  "provider.apiKey": "API key kamu",
  "provider.pasteKey": "Tempel API key di sini",
  "provider.getKey": "Belum punya key? Ambil di {url}",
  "provider.testAndFind": "Tes & cari model",
  "provider.connected": "Tersambung · {ms} ms",
  "provider.invalidKey": "API key tidak valid - periksa lagi key-nya.",
  "provider.modelsFound": "{count} model ditemukan",
  "action.delete": "Hapus",
  "action.uninstall": "Uninstall",
};
```

**Aturan tone untuk kedua bahasa:** EN = casual ("Just paste your key"), ID = santai tapi tidak lebay ("Cukup tempel key-nya"). Keduanya maksimal 1 kalimat per label. Dilarang: "silakan", "mohon", "terima kasih", "maaf terjadi kesalahan".

Kolom kiri = string Indonesia yang **sudah ada sekarang** (jadi dasar terjemahan ID - boleh dihaluskan). Kolom kanan = **EN, default baru**. Nama provider/model/harness tidak diterjemahkan.

| ID (sekarang → dipakai sebagai terjemahan ID) | EN (default) |
|---|---|
| Kelola provider AI-mu | Set up your AI coding tools |
| Pilih harness | Choose your tool |
| Klik salah satu untuk melihat provider yang terpasang. | Pick the one you use - Hakonfig sets up its providers. |
| Terpasang / Belum terpasang | Installed / Not installed |
| terkonfigurasi / belum disetel | Ready / Needs setup |
| 2 provider terhubung | 2 providers · ready |
| 1 provider, tanpa API key | 1 provider · no API key |
| eksperimental | Experimental |
| Belum ada file config | Not set up yet |
| Muat ulang / Backup / + Tambah Provider / Pengaturan | Reload / History / + Add provider / Settings |
| Default aktif / Ganti default / Atur default | Default model / Change / Set default |
| File konfigurasi | Config files |
| ada / belum ada | Ready / Missing |
| Provider | Providers |
| Ubah / Hapus | Edit / Delete |
| Tambah provider | Add a provider |
| Pilih provider yang mau dipakai. | Pick one to start. You'll only need your API key. |
| OpenAI Compatible (custom) | Use a custom OpenAI-compatible API → |
| Tambah {provider} | Add {provider} |
| Cuma butuh API key. | Just paste your API key - we handle the rest. |
| Tempel API key | Your API key |
| Lihat / Sembunyikan | Show / Hide |
| Ambil key di | Need a key? Get one at |
| Tes koneksi / Test koneksi | Test connection |
| Temukan model | Find models |
| Model yang dipakai | Models |
| 5 dari 6 dipakai | 5 of 24 selected |
| Pilih minimal satu model | Select at least one model |
| Simpan / Simpan provider / Simpan perubahan | Save / Add provider / Save changes |
| Hapus provider | Delete {provider}? |
| Jangan tanya lagi | Don't ask me again |
| Pratinjau perubahan | Review changes |
| Belum ada yang ditulis ke disk. | Nothing has been written to disk yet. |
| Tulis ke disk | Apply changes |
| rahasia disamarkan | Secrets are hidden |
| Install / Uninstall / Batal / Tutup | Install / Uninstall / Cancel / Close |
| Menginstal… / Menghapus… | Installing… / Uninstalling… |
| Konfigurasi ditulis | Changes saved |
| Provider dihapus | {provider} deleted |
| Backup dibuat | Backup saved |
| Gagal … | Couldn't … |
| Tidak ada harness yang dikenali | No coding tools found |
| Belum ada provider di harness ini | No providers yet |
| Kredensial / Preferensi / Katalog model | Credentials / Preferences / Model catalog |
| dipakai | selected |

### 5.1 Voice guide (berlaku untuk EN **dan** ID)

- **Persona**: teman yang sudah pernah setup ini - santai, yakin, tidak menggurui.
- **Panjang**: maksimal 1 kalimat per label, maksimal 2 bullet per penjelasan, maksimal 8 kata per judul tombol.
- **Kata kerja dulu**: `Add provider`, `Find models`, `Apply changes`, `Delete` - bukan `Provider addition`.
- **Hindari (kedua bahasa)**: "silakan", "mohon", "terima kasih", "maaf terjadi kesalahan", "terjadi kesalahan tak terduga", jargon tanpa glossari (`JSONC`, `YAML node`, `baseUrl`).
- **Error = 2 bagian**: apa yang terjadi + apa yang harus dilakukan.
  - EN: `Invalid API key - check the key and try again.`
  - ID: `API key tidak valid - periksa lagi key-nya.`
- **Jangan pernah menyalahkan pengguna.** "Couldn't reach Groq - check your connection." bukan "You entered a wrong URL."
- **ID: hindari partikel berlebihan.** "Cukup tempel key-nya" (bukan "Silakan terlebih dahulu menempelkan API key Anda").
- **EN: hindari "please".** `Paste your key` (bukan `Please paste your key`).

### 5.2 Aturan panjang teks (bilingual)

| Elemen | Batas EN | Batas ID | Jika lewat |
|---|---|---|---|
| Judul tombol | 18 karakter | 22 karakter | Ganti kata, jangan perkecil font |
| Judul modal | 32 karakter | 36 karakter | Pakai subtitle untuk sisanya |
| Subtitle modal | 60 karakter | 72 karakter | Pecah jadi 2 baris (`text-wrap: balance`) |
| Label form | 20 karakter | 24 karakter | Helper text di bawah |
| Chip/badge | 14 karakter | 16 karakter | Pakai ikon + tooltip |
| Toast | 40 karakter | 48 karakter | Detail dipindah ke baris kedua |

> Kalau sebuah string ID melebihi batas → **perbaiki copy-nya**, bukan mengecilkan font atau memotong dengan `…`.

---

## 6. Aksesibilitas & Ergonomi

| Item | Standar |
|---|---|
| Ukuran teks min | 12px (tidak ada 10px) |
| Target klik min | 36×36px (ikon), 44px (tombol utama), 44px (input) |
| Kontras teks | ≥ 4.5:1 (body), ≥ 3:1 (besar/icon) |
| Focus ring | 3px `brand-500/25` + border color change, selalu visible |
| Focus trap modal | Wajib (F2) |
| Keyboard | Tab/SPACE/ENTER, Esc, Cmd/Ctrl+Enter = primary |
| Screen reader | `aria-label` pada icon-only, `role="switch"` + `aria-checked` pada toggle, `aria-live="polite"` pada toast & statusbox |
| Reduced motion | `@media (prefers-reduced-motion: reduce)` → matikan animasi bar/hover-lift |
| Bahasa | `<html lang>` dinamis mengikuti bahasa aktif (`en` default, `id` opsional) |

---

## 7. Catatan Teknis (yang perlu dikerjakan saat implementasi)

| # | Item | File | Kompleksitas |
|---|---|---|---|
| T1 | Bundel 5 weight Plus Jakarta Sans + 2 JetBrains Mono + `@font-face` + preload | `web/fonts/*`, `web/styles.src.css`, `web/index.html` | S (download woff2, subset Latin) |
| T2 | Rewrite token `@theme` (warna, radius, shadow, font) | `web/styles.src.css` | S |
| T3 | Ekstraksi string ke `web/i18n/en.js` + stub `id.js` + helper `t()` | `web/main.js`, `web/i18n/*` | M |
| T4 | Set ikon SVG internal (ganti emoji) | `web/icons.js` | S |
| T5 | Bundel logo provider & harness (+ fallback monogram) | `web/assets/providers/*` | S |
| T6 | **Tambah command `open_external`** untuk membuka URL docs dari Tauri | `src-tauri/src/commands.rs`, `main.rs` | S |
| T7 | Tombol Uninstall di UI + wiring `data-uninstall` | `web/main.js` | S |
| T8 | Focus trap + return focus + Cmd/Ctrl+Enter di modal | `web/main.js` (`openModal`) | M |
| T9 | Menu "⋯" (komponen reusable) | `web/main.js` | M |
| T10 | Provider card & harness tile redesign | `web/main.js` | M |
| T11 | Gabung "Tes koneksi" + "Temukan model" jadi "Test & find models" + state machine | `web/main.js` | M |
| T12 | Pretty-print nama model + filter + select all/clear | `web/main.js` | M |
| T13 | Accordion (disclosure) reusable untuk Config files & Technical details | `web/main.js` | S |
| T14 | Ringkasan bahasa manusia di preview (butuh info op dari backend) | `src-tauri/src/engine/write.rs` (Preview struct) | M |
| T15 | Hapus dropdown preset di form advanced | `web/main.js` | S |
| T16 | Update `docs/UI-CONTRACT.md` (copy + state baru) | docs | S |

**Perubahan backend minimal.** Yang benar-benar baru: `open_external` (T6) dan field ringkasan di `Preview` (T14, opsional - bisa juga disusun di frontend dari `op.kind`).

---

## 8. Tahapan Pelaksanaan (urutan kerja)

### P0 - "Bisa dibaca & tidak salah klik" (fondasi)
1. T1 font bundle (Plus Jakarta Sans + JetBrains Mono) + T2 token warna **maroon**/radius/shadow.
2. Naikkan semua ukuran teks minimal 12px; body 15px.
3. T3 ekstraksi string → `web/i18n/en.js` (lengkap) + `id.js` (terjemahan ID), helper `t()`, switcher bahasa di Settings, default `en`, `<html lang>` dinamis.
4. Container 960px terpusat (header + konten).
5. T8 focus trap + Escape + Cmd/Ctrl+Enter.
6. T7 Uninstall bisa diakses (tombol + confirm + progress).
7. Tombol "Hapus" dipindah ke menu `⋯` (T9) - hilang klik salah.
8. Hapus kalimat teknis di header harness; ganti status manusia.

### P1 - "Alur no-brain"
9. T10 kartu provider & harness tile baru (logo asli, status key, ukuran).
10. T11 alur 2 langkah: `Test & find models`, tombol Save dinamis.
11. T12 daftar model: nama + filter + select all/clear.
12. Form advanced jadi accordion + kartu model (T15).
13. Default model: dropdown terisi + logo (F4).
14. Review changes: ringkasan manusia + diff tertutup (T14).
15. Install/Uninstall modal: kalimat manusia + log tertutup + label dinamis.
16. Empty states, settings (Language + Ask before deleting), history (timeline).
17. Terjemahan ID lengkap untuk semua layar (review pass kedua setelah EN stabil).

### P2 - "Polish"
17. T4 set ikon seragam.
18. Micro-interaction: hover lift 1px, spring pada toggle, toast slide-in.
19. `prefers-reduced-motion` + skeleton loading.
20. Copy review akhir (tone EN & ID), QA bilingual (panjang teks ID ±20% lebih panjang - cek tidak pecah layout), screenshot semua layar di kedua bahasa.

---

## 9. Checklist Acceptance (harus lolos sebelum dianggap selesai)

- [ ] Tidak ada teks di bawah 12px di seluruh app (cek via grep `text-[10px]` = 0).
- [ ] Default bahasa **English**; switcher EN/ID di Settings langsung bekerja tanpa restart; `<html lang>` ikut berubah.
- [ ] Setiap key di `en.js` ada padanannya di `id.js` (test otomatis: diff key set = 0).
- [ ] Tidak ada string Inggris/Indonesia hardcode di `main.js` (grep string literal di template = 0).
- [ ] Layout tidak pecah saat Bahasa Indonesia (teks ±20% lebih panjang).
- [ ] Font Plus Jakarta Sans ter-render offline (matikan jaringan, app tetap tampil benar).
- [ ] Setiap layar punya **maksimal 1 tombol primary**.
- [ ] Alur tambah provider: pilih → tempel key → 1 klik "Test & find models" → centang → "Add N models".
- [ ] Model yang tidak dicentang **tidak** ditulis ke config (sudah ada, dijaga test).
- [ ] Tidak ada jargon tanpa penjelasan: `models.json`, `auth.json`, `openai-compatible`, `context window`, `reasoning` → tooltip/glossari.
- [ ] Uninstall bisa diakses dari kartu harness terpasang, konfirmasi muncul, progress tampil, status home ter-refresh.
- [ ] Modal: focus trap, Esc, fokus kembali ke pemicu.
- [ ] Semua aksi destruktif punya konfirmasi + explanation + backup disebut.
- [ ] Kontras teks utama ≥ 4.5:1 (dicek dengan pixel sampling/token audit); angka di §2.3 diverifikasi ulang di layar nyata.
- [ ] **Maroon tidak dipakai untuk status sukses** (harus `ok` hijau) - cek chip "Ready"/"Key saved".
- [ ] **Tombol destruktif tidak berdampingan dengan tombol primary** dalam satu baris footer.
- [ ] Preview menampilkan ringkasan bahasa manusia di atas diff.
- [ ] 100% test tetap lulus (44 Rust + 20 JS) + test baru untuk i18n completeness (semua key ada).
- [ ] Screenshot ulang 12 layar untuk perbandingan sebelum/sesudah.

---

## 10. Lampiran A - Peta File yang Akan Disentuh

```
web/
├── index.html              → lang="en" (dinamis), preload font, meta
├── styles.src.css          → @theme baru (warna, font, radius, shadow), @font-face, reduced-motion
├── fonts/                  → BARU: woff2 + LICENSES.md
├── icons.js                → BARU: set ikon SVG
├── i18n/
│   ├── en.js               → BARU: seluruh string
│   ├── index.js            → BARU: t(), setLang(), getLang()
│   └── id.js               → BARU: terjemahan lengkap (fallback ke en)
├── assets/providers/       → BARU: logo SVG + LICENSES.md
├── main.js                 → render ulang, alur baru, i18n call
└── diff.js                 → tidak berubah (kecuali gaya)

src-tauri/src/
├── commands.rs             → + open_external
└── main.rs                 → register open_external

docs/
├── UI-CONTRACT.md          → update
└── UI-REVAMP-PLAN.md       → dokumen ini
```

---

## 11. Lampiran B - Catatan Risiko

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Logo provider punya syarat lisensi/brand usage | Bundle bermasalah | Gunakan Simple Icons (CC0) + catat di `LICENSES.md`; fallback monogram selalu tersedia |
| Font woff2 menambah ukuran repo (~200–300KB × 5 weight) | Repo membesar | Subset Latin ( latin-only ), simpan hanya 400/600/700 untuk body + 700 untuk display |
| `lang="en"` tapi README/docs masih Indonesia | Inkonsisten | README bilingual: judul + ringkasan Inggris, detail tetap Indonesia |
| Kode lama (`openProviderForm` untuk edit) memakai layout lama | Edit jadi tidak konsisten | Edit provider **juga** pakai modal simple + bagian Advanced - bukan form lama |
| Perubahan i18n menyentuh hampir semua baris `main.js` | Risiko regresi | Strategi: `t()` dengan fallback ke string lama; test "semua key ada" |
| **Maroon vs merah destruktif** berdekatan hue | Pengguna salah klik tombol hapus | Destruktif tidak pernah sebangun dengan primary; selalu pakai ikon + label; hanya muncul dari menu `⋯` atau modal konfirmasi |
| **Maroon dipakai untuk status sukses** (terbawa kebiasaan dari brand hijau) | "Ready" dan "Simpan" tampak sama | Token `ok` hijau wajib; masuk checklist QA (cek chip Ready/Key saved = hijau) |
| **Teks Indonesia ~20% lebih panjang** dari Inggris | Layout pecah saat ganti bahasa | Semua container pakai `min-w-0` + `truncate`; QA screenshot di kedua bahasa; label tombol tidak pakai lebar tetap |
| **Terjemahan ID tertinggal** saat EN berubah | Muncul teks Inggris di UI Indonesia | Fallback otomatis ke EN (tidak pernah tampil key mentah) + test set-diff key EN vs ID |

---

## 12. Status Keputusan

### Sudah dikonfirmasi

| # | Keputusan | Nilai final |
|---|---|---|
| 1 | Font | **Plus Jakarta Sans** (400/500/600/700/800) + **JetBrains Mono** (400/600) |
| 2 | Warna brand | **Maroon** - `brand-600 #8C3345` (aksi), `brand-700 #7A2A3B` (hover/teks) |
| 3 | Tone | **Casual-friendly** - "Set up your AI coding tools", bukan "Configure your AI coding harness" |
| 4 | Target | **Pemula** → diff mentah tertutup, ringkasan bahasa manusia di atas |
| 5 | Bahasa | **Bilingual EN + ID, default English**; switcher di Settings |

### Turunan otomatis dari 5 keputusan di atas

| # | Turunan | Alasan |
|---|---|---|
| 6 | Hijau `ok` **wajib** ada terpisah dari brand | Maroon tidak bisa berarti "sukses" |
| 7 | Tombol destruktif pakai merah-oranye + ikon, tidak pernah sebangun dengan primary | Maroon/merah berdekatan hue |
| 8 | Avatar provider tetap beragam, maroon hanya untuk preset utama | Menghindari "lautan maroon" di 10 kartu |
| 9 | Label tombol utama di modal tambah provider berubah sesuai bahasa & jumlah: `Add 5 models` / `Tambah 5 model` | Konfirmasi aksi jelas dalam bahasa apa pun |
| 10 | README: judul + ringkasan bilingual (EN dulu, lalu ID), detail teknis tetap Indonesia | Konsisten dengan default app, tanpa memaksa menerjemahkan seluruh dokumen |

### Masih terbuka (tidak memblokir P0)

| # | Pertanyaan | Default jika tidak dijawab |
|---|---|---|
| 11 | Auto-detect bahasa dari locale OS saat run pertama? | **Tidak** - selalu English (dokumentasi & screenshot konsisten) |
| 12 | Perlu switcher bahasa juga di header (bukan hanya Settings)? | Tidak, cukup Settings sampai ada permintaan |
| 13 | Bundel logo provider asli (perlu audit lisensi per brand) atau tetap monogram dulu? | Monogram di P0, logo asli di P2 |
| 14 | Perlu dark mode? | Tidak - light + hangat adalah identitas produk |

---

*Dokumen ini adalah rencana. Tidak ada perubahan kode yang dibuat untuk mengimplementasikannya. Perubahan yang sudah ada di working tree (install/uninstall progress + alur provider preset) tetap belum di-commit, sesuai instruksi sebelumnya untuk review dulu.*

---

## 13. Catatan Pelaksanaan (apa yang benar-benar dikerjakan)

| Rencana | Status | Catatan |
|---|---|---|
| Font Plus Jakarta Sans + JetBrains Mono | ✅ | woff2 latin subset, 140 KB total, dibundel di `web/fonts/` (offline) |
| Token maroon + netral hangat | ✅ | `@theme` di `web/styles.src.css`, kontras dihitung ulang |
| Tema gelap (default terang, toggle Settings) | ✅ | `.dark` menimpa variable; markup tidak digandakan |
| Bilingual EN default + ID | ✅ | `web/i18n/{en,id}.js` (209 key, paritas dijaga test) |
| Alur tambah provider 2 langkah | ✅ | pilih → tempel key → "Test & find models" → centang → tombol dinamis |
| Nama model di-pretty-print + filter + select all/clear | ✅ | `prettyModel()` di frontend, tanpa dependensi |
| Form lanjutan (accordion-ish, kartu model) | ✅ | dropdown preset dihapus, diganti teks balik ke daftar |
| Default model: dropdown terisi | ✅ | fallback input teks bila provider tidak punya daftar model |
| Review changes: ringkasan manusia + diff tertutup | ✅ | `<details>` native |
| Uninstall dapat diakses | ✅ | menu `⋯` kartu harness & tombol di kartu katalog |
| Install/uninstall: progress + log tertutup | ✅ | event `package-progress` |
| Menu `⋯` (bukan tombol Hapus 10px) | ✅ | `openMenu()` generik, item destruktif dipisah garis |
| Focus trap + Esc + kembalikan fokus | ✅ | di `openModal()` |
| Lebar 960px terpusat, grid 4 kolom | ✅ | header & konten memakai container yang sama |
| Ukuran teks ≥ 12px | ✅ | tidak ada lagi `text-[10px]` |
| Ikon SVG (bukan emoji) | ✅ | `web/icons.js`, 31 ikon gaya Lucide |
| `open_external` | ✅ | command baru di backend, hanya http(s) |
| Logo provider asli | ⏳ | slot siap: taruh `<id>.svg` di `web/assets/providers/`; fallback monogram |
| README bilingual | ⏳ | header EN + isi ID; bisa dilanjutkan terpisah |

### Penyimpangan dari rencana (disengaja)

1. **Nama provider preset diubah jadi brand saja** (`Groq`, bukan `Groq (free tier)`)
   dan status free/local jadi field `badge` - supaya bisa diterjemahkan dan tidak
   terpotong di kartu.
2. **Toast memakai ikon + tint**, bukan border kiri berwarna (lebih rapi di tema gelap).
3. **Toggle provider tetap langsung menulis** (tidak lewat pratinjau), tapi kini
   memunculkan toast dengan aksi **Undo** - mengembalikan perilaku reversibel
   tanpa menambah satu klik konfirmasi.
4. **Deskripsi katalog harness** disimpan bilingual di `harnesses.json`
   (`description` = EN, `descriptionId` = ID) karena itu data, bukan string UI.
