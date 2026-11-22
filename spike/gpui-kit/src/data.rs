//! Pembacaan data harness lewat `hakonfig_core::api`.
//!
//! Tidak ada logika konfigurasi di sini: semua datang dari crate core yang sama
//! dengan `hakonfig-cli` dan GUI Tauri.

use std::time::Instant;

use hakonfig_core::api;
use hakonfig_core::engine::model::HarnessSnapshot;

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
    pub error: Option<String>,
    /// (baca config, deteksi versi tool) dalam milidetik.
    pub timings: (u128, u128),
}

impl HarnessEntry {
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

        // Versi tool datang dari `<bin> --version`, jadi butuh spawn proses.
        let t_ver = Instant::now();
        let version = api::harness_version(&id);
        let ver_ms = t_ver.elapsed().as_millis();

        Self {
            timings: (read_ms, ver_ms),
            version,
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

pub fn load_all() -> (Vec<HarnessEntry>, u128) {
    let started = Instant::now();
    let entries: Vec<HarnessEntry> = api::list_harnesses()
        .into_iter()
        .map(|h| {
            let t = Instant::now();
            let entry = HarnessEntry::load(h.id);
            eprintln!(
                "[spike]   {}: {} ms total (config {} ms + versi {} ms) — {} provider, {} model",
                entry.id,
                t.elapsed().as_millis(),
                entry.timings.0,
                entry.timings.1,
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
