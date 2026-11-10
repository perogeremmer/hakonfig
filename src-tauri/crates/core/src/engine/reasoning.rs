//! Deteksi "model ini mendukung thinking" tanpa menebak satu per satu.
//!
//! Endpoint `/models` hampir tidak pernah memberi tahu apakah sebuah model
//! mendukung reasoning, jadi hasil sync selalu masuk sebagai `reasoning: false`
//! dan pengguna harus mencentangnya manual. Padahal jawabannya biasanya sudah
//! ada di mesin ini:
//!
//! 1. **Katalog pi** (`<agent dir>/models-store.json`) menyimpan `reasoning`
//!    beserta `thinkingLevelMap` per model, hasil refresh dari pi.dev.
//! 2. **Preset** bawaan Hakonfig membawa flag yang sama untuk model populer.
//! 3. Sisanya **pola id**: nama model reasoning sangat khas (`gpt-5*`, `o3*`,
//!    `claude-*-5*`, `deepseek-v4*`, `glm-*`, `kimi-k*`, ...).
//!
//! Urutannya penting: data katalog menang atas preset, preset menang atas pola,
//! dan sumbernya ikut dilaporkan supaya UI bisa membedakan "terdeteksi" dari
//! "perkiraan".

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::engine::paths;

/// Dari mana keyakinan sebuah model datang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HintSource {
    /// Katalog milik harness (pi) di mesin ini — paling akurat.
    Catalog,
    /// Preset bawaan Hakonfig.
    Preset,
    /// Pola id model: masuk akal, tapi tetap perkiraan.
    Pattern,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningHint {
    pub id: String,
    pub reasoning: bool,
    /// `thinkingLevelMap` asli dari katalog, kalau ada. Map buatan pengguna
    /// tidak pernah ditimpa; ini hanya dipakai untuk model baru.
    pub thinking_level_map: Option<Value>,
    pub source: HintSource,
}

/// Petakan id model ke dugaan reasoning-nya.
///
/// `provider_id` dipakai untuk memilih bagian katalog yang tepat; kalau model
/// tidak ada di sana, katalog provider lain dicoba (banyak model dipakai ulang
/// oleh beberapa gateway) asalkan id-nya hanya ditemukan di satu tempat.
pub fn hints(provider_id: &str, model_ids: &[String]) -> Vec<ReasoningHint> {
    let catalog = catalog_models();
    let presets = preset_models();

    model_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(|id| hint_for(id, provider_id, &catalog, &presets))
        .collect()
}

fn hint_for(
    id: &str,
    provider_id: &str,
    catalog: &BTreeMap<String, (bool, Option<Value>)>,
    presets: &BTreeMap<String, bool>,
) -> ReasoningHint {
    let catalog_key = format!("{provider_id}\u{1}{id}");
    if let Some((reasoning, map)) = catalog.get(&catalog_key) {
        return ReasoningHint {
            id: id.to_string(),
            reasoning: *reasoning,
            thinking_level_map: map.clone(),
            source: HintSource::Catalog,
        };
    }
    if let Some(reasoning) = presets.get(id) {
        return ReasoningHint {
            id: id.to_string(),
            reasoning: *reasoning,
            thinking_level_map: None,
            source: HintSource::Preset,
        };
    }
    ReasoningHint {
        id: id.to_string(),
        reasoning: looks_like_reasoning(id),
        thinking_level_map: None,
        source: HintSource::Pattern,
    }
}

/// `provider\u{1}model` -> (reasoning, thinkingLevelMap).
fn catalog_models() -> BTreeMap<String, (bool, Option<Value>)> {
    let path = paths::pi_dir().join("agent/models-store.json");
    let Some(text) = std::fs::read_to_string(path).ok() else {
        return BTreeMap::new();
    };
    let Some(root) = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| value.as_object().cloned())
    else {
        return BTreeMap::new();
    };

    let mut out = BTreeMap::new();
    for (provider, entry) in root {
        let Some(models) = entry.get("models").and_then(Value::as_array) else {
            continue;
        };
        for model in models {
            let Some(id) = model.get("id").and_then(Value::as_str) else {
                continue;
            };
            let reasoning = model
                .get("reasoning")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let map = model
                .get("thinkingLevelMap")
                .filter(|value| value.is_object())
                .cloned();
            out.insert(format!("{provider}\u{1}{}", id.trim()), (reasoning, map));
        }
    }
    out
}

/// id -> reasoning dari preset bawaan.
fn preset_models() -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    for preset in crate::engine::preset::all() {
        for model in preset.models {
            out.insert(model.id, model.reasoning);
        }
    }
    out
}

/// Pola id yang lazim menandakan model reasoning.
///
/// Sengaja konservatif: hanya keluarga yang memang selalu punya mode thinking.
/// Model yang tidak cocok tetap `false` dan bisa dicentang manual.
pub fn looks_like_reasoning(id: &str) -> bool {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return false;
    }
    // Buang prefiks vendor/namespace ("moonshotai/Kimi-K3" -> "kimi-k3").
    let short = id.rsplit('/').next().unwrap_or(&id);

    const PREFIXES: &[&str] = &[
        "o1",
        "o3",
        "o4",
        "o5",
        "gpt-5",
        "gpt-6",
        "gpt-oss",
        "claude-opus",
        "claude-sonnet",
        "claude-fable",
        "claude-haiku-5",
        "deepseek-v4",
        "deepseek-r",
        "glm-5",
        "glm-4",
        "kimi-k",
        "grok-4",
        "grok-3",
        "qwen3",
        "qwq",
        "gemini-3",
        "gemini-4",
        "minimax-m3",
        "mimo-v2",
        "hy3",
        "hy4",
        "muse-spark",
        "longcat-2",
        "step-5",
        "step-3",
        "nemotron",
        "inkling",
        "laguna",
        "seed-oss",
        "magistral",
        "ring-2",
    ];
    PREFIXES.iter().any(|prefix| short.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_reasoning_families_are_recognised() {
        for id in [
            "gpt-5.6-sol",
            "o3-mini",
            "claude-sonnet-5",
            "deepseek-v4-pro",
            "glm-5.3-flash",
            "kimi-k3",
            "grok-4.7",
            "qwen3.8-max",
            "gemini-4-argon",
        ] {
            assert!(looks_like_reasoning(id), "{id} seharusnya reasoning");
        }
    }

    #[test]
    fn namespaced_ids_are_matched_on_the_last_segment() {
        assert!(looks_like_reasoning("moonshotai/Kimi-K3"));
        assert!(looks_like_reasoning("zai-org/GLM-5.3"));
        assert!(!looks_like_reasoning("meta/llama-3.1-8b-instant"));
    }

    #[test]
    fn plain_models_are_not_claimed_to_be_reasoning() {
        for id in [
            "llama-3.1-8b-instant",
            "gpt-4o-mini",
            "text-embedding-3-large",
            "whisper-1",
            "mimo-v1",
        ] {
            assert!(!looks_like_reasoning(id), "{id} bukan reasoning");
        }
    }

    #[test]
    fn hints_prefer_catalog_then_preset_then_pattern() {
        let catalog: BTreeMap<String, (bool, Option<Value>)> = [(
            "myproxy\u{1}weird-name".to_string(),
            (true, Some(serde_json::json!({ "max": "max" }))),
        )]
        .into_iter()
        .collect();
        let presets: BTreeMap<String, bool> =
            [("preset-model".to_string(), false)].into_iter().collect();

        let from_catalog = hint_for("weird-name", "myproxy", &catalog, &presets);
        assert_eq!(from_catalog.source, HintSource::Catalog);
        assert!(from_catalog.reasoning);
        assert_eq!(from_catalog.thinking_level_map.unwrap()["max"], "max");

        // Preset menang atas pola: model di preset yang bukan reasoning tidak
        // boleh tiba-tiba jadi reasoning hanya karena namanya mirip.
        let from_preset = hint_for("preset-model", "myproxy", &catalog, &presets);
        assert_eq!(from_preset.source, HintSource::Preset);
        assert!(!from_preset.reasoning);

        let from_pattern = hint_for("gpt-5.4-mini", "myproxy", &catalog, &presets);
        assert_eq!(from_pattern.source, HintSource::Pattern);
        assert!(from_pattern.reasoning);
    }

    #[test]
    fn empty_ids_are_dropped() {
        let hints = hints("myproxy", &["".to_string(), "  ".to_string()]);
        assert!(hints.is_empty());
    }
}
