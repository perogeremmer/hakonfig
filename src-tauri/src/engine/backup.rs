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
    base.join(".hakonfig").join("backups")
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

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::engine::doc::{Doc, FileSpec, Format};
    use crate::engine::testutil::temp_dir;

    fn doc_for(base: &Path, role: &'static str, name: &str, contents: &str) -> Doc {
        let path = base.join(name);
        std::fs::write(&path, contents).unwrap();
        Doc::load(&FileSpec {
            role,
            path,
            format: Format::Json,
            sensitive: false,
        })
        .unwrap()
    }

    #[test]
    fn tanpa_file_tidak_membuat_backup() {
        let base = temp_dir("backup-none");
        let missing = base.join("tidak-ada.json");
        let doc = Doc::load(&FileSpec {
            role: "config",
            path: missing,
            format: Format::Json,
            sensitive: false,
        })
        .unwrap();
        assert!(create("pi", &base, &[&doc]).unwrap().is_none());
        assert!(list(&base).is_empty());
    }

    #[test]
    fn create_lalu_list_lalu_restore() {
        let base = temp_dir("backup-roundtrip");
        let original = r#"{"a":1}"#;
        let doc = doc_for(&base, "models", "models.json", original);

        let dir = create("pi", &base, &[&doc]).unwrap().expect("harus ada backup");
        assert!(dir.join(MANIFEST).exists());
        assert!(dir.join("models-models.json").exists());

        let entries = list(&base);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].files.len(), 1);
        assert_eq!(entries[0].files[0].role, "models");
        assert!(entries[0].files[0].exists);

        // ubah file, lalu pulihkan
        std::fs::write(base.join("models.json"), r#"{"a":2}"#).unwrap();
        let restored = restore(&base, &entries[0].id).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].1, original);

        // manifest mencatat path asli
        assert_eq!(
            restored[0].0,
            std::fs::canonicalize(&base).unwrap().join("models.json")
        );
    }

    #[test]
    fn manifest_rusak_diabaikan_dan_id_asing_ditolak() {
        let base = temp_dir("backup-corrupt");
        let doc = doc_for(&base, "models", "models.json", "{}");
        create("pi", &base, &[&doc]).unwrap();

        // direktori tanpa manifest tidak boleh membuat list panik
        std::fs::create_dir_all(backups_root(&base).join("sampah")).unwrap();
        assert_eq!(list(&base).len(), 1);

        // manifest rusak juga diabaikan
        let broken = backups_root(&base).join("rusak");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join(MANIFEST), "{ bukan json").unwrap();
        assert_eq!(list(&base).len(), 1);

        assert!(restore(&base, "tidak-ada").is_err());
    }

    #[test]
    fn beberapa_backup_diurutkan_terbaru_dulu() {
        let base = temp_dir("backup-order");
        let doc = doc_for(&base, "models", "models.json", "{}");
        create("pi", &base, &[&doc]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(12));
        create("pi", &base, &[&doc]).unwrap();

        let entries = list(&base);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].created_at >= entries[1].created_at);
    }
}
