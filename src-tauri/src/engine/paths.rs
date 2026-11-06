//! Resolusi path konfigurasi lintas OS.
//!
//! Semua path OS-specific terkurung di modul ini — adapter tidak pernah
//! menyusun path sendiri.
use std::env;
use std::path::PathBuf;

/// Override root untuk pengujian/portable mode.
///
/// Bila `REKONFIG_ROOT` diset, seluruh direktori harness di-resolve di bawah
/// root tersebut, sehingga pengujian tidak pernah menyentuh config asli.
pub fn override_root() -> Option<PathBuf> {
    env::var_os("REKONFIG_ROOT").map(PathBuf::from)
}

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(fallback)
}

fn first_dir_with(candidates: Vec<PathBuf>, probes: &[&str]) -> PathBuf {
    for dir in &candidates {
        if probes.iter().any(|f| dir.join(f).exists()) {
            return dir.clone();
        }
    }
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| home().join(".config"))
}

/// `~/.pi`
pub fn pi_dir() -> PathBuf {
    override_root()
        .map(|r| r.join("pi"))
        .unwrap_or_else(|| home().join(".pi"))
}

/// `~/.omp` (oh-my-pi)
pub fn omp_dir() -> PathBuf {
    override_root()
        .map(|r| r.join("omp"))
        .unwrap_or_else(|| home().join(".omp"))
}

/// `~/.hermes`
pub fn hermes_dir() -> PathBuf {
    override_root()
        .map(|r| r.join("hermes"))
        .unwrap_or_else(|| home().join(".hermes"))
}

/// Direktori config OpenCode (berisi `opencode.json`).
pub fn opencode_config_dir() -> PathBuf {
    if let Some(root) = override_root() {
        return root.join("opencode/config");
    }
    let mut candidates = Vec::new();
    if let Some(x) = env::var_os("XDG_CONFIG_HOME") {
        candidates.push(PathBuf::from(x).join("opencode"));
    }
    candidates.push(home().join(".config/opencode"));
    if cfg!(target_os = "macos") {
        candidates.push(home().join("Library/Application Support/opencode"));
    }
    if cfg!(target_os = "windows") {
        if let Some(a) = env::var_os("APPDATA") {
            candidates.push(PathBuf::from(a).join("opencode"));
        }
    }
    first_dir_with(candidates, &["opencode.json", "opencode.jsonc"])
}

/// Direktori data OpenCode (berisi `auth.json`).
pub fn opencode_data_dir() -> PathBuf {
    if let Some(root) = override_root() {
        return root.join("opencode/data");
    }
    let mut candidates = Vec::new();
    candidates.push(xdg("XDG_DATA_HOME", home().join(".local/share")).join("opencode"));
    if cfg!(target_os = "macos") {
        candidates.push(home().join("Library/Application Support/opencode"));
    }
    if cfg!(target_os = "windows") {
        if let Some(a) = env::var_os("APPDATA") {
            candidates.push(PathBuf::from(a).join("opencode"));
        }
    }
    first_dir_with(candidates, &["auth.json"])
}

/// Nama file config OpenCode yang benar-benar dipakai (`.json` atau `.jsonc`).
pub fn opencode_config_file(dir: &PathBuf) -> PathBuf {
    let jsonc = dir.join("opencode.jsonc");
    if jsonc.exists() {
        return jsonc;
    }
    dir.join("opencode.json")
}
