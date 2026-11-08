//! Adapter Pi (`~/.pi/agent`).
//!
//! Pi menyimpan katalog di `models.json` (JSONC, mendukung komentar) dan
//! kredensial di `auth.json` - dua file terpisah yang harus konsisten:
//!   * katalog : `providers.<id>` = { name, baseUrl, api, apiKey?, models[] }
//!   * kredensial: `auth.json.<id>` = { type: "api_key", key }
//!
//! Pi mendahulukan kredensial tersimpan di atas referensi environment, jadi
//! Hakonfig tidak pernah menulis keduanya sekaligus.
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::model::{ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, ProviderSummary};
use crate::engine::paths;

pub struct PiAdapter;

impl PiAdapter {
    fn agent_dir() -> PathBuf {
        paths::pi_dir().join("agent")
    }
}

impl Adapter for PiAdapter {
    fn id(&self) -> &'static str {
        "pi"
    }

    fn name(&self) -> &'static str {
        "Pi"
    }

    fn notes(&self) -> &'static str {
        "Katalog di models.json, API key di auth.json. Menonaktifkan provider = mengomentarinya di models.json."
    }

    fn supports_toggle(&self) -> bool {
        true
    }

    fn specs(&self) -> Vec<FileSpec> {
        let dir = Self::agent_dir();
        vec![
            FileSpec {
                role: "models",
                path: dir.join("models.json"),
                format: Format::Jsonc,
                sensitive: false,
            },
            FileSpec {
                role: "auth",
                path: dir.join("auth.json"),
                format: Format::Json,
                sensitive: true,
            },
            FileSpec {
                role: "settings",
                path: dir.join("settings.json"),
                format: Format::Json,
                sensitive: false,
            },
        ]
    }

    fn backup_base(&self) -> PathBuf {
        Self::agent_dir()
    }

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot> {
        let models = docs.require("models")?;
        let auth = docs.get("auth");
        let settings = docs.get("settings");

        let mut warnings = Vec::new();
        if models.has_comments {
            warnings.push(
                "models.json contains comments. Comments are not preserved when Hakonfig writes (a backup is created automatically)."
                    .to_string(),
            );
        }

        let mut providers = Vec::new();
        let mut active_ids: Vec<String> = Vec::new();
        if let Some(map) = models.data().get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
                active_ids.push(id.clone());
                let config_key = get_str(entry, "apiKey");
                let stored_key = auth
                    .and_then(|doc| doc.data().get(id))
                    .and_then(|credential| get_str(credential, "key"));

                let (has_key, key_display) = match (stored_key, config_key) {
                    (Some(key), _) => (true, Some(mask_key(&key))),
                    (None, Some(key)) => (true, Some(mask_key(&key))),
                    (None, None) => (false, None),
                };

                providers.push(ProviderSummary {
                    id: id.clone(),
                    name: get_str(entry, "name").unwrap_or_else(|| id.clone()),
                    base_url: get_str(entry, "baseUrl"),
                    api: get_str(entry, "api"),
                    has_key,
                    key_display,
                    models: shared::models_from_pi_style(entry),
                    enabled: true,
                    can_toggle: self.supports_toggle(),
                });
            }
        }
        // Provider yang dinonaktifkan tetap tampil (dari blok komentar).
        for id in models.parked_ids() {
            if active_ids.contains(&id) {
                continue;
            }
            let entry = models.parked_value(&id).unwrap_or(Value::Null);
            let stored_key = auth
                .and_then(|doc| doc.data().get(&id))
                .and_then(|credential| get_str(credential, "key"));
            let config_key = get_str(&entry, "apiKey");
            let (has_key, key_display) = match (stored_key, config_key) {
                (Some(key), _) => (true, Some(mask_key(&key))),
                (None, Some(key)) => (true, Some(mask_key(&key))),
                (None, None) => (false, None),
            };

            providers.push(ProviderSummary {
                id: id.clone(),
                name: get_str(&entry, "name").unwrap_or_else(|| id.clone()),
                base_url: get_str(&entry, "baseUrl"),
                api: get_str(&entry, "api"),
                has_key,
                key_display,
                models: shared::models_from_pi_style(&entry),
                enabled: false,
                can_toggle: true,
            });
        }

        providers.sort_by(|a, b| a.id.cmp(&b.id));

        let default_provider = settings.and_then(|doc| get_str(doc.data(), "defaultProvider"));
        let default_model = settings.and_then(|doc| get_str(doc.data(), "defaultModel"));

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
                let mut warnings = Vec::new();

                let existing_entry = docs
                    .require("models")?
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&draft.id))
                    .cloned();
                let is_new = existing_entry.is_none();

                let key_value = draft.key_value().map(str::to_string);
                let env_var = crate::engine::doc::env_name_from(draft.id.trim());
                let mode = shared::key_mode(draft);

                // --- models.json
                let api_key_field = match (&key_value, mode) {
                    (Some(_), KeyMode::Env) => Some(format!("${}", env_var)),
                    // Key inline disimpan di auth.json; jangan sisakan apiKey
                    // lama yang bisa menimpa atau membingungkan.
                    (Some(_), KeyMode::Inline) => None,
                    // Edit tanpa key baru: **pertahankan** apiKey yang sudah ada.
                    // Menghapusnya di sini akan membuang kunci milik pengguna
                    // hanya karena ia mengganti nama atau model.
                    (None, _) => existing_entry
                        .as_ref()
                        .and_then(|entry| get_str(entry, "apiKey")),
                };

                let models = docs.require_mut("models")?;
                let entry = shared::pi_style_entry(draft, existing_entry.as_ref(), api_key_field);
                models
                    .object_at("providers")
                    .insert(draft.id.trim().to_string(), entry);

                // --- auth.json
                if let Some(key) = &key_value {
                    let auth = docs.require_mut("auth")?;
                    match mode {
                        KeyMode::Inline => {
                            let entry = serde_json::json!({
                                "type": "api_key",
                                "key": key,
                            });
                            auth.root_object()
                                .insert(draft.id.trim().to_string(), entry);
                        }
                        KeyMode::Env => {
                            if auth.data().get(draft.id.trim()).is_some() {
                                auth.root_object().remove(draft.id.trim());
                            }
                            warnings.push(format!(
                                "Environment mode: the key is not written. Set {} in your shell.",
                                env_var
                            ));
                        }
                    }
                } else if is_new {
                    warnings.push(
                        "Provider created without an API key. Fill the key in via Change so Pi can authenticate."
                            .to_string(),
                    );
                }

                Ok(warnings)
            }
            EditOp::SetEnabled {
                provider_id,
                enabled,
            } => {
                let target = provider_id.trim().to_string();
                let models = docs.require_mut("models")?;
                let existing = models
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&target))
                    .cloned();

                if *enabled {
                    let Some(value) = models.parked_value(&target) else {
                        if existing.is_some() {
                            return Ok(vec![]); // sudah aktif
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    models.object_at("providers").insert(target.clone(), value);
                    models.drop_parked(&target);
                    Ok(vec![format!("Provider '{}' re-enabled.", target)])
                } else {
                    let Some(entry) = existing else {
                        if models.parked_value(&target).is_some() {
                            return Ok(vec![]); // sudah nonaktif
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    let removed = models
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                        .and_then(|map| map.remove(&target))
                        .is_some();
                    if removed {
                        models.park(&target, &entry);
                    }
                    Ok(vec![format!(
                        "Provider '{}' disabled (commented out in models.json).",
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
                let settings = docs.require_mut("settings")?;
                let root = settings.root_object();
                root.insert(
                    "defaultProvider".to_string(),
                    Value::String(provider.clone()),
                );
                let reference = format!("{}/{}", provider, model);
                root.insert("defaultModel".to_string(), Value::String(reference.clone()));

                // Pi memakai model PERTAMA dari scope `enabledModels` bila scope
                // tidak kosong, dan hanya jatuh ke defaultModel kalau scope kosong
                // (`findInitialModel` di dist/core/model-resolver.js). Jadi default
                // harus ikut jadi entri pertama scope, kalau tidak pilihan user
                // tidak pernah terpakai.
                let mut warnings = Vec::new();
                let mut rescoped = false;
                if let Some(list) = root.get_mut("enabledModels").and_then(Value::as_array_mut) {
                    if !list.is_empty() {
                        let before = list.len();
                        list.retain(|item| item.as_str() != Some(reference.as_str()));
                        list.insert(0, Value::String(reference.clone()));
                        rescoped = true;
                        if before == list.len() - 1 && before > 0 {
                            warnings.push(format!(
                                "'{}' moved to the first position of enabledModels: Pi uses the first model in the scope, not defaultModel, as long as the scope is not empty.",
                                reference
                            ));
                        } else {
                            warnings.push(format!(
                                "'{}' added as the first enabledModels entry: Pi uses the first model in the scope, not defaultModel, as long as the scope is not empty.",
                                reference
                            ));
                        }
                    }
                }

                warnings.insert(
                    0,
                    format!(
                        "Pi default: {}/{} (written to settings.json{}).",
                        provider,
                        model,
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

                if let Some(auth) = docs.get_mut("auth") {
                    if auth.data().get(&target).is_some() {
                        auth.root_object().remove(&target);
                    }
                }
                if let Some(models) = docs.get_mut("models") {
                    if models.parked_value(&target).is_some() {
                        models.drop_parked(&target);
                    }
                }

                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!("Provider '{}' not found in models.json.", target));
                }
                Ok(warnings)
            }
        }
    }

    /// Pi mendahulukan kredensial di `auth.json` di atas `apiKey` di
    /// `models.json` - urutan yang sama dengan `snapshot`.
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
        let models = docs.get("models")?;
        let entry = models
            .data()
            .get("providers")
            .and_then(|map| map.get(target))
            .cloned()
            // provider yang dinonaktifkan tetap punya key tersimpan (blok komentar)
            .or_else(|| models.parked_value(target))?;
        get_str(&entry, "apiKey")
    }
}
