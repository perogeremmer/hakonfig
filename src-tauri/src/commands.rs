//! Tauri commands - satu-satunya jembatan antara UI dan engine.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use serde_json::Value;

use crate::engine::adapters::{self, Adapter};
use crate::engine::catalog;
use crate::engine::version;
use crate::engine::backup;
use crate::engine::doc::{Doc, Docs};
use crate::engine::model::{
    ApplyResult, BackupInfo, EditOp, FileDiff, HarnessInfo, HarnessSnapshot, PreviewResult,
    ProviderPreset, TestResult,
};
use crate::engine::{preset, write};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub ok: bool,
    pub command: String,
    pub output: String,
}

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
        let (adapter, docs, mut warnings) = simulate(&harness_id, &op)?;
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

        // Backup lama dipangkas otomatis; beri tahu kalau ada yang dibuang.
        let pruned = backup::prune(&adapter.backup_base(), backup::MAX_BACKUPS);
        if pruned > 0 {
            warnings.push(format!(
                "{} backup lama dihapus. Maksimal {} backup disimpan per harness.",
                pruned,
                backup::MAX_BACKUPS
            ));
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

/// Key untuk request keluar: pakai yang diketik pengguna, atau baca yang
/// tersimpan di config harness lewat adapter.
///
/// Ini satu-satunya tempat di mana key mentah keluar dari file config, dan
/// hasilnya **tidak pernah** dikirim ke UI - hanya dipakai sebagai header.
pub(crate) fn request_key(
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

#[tauri::command]
pub async fn test_connection(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<TestResult, String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL belum diisi.".to_string());
    }
    let resolved = request_key(&key, harness_id.as_deref(), provider_id.as_deref()).map_err(err)?;
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
            "Endpoint {} tidak ditemukan (404). Base URL mungkin salah; untuk OpenAI-compatible biasanya diakhiri /v1.",
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

/// Buka URL dokumentasi di browser default pengguna.
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("hanya URL http(s) yang boleh dibuka".to_string());
    }

    let mut command = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]);
        cmd
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };

    command
        .arg(trimmed)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("tidak bisa membuka browser: {}", error))?;

    Ok(())
}

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

// ---------------------------------------------------------------- katalog

#[tauri::command]
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

#[tauri::command]
pub async fn harness_version(harness_id: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || version::detect(&harness_id))
        .await
        .ok()
        .flatten()
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageEvent {
    harness_id: String,
    line: String,
}

#[tauri::command]
pub async fn install_harness(
    app: tauri::AppHandle,
    harness_id: String,
) -> Result<InstallResult, String> {
    let entry = catalog::get(&harness_id)
        .ok_or_else(|| format!("harness '{}' tidak ada di katalog", harness_id))?;
    let command = entry
        .install_command()
        .ok_or_else(|| format!("Belum ada perintah install otomatis untuk {}.", entry.name))?;
    run_package(app, harness_id, command).await
}

#[tauri::command]
pub async fn uninstall_harness(
    app: tauri::AppHandle,
    harness_id: String,
) -> Result<InstallResult, String> {
    let entry = catalog::get(&harness_id)
        .ok_or_else(|| format!("harness '{}' tidak ada di katalog", harness_id))?;
    let command = entry
        .uninstall_command()
        .ok_or_else(|| format!("Belum ada perintah uninstall otomatis untuk {}.", entry.name))?;
    run_package(app, harness_id, command).await
}

async fn run_package(
    app: tauri::AppHandle,
    harness_id: String,
    command: String,
) -> Result<InstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || execute_package(&app, &harness_id, &command))
        .await
        .map_err(|error| error.to_string())
}

fn execute_package(app: &tauri::AppHandle, harness_id: &str, command: &str) -> InstallResult {
    use tauri::Emitter;
    execute_with(harness_id, command, |line: &str| {
        let _ = app.emit(
            "package-progress",
            PackageEvent {
                harness_id: harness_id.to_string(),
                line: line.to_string(),
            },
        );
    })
}

/// Inti eksekusi: jalankan perintah, laporkan tiap baris lewat `on_line`.
fn execute_with<F: Fn(&str)>(harness_id: &str, command: &str, on_line: F) -> InstallResult {
    let _ = harness_id;
    use std::io::{BufRead, BufReader};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    const LIMIT: Duration = Duration::from_secs(600);

    // stderr digabung ke stdout agar urutannya masuk akal di log.
    // Di POSIX, dibungkus `{ ...; }` supaya redireksi berlaku untuk seluruh
    // rangkaian perintah, bukan hanya perintah terakhir.
    let full = if cfg!(windows) {
        format!("{} 2>&1", command)
    } else {
        format!("{{ {}; }} 2>&1", command)
    };
    let mut process = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &full]);
        cmd
    } else {
        let mut cmd = Command::new("bash");
        cmd.args(["-lc", &full]);
        cmd
    };
    process
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            return InstallResult {
                ok: false,
                command: command.to_string(),
                output: format!("gagal menjalankan perintah: {}", error),
            }
        }
    };

    let stdout = child.stdout.take();
    let child = Arc::new(Mutex::new(child));
    let finished = Arc::new(AtomicBool::new(false));
    let timed_out = Arc::new(AtomicBool::new(false));

    // watchdog: bunuh proses bila melewati batas waktu
    let watchdog = {
        let child = Arc::clone(&child);
        let finished = Arc::clone(&finished);
        let timed_out = Arc::clone(&timed_out);
        let started = Instant::now();
        std::thread::spawn(move || loop {
            if finished.load(Ordering::Relaxed) {
                return;
            }
            if started.elapsed() > LIMIT {
                timed_out.store(true, Ordering::Relaxed);
                if let Ok(mut child) = child.lock() {
                    let _ = child.kill();
                }
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        })
    };

    let mut output = String::new();
    if let Some(stdout) = stdout {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            on_line(&line);
            if output.len() < 60_000 {
                output.push_str(&line);
                output.push('\n');
            }
        }
    }

    let status = child.lock().ok().and_then(|mut child| child.wait().ok());
    finished.store(true, Ordering::Relaxed);
    let _ = watchdog.join();

    let timed_out = timed_out.load(Ordering::Relaxed);
    let ok = !timed_out && status.map(|s| s.success()).unwrap_or(false);
    let mut tail = output;
    if timed_out {
        tail.push_str("\n[waktu habis: proses dihentikan]");
    }
    let trimmed: String = tail
        .chars()
        .rev()
        .take(4000)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    InstallResult {
        ok,
        command: command.to_string(),
        output: trimmed.trim().to_string(),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::execute_with;

    #[test]
    fn perintah_sukses_menghasilkan_ok_dan_output() {
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&lines);
        let result = execute_with("test", "echo hakonfig-ok", move |line| {
            sink.lock().unwrap().push(line.to_string());
        });
        assert!(
            lines.lock().unwrap().iter().any(|l| l.contains("hakonfig-ok")),
            "streaming harus melaporkan baris output"
        );
        assert!(result.ok, "harus sukses: {:?}", result);
        assert!(result.output.contains("hakonfig-ok"));
        assert_eq!(result.command, "echo hakonfig-ok");
    }

    #[test]
    fn perintah_gagal_tidak_dianggap_sukses() {
        let result = execute_with("test", "exit 3", |_| {});
        assert!(!result.ok);
    }

    #[test]
    fn perintah_tidak_ada_tidak_panik() {
        let result = execute_with("test", "perintah-yang-tidak-ada-xyz", |_| {});
        assert!(!result.ok);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn stderr_ikut_tertangkap() {
        let result = execute_with("test", "echo pesan-error 1>&2; exit 1", |_| {});
        assert!(!result.ok);
        assert!(result.output.contains("pesan-error"), "output: {:?}", result.output);
    }
}

// ---------------------------------------------------------------- sync model

/// Ambil daftar model langsung dari endpoint provider (`GET /models`).
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

#[tauri::command]
pub async fn fetch_models(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<Vec<ModelOption>, String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err("Base URL belum diisi.".to_string());
    }
    let resolved = request_key(&key, harness_id.as_deref(), provider_id.as_deref()).map_err(err)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("hakonfig/0.1")
        .build()
        .map_err(|error| format!("gagal menyiapkan HTTP client: {}", error))?;

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
        .map_err(|error| format!("tidak bisa menghubungi {}. {}", base, error))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "endpoint membalas HTTP {}. {}",
            status.as_u16(),
            body.chars().take(200).collect::<String>().trim()
        ));
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|_| "balasan endpoint bukan JSON yang dikenali.".to_string())?;

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
