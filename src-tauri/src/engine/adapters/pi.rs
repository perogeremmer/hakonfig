//! Adapter Pi (`~/.pi/agent`).
//!
//! Pi menyimpan katalog di `models.json` (JSONC, mendukung komentar) dan
//! kredensial di `auth.json` — dua file terpisah yang harus konsisten:
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
        "Katalog di models.json, API key di auth.json. Hakonfig menulis keduanya sekaligus."
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
                "models.json memuat komentar. Komentar tidak dipertahankan saat Hakonfig menulis (backup otomatis dibuat)."
                    .to_string(),
            );
        }

        let mut providers = Vec::new();
        if let Some(map) = models.data().get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
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
                    // Saat edit tanpa mengganti key, jangan sentuh field ini.
                    (None, _) => None,
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
                                "Mode environment: key tidak ditulis. Set variabel {} di shell kamu.",
                                env_var
                            ));
                        }
                    }
                } else if is_new {
                    warnings.push(
                        "Provider dibuat tanpa API key. Isi key lewat tombol Ubah agar Pi bisa mengautentikasi."
                            .to_string(),
                    );
                }

                Ok(warnings)
            }
            EditOp::SetEnabled { .. } => {
                anyhow::bail!(
                    "Pi tidak punya daftar provider nonaktif. Nonaktifkan provider dengan menghapusnya."
                )
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

                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!(
                        "Provider '{}' tidak ditemukan di models.json.",
                        target
                    ));
                }
                Ok(warnings)
            }
        }
    }
}
