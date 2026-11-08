//! Helper yang dipakai bersama oleh beberapa adapter.
use anyhow::{bail, Result};
use serde_json::{json, Value};

use super::super::doc::is_slug;
use super::super::model::{KeyMode, ModelDraft, ModelSummary, ProviderDraft};

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
