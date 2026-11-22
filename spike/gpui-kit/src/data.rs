//! Pembacaan data harness lewat `hakonfig_core::api`.
//!
//! Tidak ada logika konfigurasi di sini: semua datang dari crate core yang sama
//! dengan `hakonfig-cli` dan GUI Tauri.

use std::time::Instant;


use hakonfig_core::api;
use hakonfig_core::engine::model::{HarnessSnapshot, Notice};

pub struct HarnessEntry {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub experimental: bool,
    pub provider_count: usize,
    pub connected_count: usize,
    pub version: Option<String>,
    pub config_path: String,
    pub snapshot: Option<HarnessSnapshot>,
    pub notices: Vec<Notice>,
    pub error: Option<String>,
    /// (baca config, deteksi versi tool) dalam milidetik.
    pub timings: (u128, u128),
}

impl HarnessEntry {
    /// Baca config saja. `harness_version()` sengaja TIDAK dipanggil di sini:
    /// dia spawn proses `<bin> --version` (±1.000 ms di mesin ini), dan itu
    /// membuat frame pertama menunggu pekerjaan yang tidak dibutuhkan layar.
    pub fn load(id: String) -> Self {
        let info = api::list_harnesses()
            .into_iter()
            .find(|h| h.id == id)
            .expect("harness dari list_harnesses selalu bisa dibaca ulang");

        // `read_harness` mengembalikan Result<String>, sama seperti yang
        // diterima frontend Tauri lewat `#[tauri::command]`.
        let t_read = Instant::now();
        let (snapshot, error) = match api::read_harness(id.clone()) {
            Ok(snap) => (Some(snap), None),
            Err(e) => (None, Some(e)),
        };
        let read_ms = t_read.elapsed().as_millis();

        // Notice = hal di config yang membuat provider/model tidak terlihat
        // harness. Murah (membaca berkas yang sama) dan penting untuk ditampilkan
        // sebelum pengguna bingung "sudah ditulis tapi tidak muncul".
        let notices = api::list_notices(Some(id.clone())).unwrap_or_default();

        Self {
            notices,
            timings: (read_ms, 0),
            version: None,
            config_path: info
                .files
                .first()
                .map(|f| f.path.clone())
                .unwrap_or_default(),
            id,
            name: info.name,
            installed: info.installed,
            experimental: info.experimental,
            provider_count: info.provider_count,
            connected_count: info.connected_count,
            snapshot,
            error,
        }
    }

    pub fn model_count(&self) -> usize {
        self.snapshot
            .as_ref()
            .map(|s| s.providers.iter().map(|p| p.models.len()).sum())
            .unwrap_or(0)
    }
}

/// Probe versi tool di background, terpisah dari pembacaan config.
///
/// Diukur: 4 harness = ±1.000 ms, dan 604 ms di antaranya `omp` (pembungkus
/// Node). Karena itu hasilnya datang belakangan, bukan menahan frame pertama.
pub fn probe_versions(ids: Vec<String>) -> Vec<(String, Option<String>, u128)> {
    ids.into_iter()
        .map(|id| {
            let t = Instant::now();
            let version = api::harness_version(&id);
            (id, version, t.elapsed().as_millis())
        })
        .collect()
}

pub fn load_all() -> (Vec<HarnessEntry>, u128) {
    let started = Instant::now();
    let entries: Vec<HarnessEntry> = api::list_harnesses()
        .into_iter()
        .map(|h| {
            let t = Instant::now();
            let entry = HarnessEntry::load(h.id);
            eprintln!(
                "[spike]   {}: {} ms baca config — {} provider, {} model",
                entry.id,
                t.elapsed().as_millis(),
                entry.provider_count,
                entry.model_count()
            );
            entry
        })
        .collect();
    (entries, started.elapsed().as_millis())
}

/// Dua huruf pertama dari nama/id — pengganti logo provider di spike.
pub fn monogram(name: &str, id: &str) -> String {
    let source = name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();
    let source = if source.is_empty() {
        id.to_string()
    } else {
        source
    };
    source.chars().take(2).collect::<String>().to_uppercase()
}

/// Ringkasan hasil load terakhir, dibagikan ke view lain lewat setter
/// (`SettingsView::set_metrics`) alih-alih global GPUI.
#[derive(Clone, Copy, Default)]
#[allow(dead_code)] // dibaca view Pengaturan
pub struct Metrics {
    pub load_ms: u128,
    pub data_ready_ms: Option<u128>,
    pub harnesses: usize,
    pub providers: usize,
    pub models: usize,
}

impl Metrics {
    pub fn of(entries: &[HarnessEntry], load_ms: u128, ready: Option<u128>) -> Self {
        Self {
            load_ms,
            data_ready_ms: ready,
            harnesses: entries.len(),
            providers: entries.iter().map(|e| e.provider_count).sum(),
            models: entries.iter().map(HarnessEntry::model_count).sum(),
        }
    }
}

