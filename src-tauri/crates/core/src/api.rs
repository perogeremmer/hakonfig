//! Permukaan API Hakonfig tanpa Tauri.
//!
//! Semua fungsi di sini bebas UI: dipakai aplikasi desktop lewat pembungkus
//! `#[tauri::command]`, dan akan dipakai CLI serta server HTTP apa adanya.
//!
//! Konvensi error: `Result<T, String>` agar bisa langsung dikirim ke frontend
//! maupun dicetak CLI. Gunakan [`err`] untuk mengubah `anyhow::Error`.
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use serde_json::Value;

use crate::engine::adapters::shared::ProviderRef;
use crate::engine::adapters::{self, Adapter};
use crate::engine::backup;
use crate::engine::catalog;
use crate::engine::doc::{Doc, Docs};
use crate::engine::model::{
    ApplyResult, BackupInfo, CatalogTarget, EditOp, FileDiff, HarnessInfo, HarnessSnapshot, Notice,
    PreviewResult, ProviderPreset, TestResult,
};
use crate::engine::{preset, version, write};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

pub fn err(error: anyhow::Error) -> String {
    format!("{:#}", error)
}

fn harness(id: &str) -> Result<Box<dyn Adapter>> {
    adapters::get(id)
}

// ---------------------------------------------------------------- discovery

pub fn list_harnesses() -> Vec<HarnessInfo> {
    adapters::all()
        .iter()
        .map(|adapter| adapter.info())
        .collect()
}

pub fn list_presets() -> Vec<ProviderPreset> {
    preset::all()
}

pub fn read_harness(harness_id: String) -> Result<HarnessSnapshot, String> {
    (|| -> Result<HarnessSnapshot> {
        let adapter = harness(&harness_id)?;
        let docs = Docs::load(&adapter.specs())?;
        adapter.snapshot(&docs)
    })()
    .map_err(err)
}

// ---------------------------------------------------------------- editing

/// Terapkan operasi di memori lalu hasilkan diff. Tidak menyentuh disk.
fn simulate(harness_id: &str, op: &EditOp) -> Result<(Box<dyn Adapter>, Docs, Vec<String>)> {
    let adapter = harness(harness_id)?;
    let mut docs = Docs::load(&adapter.specs())?;
    let mut warnings = adapter.build(&mut docs, op)?;

    for doc in docs.comment_files() {
        let message = format!(
            "{} contains comments that will be lost after writing.",
            doc.path.display()
        );
        if !warnings.contains(&message) {
            warnings.push(message);
        }
    }
    Ok((adapter, docs, warnings))
}

pub fn preview_edit(harness_id: String, op: EditOp) -> Result<PreviewResult, String> {
    (|| -> Result<PreviewResult> {
        let (_adapter, docs, mut warnings) = simulate(&harness_id, &op)?;
        let mut files = Vec::new();
        let mut will_create = Vec::new();
        let mut will_modify = Vec::new();
        let mut redacted_any = false;

        for doc in docs.iter() {
            let changed = doc.is_changed();
            if changed {
                if doc.exists {
                    will_modify.push(doc.path.display().to_string());
                } else {
                    will_create.push(doc.path.display().to_string());
                }
            }
            if changed && doc.sensitive {
                redacted_any = true;
            }
            files.push(FileDiff {
                path: doc.path.display().to_string(),
                role: doc.role.clone(),
                sensitive: doc.sensitive,
                before: if changed {
                    Some(doc.before_redacted())
                } else {
                    None
                },
                after: if changed {
                    Some(doc.render_redacted())
                } else {
                    None
                },
                changed,
            });
        }

        if will_create.is_empty() && will_modify.is_empty() {
            warnings.push("No changes to write.".to_string());
        }
        if redacted_any {
            warnings
                .push("Secret values (API key, token) are redacted in this preview.".to_string());
        }

        Ok(PreviewResult {
            files,
            warnings,
            will_create,
            will_modify,
        })
    })()
    .map_err(err)
}

pub fn apply_edit(harness_id: String, op: EditOp) -> Result<ApplyResult, String> {
    (|| -> Result<ApplyResult> {
        let (adapter, docs, warnings) = simulate(&harness_id, &op)?;
        let touched = op_provider(&op).into_iter().collect::<Vec<_>>();
        commit(&*adapter, &docs, warnings, &touched)
    })()
    .map_err(err)
}

/// Provider yang disentuh satu `EditOp`, untuk memilih notice yang relevan.
fn op_provider(op: &EditOp) -> Option<ProviderRef<'_>> {
    match op {
        EditOp::Upsert(draft) => Some(ProviderRef::new(
            draft.id.trim(),
            Some(draft.base_url.trim()),
        )),
        EditOp::Remove { provider_id }
        | EditOp::SetEnabled { provider_id, .. }
        | EditOp::SetDefault { provider_id, .. }
        | EditOp::SetKey { provider_id, .. }
        | EditOp::SetModels { provider_id, .. } => Some(ProviderRef::new(provider_id.trim(), None)),
    }
}

/// Tulis dokumen yang berubah, lalu kumpulkan notice untuk provider yang
/// disentuh. Dipakai `apply_edit` dan `fix_notice` supaya jalur tulis, backup,
/// dan peringatannya tidak bisa berbeda.
fn commit(
    adapter: &dyn Adapter,
    docs: &Docs,
    mut warnings: Vec<String>,
    touched: &[ProviderRef<'_>],
) -> Result<ApplyResult> {
    let notices = adapter.notices(docs, touched);
    for notice in &notices {
        if !warnings.contains(&notice.message) {
            warnings.push(notice.message.clone());
        }
    }

    let changed: Vec<&Doc> = docs.changed();
    if changed.is_empty() {
        return Ok(ApplyResult {
            files_written: Vec::new(),
            backup_dir: None,
            warnings,
            notices,
        });
    }

    let backup_dir = backup::create(adapter.id(), &adapter.backup_base(), &changed)?;
    let mut files_written = Vec::new();
    for doc in &changed {
        write::atomic_write(&doc.path, &doc.render(), doc.sensitive)?;
        files_written.push(doc.path.display().to_string());
    }

    // Backup lama dipangkas otomatis; beri tahu kalau ada yang dibuang.
    let pruned = backup::prune(&adapter.backup_base(), backup::MAX_BACKUPS);
    if pruned > 0 {
        warnings.push(format!(
            "{} old backups removed. At most {} backups are kept per harness.",
            pruned,
            backup::MAX_BACKUPS
        ));
    }

    Ok(ApplyResult {
        files_written,
        backup_dir: backup_dir.map(|path| path.display().to_string()),
        warnings,
        notices,
    })
}

// ---------------------------------------------------------------- notices

/// Temuan yang membuat config yang sudah ditulis tetap tidak terlihat harness.
///
/// `harness_id = None` memeriksa semua harness; harness yang config-nya tidak
/// bisa dibaca tidak menggagalkan pemeriksaan harness lain.
pub fn list_notices(harness_id: Option<String>) -> Result<Vec<Notice>, String> {
    let collect = || -> Result<Vec<Notice>> {
        let ids: Vec<String> = match harness_id {
            Some(id) => vec![id],
            None => adapters::all()
                .iter()
                .map(|adapter| adapter.id().to_string())
                .collect(),
        };

        let mut notices = Vec::new();
        for id in ids {
            let adapter = match harness(&id) {
                Ok(adapter) => adapter,
                Err(error) => {
                    notices.push(Notice {
                        code: "harness.unknown".to_string(),
                        harness: id,
                        provider: String::new(),
                        message: format!("{:#}", error),
                        fix: None,
                    });
                    continue;
                }
            };

            let docs = match Docs::load(&adapter.specs()) {
                Ok(docs) => docs,
                Err(error) => {
                    notices.push(Notice {
                        code: "harness.unreadable".to_string(),
                        harness: adapter.id().to_string(),
                        provider: String::new(),
                        message: format!("config could not be read: {:#}", error),
                        fix: None,
                    });
                    continue;
                }
            };

            let snapshot = match adapter.snapshot(&docs) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    notices.push(Notice {
                        code: "harness.unreadable".to_string(),
                        harness: adapter.id().to_string(),
                        provider: String::new(),
                        message: format!("config could not be read: {:#}", error),
                        fix: None,
                    });
                    continue;
                }
            };

            let providers: Vec<ProviderRef<'_>> = snapshot
                .providers
                .iter()
                .map(|entry| ProviderRef::new(&entry.id, entry.base_url.as_deref()))
                .collect();
            notices.extend(adapter.notices(&docs, &providers));
        }
        Ok(notices)
    };
    collect().map_err(err)
}

/// Terapkan perbaikan otomatis untuk satu notice, dengan backup seperti edit
/// biasa. Notice yang tidak punya `fix` akan ditolak adapter.
pub fn fix_notice(
    harness_id: String,
    code: String,
    provider: String,
) -> Result<ApplyResult, String> {
    (|| -> Result<ApplyResult> {
        let adapter = harness(&harness_id)?;
        let mut docs = Docs::load(&adapter.specs())?;

        let base_url = adapter.snapshot(&docs).ok().and_then(|snapshot| {
            snapshot
                .providers
                .into_iter()
                .find(|entry| entry.id == provider)
                .and_then(|entry| entry.base_url)
        });
        let reference = ProviderRef::new(&provider, base_url.as_deref());
        let notice = adapter
            .notices(&docs, std::slice::from_ref(&reference))
            .into_iter()
            .find(|notice| notice.code == code && notice.provider == provider)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no '{}' notice for provider '{}' on {} (already fixed?)",
                    code,
                    provider,
                    adapter.id()
                )
            })?;

        if notice.fix.is_none() {
            bail!("'{}' has no automatic fix", code);
        }

        let summary = adapter
            .fix_notice(&mut docs, &notice)?
            .ok_or_else(|| anyhow::anyhow!("nothing to change for '{}' on '{}'", code, provider))?;

        let result = commit(
            &*adapter,
            &docs,
            vec![summary],
            std::slice::from_ref(&reference),
        )?;
        Ok(result)
    })()
    .map_err(err)
}

// ---------------------------------------------------------------- katalog

/// Cek katalog jarak jauh yang di-refresh harness, satu permintaan per provider.
///
/// Ini satu-satunya bagian `doctor` yang butuh jaringan, karena kegagalan
/// refresh tidak meninggalkan jejak di file config: harness diam-diam memakai
/// katalog bawaan dan menampilkan "showing cached models". Permintaannya
/// meniru pi (batas 4 detik per percobaan, dua retry) supaya hasilnya
/// mencerminkan yang dialami harness, bukan kondisi jaringan yang berbeda.
pub fn probe_catalogs(harness_id: Option<String>) -> Result<Vec<Notice>, String> {
    let ids: Vec<String> = match harness_id {
        Some(id) => vec![id],
        None => adapters::all()
            .iter()
            .map(|adapter| adapter.id().to_string())
            .collect(),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to start a runtime: {error}"))?;

    runtime.block_on(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("hakonfig/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| format!("failed to build the HTTP client: {error}"))?;

        let mut notices = Vec::new();
        for id in ids {
            let adapter = match harness(&id) {
                Ok(adapter) => adapter,
                Err(_) => continue,
            };
            let Ok(docs) = Docs::load(&adapter.specs()) else {
                continue;
            };
            for target in adapter.catalog_targets(&docs) {
                if let Some(notice) = probe_one(&client, adapter.id(), &target).await {
                    notices.push(notice);
                }
            }
        }
        Ok(notices)
    })
}

async fn probe_one(
    client: &reqwest::Client,
    harness: &str,
    target: &CatalogTarget,
) -> Option<Notice> {
    let mut last_error = String::new();
    for attempt in 0..3 {
        const RETRYABLE: [u16; 7] = [408, 425, 429, 500, 502, 503, 504];
        let response = match client
            .get(&target.url)
            .header("accept", "application/json")
            .timeout(Duration::from_secs(4))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_error = if error.is_timeout() {
                    "timed out after 4s".to_string()
                } else {
                    format!("{error}")
                };
                continue;
            }
        };

        let status = response.status().as_u16();
        if status < 400 || status == 304 {
            return None;
        }
        let body = response.text().await.unwrap_or_default();
        last_error = format!(
            "HTTP {status}{}",
            if body.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", body.chars().take(160).collect::<String>().trim())
            }
        );
        if !RETRYABLE.contains(&status) || attempt == 2 {
            break;
        }
    }

    Some(Notice {
        code: "catalog.refresh".to_string(),
        harness: harness.to_string(),
        provider: target.provider.clone(),
        message: format!(
            "the remote catalog request for '{}' failed ({}). The harness keeps using the catalog bundled with the installed version + the last cached copy, which is what shows up as a refresh warning.",
            target.provider, last_error
        ),
        fix: None,
    })
}

// ---------------------------------------------------------------- backup

pub fn list_backups(harness_id: String) -> Result<Vec<BackupInfo>, String> {
    (|| -> Result<Vec<BackupInfo>> {
        let adapter = harness(&harness_id)?;
        Ok(backup::list(&adapter.backup_base()))
    })()
    .map_err(err)
}

pub fn restore_backup(harness_id: String, backup_id: String) -> Result<ApplyResult, String> {
    (|| -> Result<ApplyResult> {
        let adapter = harness(&harness_id)?;
        let base = adapter.backup_base();
        let files = backup::restore(&base, &backup_id)?;
        if files.is_empty() {
            bail!("backup '{}' contains no restorable files", backup_id);
        }

        // Snapshot keadaan sekarang supaya restore juga bisa dibatalkan.
        let docs = Docs::load(&adapter.specs())?;
        let existing: Vec<&Doc> = docs.iter().filter(|doc| doc.exists).collect();
        let safety = backup::create(adapter.id(), &base, &existing)?;

        let mut files_written = Vec::new();
        for (path, contents, sensitive) in files {
            write::atomic_write(&path, &contents, sensitive)?;
            files_written.push(path.display().to_string());
        }

        Ok(ApplyResult {
            files_written,
            backup_dir: safety.map(|path| path.display().to_string()),
            warnings: vec![format!("Restored from backup {}.", backup_id)],
            notices: Vec::new(),
        })
    })()
    .map_err(err)
}

// ---------------------------------------------------------------- koneksi

/// Key untuk request keluar: pakai yang diketik pengguna, atau baca yang
/// tersimpan di config harness lewat adapter.
///
/// Ini satu-satunya tempat di mana key mentah keluar dari file config, dan
/// hasilnya **tidak pernah** dikirim ke UI - hanya dipakai sebagai header.
pub fn request_key(
    typed: &str,
    harness_id: Option<&str>,
    provider_id: Option<&str>,
) -> Result<String> {
    let typed = typed.trim();
    if !typed.is_empty() {
        return Ok(resolve_env_key(typed));
    }

    let (Some(harness_id), Some(provider_id)) = (harness_id, provider_id) else {
        return Ok(String::new());
    };
    if harness_id.trim().is_empty() || provider_id.trim().is_empty() {
        return Ok(String::new());
    }

    let adapter = harness(harness_id)?;
    let docs = Docs::load(&adapter.specs())?;
    let stored = adapter.stored_key(&docs, provider_id).unwrap_or_default();
    Ok(resolve_env_key(&stored))
}

/// API key yang **sudah tersimpan** untuk sebuah provider, tanpa perlu
/// mengetiknya ulang. Kosong berarti tidak ada key tersimpan (misalnya
/// provider yang memakai environment variable).
pub fn stored_key(harness_id: String, provider_id: String) -> Result<String, String> {
    let adapter = harness(&harness_id).map_err(err)?;
    let docs = Docs::load(&adapter.specs()).map_err(err)?;
    Ok(adapter
        .stored_key(&docs, &provider_id)
        .unwrap_or_default()
        .trim()
        .to_string())
}

pub async fn test_connection(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<TestResult, String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL is empty.".to_string());
    }
    let resolved = request_key(&key, harness_id.as_deref(), provider_id.as_deref()).map_err(err)?;
    let url = format!("{}/models", base);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("hakonfig/0.1")
        .build()
    {
        Ok(client) => client,
        Err(error) => return Err(format!("failed to build HTTP client: {}", error)),
    };

    let mut request = client.get(&url).header("Accept", "application/json");
    if !resolved.is_empty() {
        request = request.bearer_auth(&resolved);
        if api == "google-generative-ai" {
            request = request.header("x-goog-api-key", &resolved);
        }
    }

    let started = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(TestResult {
                ok: false,
                status: None,
                message: format!("Could not reach {}. {}", url, error),
                latency_ms: Some(started.elapsed().as_millis() as u64),
                models_sample: Vec::new(),
            })
        }
    };

    let latency = started.elapsed().as_millis() as u64;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    let models_sample = extract_model_ids(&body);

    let message = if status.is_success() {
        format!("Connection ok (HTTP {}).", status.as_u16())
    } else if status.as_u16() == 401 || status.as_u16() == 403 {
        format!(
            "API key rejected (HTTP {}). Check the key again.",
            status.as_u16()
        )
    } else if status.as_u16() == 404 {
        format!(
            "Endpoint {} not found (404). The base URL may be wrong; OpenAI-compatible endpoints usually end with /v1.",
            url
        )
    } else {
        let snippet: String = body.chars().take(240).collect();
        format!(
            "Server returned HTTP {}. {}",
            status.as_u16(),
            snippet.trim()
        )
    };

    Ok(TestResult {
        ok: status.is_success(),
        status: Some(status.as_u16()),
        message,
        latency_ms: Some(latency),
        models_sample,
    })
}

fn extract_model_ids(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    for key in ["data", "models"] {
        if let Some(items) = value.get(key).and_then(Value::as_array) {
            for item in items {
                let candidate = item
                    .get("id")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("name").and_then(Value::as_str));
                if let Some(id) = candidate {
                    let cleaned = id.trim_start_matches("models/").to_string();
                    if !ids.contains(&cleaned) {
                        ids.push(cleaned);
                    }
                }
            }
        }
        if !ids.is_empty() {
            break;
        }
    }
    ids.truncate(12);
    ids
}

// ---------------------------------------------------------------- katalog

pub fn list_harness_catalog() -> Vec<serde_json::Value> {
    catalog::all()
        .into_iter()
        .map(|entry| {
            let install = entry.install_command();
            let uninstall = entry.uninstall_command();
            let mut value = serde_json::to_value(&entry).unwrap_or_default();
            if let Some(object) = value.as_object_mut() {
                object.insert("installCommand".to_string(), serde_json::json!(install));
                object.insert("uninstallCommand".to_string(), serde_json::json!(uninstall));
            }
            value
        })
        .collect()
}

/// Versi harness terpasang (blocking). Pembungkus async memakai
/// `spawn_blocking` di crate pemanggil.
pub fn harness_version(harness_id: &str) -> Option<String> {
    version::detect(harness_id)
}

// ---------------------------------------------------------------- sync model

/// `$VAR` / `${VAR}` pada config berarti "ambil dari environment".
/// Backend yang me-resolve - key asli tidak pernah dikirim ke UI.
fn resolve_env_key(raw: &str) -> String {
    let trimmed = raw.trim();
    let name = trimmed
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .or_else(|| trimmed.strip_prefix('$'));
    match name {
        Some(name) if !name.is_empty() => std::env::var(name).unwrap_or_default(),
        _ => trimmed.to_string(),
    }
}

/// Ambil daftar model langsung dari endpoint provider (`GET /models`).
pub async fn fetch_models(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<Vec<ModelOption>, String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL is empty.".to_string());
    }
    let resolved = request_key(&key, harness_id.as_deref(), provider_id.as_deref()).map_err(err)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("hakonfig/0.1")
        .build()
        .map_err(|error| format!("failed to build HTTP client: {}", error))?;

    let mut request = client
        .get(format!("{}/models", base))
        .header("Accept", "application/json");
    if !resolved.is_empty() {
        request = request.bearer_auth(&resolved);
        if api == "google-generative-ai" {
            request = request.header("x-goog-api-key", &resolved);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("could not reach {}. {}", base, error))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "endpoint returned HTTP {}. {}",
            status.as_u16(),
            body.chars().take(200).collect::<String>().trim()
        ));
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|_| "endpoint response is not recognizable JSON.".to_string())?;

    let mut models: Vec<ModelOption> = Vec::new();
    for key_name in ["data", "models"] {
        let Some(items) = value.get(key_name).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str));
            let Some(id) = id else { continue };
            let id = id.trim().trim_start_matches("models/").to_string();
            if id.is_empty() || models.iter().any(|model| model.id == id) {
                continue;
            }
            models.push(ModelOption {
                id,
                context_window: pick_context(item),
            });
        }
        if !models.is_empty() {
            break;
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

/// Context window kadang ada di endpoint, dengan nama field yang berbeda-beda.
fn pick_context(item: &Value) -> Option<u64> {
    let candidates = [
        "context_length",
        "context_window",
        "max_context_length",
        "input_token_limit",
        "max_model_len",
    ];
    for field in candidates {
        if let Some(value) = item.get(field).and_then(Value::as_u64) {
            return Some(value);
        }
    }
    item.get("limit")
        .and_then(|limit| limit.get("context"))
        .and_then(Value::as_u64)
}
