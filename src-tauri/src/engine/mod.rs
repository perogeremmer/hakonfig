//! Inti Hakonfig: seluruh logika baca/ubah/tulis konfigurasi harness.
//! Modul ini tidak tahu apa-apa soal UI.
pub mod adapters;
pub mod backup;
pub mod catalog;
pub mod version;
pub mod doc;
pub mod model;
pub mod paths;
pub mod preset;
pub mod write;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub mod testutil;
