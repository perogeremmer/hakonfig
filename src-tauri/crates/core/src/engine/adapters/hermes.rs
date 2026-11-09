//! Adapter Hermes (`~/.hermes`) - **eksperimental**.
//!
//! * `config.yaml` : `providers.<id>` = { name, base_url, key_env, api_mode }
//! * `.env`        : nilai API key yang dirujuk `key_env`
//!
//! Hermes tidak menyimpan daftar model di config; ia mengambilnya dari endpoint
//! provider. Karena itu Hakonfig hanya menulis provider + kredensial.
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{env_name_from, get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::model::{
    ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, Notice, ProviderSummary,
};
use crate::engine::paths;

pub struct HermesAdapter;

impl HermesAdapter {
    fn dir() -> PathBuf {
        paths::hermes_dir()
    }
}

impl Adapter for HermesAdapter {
    fn id(&self) -> &'static str {
        "hermes"
    }

    fn name(&self) -> &'static str {
        "Hermes"
    }

    fn experimental(&self) -> bool {
        true
    }

    fn supports_toggle(&self) -> bool {
        true
    }

    fn notes(&self) -> &'static str {
        "Experimental. Providers live in config.yaml, the key is stored in .env via key_env. Hermes fetches the model list from the endpoint."
    }

    fn specs(&self) -> Vec<FileSpec> {
        let dir = Self::dir();
        vec![
            FileSpec {
                role: "config",
                path: dir.join("config.yaml"),
                format: Format::Yaml,
                sensitive: false,
            },
            FileSpec {
                role: "env",
                path: dir.join(".env"),
                format: Format::DotEnv,
                sensitive: true,
            },
        ]
    }

    fn backup_base(&self) -> PathBuf {
        Self::dir()
    }

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot> {
        let config = docs.require("config")?;
        let env = docs.get("env");
        let data = config.data();

        let mut warnings = vec![
            "Hermes support is still experimental: only providers and the API key are written."
                .to_string(),
        ];
        if config.has_comments {
            warnings.push(
                "config.yaml contains comments. Comments are not preserved when Hakonfig writes (a backup is created automatically)."
                    .to_string(),
            );
        }

        let mut providers = Vec::new();
        let mut active_ids: Vec<String> = Vec::new();
        if let Some(map) = data.get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
                active_ids.push(id.clone());
                let key_env = get_str(entry, "key_env");
                let stored = key_env
                    .as_deref()
                    .and_then(|var| env.and_then(|doc| doc.env_get(var)));
                let key_display = match (&stored, &key_env) {
                    (Some(value), _) => Some(mask_key(value)),
                    (None, Some(var)) => Some(format!("${} (not set)", var)),
                    (None, None) => None,
                };
                providers.push(ProviderSummary {
                    id: id.clone(),
                    name: get_str(entry, "name").unwrap_or_else(|| id.clone()),
                    base_url: get_str(entry, "base_url"),
                    api: get_str(entry, "api_mode"),
                    has_key: stored.is_some(),
                    key_display,
                    models: Vec::new(),
                    enabled: true,
                    can_toggle: self.supports_toggle(),
                });
            }
        }
        // Provider nonaktif tetap tampil dari blok komentar.
        for id in config.parked_ids() {
            if active_ids.contains(&id) {
                continue;
            }
            let entry = config.parked_value(&id).unwrap_or(Value::Null);
            let key_env = get_str(&entry, "key_env");
            let stored = key_env
                .as_deref()
                .and_then(|var| env.and_then(|doc| doc.env_get(var)));
            let key_display = match (&stored, &key_env) {
                (Some(value), _) => Some(mask_key(value)),
                (None, Some(var)) => Some(format!("${} (not set)", var)),
                (None, None) => None,
            };
            providers.push(ProviderSummary {
                id: id.clone(),
                name: get_str(&entry, "name").unwrap_or_else(|| id.clone()),
                base_url: get_str(&entry, "base_url"),
                api: get_str(&entry, "api_mode"),
                has_key: stored.is_some(),
                key_display,
                models: Vec::new(),
                enabled: false,
                can_toggle: true,
            });
        }

        providers.sort_by(|a, b| a.id.cmp(&b.id));

        let model_section = data.get("model");
        let default_model = model_section.and_then(|m| get_str(m, "default"));
        let default_provider = model_section
            .and_then(|m| get_str(m, "provider"))
            .map(|p| p.trim_start_matches("custom:").to_string());

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
            files,
            providers,
            default_provider,
            default_model,
            warnings,
        })
    }

    fn build(&self, docs: &mut Docs, op: &EditOp) -> Result<Vec<String>> {
        match op {
            EditOp::Upsert(draft) => {
                shared::validate_draft(draft)?;
                let id = draft.id.trim().to_string();
                let mode = shared::key_mode(draft);
                let mut warnings = Vec::new();

                if !draft.models.is_empty() {
                    warnings.push(
                        "Hermes fetches the model list from the provider endpoint, so models are not written to the config."
                            .to_string(),
                    );
                }

                let existing = docs
                    .require("config")?
                    .data()
                    .get("providers")
                    .and_then(|m| m.get(&id))
                    .cloned();
                let is_new = existing.is_none();

                let key_env = existing
                    .as_ref()
                    .and_then(|entry| get_str(entry, "key_env"))
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| env_name_from(&id));

                let mut entry = existing
                    .filter(Value::is_object)
                    .unwrap_or_else(|| serde_json::json!({}));
                {
                    let obj = entry.as_object_mut().expect("dipastikan object");
                    obj.insert(
                        "name".to_string(),
                        Value::String(draft.name.trim().to_string()),
                    );
                    obj.insert(
                        "base_url".to_string(),
                        Value::String(draft.base_url.trim().to_string()),
                    );
                    obj.insert(
                        "api_mode".to_string(),
                        Value::String(shared::hermes_api_mode(draft.api.trim())),
                    );
                    obj.insert("key_env".to_string(), Value::String(key_env.clone()));
                }
                docs.require_mut("config")?
                    .object_at("providers")
                    .insert(id.clone(), entry);

                if let Some(key) = draft.key_value() {
                    match mode {
                        KeyMode::Inline => {
                            docs.require_mut("env")?.env_set(&key_env, key);
                            warnings.push(format!(
                                "API key written to ~/.hermes/.env as {} (permission 0600).",
                                key_env
                            ));
                        }
                        KeyMode::Env => {
                            warnings
                                .push(format!("Environment mode: make sure {} is set.", key_env));
                        }
                    }
                } else if is_new {
                    warnings.push(format!(
                        "Provider created without a key. Fill {} in ~/.hermes/.env.",
                        key_env
                    ));
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
                let existing = docs
                    .require("config")?
                    .data()
                    .get("providers")
                    .and_then(|map| map.get(&target))
                    .cloned();
                let Some(entry) = existing else {
                    anyhow::bail!("Provider '{}' not found in config.yaml.", target);
                };
                let key_env = get_str(&entry, "key_env")
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| env_name_from(&target));

                match key_mode {
                    KeyMode::Inline => {
                        docs.require_mut("env")?.env_set(&key_env, &key);
                        Ok(vec![format!(
                            "API key written to ~/.hermes/.env as {} (permission 0600).",
                            key_env
                        )])
                    }
                    KeyMode::Env => Ok(vec![format!(
                        "Environment mode: set {} in your shell; .env was not touched.",
                        key_env
                    )]),
                }
            }
            EditOp::SetModels { .. } => {
                anyhow::bail!(
                    "Hermes has no model list in its config (it fetches models from the endpoint), so there is nothing to change here."
                )
            }
            EditOp::SetEnabled {
                provider_id,
                enabled,
            } => {
                let target = provider_id.trim().to_string();
                let config = docs.require_mut("config")?;
                let existing = config
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&target))
                    .cloned();

                if *enabled {
                    let Some(value) = config.parked_value(&target) else {
                        if existing.is_some() {
                            return Ok(vec![]);
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    config.object_at("providers").insert(target.clone(), value);
                    config.drop_parked(&target);
                    Ok(vec![format!("Provider '{}' re-enabled.", target)])
                } else {
                    let Some(entry) = existing else {
                        if config.parked_value(&target).is_some() {
                            return Ok(vec![]);
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    let removed = config
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                        .and_then(|map| map.remove(&target))
                        .is_some();
                    if removed {
                        config.park(&target, &entry);
                    }
                    Ok(vec![format!(
                        "Provider '{}' disabled (commented out in config.yaml).",
                        target
                    )])
                }
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

                // Hermes memakai provider kustom dengan awalan "custom:".
                let base_url = docs
                    .require("config")?
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&provider))
                    .and_then(|entry| get_str(entry, "base_url"));

                let config = docs.require_mut("config")?;
                let model_section = config.object_at("model");
                model_section.insert(
                    "provider".to_string(),
                    Value::String(format!("custom:{}", provider)),
                );
                model_section.insert("default".to_string(), Value::String(model.clone()));
                if let Some(url) = base_url {
                    model_section.insert("base_url".to_string(), Value::String(url));
                }

                Ok(vec![format!(
                    "Hermes default: custom:{} / {} (written to config.yaml).",
                    provider, model
                )])
            }
            EditOp::Remove { provider_id } => {
                let target = provider_id.trim().to_string();
                let key_env = docs
                    .require("config")?
                    .data()
                    .get("providers")
                    .and_then(|m| m.get(&target))
                    .and_then(|entry| get_str(entry, "key_env"));

                let mut removed = false;
                {
                    let config = docs.require_mut("config")?;
                    if let Some(map) = config
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                    {
                        removed = map.remove(&target).is_some();
                    }
                }

                if let Some(config) = docs.get_mut("config") {
                    if config.parked_value(&target).is_some() {
                        config.drop_parked(&target);
                    }
                }

                if let (Some(var), Some(env)) = (key_env, docs.get_mut("env")) {
                    if env.env_get(&var).is_some() {
                        env.env_remove(&var);
                    }
                }

                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!("Provider '{}' not found in config.yaml.", target));
                }
                Ok(warnings)
            }
        }
    }

    /// Hermes hanya menyimpan **nama** variabel (`key_env`); nilainya ada di
    /// `.env`. Bila `.env` belum diisi, kembalikan `$VAR` supaya pemanggil
    /// masih bisa mendapatkannya dari environment proses.
    fn builtin_providers(&self) -> &'static [(&'static str, &'static str)] {
        shared::BUILTIN_PROVIDERS
    }

    fn notices(&self, _docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        // Hermes mengambil daftar model langsung dari endpoint dan menyimpannya
        // di cache per provider (`provider_models_cache.json`, TTL 1 jam), jadi
        // model yang ditulis tidak bisa "hilang". Yang tersisa hanya bentrok id.
        shared::builtin_id_notices(self.id(), self.builtin_providers(), providers)
    }

    fn stored_key(&self, docs: &Docs, provider_id: &str) -> Option<String> {
        let target = provider_id.trim();
        if target.is_empty() {
            return None;
        }
        let config = docs.get("config")?;
        let entry = config
            .data()
            .get("providers")
            .and_then(|map| map.get(target))
            .cloned()
            // provider nonaktif tetap punya `key_env` di blok komentar
            .or_else(|| config.parked_value(target))?;
        let var = get_str(&entry, "key_env")?;
        let var = var.trim();
        if var.is_empty() {
            return None;
        }
        match docs
            .get("env")
            .and_then(|doc| doc.env_get(var))
            .filter(|value| !value.trim().is_empty())
        {
            Some(value) => Some(value),
            None => Some(format!("${}", var)),
        }
    }
}
