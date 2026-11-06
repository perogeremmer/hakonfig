//! Tauri commands — satu-satunya jembatan antara UI dan engine.
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use serde_json::Value;

use crate::engine::adapters::{self, Adapter};
use crate::engine::backup;
use crate::engine::doc::{Doc, Docs};
use crate::engine::model::{
    ApplyResult, BackupInfo, EditOp, FileDiff, HarnessInfo, HarnessSnapshot, PreviewResult,
    ProviderPreset, TestResult,
};
use crate::engine::{preset, write};

fn err(error: anyhow::Error) -> String {
    format!("{:#}", error)
}

fn harness(id: &str) -> Result<Box<dyn Adapter>> {
    adapters::get(id)
}

// ---------------------------------------------------------------- discovery

#[tauri::command]
pub fn list_harnesses() -> Vec<HarnessInfo> {
    adapters::all()
        .iter()
        .map(|adapter| adapter.info())
        .collect()
}

#[tauri::command]
pub fn list_presets() -> Vec<ProviderPreset> {
    preset::all()
}

#[tauri::command]
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
            "{} memuat komentar yang akan hilang setelah ditulis.",
            doc.path.display()
        );
        if !warnings.contains(&message) {
            warnings.push(message);
        }
    }
    Ok((adapter, docs, warnings))
}

#[tauri::command]
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
            warnings.push("Tidak ada perubahan yang perlu ditulis.".to_string());
        }
        if redacted_any {
            warnings
                .push("Nilai rahasia (API key, token) disamarkan pada pratinjau ini.".to_string());
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

#[tauri::command]
pub fn apply_edit(harness_id: String, op: EditOp) -> Result<ApplyResult, String> {
    (|| -> Result<ApplyResult> {
        let (adapter, docs, warnings) = simulate(&harness_id, &op)?;
        let changed: Vec<&Doc> = docs.changed();

        if changed.is_empty() {
            return Ok(ApplyResult {
                files_written: Vec::new(),
                backup_dir: None,
                warnings,
            });
        }

        let backup_dir = backup::create(adapter.id(), &adapter.backup_base(), &changed)?;
        let mut files_written = Vec::new();
        for doc in &changed {
            write::atomic_write(&doc.path, &doc.render(), doc.sensitive)?;
            files_written.push(doc.path.display().to_string());
        }

        Ok(ApplyResult {
            files_written,
            backup_dir: backup_dir.map(|path| path.display().to_string()),
            warnings,
        })
    })()
    .map_err(err)
}

// ---------------------------------------------------------------- backup

#[tauri::command]
pub fn list_backups(harness_id: String) -> Result<Vec<BackupInfo>, String> {
    (|| -> Result<Vec<BackupInfo>> {
        let adapter = harness(&harness_id)?;
        Ok(backup::list(&adapter.backup_base()))
    })()
    .map_err(err)
}

#[tauri::command]
pub fn restore_backup(harness_id: String, backup_id: String) -> Result<ApplyResult, String> {
    (|| -> Result<ApplyResult> {
        let adapter = harness(&harness_id)?;
        let base = adapter.backup_base();
        let files = backup::restore(&base, &backup_id)?;
        if files.is_empty() {
            bail!(
                "backup '{}' tidak berisi file yang bisa dipulihkan",
                backup_id
            );
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
            warnings: vec![format!("Dipulihkan dari backup {}.", backup_id)],
        })
    })()
    .map_err(err)
}

// ---------------------------------------------------------------- koneksi

#[tauri::command]
pub async fn test_connection(
    base_url: String,
    api: String,
    key: String,
) -> Result<TestResult, String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL belum diisi.".to_string());
    }
    let url = format!("{}/models", base);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("hakonfig/0.1")
        .build()
    {
        Ok(client) => client,
        Err(error) => return Err(format!("gagal menyiapkan HTTP client: {}", error)),
    };

    let mut request = client.get(&url).header("Accept", "application/json");
    if !key.trim().is_empty() {
        request = request.bearer_auth(key.trim());
        if api == "google-generative-ai" {
            request = request.header("x-goog-api-key", key.trim());
        }
    }

    let started = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(TestResult {
                ok: false,
                status: None,
                message: format!("Tidak bisa menghubungi {}. {}", url, error),
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
        format!("Koneksi berhasil (HTTP {}).", status.as_u16())
    } else if status.as_u16() == 401 || status.as_u16() == 403 {
        format!(
            "API key ditolak (HTTP {}). Periksa kembali key kamu.",
            status.as_u16()
        )
    } else if status.as_u16() == 404 {
        format!(
            "Endpoint {} tidak ditemukan (404). Base URL mungkin salah — untuk OpenAI-compatible biasanya diakhiri /v1.",
            url
        )
    } else {
        let snippet: String = body.chars().take(240).collect();
        format!(
            "Server membalas HTTP {}. {}",
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

// ---------------------------------------------------------------- utilitas

#[tauri::command]
pub fn open_in_file_manager(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    let directory = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or(target)
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&directory);
        cmd
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg(&directory);
        cmd
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(&directory);
        cmd
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("gagal membuka {}: {}", directory.display(), error))
}
