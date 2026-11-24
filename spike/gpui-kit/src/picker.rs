//! Filter baris picker model default.

use hakonfig_core::engine::model::{ModelSummary, ProviderSummary};

/// Satu baris picker model default: provider nonaktif tidak pernah muncul,
/// karena memilih default darinya tidak akan dipakai harness.
pub fn default_model_candidate(
    provider: &ProviderSummary,
    model: &ModelSummary,
    filter: Option<&str>,
    query: &str,
) -> bool {
    if !provider.enabled {
        return false;
    }
    if let Some(filter) = filter {
        if provider.id != filter {
            return false;
        }
    }
    if query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {}",
        provider.id,
        provider.name,
        model.id,
        model.name.clone().unwrap_or_default()
    )
    .to_lowercase()
    .contains(query)
}

#[cfg(test)]
mod tests {
    use super::default_model_candidate;
    use hakonfig_core::engine::model::{ModelSummary, ProviderSummary};

    fn provider(id: &str, enabled: bool) -> ProviderSummary {
        ProviderSummary {
            id: id.to_string(),
            name: id.to_string(),
            base_url: None,
            api: None,
            has_key: false,
            key_display: None,
            key_mode: String::new(),
            key_env: None,
            key_stored: false,
            models: Vec::new(),
            enabled,
            can_toggle: true,
        }
    }

    fn model(id: &str, name: Option<&str>) -> ModelSummary {
        ModelSummary {
            id: id.to_string(),
            name: name.map(str::to_string),
            context_window: None,
            max_tokens: None,
            reasoning: false,
        }
    }

    #[test]
    fn disabled_provider_never_appears_in_default_model_picker() {
        let disabled = provider("opencode-go", false);
        let visible = provider("commandcode", true);
        let m = model("glm-5.3", None);
        assert!(!default_model_candidate(&disabled, &m, None, ""));
        assert!(default_model_candidate(&visible, &m, None, ""));
        assert!(default_model_candidate(&visible, &m, Some("commandcode"), ""));
        assert!(!default_model_candidate(&visible, &m, Some("opencode-go"), ""));
    }

    #[test]
    fn picker_query_matches_provider_and_model_names() {
        let p = provider("commandcode", true);
        let m = model("deepseek/deepseek-v4.1-flash", Some("DeepSeek V4.1 Flash"));
        assert!(default_model_candidate(&p, &m, None, "deepseek"));
        assert!(default_model_candidate(&p, &m, None, "v4.1"));
        assert!(!default_model_candidate(&p, &m, None, "opencode"));
    }
}
