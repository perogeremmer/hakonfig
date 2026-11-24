//! Registry view. Satu modul per layar supaya beberapa orang (atau agent) bisa
//! bekerja paralel tanpa menyentuh file yang sama.
//!
//! Kontrak tiap view:
//! - punya `struct XView` sendiri, dibuat oleh `App` lewat `XView::new(cx)`;
//! - punya `fn set_harness(&mut self, harness_id: String, cx)` yang dipanggil
//!   `App` setiap kali harness terpilih berubah (boleh no-op);
//! - implement `Render`, jadi bisa dipasang sebagai `Entity<XView>`.
//!
//! `providers` beda: ia bukan entity, melainkan fungsi render yang membaca
//! state `App` (daftar provider adalah inti layar ini).

pub mod add;
pub mod history;
pub mod providers;
pub mod settings;
