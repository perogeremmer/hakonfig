//! Deteksi versi harness terpasang (best-effort, tanpa memblokir UI).
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(4);

/// Nama biner diambil dari katalog; fallback ke id harness.
fn binary_for(id: &str) -> Option<String> {
    if let Some(entry) = crate::engine::catalog::get(id) {
        if let Some(binary) = entry.binary.filter(|value| !value.trim().is_empty()) {
            return Some(binary);
        }
    }
    if id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(id.to_string())
    } else {
        None
    }
}

/// Jalankan `<bin> --version` lewat shell login agar PATH dari shell pengguna
/// ikut terbaca (aplikasi GUI sering punya PATH minimal).
///
/// Probe dijalankan dengan env target: `HOME` dari [`paths::target_home`]
/// dan `<home>/.local/bin` di depan `PATH`. Dengan begitu harness milik user
/// lain (mis. agent dengan home sendiri di ADE/VPS, `--home /var/lib/openbot`)
/// ditemukan di tempat yang sama seperti yang dilihat harness-nya.
pub fn detect(id: &str) -> Option<String> {
    use crate::engine::paths::target_home;

    let binary = binary_for(id)?;
    let invocation = format!("{} --version", binary);

    let mut command = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &invocation]);
        cmd
    } else {
        let mut cmd = Command::new("bash");
        cmd.args(["-lc", &invocation]);
        cmd
    };

    let home = target_home();
    let local_bin = home.join(".local/bin");
    let parts: Vec<_> = std::iter::once(local_bin)
        .chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ))
        .collect();
    if let Ok(path) = std::env::join_paths(parts) {
        command.env("PATH", path);
    }
    command.env("HOME", &home);
    command.env("USERPROFILE", &home);

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        }
    }

    let output = child.wait_with_output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    parse_version(&text)
}

/// Ambil baris pertama yang masuk akal dan rapikan jadi `x.y.z`.
fn parse_version(text: &str) -> Option<String> {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // Ambil token pertama yang mengandung angka dan titik (mis. "v0.9.12").
        for token in line.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
            });
            let candidate = cleaned.trim_start_matches('v');
            let digits = candidate.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0 && candidate.contains('.') {
                return Some(candidate.chars().take(24).collect());
            }
        }
    }
    None
}

#[cfg(test)]
mod unit_tests {
    use super::parse_version;

    #[test]
    fn parses_version_from_various_formats() {
        assert_eq!(parse_version("0.9.12").as_deref(), Some("0.9.12"));
        assert_eq!(parse_version("v1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(parse_version("pi 1.5.0\n").as_deref(), Some("1.5.0"));
        assert_eq!(
            parse_version("opencode version: 2.4.1-beta.2").as_deref(),
            Some("2.4.1-beta.2")
        );
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("tidak ada versi"), None);
    }
}
