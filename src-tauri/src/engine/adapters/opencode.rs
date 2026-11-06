//! Adapter OpenCode (`~/.config/opencode`).
//!
//! * `opencode.json` : provider berada di map `provider` (lama) atau
//!   `providers` (baru). Rekonfig memakai map yang sudah ada di file.
//! * `~/.local/share/opencode/auth.json` : `{ "<id>": { "type": "api", "key": ... } }`
//!
//! Bentuk provider OpenCode berbeda dari Pi: model berupa objek berkunci id,
//! endpoint di `options.baseURL`, dan paket SDK di `npm`.
use anyhow::Result;
use serde_json::{json, Map, Map as JsonMap, Value};
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::model::{
    ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, ModelSummary, ProviderSummary,
};
use crate::engine::paths;

pub struct OpenCodeAdapter;

/// Pilih nama map provider yang dipakai file: hormati yang sudah ada.
fn providers_key(data: &Value) -> &'static str {
    if data.get("providers").map(Value::is_object).unwrap_or(false) {
        "providers"
    } else {
        "provider"
    }
}

fn model_summary(id: &str, value: &Value) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: get_str(value, "name"),
        context_window: value
            .get("limit")
            .and_then(|l| l.get("context"))
            .and_then(Value::as_u64),
        max_tokens: value
            .get("limit")
            .and_then(|l| l.get("output"))
            .and_then(Value::as_u64),
        reasoning: value
            .get("reasoning")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn opencode_model_value(
    draft: &crate::engine::model::ModelDraft,
    existing: Option<&Value>,
) -> Value {
    let mut value = existing
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let obj = value.as_object_mut().expect("dipastikan object");

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
    obj.insert("reasoning".to_string(), Value::Bool(draft.reasoning));
    if !obj.contains_key("tool_call") {
        obj.insert("tool_call".to_string(), Value::Bool(true));
    }
    if !obj.contains_key("attachment") {
        obj.insert("attachment".to_string(), Value::Bool(false));
    }

    let mut limit = obj
        .get("limit")
        .cloned()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    {
        let limit_obj = limit.as_object_mut().expect("dipastikan object");
        match draft.context_window {
            Some(n) if n > 0 => {
                limit_obj.insert("context".to_string(), json!(n));
            }
            _ => {
                limit_obj.remove("context");
            }
        }
        match draft.max_tokens {
            Some(n) if n > 0 => {
                limit_obj.insert("output".to_string(), json!(n));
            }
            _ => {
                limit_obj.remove("output");
            }
        }
    }
    if limit.as_object().map(Map::is_empty).unwrap_or(true) {
        obj.remove("limit");
    } else {
        obj.insert("limit".to_string(), limit);
    }

    value
}

impl Adapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn name(&self) -> &'static str {
        "OpenCode"
    }

    fn notes(&self) -> &'static str {
        "Provider di opencode.json, API key di auth.json milik OpenCode."
    }

    fn specs(&self) -> Vec<FileSpec> {
        let config_dir = paths::opencode_config_dir();
        vec![
            FileSpec {
                role: "config",
                path: paths::opencode_config_file(&config_dir),
                format: Format::Json,
                sensitive: false,
            },
            FileSpec {
                role: "auth",
                path: paths::opencode_data_dir().join("auth.json"),
                format: Format::Json,
                sensitive: true,
            },
        ]
    }

    fn backup_base(&self) -> PathBuf {
        paths::opencode_config_dir()
    }

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot> {
        let config = docs.require("config")?;
        let auth = docs.get("auth");
        let data = config.data();

        // gabungkan map lama dan baru; map baru menang bila id sama
        let mut merged: JsonMap<String, Value> = JsonMap::new();
        for key in ["provider", "providers"] {
            if let Some(map) = data.get(key).and_then(Value::as_object) {
                for (id, entry) in map {
                    merged.insert(id.clone(), entry.clone());
                }
            }
        }

        let mut providers = Vec::new();
        for (id, entry) in &merged {
            let stored = auth
                .and_then(|doc| doc.data().get(id))
                .and_then(|credential| get_str(credential, "key"));
            let options_key = entry
                .get("options")
                .and_then(|o| o.get("apiKey"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let env_var = entry
                .get("env")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(Value::as_str)
                .map(|v| format!("${}", v));

            let (has_key, key_display) = match (stored, options_key, env_var) {
                (Some(key), _, _) => (true, Some(mask_key(&key))),
                (None, Some(key), _) => (true, Some(mask_key(&key))),
                (None, None, Some(var)) => (true, Some(var)),
                _ => (false, None),
            };

            let models = entry
                .get("models")
                .and_then(Value::as_object)
                .map(|map| {
                    let mut list: Vec<ModelSummary> = map
                        .iter()
                        .map(|(model_id, value)| model_summary(model_id, value))
                        .collect();
                    list.sort_by(|a, b| a.id.cmp(&b.id));
                    list
                })
                .unwrap_or_default();

            providers.push(ProviderSummary {
                id: id.clone(),
                name: get_str(entry, "name").unwrap_or_else(|| id.clone()),
                base_url: entry
                    .get("options")
                    .and_then(|o| get_str(o, "baseURL"))
                    .or_else(|| get_str(entry, "api")),
                api: get_str(entry, "npm"),
                has_key,
                key_display,
                models,
            });
        }
        providers.sort_by(|a, b| a.id.cmp(&b.id));

        let default = config.data().get("model").and_then(Value::as_str);
        let (default_provider, default_model) = default
            .map(shared::split_provider_model)
            .unwrap_or((None, None));

        let files: Vec<ConfigFileInfo> = self
            .specs()
            .into_iter()
            .map(|spec| ConfigFileInfo {
                path: spec.path.display().to_string(),
                role: spec.role.to_string(),
                exists: spec.path.exists(),
                sensitive: spec.sensitive,
            })
            .collect();

        Ok(HarnessSnapshot {
            harness: self.id().to_string(),
            providers,
            files,
            default_provider,
            default_model,
            warnings: Vec::new(),
        })
    }

    fn build(&self, docs: &mut Docs, op: &EditOp) -> Result<Vec<String>> {
        match op {
            EditOp::Upsert(draft) => {
                shared::validate_draft(draft)?;
                let mut warnings = Vec::new();
                let id = draft.id.trim().to_string();
                let env_var = crate::engine::doc::env_name_from(&id);
                let mode = shared::key_mode(draft);
                let container = providers_key(docs.require("config")?.data());
                let existing = docs
                    .require("config")?
                    .data()
                    .get(container)
                    .and_then(|m| m.get(&id))
                    .cloned();
                let is_new = existing.is_none();

                // --- provider di opencode.json
                let mut entry = existing
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({}));
                {
                    let obj = entry.as_object_mut().expect("dipastikan object");
                    obj.insert(
                        "name".to_string(),
                        Value::String(draft.name.trim().to_string()),
                    );
                    if is_new || !obj.contains_key("npm") {
                        obj.insert(
                            "npm".to_string(),
                            Value::String(shared::opencode_npm(draft.api.trim()).to_string()),
                        );
                    }

                    let mut options = obj
                        .get("options")
                        .cloned()
                        .filter(Value::is_object)
                        .unwrap_or_else(|| json!({}));
                    {
                        let options_obj = options.as_object_mut().expect("dipastikan object");
                        options_obj.insert(
                            "baseURL".to_string(),
                            Value::String(draft.base_url.trim().to_string()),
                        );
                        // Key tidak pernah diduplikasi di dua tempat.
                        options_obj.remove("apiKey");
                    }
                    obj.insert("options".to_string(), options);

                    match mode {
                        KeyMode::Env => {
                            obj.insert(
                                "env".to_string(),
                                Value::Array(vec![Value::String(env_var.clone())]),
                            );
                            warnings.push(format!(
                                "Mode environment: set variabel {} di shell kamu.",
                                env_var
                            ));
                        }
                        KeyMode::Inline => {
                            obj.remove("env");
                        }
                    }

                    if !draft.models.is_empty() {
                        let existing_models = obj
                            .get("models")
                            .and_then(Value::as_object)
                            .cloned()
                            .unwrap_or_default();
                        let mut models = JsonMap::new();
                        for model in &draft.models {
                            let previous = existing_models.get(model.id.trim());
                            models.insert(
                                model.id.trim().to_string(),
                                opencode_model_value(model, previous),
                            );
                        }
                        obj.insert("models".to_string(), Value::Object(models));
                    }
                }
                docs.require_mut("config")?
                    .object_at(container)
                    .insert(id.clone(), entry);

                // --- kredensial
                if let Some(key) = draft.key_value() {
                    let auth = docs.require_mut("auth")?;
                    if mode == KeyMode::Inline {
                        auth.root_object()
                            .insert(id.clone(), json!({ "type": "api", "key": key }));
                    } else if auth.data().get(&id).is_some() {
                        auth.root_object().remove(&id);
                    }
                } else if is_new {
                    warnings.push(
                        "Provider dibuat tanpa API key. Isi key lewat tombol Ubah.".to_string(),
                    );
                }

                Ok(warnings)
            }
            EditOp::Remove { provider_id } => {
                let target = provider_id.trim().to_string();
                let container = providers_key(docs.require("config")?.data());
                let mut removed = false;
                {
                    let config = docs.require_mut("config")?;
                    if let Some(map) = config
                        .tree_mut()
                        .get_mut(container)
                        .and_then(Value::as_object_mut)
                    {
                        removed = map.remove(&target).is_some();
                    }
                }
                if let Some(auth) = docs.get_mut("auth") {
                    if auth.data().get(&target).is_some() {
                        auth.root_object().remove(&target);
                    }
                }
                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!(
                        "Provider '{}' tidak ditemukan di opencode.json.",
                        target
                    ));
                }
                Ok(warnings)
            }
        }
    }
}
