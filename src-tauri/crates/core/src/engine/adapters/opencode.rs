//! Adapter OpenCode (`~/.config/opencode`).
//!
//! * `opencode.json` : provider berada di map `provider` (lama) atau
//!   `providers` (baru). Hakonfig memakai map yang sudah ada di file.
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
    ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, ModelSummary, Notice, ProviderSummary,
};
use crate::engine::paths;

pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    /// Default (`model`) tidak boleh menunjuk provider yang dinonaktifkan.
    ///
    /// OpenCode punya `disabled_providers`, tetapi `model` tetap tersimpan
    /// sebagai referensi; membiarkannya berarti config menyebut provider yang
    /// tidak pernah dimuat.
    fn sync_default(docs: &mut Docs) -> Result<Vec<String>> {
        let Some(current) = docs
            .get("config")
            .and_then(|doc| get_str(doc.data(), "model"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            return Ok(Vec::new());
        };

        let disabled: Vec<String> = docs
            .get("config")
            .and_then(|doc| doc.data().get("disabled_providers"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let (provider, _) = shared::split_provider_model(&current);
        let Some(provider) = provider else {
            return Ok(Vec::new());
        };
        if !disabled.iter().any(|candidate| candidate == &provider) {
            return Ok(Vec::new());
        }

        // Pengganti: provider pertama yang masih aktif dan punya model.
        let replacement: Option<(String, String)> = docs.get("config").and_then(|doc| {
            let data = doc.data();
            let container = providers_key(data);
            let providers = data.get(container).and_then(Value::as_object)?;
            providers
                .iter()
                .filter(|(id, _)| !disabled.iter().any(|candidate| candidate == *id))
                .find_map(|(id, entry)| {
                    entry
                        .get("models")
                        .and_then(Value::as_object)
                        .and_then(|models| models.keys().next())
                        .map(|model| (id.clone(), model.clone()))
                })
        });

        let config = docs.require_mut("config")?;
        let root = config.root_object();
        match replacement {
            Some((provider, model)) => {
                root.insert(
                    "model".to_string(),
                    Value::String(format!("{provider}/{model}")),
                );
                Ok(vec![format!(
                    "Default model moved to '{provider}/{model}' because '{current}' points at a disabled provider."
                )])
            }
            None => {
                root.remove("model");
                Ok(vec![format!(
                    "No provider is active any more, so the default model '{current}' was cleared."
                )])
            }
        }
    }
}

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

impl OpenCodeAdapter {
    /// Provider di `disabled_providers` tidak muncul di pemilih model OpenCode,
    /// walaupun entri dan modelnya tetap ada di `opencode.json`.
    fn disabled_notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let disabled: Vec<String> = docs
            .get("config")
            .and_then(|doc| doc.data().get("disabled_providers"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        providers
            .iter()
            .filter(|provider| disabled.iter().any(|entry| entry == provider.id))
            .map(|provider| Notice {
                provider: provider.id.to_string(),
                code: "opencode.disabled".to_string(),
                harness: "opencode".to_string(),
                message: format!(
                    "provider '{}' is in disabled_providers in opencode.json, so OpenCode hides its models.",
                    provider.id
                ),
                fix: Some(format!("Remove '{}' from disabled_providers", provider.id)),
                destructive: false,
            })
            .collect()
    }

    /// Keluarkan provider dari `disabled_providers` di `opencode.json`.
    fn re_enable(docs: &mut Docs, provider: &str) -> Result<Option<String>> {
        // Jangan membuat file yang belum ada hanya untuk mengurus daftar disable.
        let Some(doc) = docs.get_mut("config") else {
            return Ok(None);
        };
        if !doc.exists {
            return Ok(None);
        }
        let Some(Value::Array(list)) = doc.root_object().get_mut("disabled_providers") else {
            return Ok(None);
        };
        let before = list.len();
        list.retain(|item| item.as_str() != Some(provider));
        if list.len() == before {
            return Ok(None);
        }
        Ok(Some(format!(
            "opencode.json: '{provider}' removed from disabled_providers so its models show up."
        )))
    }
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

    fn supports_toggle(&self) -> bool {
        true
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

        let disabled: Vec<String> = data
            .get("disabled_providers")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

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
                key_stored: has_key,
                // Kredensial OpenCode tersimpan di auth.json miliknya sendiri.
                key_mode: if has_key {
                    "stored".to_string()
                } else {
                    "none".to_string()
                },
                // Key dari blok `options` juga bisa berupa referensi env.
                key_env: entry
                    .get("options")
                    .and_then(|options| get_str(options, "apiKey"))
                    .or_else(|| get_str(entry, "apiKey"))
                    .map(|value| value.trim().to_string())
                    .filter(|value| value.starts_with('$'))
                    .map(|value| {
                        value
                            .trim_start_matches('$')
                            .trim_start_matches('{')
                            .trim_end_matches('}')
                            .to_string()
                    }),
                models,
                enabled: !disabled.iter().any(|d| d == id),
                can_toggle: self.supports_toggle(),
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
            home: None,
            user: None,
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
                        // Key baru diberikan: simpan di satu tempat saja
                        // (auth.json untuk inline, `env` untuk environment).
                        // Tanpa key baru, `apiKey`/`env` lama dibiarkan apa adanya
                        // supaya edit nama/model tidak membuang kredensial.
                        if draft.key_value().is_some() {
                            options_obj.remove("apiKey");
                        }
                    }
                    obj.insert("options".to_string(), options);

                    if draft.key_value().is_some() {
                        match mode {
                            KeyMode::Env => {
                                obj.insert(
                                    "env".to_string(),
                                    Value::Array(vec![Value::String(env_var.clone())]),
                                );
                                warnings.push(format!(
                                    "Environment mode: set {} in your shell.",
                                    env_var
                                ));
                            }
                            KeyMode::Inline => {
                                obj.remove("env");
                            }
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
                        "Provider created without an API key. Fill the key in via Change."
                            .to_string(),
                    );
                }

                // Provider yang ada di disabled_providers tidak akan terlihat
                // walau modelnya baru ditulis, jadi keluarkan dari daftar itu.
                if let Some(summary) = Self::re_enable(docs, &id)? {
                    warnings.push(summary);
                }

                Ok(warnings)
            }
            EditOp::SetKey {
                provider_id,
                key,
                key_mode,
            } => {
                let target = provider_id.trim().to_string();
                let key = key.trim().to_string();
                if key.is_empty() {
                    anyhow::bail!("API key is empty.");
                }
                let env_var = crate::engine::doc::env_name_from(&target);

                let container = providers_key(docs.require("config")?.data());
                if docs
                    .require("config")?
                    .data()
                    .get(container)
                    .and_then(|map| map.get(&target))
                    .is_none()
                {
                    anyhow::bail!("Provider '{}' not found in opencode.json.", target);
                }

                match key_mode {
                    KeyMode::Inline => {
                        let auth = docs.require_mut("auth")?;
                        auth.root_object()
                            .insert(target.clone(), json!({ "type": "api", "key": key }));
                        if let Some(config) = docs.get_mut("config") {
                            if let Some(options) = config
                                .object_at(container)
                                .get_mut(&target)
                                .and_then(|entry| entry.get_mut("options"))
                                .and_then(Value::as_object_mut)
                            {
                                options.remove("apiKey");
                            }
                        }
                        Ok(Vec::new())
                    }
                    KeyMode::Env => {
                        if let Some(config) = docs.get_mut("config") {
                            if let Some(entry) = config
                                .object_at(container)
                                .get_mut(&target)
                                .and_then(Value::as_object_mut)
                            {
                                let options = entry
                                    .entry("options".to_string())
                                    .or_insert_with(|| json!({}));
                                if let Some(options) = options.as_object_mut() {
                                    options.insert(
                                        "apiKey".to_string(),
                                        Value::String(format!("{{env:{}}}", env_var)),
                                    );
                                }
                            }
                        }
                        if let Some(auth) = docs.get_mut("auth") {
                            if auth.data().get(&target).is_some() {
                                auth.root_object().remove(&target);
                            }
                        }
                        Ok(vec![format!(
                            "Environment mode: set {} in your shell.",
                            env_var
                        )])
                    }
                }
            }
            EditOp::SetModels {
                provider_id,
                models,
            } => {
                let target = provider_id.trim().to_string();
                if models.is_empty() {
                    anyhow::bail!("Tambahkan minimal satu model.");
                }
                let container = providers_key(docs.require("config")?.data());
                let document = docs.require_mut("config")?;
                let Some(entry) = document
                    .object_at(container)
                    .get_mut(&target)
                    .and_then(Value::as_object_mut)
                else {
                    anyhow::bail!("Provider '{}' not found in opencode.json.", target);
                };

                // Model OpenCode berupa map berkunci id; metadata yang sudah ada
                // dipertahankan supaya limit/opsi tidak hilang saat menyalakan
                // atau mematikan model lain.
                let existing = entry
                    .get("models")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let mut map = JsonMap::new();
                for model in models {
                    let id = model.id.trim().to_string();
                    let mut value = existing.get(&id).cloned().unwrap_or_else(|| json!({}));
                    if let Some(name) = model
                        .name
                        .as_deref()
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                    {
                        if let Some(object) = value.as_object_mut() {
                            object.insert("name".to_string(), Value::String(name.to_string()));
                        }
                    }
                    map.insert(id, value);
                }
                entry.insert("models".to_string(), Value::Object(map));
                Ok(vec![format!(
                    "{} model ditulis untuk '{}'.",
                    entry
                        .get("models")
                        .and_then(Value::as_object)
                        .map(|map| map.len())
                        .unwrap_or(0),
                    target
                )])
            }
            EditOp::SetEnabled {
                provider_id,
                enabled,
            } => {
                let target = provider_id.trim().to_string();
                let container = providers_key(docs.require("config")?.data());
                let known = docs
                    .require("config")?
                    .data()
                    .get(container)
                    .and_then(|m| m.get(&target))
                    .is_some()
                    || docs
                        .require("config")?
                        .data()
                        .get("providers")
                        .and_then(|m| m.get(&target))
                        .is_some();
                if !known {
                    anyhow::bail!("Provider '{}' not found in opencode.json.", target);
                }

                let config = docs.require_mut("config")?;
                let list = config.array_at("disabled_providers");
                list.retain(|item| item.as_str() != Some(target.as_str()));
                if !*enabled {
                    list.push(Value::String(target.clone()));
                }

                let mut warnings = vec![if *enabled {
                    format!("Provider '{}' re-enabled.", target)
                } else {
                    format!(
                        "Provider '{}' disabled (the entry stays in opencode.json).",
                        target
                    )
                }];
                warnings.extend(Self::sync_default(docs)?);
                Ok(warnings)
            }
            EditOp::SetDefault {
                provider_id,
                model_id,
            } => {
                let provider = provider_id.trim().to_string();
                let model = model_id.trim().to_string();
                if provider.is_empty() || model.is_empty() {
                    anyhow::bail!("Provider and model are required.");
                }
                let config = docs.require_mut("config")?;
                config.root_object().insert(
                    "model".to_string(),
                    Value::String(format!("{}/{}", provider, model)),
                );
                Ok(vec![format!(
                    "OpenCode default: {}/{} (written to opencode.json).",
                    provider, model
                )])
            }
            EditOp::ForgetKey { .. } => anyhow::bail!(
                "OPENCODE has no separate credential store: the API key lives in the provider entry, so remove the provider instead.",
            ),
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
                    warnings.push(format!("Provider '{}' not found in opencode.json.", target));
                }
                Ok(warnings)
            }
        }
    }

    /// Urutan sama dengan `snapshot`: `auth.json` → `options.apiKey` → `env[0]`
    /// (dikembalikan sebagai `$VAR` agar pemanggil bisa me-resolve environment).
    fn builtin_providers(&self) -> &'static [(&'static str, &'static str)] {
        shared::BUILTIN_PROVIDERS
    }

    fn notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let mut notices =
            shared::builtin_id_notices(self.id(), self.builtin_providers(), providers);
        notices.extend(self.disabled_notices(docs, providers));
        notices
    }

    fn fix_notice(&self, docs: &mut Docs, notice: &Notice) -> Result<Option<String>> {
        match notice.code.as_str() {
            "opencode.disabled" => Self::re_enable(docs, &notice.provider),
            other => anyhow::bail!("opencode has no automatic fix for '{other}'"),
        }
    }

    fn stored_key(&self, docs: &Docs, provider_id: &str) -> Option<String> {
        let target = provider_id.trim();
        if target.is_empty() {
            return None;
        }
        if let Some(key) = docs
            .get("auth")
            .and_then(|doc| doc.data().get(target))
            .and_then(|credential| get_str(credential, "key"))
        {
            return Some(key);
        }

        let config = docs.get("config")?;
        let data = config.data();
        // Map baru (`providers`) menang atas map lama (`provider`).
        let mut found = None;
        for container in ["provider", "providers"] {
            if let Some(entry) = data.get(container).and_then(|map| map.get(target)) {
                found = Some(entry);
            }
        }
        let entry = found?;
        if let Some(key) = entry
            .get("options")
            .and_then(|options| get_str(options, "apiKey"))
        {
            return Some(key);
        }
        entry
            .get("env")
            .and_then(Value::as_array)
            .and_then(|vars| vars.iter().find_map(Value::as_str))
            .map(|var| format!("${}", var.trim()))
    }
}
