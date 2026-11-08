//! Katalog harness: apa saja yang bisa dipasang, dan bagaimana caranya.
use serde::{Deserialize, Serialize};

const CATALOG: &str = include_str!("../../presets/harnesses.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessCatalogEntry {
    pub id: String,
    pub name: String,
    /// "adapter" = sudah didukung Hakonfig, "planned" = baru bisa dipasang.
    pub kind: String,
    pub description: String,
    /// Deskripsi versi Indonesia (opsional; frontend pakai ini bila bahasa = id).
    #[serde(default)]
    pub description_id: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub install_unix: Option<String>,
    #[serde(default)]
    pub install_windows: Option<String>,
    /// Perintah uninstall (null = tidak tersedia otomatis).
    #[serde(default)]
    pub uninstall_unix: Option<String>,
    #[serde(default)]
    pub uninstall_windows: Option<String>,
    /// Untuk `kind: "web"`: alamat yang dibuka tombol Open.
    #[serde(default)]
    pub launch_url: Option<String>,
    /// Nama biner untuk deteksi versi (`<bin> --version`).
    #[serde(default)]
    pub binary: Option<String>,
    /// false = perintah install belum diverifikasi, tampilkan peringatan.
    #[serde(default)]
    pub verified: bool,
}

impl HarnessCatalogEntry {
    fn pick(unix: &Option<String>, windows: &Option<String>) -> Option<String> {
        let raw = if cfg!(windows) {
            windows.clone().or_else(|| unix.clone())
        } else {
            unix.clone()
        }?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn uninstall_command(&self) -> Option<String> {
        Self::pick(&self.uninstall_unix, &self.uninstall_windows)
    }
}

impl HarnessCatalogEntry {
    pub fn install_command(&self) -> Option<String> {
        Self::pick(&self.install_unix, &self.install_windows)
    }
}

pub fn all() -> Vec<HarnessCatalogEntry> {
    serde_json::from_str(CATALOG).unwrap_or_else(|error| {
        eprintln!("[hakonfig] presets/harnesses.json is not valid: {}", error);
        Vec::new()
    })
}

pub fn get(id: &str) -> Option<HarnessCatalogEntry> {
    all().into_iter().find(|entry| entry.id == id)
}
