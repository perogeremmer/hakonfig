//! Tauri commands - pembungkus tipis di atas `hakonfig_core::api`.
//!
//! Semua logika nyata tinggal di `hakonfig-core` supaya CLI dan server HTTP
//! memakai jalur kode yang sama. File ini hanya boleh berisi hal yang memang
//! butuh Tauri: `#[tauri::command]`, `AppHandle`, dan pembuka aplikasi OS.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hakonfig_core::api as core_api;
use hakonfig_core::engine::catalog;
use hakonfig_core::engine::model::{
    ApplyResult, BackupInfo, EditOp, HarnessInfo, HarnessSnapshot, PreviewResult, ProviderPreset,
    TestResult,
};
use hakonfig_core::engine::version;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub ok: bool,
    pub command: String,
    pub output: String,
}

// ---------------------------------------------------------------- discovery

#[tauri::command]
pub fn list_harnesses() -> Vec<HarnessInfo> {
    core_api::list_harnesses()
}

/// Apakah variabel environment ini ada di mesin ini (proses atau shell login).
#[tauri::command]
pub fn env_presence(names: Vec<String>) -> std::collections::BTreeMap<String, bool> {
    core_api::env_presence(names)
}

/// Dugaan dukungan thinking untuk model yang baru di-sync dari endpoint.
#[tauri::command]
pub fn reasoning_hints(
    provider_id: String,
    model_ids: Vec<String>,
) -> Result<Vec<hakonfig_core::engine::reasoning::ReasoningHint>, String> {
    core_api::reasoning_hints(provider_id, model_ids)
}

/// Temuan yang membuat config tidak terlihat harness (mis. provider yang masih
/// punya kredensial padahal sudah dinonaktifkan).
///
/// GUI membutuhkannya di luar alur `apply_edit`: penjelasan seperti itu harus
/// terbaca saat membuka harness, bukan hanya setelah menekan tombol.
#[tauri::command]
pub fn list_notices(
    harness_id: Option<String>,
) -> Result<Vec<hakonfig_core::engine::model::Notice>, String> {
    core_api::list_notices(harness_id)
}

/// Home user lain (ADE / VPS) yang punya harness terpasang.
///
/// GUI tidak mengelola home itu (file milik user lain), tetapi harus
/// menyebutkannya: kalau tidak, pengguna mengira harness-nya hilang padahal
/// hanya berada di home agent.
#[tauri::command]
pub fn list_targets() -> Vec<core_api::TargetInfo> {
    core_api::list_targets()
}

#[tauri::command]
pub fn list_presets() -> Vec<ProviderPreset> {
    core_api::list_presets()
}

#[tauri::command]
pub fn read_harness(harness_id: String) -> Result<HarnessSnapshot, String> {
    core_api::read_harness(harness_id)
}

// ---------------------------------------------------------------- editing

#[tauri::command]
pub fn preview_edit(harness_id: String, op: EditOp) -> Result<PreviewResult, String> {
    core_api::preview_edit(harness_id, op)
}

#[tauri::command]
pub fn apply_edit(harness_id: String, op: EditOp) -> Result<ApplyResult, String> {
    core_api::apply_edit(harness_id, op)
}

/// Terapkan perbaikan otomatis untuk notice yang membuat config tertulis tidak
/// terlihat harness (mis. scope `enabledModels` di pi).
#[tauri::command]
pub fn fix_notice(
    harness_id: String,
    code: String,
    provider: String,
) -> Result<ApplyResult, String> {
    core_api::fix_notice(harness_id, code, provider)
}

// ---------------------------------------------------------------- backup

#[tauri::command]
pub fn list_backups(harness_id: String) -> Result<Vec<BackupInfo>, String> {
    core_api::list_backups(harness_id)
}

#[tauri::command]
pub fn restore_backup(harness_id: String, backup_id: String) -> Result<ApplyResult, String> {
    core_api::restore_backup(harness_id, backup_id)
}

// ---------------------------------------------------------------- koneksi

#[tauri::command]
pub async fn test_connection(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<TestResult, String> {
    core_api::test_connection(base_url, api, key, harness_id, provider_id).await
}

// ---------------------------------------------------------------- katalog

#[tauri::command]
pub fn list_harness_catalog() -> Vec<serde_json::Value> {
    core_api::list_harness_catalog()
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
        .ok_or_else(|| format!("harness '{}' is not in the catalog", harness_id))?;
    let command = entry
        .install_command()
        .ok_or_else(|| format!("No automatic install command for {} yet.", entry.name))?;
    run_package(app, harness_id, command).await
}

#[tauri::command]
pub async fn uninstall_harness(
    app: tauri::AppHandle,
    harness_id: String,
) -> Result<InstallResult, String> {
    let entry = catalog::get(&harness_id)
        .ok_or_else(|| format!("harness '{}' is not in the catalog", harness_id))?;
    let command = entry
        .uninstall_command()
        .ok_or_else(|| format!("No automatic uninstall command for {} yet.", entry.name))?;
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
                output: format!("failed to run command: {}", error),
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
        tail.push_str("\n[timed out: process stopped]");
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
    fn successful_command_yields_ok_and_output() {
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&lines);
        let result = execute_with("test", "echo hakonfig-ok", move |line| {
            sink.lock().unwrap().push(line.to_string());
        });
        assert!(
            lines
                .lock()
                .unwrap()
                .iter()
                .any(|l| l.contains("hakonfig-ok")),
            "streaming harus melaporkan baris output"
        );
        assert!(result.ok, "harus sukses: {:?}", result);
        assert!(result.output.contains("hakonfig-ok"));
        assert_eq!(result.command, "echo hakonfig-ok");
    }

    #[test]
    fn failed_command_is_not_reported_as_ok() {
        let result = execute_with("test", "exit 3", |_| {});
        assert!(!result.ok);
    }

    #[test]
    fn missing_command_does_not_panic() {
        let result = execute_with("test", "perintah-yang-tidak-ada-xyz", |_| {});
        assert!(!result.ok);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn stderr_is_captured_too() {
        let result = execute_with("test", "echo pesan-error 1>&2; exit 1", |_| {});
        assert!(!result.ok);
        assert!(
            result.output.contains("pesan-error"),
            "output: {:?}",
            result.output
        );
    }
}

// ---------------------------------------------------------------- utilitas

/// Buka URL dokumentasi di browser default pengguna.
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("only http(s) URLs can be opened".to_string());
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
        .map_err(|error| format!("could not open the browser: {}", error))?;

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
        .map_err(|error| format!("failed to open {}: {}", directory.display(), error))
}

// ---------------------------------------------------------------- sync model

#[tauri::command]
pub async fn fetch_models(
    base_url: String,
    api: String,
    key: String,
    harness_id: Option<String>,
    provider_id: Option<String>,
) -> Result<Vec<core_api::ModelOption>, String> {
    core_api::fetch_models(base_url, api, key, harness_id, provider_id).await
}
