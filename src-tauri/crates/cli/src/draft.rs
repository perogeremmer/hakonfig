//! Keputusan murni untuk `add`: tanpa IO, tanpa prompt, tanpa TTY.
//!
//! Semua yang bisa salah (pemetaan preset, parsing `--models`, validasi id)
//! dikumpulkan di sini supaya bisa diuji tanpa terminal.
use hakonfig_core::engine::model::{KeyMode, ModelDraft, ProviderDraft, ProviderPreset};

use crate::i18n::{self, Msg};

/// id untuk provider yang tidak ada di katalog.
pub const CUSTOM_ID: &str = "custom";

/// Tipe API bawaan untuk endpoint OpenAI-compatible.
pub const DEFAULT_API: &str = "openai-completions";

/// Semua model yang dicatat preset, jadi `ModelDraft`.
pub fn models_from_preset(preset: &ProviderPreset) -> Vec<ModelDraft> {
    preset
        .models
        .iter()
        .map(|model| ModelDraft {
            id: model.id.clone(),
            name: model.name.clone(),
            context_window: model.context_window,
            max_tokens: model.max_tokens,
            reasoning: model.reasoning,
        })
        .collect()
}

/// Parse `--models a,b,c`.
///
/// Aturan: id dipakai apa adanya (endpoint custom boleh punya id bebas),
/// urutan dipertahankan, duplikat dibuang, dan metadata (nama/context window)
/// diambil dari preset bila id-nya dikenal.
pub fn models_from_list(raw: &str, preset: Option<&ProviderPreset>) -> Vec<ModelDraft> {
    let mut models: Vec<ModelDraft> = Vec::new();
    for candidate in raw.split(',') {
        let id = candidate.trim();
        if id.is_empty() || models.iter().any(|model| model.id == id) {
            continue;
        }
        let known =
            preset.and_then(|preset| preset.models.iter().find(|model| model.id.trim() == id));
        models.push(ModelDraft {
            id: id.to_string(),
            name: known.and_then(|model| model.name.clone()),
            context_window: known.and_then(|model| model.context_window),
            max_tokens: known.and_then(|model| model.max_tokens),
            reasoning: known.map(|model| model.reasoning).unwrap_or(false),
        });
    }
    models
}

/// Draft untuk provider yang ada di katalog: endpoint dan tipe API mengikuti
/// preset, bukan input pengguna, supaya tidak ada salah ketik base URL.
pub fn draft_from_preset(
    preset: &ProviderPreset,
    key: Option<&str>,
    mode: KeyMode,
    models: Vec<ModelDraft>,
) -> ProviderDraft {
    ProviderDraft {
        id: preset.id.clone(),
        name: preset.name.clone(),
        base_url: preset.base_url.clone(),
        api: preset.api.clone(),
        key: key.map(str::to_string),
        key_mode: Some(mode),
        models,
    }
}

/// Draft untuk provider custom. `name` dan `base_url` wajib.
pub fn draft_custom(
    id: &str,
    name: Option<&str>,
    base_url: Option<&str>,
    api: Option<&str>,
    key: Option<&str>,
    mode: KeyMode,
    models: Vec<ModelDraft>,
) -> Result<ProviderDraft, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err(i18n::t(Msg::EmptyId).to_string());
    }
    if id == CUSTOM_ID {
        return Err(i18n::fill(
            i18n::t(Msg::NeedCustomId),
            &[("custom", CUSTOM_ID.to_string())],
        ));
    }
    let name = name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| i18n::t(Msg::CustomNeedsName).to_string())?;
    let base_url = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| i18n::t(Msg::CustomNeedsBaseUrl).to_string())?;
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(i18n::fill(
            i18n::t(Msg::BaseUrlMustBeHttp),
            &[("url", base_url.to_string())],
        ));
    }

    Ok(ProviderDraft {
        id: id.to_string(),
        name: name.to_string(),
        base_url: base_url.to_string(),
        api: api
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_API)
            .to_string(),
        key: key.map(str::to_string),
        key_mode: Some(mode),
        models,
    })
}

/// Cari preset menurut id (tidak peka huruf besar/kecil).
pub fn find_preset<'a>(
    presets: &'a [ProviderPreset],
    id: &str,
) -> Result<&'a ProviderPreset, String> {
    let needle = id.trim().to_lowercase();
    presets
        .iter()
        .find(|preset| preset.id.to_lowercase() == needle)
        .ok_or_else(|| match nearest(presets, &needle) {
            Some(candidate) => i18n::fill(
                i18n::t(Msg::UnknownProviderSuggest),
                &[("id", id.to_string()), ("candidate", candidate)],
            ),
            None => i18n::fill(i18n::t(Msg::UnknownProvider), &[("id", id.to_string())]),
        })
}

/// Kandidat terdekat untuk pesan error, memakai jarak edit (Levenshtein).
fn nearest(presets: &[ProviderPreset], needle: &str) -> Option<String> {
    if needle.chars().count() < 3 {
        return None;
    }

    let mut best: Option<(usize, &str)> = None;
    for preset in presets {
        let id = preset.id.to_lowercase();
        let distance = if id.contains(needle) || needle.contains(id.as_str()) {
            0
        } else {
            levenshtein(needle, &id)
        };
        // Toleransi tumbuh bersama panjang id: typo 1 huruf pada id pendek
        // sudah terlalu jauh, sementara pada id panjang masih wajar.
        let limit = (id.chars().count() / 3).max(2);
        if distance > limit {
            continue;
        }
        if best.map(|(current, _)| distance < current).unwrap_or(true) {
            best = Some((distance, preset.id.as_str()));
        }
    }
    best.map(|(_, id)| id.to_string())
}

/// Jarak edit klasik (insert/hapus/ganti). Cukup untuk membandingkan id pendek.
fn levenshtein(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();

    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];

    for (i, a) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, b) in right.iter().enumerate() {
            let cost = usize::from(a != b);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use hakonfig_core::engine::model::ModelSummary;

    fn preset(id: &str, models: &[(&str, Option<u64>)]) -> ProviderPreset {
        ProviderPreset {
            id: id.to_string(),
            name: format!("Nama {id}"),
            base_url: format!("https://{id}.test/v1"),
            api: DEFAULT_API.to_string(),
            env_var: Some(format!("{}_API_KEY", id.to_uppercase())),
            docs_url: None,
            key_required: true,
            badge: Some("free".to_string()),
            notes: String::new(),
            models: models
                .iter()
                .map(|(id, window)| ModelSummary {
                    id: id.to_string(),
                    name: None,
                    context_window: *window,
                    max_tokens: None,
                    reasoning: false,
                })
                .collect(),
        }
    }

    #[test]
    fn models_from_preset_copies_metadata() {
        let groq = preset(
            "groq",
            &[("llama-3.3-70b", Some(128_000)), ("llama-3.1-8b", None)],
        );
        let models = models_from_preset(&groq);

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "llama-3.3-70b");
        assert_eq!(models[0].context_window, Some(128_000));
        assert_eq!(models[1].id, "llama-3.1-8b");
        assert_eq!(models[1].context_window, None);
    }

    #[test]
    fn models_from_preset_is_empty_for_preset_without_models() {
        let empty = preset("npm", &[]);
        assert!(models_from_preset(&empty).is_empty());
    }

    #[test]
    fn models_from_list_trims_and_drops_duplicates() {
        let groq = preset("groq", &[("a", Some(1000))]);
        let models = models_from_list("  a , b ,, a ,  ", Some(&groq));

        assert_eq!(
            models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "urutan dipertahankan dan duplikat dibuang"
        );
    }

    #[test]
    fn models_from_list_takes_metadata_from_preset_when_known() {
        let groq = preset("groq", &[("a", Some(1000))]);
        let models = models_from_list("a,tidak-dikenal", Some(&groq));

        assert_eq!(
            models[0].context_window,
            Some(1000),
            "id yang dikenal ikut metadata preset"
        );
        assert_eq!(
            models[1].context_window, None,
            "id asing tetap diterima apa adanya"
        );
    }

    #[test]
    fn models_from_list_works_without_a_preset() {
        let models = models_from_list("x,y", None);
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn draft_from_preset_follows_the_preset_endpoint() {
        let groq = preset("groq", &[("a", None)]);
        let draft = draft_from_preset(
            &groq,
            Some("sk-1"),
            KeyMode::Inline,
            models_from_preset(&groq),
        );

        assert_eq!(draft.id, "groq");
        assert_eq!(draft.base_url, "https://groq.test/v1");
        assert_eq!(draft.api, DEFAULT_API);
        assert_eq!(draft.key.as_deref(), Some("sk-1"));
        assert_eq!(draft.key_mode(), KeyMode::Inline);
        assert_eq!(draft.models.len(), 1);
    }

    #[test]
    fn draft_custom_requires_name_and_base_url() {
        let error = draft_custom(
            "toko",
            None,
            Some("https://a.test/v1"),
            None,
            None,
            KeyMode::Inline,
            vec![],
        )
        .unwrap_err();
        assert!(
            error.contains("--name"),
            "pesan harus menyebut flag yang kurang: {error}"
        );

        let error = draft_custom(
            "toko",
            Some("Toko"),
            None,
            None,
            None,
            KeyMode::Inline,
            vec![],
        )
        .unwrap_err();
        assert!(error.contains("--base-url"));
    }

    #[test]
    fn draft_custom_rejects_non_http_base_url() {
        let error = draft_custom(
            "toko",
            Some("Toko"),
            Some("ftp://a.test"),
            None,
            None,
            KeyMode::Inline,
            vec![],
        )
        .unwrap_err();
        assert!(error.contains("http"), "{error}");
    }

    #[test]
    fn draft_custom_uses_the_default_api() {
        let draft = draft_custom(
            "toko",
            Some("Toko"),
            Some("https://a.test/v1"),
            None,
            Some("k"),
            KeyMode::Env,
            vec![],
        )
        .unwrap();

        assert_eq!(draft.api, DEFAULT_API);
        assert_eq!(draft.key_mode(), KeyMode::Env);
        assert_eq!(draft.name, "Toko");
    }

    #[test]
    fn find_preset_is_case_insensitive() {
        let presets = vec![preset("groq", &[])];
        assert_eq!(find_preset(&presets, "GROQ").unwrap().id, "groq");
        assert_eq!(find_preset(&presets, "  Groq ").unwrap().id, "groq");
    }

    #[test]
    fn find_preset_suggests_a_candidate_on_typo() {
        let presets = vec![preset("openrouter", &[])];
        let error = find_preset(&presets, "openroutr").unwrap_err();
        assert!(
            error.contains("openrouter"),
            "harus menyarankan kandidat: {error}"
        );
    }

    #[test]
    fn find_preset_mentions_the_catalog_command_when_nothing_matches() {
        let presets = vec![preset("groq", &[])];
        let error = find_preset(&presets, "zzz").unwrap_err();
        assert!(error.contains("hakonfig-cli providers"), "{error}");
    }

    #[test]
    fn levenshtein_measures_edit_distance() {
        assert_eq!(levenshtein("openrouter", "openrouter"), 0);
        assert_eq!(
            levenshtein("openroutr", "openrouter"),
            1,
            "satu huruf hilang"
        );
        assert_eq!(
            levenshtein("openrouter", "openruter"),
            1,
            "satu huruf tertukar"
        );
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn find_preset_suggests_even_with_a_typo_in_the_middle() {
        let presets = vec![preset("openrouter", &[]), preset("groq", &[])];
        let error = find_preset(&presets, "openruter").unwrap_err();
        assert!(error.contains("openrouter"), "{error}");
        assert!(
            !error.contains("groq"),
            "jangan menyarankan yang jauh: {error}"
        );
    }
}
