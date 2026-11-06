//! Backup otomatis sebelum setiap penulisan.
//!
//! Setiap backup adalah satu direktori berisi salinan file + `manifest.json`,
//! sehingga bisa dipulihkan tanpa menebak-nebak.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::doc::Doc;
use super::model::{BackupFile, BackupInfo};
use super::write;

const MANIFEST: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    original_path: String,
    role: String,
    sensitive: bool,
    stored_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    created_at: String,
    harness: String,
    entries: Vec<ManifestEntry>,
}

fn backups_root(base: &Path) -> PathBuf {
    base.join(".rekonfig").join("backups")
}

/// Simpan snapshot file-file yang akan diubah. Mengembalikan direktori backup.
pub fn create(harness: &str, base: &Path, docs: &[&Doc]) -> Result<Option<PathBuf>> {
    let pending: Vec<&&Doc> = docs.iter().filter(|d| d.exists).collect();
    if pending.is_empty() {
        return Ok(None);
    }

    let now = chrono::Local::now();
    let id = now.format("%Y%m%d-%H%M%S-%3f").to_string();
    let dir = backups_root(base).join(&id);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("gagal membuat direktori backup {}", dir.display()))?;

    let mut entries = Vec::new();
    for doc in pending {
        let file_name = doc
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "config".to_string());
        let stored_name = format!("{}-{}", doc.role, file_name);
        let contents = write::read_to_string(&doc.path)?;
        write::atomic_write(&dir.join(&stored_name), &contents, true)?;
        entries.push(ManifestEntry {
            original_path: doc.path.display().to_string(),
            role: doc.role.clone(),
            sensitive: doc.sensitive,
            stored_name: Some(stored_name),
        });
    }

    let manifest = Manifest {
        created_at: now.to_rfc3339(),
        harness: harness.to_string(),
        entries,
    };
    let text = serde_json::to_string_pretty(&manifest)?;
    write::atomic_write(&dir.join(MANIFEST), &text, false)?;
    Ok(Some(dir))
}

fn read_manifest(dir: &Path) -> Option<Manifest> {
    let text = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn list(base: &Path) -> Vec<BackupInfo> {
    let root = backups_root(base);
    let mut result = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(&root) else {
        return result;
    };

    for entry in read_dir.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(manifest) = read_manifest(&dir) else {
            continue;
        };
        let id = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        result.push(BackupInfo {
            id,
            created_at: manifest.created_at,
            dir: dir.display().to_string(),
            files: manifest
                .entries
                .iter()
                .map(|e| BackupFile {
                    path: e.original_path.clone(),
                    role: e.role.clone(),
                    exists: e.stored_name.is_some(),
                })
                .collect(),
        });
    }

    // terbaru dulu
    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    result
}

/// Isi file yang perlu ditulis ulang saat restore: (path asli, isi, sensitif).
pub fn restore(base: &Path, backup_id: &str) -> Result<Vec<(PathBuf, String, bool)>> {
    let dir = backups_root(base).join(backup_id);
    if !dir.is_dir() {
        anyhow::bail!("backup '{}' tidak ditemukan", backup_id);
    }
    let manifest = read_manifest(&dir)
        .ok_or_else(|| anyhow::anyhow!("manifest backup '{}' tidak bisa dibaca", backup_id))?;

    let mut out = Vec::new();
    for entry in manifest.entries {
        let Some(stored) = entry.stored_name else {
            continue;
        };
        let contents = write::read_to_string(&dir.join(stored))?;
        out.push((
            PathBuf::from(entry.original_path),
            contents,
            entry.sensitive,
        ));
    }
    Ok(out)
}
