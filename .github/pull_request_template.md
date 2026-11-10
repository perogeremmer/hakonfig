<!--
Terima kasih. Isi semua bagian; hapus bagian yang tidak relevan,
tapi jangan hapus judulnya supaya reviewer mudah membandingkan PR.
-->

## Ringkasan

<!-- Apa yang berubah dan kenapa. Satu paragraf cukup. -->

## Masalah yang diselesaikan

<!--
Gejala yang dilihat pengguna, atau bug yang diperbaiki.
Sertakan issue/diskusi bila ada: "Closes #12".
Kalau ini fitur baru, jelaskan siapa yang butuh dan kapan.
-->

## Perubahan

<!--
Daftar perubahan per area. Sebutkan file atau modul yang penting,
terutama kalau ada keputusan desain yang tidak terlihat dari diff.
-->

-

## Bukti

<!--
Apa yang membuat kita yakin ini bekerja. Pilih yang relevan:
- test baru/berubah dan hasilnya
- langkah reproduksi manual + hasilnya
- screenshot (UI) atau output terminal (CLI)
- diuji di platform apa saja
-->

- [ ] `bun run test` (198 Rust + 51 JS)
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets`
- [ ] `bun run version:check`

Diuji di:

<!-- Linux / macOS / Windows, versi harness apa, resolusi/skala layar bila relevan untuk UI. -->

## Dampak

- [ ] Perubahan pada file config milik pengguna (harness) — jelaskan:
- [ ] Perubahan pada CLI, JSON, atau kontrak UI (`docs/UI-CONTRACT.md`)
- [ ] Perubahan pada CHANGELOG dan versi
- [ ] Tidak ada breaking change

## Catatan untuk reviewer

<!--
Hal yang masih terbuka, trade-off, atau bagian yang sengaja tidak dikerjakan.
Kalau sesuatu sengaja ditunda, tulis alasannya di sini supaya tidak hilang.
-->
