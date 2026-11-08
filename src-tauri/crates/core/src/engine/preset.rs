//! Preset provider siap pakai (data statis, mudah diperbarui tanpa sentuh kode).
use super::model::ProviderPreset;

const PRESETS: &str = include_str!("../../presets/providers.json");

pub fn all() -> Vec<ProviderPreset> {
    match serde_json::from_str::<Vec<ProviderPreset>>(PRESETS) {
        Ok(list) => list,
        Err(err) => {
            eprintln!("[hakonfig] presets/providers.json is not valid: {}", err);
            Vec::new()
        }
    }
}
