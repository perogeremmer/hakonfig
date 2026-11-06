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

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::engine::testutil::sandbox;

    #[test]
    fn override_root_mengalihkan_semua_direktori_harness() {
        let (_lock, root) = sandbox("paths-override");
        assert_eq!(override_root().as_deref(), Some(root.as_path()));
        assert_eq!(pi_dir(), root.join("pi"));
        assert_eq!(omp_dir(), root.join("omp"));
        assert_eq!(hermes_dir(), root.join("hermes"));
        assert_eq!(opencode_config_dir(), root.join("opencode/config"));
        assert_eq!(opencode_data_dir(), root.join("opencode/data"));
    }

    #[test]
    fn opencode_memilih_ekstensi_yang_benar_benar_ada() {
        let (_lock, root) = sandbox("paths-opencode-ext");
        let dir = root.join("opencode/config");

        // belum ada file → default .json
        assert_eq!(opencode_config_file(&dir).file_name().unwrap(), "opencode.json");

        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("opencode.jsonc"), "{}").unwrap();
        assert_eq!(
            opencode_config_file(&dir).file_name().unwrap(),
            "opencode.jsonc",
            "jsonc harus didahulukan bila ada"
        );
    }
}
