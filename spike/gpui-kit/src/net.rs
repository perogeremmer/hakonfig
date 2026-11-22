//! Pembungkus kecil untuk panggilan core yang butuh runtime async.
//!
//! `hakonfig_core::api::test_connection` adalah `async fn` yang memakai reqwest,
//! dan reqwest membutuhkan reaktor tokio. GPUI menjalankan `background_spawn`
//! di executor smol, jadi panggilan itu dibungkus runtime current-thread di
//! thread background — bukan di thread UI.

use std::future::Future;

use hakonfig_core::api;
use hakonfig_core::engine::model::TestResult;

/// Jalankan future yang butuh reaktor tokio. Aman dipanggil dari thread
/// background GPUI, jangan dari render.
#[allow(dead_code)] // dipakai view Tambah provider
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("gagal membuat runtime tokio")
        .block_on(future)
}

/// Tes koneksi provider (sama persis dengan yang dipanggil GUI web).
///
/// `key` boleh kosong: core akan memakai key tersimpan atau environment.
#[allow(dead_code)] // seam untuk view Tambah provider
pub fn test_connection(
    base_url: String,
    api_type: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<TestResult, String> {
    block_on(api::test_connection(
        base_url,
        api_type,
        key,
        harness_id,
        provider_id,
    ))
}
