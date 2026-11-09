//! Helper yang dipakai bersama oleh beberapa adapter.
use anyhow::{bail, Result};
use serde_json::{json, Value};

use super::super::doc::is_slug;
use super::super::model::{KeyMode, ModelDraft, ModelSummary, Notice, ProviderDraft};

pub fn validate_draft(draft: &ProviderDraft) -> Result<()> {
    if !is_slug(draft.id.trim()) {
        bail!(
            "Provider id '{}' is not valid. Use lowercase letters, digits, dot, minus, or underscore.",
            draft.id
        );
    }
    if draft.name.trim().is_empty() {
        bail!("Provider name is required.");
    }
    let base = draft.base_url.trim();
    if base.is_empty() {
        bail!("Base URL is required.");
    }
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        bail!("Base URL must start with http:// or https://");
    }
    if draft.api.trim().is_empty() {
        bail!("API type is required.");
    }
    if draft.models.is_empty() {
        bail!("Tambahkan minimal satu model.");
    }
    for model in &draft.models {
        if model.id.trim().is_empty() {
            bail!("A model row still has an empty id.");
        }
    }
    Ok(())
}

pub fn key_mode(draft: &ProviderDraft) -> KeyMode {
    draft.key_mode()
}

// ------------------------------------------------------------ model catalog

pub fn model_summary_from_pi_style(value: &Value) -> ModelSummary {
    ModelSummary {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
        context_window: value.get("contextWindow").and_then(Value::as_u64),
        max_tokens: value.get("maxTokens").and_then(Value::as_u64),
        reasoning: value
            .get("reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

pub fn models_from_pi_style(value: &Value) -> Vec<ModelSummary> {
    value
        .get("models")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(model_summary_from_pi_style).collect())
        .unwrap_or_default()
}

/// Gabungkan satu model draft dengan entri model lama (kalau ada) supaya
/// field seperti `thinking`, `cost`, atau `compat` tidak hilang.
pub fn model_value_from_draft(draft: &ModelDraft, existing: Option<&Value>) -> Value {
    let mut value = existing
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let obj = value.as_object_mut().expect("dipastikan object di atas");

    obj.insert("id".to_string(), Value::String(draft.id.trim().to_string()));

    match draft
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        Some(name) => {
            obj.insert("name".to_string(), Value::String(name.to_string()));
        }
        None => {
            obj.remove("name");
        }
    }
    match draft.context_window {
        Some(n) if n > 0 => {
            obj.insert("contextWindow".to_string(), json!(n));
        }
        _ => {
            obj.remove("contextWindow");
        }
    }
    match draft.max_tokens {
        Some(n) if n > 0 => {
            obj.insert("maxTokens".to_string(), json!(n));
        }
        _ => {
            obj.remove("maxTokens");
        }
    }
    if draft.reasoning {
        obj.insert("reasoning".to_string(), Value::Bool(true));
    } else {
        obj.remove("reasoning");
    }

    value
}

/// Entri provider gaya Pi (`baseUrl`/`api`/`apiKey`/`models[]`).
/// `api_key`: `Some` untuk menulis field `apiKey`, `None` untuk menghapusnya.
pub fn pi_style_entry(
    draft: &ProviderDraft,
    existing: Option<&Value>,
    api_key: Option<String>,
) -> Value {
    let mut entry = existing
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let obj = entry.as_object_mut().expect("dipastikan object di atas");

    obj.insert(
        "name".to_string(),
        Value::String(draft.name.trim().to_string()),
    );
    obj.insert(
        "baseUrl".to_string(),
        Value::String(draft.base_url.trim().to_string()),
    );
    obj.insert(
        "api".to_string(),
        Value::String(draft.api.trim().to_string()),
    );

    match api_key {
        Some(key) => {
            obj.insert("apiKey".to_string(), Value::String(key));
        }
        None => {
            obj.remove("apiKey");
        }
    }

    if !draft.models.is_empty() {
        let existing_models: Vec<Value> = obj
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let models: Vec<Value> = draft
            .models
            .iter()
            .map(|model| {
                let previous = existing_models.iter().find(|candidate| {
                    candidate.get("id").and_then(Value::as_str) == Some(model.id.trim())
                });
                model_value_from_draft(model, previous)
            })
            .collect();
        obj.insert("models".to_string(), Value::Array(models));
    }

    entry
}

// ------------------------------------------------------------ misc mapping

/// `"opencode-go/deepseek-v4.1-flash"` -> ("opencode-go", "deepseek-v4.1-flash")
pub fn split_provider_model(value: &str) -> (Option<String>, Option<String>) {
    match value.split_once('/') {
        Some((provider, model)) if !provider.is_empty() && !model.is_empty() => {
            (Some(provider.to_string()), Some(model.to_string()))
        }
        _ => (None, None),
    }
}

pub fn hermes_api_mode(api: &str) -> String {
    match api {
        "anthropic-messages" => "anthropic_messages".to_string(),
        "openai-responses" => "responses".to_string(),
        _ => "chat_completions".to_string(),
    }
}

pub fn opencode_npm(api: &str) -> &'static str {
    match api {
        "anthropic-messages" => "@ai-sdk/anthropic",
        "google-generative-ai" => "@ai-sdk/google",
        "openai-responses" => "@ai-sdk/openai",
        _ => "@ai-sdk/openai-compatible",
    }
}

// ---------------------------------------------------------------- notices

/// Provider yang katalog modelnya datang dari server pi.dev / models.dev,
/// beserta host endpoint resminya.
///
/// Dipakai untuk mendeteksi id yang dipakai ulang oleh gateway lain. Host ikut
/// dibandingkan supaya provider asli (mis. `deepseek` dengan base URL
/// `api.deepseek.com`) tidak diperingatkan — hanya endpoint yang berbeda.
pub const BUILTIN_PROVIDERS: &[(&str, &str)] = &[
    ("anthropic", "api.anthropic.com"),
    ("cerebras", "api.cerebras.ai"),
    ("chutes", "llm.chutes.ai"),
    ("deepseek", "api.deepseek.com"),
    ("factory-droid", "api.factory.ai"),
    ("fireworks", "api.fireworks.ai"),
    ("github-copilot", "api.githubcopilot.com"),
    ("google", "generativelanguage.googleapis.com"),
    ("groq", "api.groq.com"),
    ("lmstudio", "localhost"),
    ("mistral", "api.mistral.ai"),
    ("ollama", "localhost"),
    ("openai", "api.openai.com"),
    ("openai-codex", "chatgpt.com"),
    ("opencode", "opencode.ai"),
    ("opencode-go", "opencode.ai"),
    ("openrouter", "openrouter.ai"),
    ("xai", "api.x.ai"),
    ("zai", "api.z.ai"),
];

/// Provider yang sedang diperiksa, seperlunya saja.
#[derive(Debug, Clone, Copy)]
pub struct ProviderRef<'a> {
    pub id: &'a str,
    pub base_url: Option<&'a str>,
}

impl<'a> ProviderRef<'a> {
    pub fn new(id: &'a str, base_url: Option<&'a str>) -> Self {
        Self { id, base_url }
    }
}

/// Host dari base URL, tanpa skema, port, dan path.
fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()?
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(after_scheme.split(['/', '?', '#']).next()?);
    let host = host.split(':').next()?.trim().to_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// Apakah provider ini memakai id bawaan harness untuk endpoint yang berbeda.
fn reuses_builtin_id(provider: &ProviderRef<'_>, builtins: &[(&str, &str)]) -> bool {
    let Some((_, expected_host)) = builtins.iter().find(|(id, _)| *id == provider.id) else {
        return false;
    };
    // Tanpa base URL tidak ada yang bisa dibandingkan: anggap provider asli.
    let Some(base_url) = provider.base_url else {
        return false;
    };
    match host_of(base_url) {
        Some(host) => host != *expected_host,
        None => false,
    }
}

/// Notice untuk provider yang memakai id provider bawaan dengan endpoint lain.
pub fn builtin_id_notices(
    harness: &str,
    builtins: &[(&str, &str)],
    providers: &[ProviderRef<'_>],
) -> Vec<Notice> {
    providers
        .iter()
        .filter(|provider| reuses_builtin_id(provider, builtins))
        .map(|provider| Notice {
            code: "harness.builtin-id".to_string(),
            harness: harness.to_string(),
            provider: provider.id.to_string(),
            message: format!(
                "'{}' uses the id of a built-in {harness} provider but points somewhere else, so the remote catalog for that id is merged with your models. Rename the gateway (for example '{}-gw') if the list looks mixed or refresh warnings mention it.",
                provider.id, provider.id
            ),
            fix: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_is_extracted_without_scheme_port_or_path() {
        assert_eq!(
            host_of("https://api.deepseek.com/v1").as_deref(),
            Some("api.deepseek.com")
        );
        assert_eq!(
            host_of("http://127.0.0.1:8731/v1").as_deref(),
            Some("127.0.0.1")
        );
        assert_eq!(
            host_of("api.groq.com/openai/v1").as_deref(),
            Some("api.groq.com")
        );
        assert_eq!(host_of("   "), None);
    }

    #[test]
    fn real_provider_is_not_reported() {
        let refs = [ProviderRef::new(
            "deepseek",
            Some("https://api.deepseek.com"),
        )];
        assert!(builtin_id_notices("pi", BUILTIN_PROVIDERS, &refs).is_empty());
    }

    #[test]
    fn gateway_that_reuses_a_builtin_id_is_reported() {
        let refs = [ProviderRef::new(
            "openrouter",
            Some("https://my-gw.test/v1"),
        )];
        let notices = builtin_id_notices("pi", BUILTIN_PROVIDERS, &refs);
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].code, "harness.builtin-id");
        assert!(notices[0].message.contains("openrouter-gw"));
    }

    #[test]
    fn custom_id_without_a_builtin_twin_is_silent() {
        let refs = [ProviderRef::new("gateway-saya", Some("https://gw.test/v1"))];
        assert!(builtin_id_notices("pi", BUILTIN_PROVIDERS, &refs).is_empty());
    }
}
