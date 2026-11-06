//! Preset provider siap pakai (data statis, mudah diperbarui tanpa sentuh kode).
use super::model::ProviderPreset;

const PRESETS: &str = include_str!("../../presets/providers.json");

pub fn all() -> Vec<ProviderPreset> {
    match serde_json::from_str::<Vec<ProviderPreset>>(PRESETS) {
        Ok(list) => list,
        Err(err) => {
            eprintln!("[rekonfig] presets/providers.json tidak valid: {}", err);
            Vec::new()
        }
    }
}
