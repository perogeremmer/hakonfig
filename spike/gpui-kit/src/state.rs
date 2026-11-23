//! Satu-satunya state yang ditulis spike: daftar harness yang dipilih
//! pengguna lewat "Tambah harness baru".
//!
//! Spike tetap read-only terhadap config harness. File di sini hanya mencatat
//! pilihan UI supaya harness yang belum terpasang tidak hilang saat aplikasi
//! dibuka ulang. Lokasinya sengaja diberi nama sendiri (`hakonfig-spike`),
//! terpisah dari state aplikasi Hakonfig yang sebenarnya.

use std::fs;
use std::path::PathBuf;

fn config_base() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        if dir.is_absolute() {
            return dir;
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".config")
}

fn file() -> PathBuf {
    config_base().join("hakonfig-spike").join("harnesses")
}

/// Id harness yang pernah ditambahkan, urutan kemunculan dipertahankan.
pub fn added() -> Vec<String> {
    let Ok(text) = fs::read_to_string(file()) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = Vec::new();
    for line in text.lines() {
        let id = line.trim();
        if !id.is_empty() && !ids.iter().any(|known| known == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

/// Catat satu pilihan. Gagal menulis bukan alasan untuk menolak aksi di sesi
/// ini; pemanggil tetap menambahkan harness ke `App::added`.
pub fn add(id: &str) {
    let mut ids = added();
    if ids.iter().any(|known| known == id) {
        return;
    }
    ids.push(id.to_string());

    let path = file();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let body = format!("{}\n", ids.join("\n"));
    if let Err(error) = fs::write(&path, body) {
        eprintln!("[spike] gagal menyimpan daftar harness ({}): {error}", path.display());
    }
}
