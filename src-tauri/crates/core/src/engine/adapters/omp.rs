//! Adapter oh-my-pi (`~/.omp/agent`).
//!
//! oh-my-pi adalah turunan Pi, tetapi memakai YAML:
//!   * `models.yml` : `providers.<id>` = { baseUrl, api, apiKey?, models[], modelOverrides{} }
//!   * `config.yml` : preferensi, termasuk `modelRoles.default`
//!
//! Tidak ada file kredensial terpisah yang stabil untuk ditulis, jadi API key
//! ditulis langsung pada field `apiKey` di `models.yml`.
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::model::{
    ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, Notice, ProviderSummary,
};
use crate::engine::paths;

pub struct OmpAdapter;

impl OmpAdapter {
    fn agent_dir() -> PathBuf {
        paths::omp_dir().join("agent")
    }

    /// Provider yang tercantum di `disabledProviders` milik omp: modelnya ada di
    /// `models.yml` tetapi tidak pernah muncul di pemilih model.
    fn disabled_providers(docs: &Docs) -> Vec<String> {
        docs.get("settings")
            .and_then(|doc| doc.data().get("disabledProviders"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Keluarkan provider dari `disabledProviders`; dipakai saat provider
    /// ditulis ulang supaya modelnya benar-benar muncul.
    fn re_enable(docs: &mut Docs, provider: &str) -> Result<Option<String>> {
        // Jangan membuat file yang belum ada hanya untuk mengurus daftar disable.
        let Some(doc) = docs.get_mut("settings") else {
            return Ok(None);
        };
        if !doc.exists {
            return Ok(None);
        }
        let Some(Value::Array(list)) = doc.root_object().get_mut("disabledProviders") else {
            return Ok(None);
        };
        let before = list.len();
        list.retain(|item| item.as_str() != Some(provider));
        if list.len() == before {
            return Ok(None);
        }
        Ok(Some(format!(
            "config.yml: '{provider}' removed from disabledProviders so its models show up."
        )))
    }

    fn disabled_notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let disabled = Self::disabled_providers(docs);
        providers
            .iter()
            .filter(|provider| disabled.iter().any(|entry| entry == provider.id))
            .map(|provider| Notice {
                provider: provider.id.to_string(),
                code: "omp.disabled".to_string(),
                harness: "omp".to_string(),
                message: format!(
                    "provider '{}' is listed in disabledProviders in config.yml, so omp hides its models even though they are in models.yml.",
                    provider.id
                ),
                fix: Some(format!("Remove '{}' from disabledProviders", provider.id)),
            })
            .collect()
    }
}

impl Adapter for OmpAdapter {
    fn id(&self) -> &'static str {
        "omp"
    }

    fn name(&self) -> &'static str {
        "Oh My Pi"
    }

    fn notes(&self) -> &'static str {
        "Catalog in models.yml. The key is written to the apiKey field (there is no separate auth.json)."
    }

    fn supports_toggle(&self) -> bool {
        true
    }

    fn specs(&self) -> Vec<FileSpec> {
        let dir = Self::agent_dir();
        vec![
            FileSpec {
                role: "models",
                path: dir.join("models.yml"),
                format: Format::Yaml,
                sensitive: true,
            },
            FileSpec {
                role: "settings",
                path: dir.join("config.yml"),
                format: Format::Yaml,
                sensitive: false,
            },
        ]
    }

    fn backup_base(&self) -> PathBuf {
        Self::agent_dir()
    }

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot> {
        let models = docs.require("models")?;
        let settings = docs.get("settings");

        let mut warnings = Vec::new();
        if models.has_comments {
            warnings.push(
                "models.yml contains comments. Comments are not preserved when Hakonfig writes (a backup is created automatically)."
                    .to_string(),
            );
        }

        let disabled: Vec<String> = settings
            .and_then(|doc| doc.data().get("disabledProviders"))
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
        if let Some(map) = models.data().get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
                let api_key = get_str(entry, "apiKey");
                providers.push(ProviderSummary {
                    id: id.clone(),
                    name: get_str(entry, "name").unwrap_or_else(|| id.clone()),
                    base_url: get_str(entry, "baseUrl"),
                    api: get_str(entry, "api"),
                    has_key: api_key.is_some(),
                    key_display: api_key.map(|k| mask_key(&k)),
                    models: shared::models_from_pi_style(entry),
                    enabled: !disabled.iter().any(|d| d == id),
                    can_toggle: self.supports_toggle(),
                });
            }
        }
        providers.sort_by(|a, b| a.id.cmp(&b.id));

        let mut default_provider = None;
        let mut default_model = None;
        if let Some(default) = settings
            .and_then(|doc| doc.data().get("modelRoles"))
            .and_then(|roles| roles.get("default"))
            .and_then(Value::as_str)
        {
            let (provider, model) = shared::split_provider_model(default);
            default_provider = provider;
            default_model = model;
        }

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
                let mut warnings = Vec::new();

                let existing = docs
                    .require("models")?
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&draft.id))
                    .cloned();
                let is_new = existing.is_none();

                let env_var = crate::engine::doc::env_name_from(draft.id.trim());
                let mode = shared::key_mode(draft);

                let api_key_field = match (draft.key_value(), mode) {
                    (Some(key), KeyMode::Inline) => {
                        warnings.push(
                            "Oh My Pi: the API key is written in plaintext to models.yml (the file is given permission 0600)."
                                .to_string(),
                        );
                        Some(key.to_string())
                    }
                    (Some(_), KeyMode::Env) => {
                        warnings.push(format!("Environment mode: set {} in your shell.", env_var));
                        Some(format!("${}", env_var))
                    }
                    // Edit tanpa key baru: pertahankan apiKey yang sudah ada.
                    (None, _) => existing.as_ref().and_then(|entry| get_str(entry, "apiKey")),
                };

                let entry = shared::pi_style_entry(draft, existing.as_ref(), api_key_field);
                docs.require_mut("models")?
                    .object_at("providers")
                    .insert(draft.id.trim().to_string(), entry);

                if is_new && draft.key_value().is_none() {
                    warnings.push("Provider created without an API key.".to_string());
                }

                // Provider yang sedang di-disable tidak akan terlihat walau
                // modelnya baru saja ditulis, jadi hapus dari daftar itu. Kalau
                // pengguna memang ingin menyembunyikannya, ia bisa mematikan
                // provider ini lagi sesudahnya.
                if let Some(summary) = Self::re_enable(docs, draft.id.trim())? {
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
                let document = docs.require_mut("models")?;
                let Some(entry) = document
                    .object_at("providers")
                    .get_mut(&target)
                    .and_then(Value::as_object_mut)
                else {
                    anyhow::bail!("Provider '{}' not found in models.yml.", target);
                };

                match key_mode {
                    KeyMode::Inline => {
                        entry.insert("apiKey".to_string(), Value::String(key));
                        Ok(vec![
                            "Oh My Pi: the API key is written in plaintext to models.yml (the file is given permission 0600)."
                                .to_string(),
                        ])
                    }
                    KeyMode::Env => {
                        entry.insert("apiKey".to_string(), Value::String(format!("${}", env_var)));
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
                let document = docs.require_mut("models")?;
                let Some(entry) = document
                    .object_at("providers")
                    .get_mut(&target)
                    .and_then(Value::as_object_mut)
                else {
                    anyhow::bail!("Provider '{}' not found in models.yml.", target);
                };
                entry.insert(
                    "models".to_string(),
                    Value::Array(models.iter().map(model_to_value).collect()),
                );
                Ok(vec![format!(
                    "{} model ditulis untuk '{}'.",
                    models.len(),
                    target
                )])
            }
            EditOp::SetEnabled {
                provider_id,
                enabled,
            } => {
                let target = provider_id.trim().to_string();
                let known = docs
                    .require("models")?
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&target))
                    .is_some();
                if !known {
                    anyhow::bail!("Provider '{}' not found in models.yml.", target);
                }

                let settings = docs.require_mut("settings")?;
                let list = settings.array_at("disabledProviders");
                list.retain(|item| item.as_str() != Some(target.as_str()));
                if !*enabled {
                    list.push(Value::String(target.clone()));
                }

                Ok(vec![if *enabled {
                    format!("Provider '{}' re-enabled.", target)
                } else {
                    format!(
                        "Provider '{}' disabled (the entry stays in models.yml).",
                        target
                    )
                }])
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
                let reference = format!("{}/{}", provider, model);
                let settings = docs.require_mut("settings")?;
                settings
                    .object_at("modelRoles")
                    .insert("default".to_string(), Value::String(reference.clone()));

                // oh-my-pi (turunan Pi) juga memakai model PERTAMA dari scope
                // `enabledModels` bila scope tidak kosong, dan baru jatuh ke
                // `modelRoles.default` kalau scope kosong. Tanpa menyisipkan
                // default ke urutan pertama, pilihan user tidak pernah dipakai.
                let mut warnings = Vec::new();
                let mut rescoped = false;
                let root = settings.root_object();
                if let Some(list) = root.get_mut("enabledModels").and_then(Value::as_array_mut) {
                    if !list.is_empty() {
                        list.retain(|item| item.as_str() != Some(reference.as_str()));
                        list.insert(0, Value::String(reference.clone()));
                        rescoped = true;
                    }
                }

                if rescoped {
                    warnings.push(format!(
                        "'{}' moved to the first position of enabledModels: oh-my-pi uses the first model in the scope, not modelRoles.default, as long as the scope is not empty.",
                        reference
                    ));
                }
                warnings.insert(
                    0,
                    format!(
                        "Oh My Pi default: {} (written to config.yml{}).",
                        reference,
                        if rescoped {
                            " dan urutan enabledModels"
                        } else {
                            ""
                        }
                    ),
                );
                Ok(warnings)
            }
            EditOp::Remove { provider_id } => {
                let target = provider_id.trim().to_string();
                let mut removed = false;
                {
                    let models = docs.require_mut("models")?;
                    if let Some(map) = models
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                    {
                        removed = map.remove(&target).is_some();
                    }
                }
                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!("Provider '{}' not found in models.yml.", target));
                }
                Ok(warnings)
            }
        }
    }

    /// oh-my-pi menyimpan key langsung di `models.yml` (`apiKey`), dan entry
    /// provider yang dinonaktifkan tetap ada di file - jadi cukup satu lookup.
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
            "omp.disabled" => Self::re_enable(docs, &notice.provider),
            other => anyhow::bail!("omp has no automatic fix for '{other}'"),
        }
    }

    fn stored_key(&self, docs: &Docs, provider_id: &str) -> Option<String> {
        let target = provider_id.trim();
        if target.is_empty() {
            return None;
        }
        docs.get("models")?
            .data()
            .get("providers")
            .and_then(|map| map.get(target))
            .and_then(|entry| get_str(entry, "apiKey"))
    }
}

/// Satu entri model untuk `models.yml`.
fn model_to_value(model: &crate::engine::model::ModelDraft) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("id".to_string(), Value::String(model.id.trim().to_string()));
    if let Some(name) = model
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        object.insert("name".to_string(), Value::String(name.to_string()));
    }
    if let Some(window) = model.context_window.filter(|value| *value > 0) {
        object.insert("contextWindow".to_string(), Value::from(window));
    }
    if let Some(max) = model.max_tokens.filter(|value| *value > 0) {
        object.insert("maxTokens".to_string(), Value::from(max));
    }
    if model.reasoning {
        object.insert("reasoning".to_string(), Value::Bool(true));
    }
    Value::Object(object)
}
