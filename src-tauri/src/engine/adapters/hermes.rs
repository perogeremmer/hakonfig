//! Adapter Hermes (`~/.hermes`) — **eksperimental**.
//!
//! * `config.yaml` : `providers.<id>` = { name, base_url, key_env, api_mode }
//! * `.env`        : nilai API key yang dirujuk `key_env`
//!
//! Hermes tidak menyimpan daftar model di config; ia mengambilnya dari endpoint
//! provider. Karena itu Rekonfig hanya menulis provider + kredensial.
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{env_name_from, get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::model::{ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, ProviderSummary};
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

    fn notes(&self) -> &'static str {
        "Eksperimental. Provider di config.yaml, key disimpan di .env lewat key_env. Daftar model diambil Hermes dari endpoint."
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
            "Dukungan Hermes masih eksperimental: hanya provider dan API key yang ditulis."
                .to_string(),
        ];
        if config.has_comments {
            warnings.push(
                "config.yaml memuat komentar. Komentar tidak dipertahankan saat Rekonfig menulis (backup otomatis dibuat)."
                    .to_string(),
            );
        }

        let mut providers = Vec::new();
        if let Some(map) = data.get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
                let key_env = get_str(entry, "key_env");
                let stored = key_env
                    .as_deref()
                    .and_then(|var| env.and_then(|doc| doc.env_get(var)));
                let key_display = match (&stored, &key_env) {
                    (Some(value), _) => Some(mask_key(value)),
                    (None, Some(var)) => Some(format!("${} (belum diisi)", var)),
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
                        "Hermes mengambil daftar model dari endpoint provider, jadi daftar model tidak ditulis ke config."
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
                                "API key ditulis ke ~/.hermes/.env sebagai {} (permission 0600).",
                                key_env
                            ));
                        }
                        KeyMode::Env => {
                            warnings.push(format!(
                                "Mode environment: pastikan variabel {} sudah diset.",
                                key_env
                            ));
                        }
                    }
                } else if is_new {
                    warnings.push(format!(
                        "Provider dibuat tanpa key. Isi variabel {} di ~/.hermes/.env.",
                        key_env
                    ));
                }

                Ok(warnings)
            }
            EditOp::SetEnabled { .. } => {
                anyhow::bail!(
                    "Hermes tidak punya daftar provider nonaktif. Nonaktifkan provider dengan menghapusnya."
                )
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

                if let (Some(var), Some(env)) = (key_env, docs.get_mut("env")) {
                    if env.env_get(&var).is_some() {
                        env.env_remove(&var);
                    }
                }

                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!(
                        "Provider '{}' tidak ditemukan di config.yaml.",
                        target
                    ));
                }
                Ok(warnings)
            }
        }
    }
}
