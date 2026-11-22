# progress/add.md — agen `agent/add` (layar Tambah provider)

## Kerja
- Implementasi penuh `src/views/add.rs` (sebelumnya stub): form tiga langkah
  1. **Pilih provider** — kotak cari (InputState + subscribe InputEvent::Change),
     grid kartu preset `.flex_wrap()` dari `api::list_presets()` (diisi sinkron di
     `new`), kartu: nama, id mono, chip badge (free/local), jumlah model, chip
     "tanpa key"; kartu **Custom** di akhir. Klik kartu → `picked`, `step = 2`.
  2. **Key** — info preset (base_url + api mono, truncate, notes, docs chip); mode
     Custom: 4 InputState terpisah (id/name/base_url/api). Dua pil mode key
     "Simpan key" / "Pakai environment" (env menampilkan `<ID>_API_KEY` + chip
     terisi/belum di env proses). Input key (tanpa mask — gpui-kit tak punya API
     masked; ringkasan menampilkan `••••` + 4 char terakhir). Tombol
     "Test & find models" → `cx.spawn` + `cx.background_spawn` memanggil
     `crate::net::test_connection(...)`; hasil: ok/gagal, HTTP status, latensi,
     contoh 5 model.
  3. **Model** — baris model dari `preset.models`, fallback `models_sample` hasil
     tes (untuk preset kosong/Custom): kotak centang klik (div ✓ brand), id mono,
     context window (`128k ctx`), chip reasoning. "Pilih semua"/"Kosongkan" + cari
     model (InputState). "Pratinjau diff" membangun `ProviderDraft` +
     `EditOp::Upsert` → `api::preview_edit(harness, op)` di background; ringkasan:
     N file baru / M file diubah + daftar warning. `apply_edit` TIDAK dipanggil.
- Stepper 1-2-3 di atas (aktif = `brand_soft`/`brand_text`, klik = kembali ke
  langkah sebelumnya), nav "Kembali"/"Lanjut" (Lanjut disabled bila meta Custom
  belum lengkap), chip harness di header.
- `set_harness` mereset `test` + `preview` saat harness berganti.

## File disentuh (hanya dua, sesuai batasan)
- `spike/gpui-kit/src/views/add.rs` (tulis penuh)
- `spike/gpui-kit/progress/add.md` (file ini)

## Verifikasi
- `cargo build -j 4`: (status diisi setelah build)
- Screenshot: (diisi setelah run)

## Kendala
- Build pertama di worktree ini dingin (target/ kosong) + mesin dipakai 2 agen
  lain → build > 15 menit, harus dijalankan di background dengan log.

## Sisa / catatan
- Nilai `api` yang diterima `preview_edit` tidak diverifikasi ulang dari core
  (sesuai protokol); preset menyediakan nilai yang valid, field Custom divalidasi
  hanya "tidak kosong" di UI — draft buruk akan tampil sebagai Err di panel
  pratinjau.
- Mode env membaca nilai variabel dari env proses spike untuk tes koneksi.
- READ-ONLY terjaga: hanya `list_presets`, `preview_edit`, `net::test_connection`.
