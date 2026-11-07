//! Adapter per harness: satu-satunya tempat yang tahu format tiap tool.
use anyhow::Result;
use std::path::PathBuf;

use super::doc::{Docs, FileSpec};
use super::model::{ConfigFileInfo, EditOp, HarnessInfo, HarnessSnapshot};

pub mod hermes;
pub mod omp;
pub mod opencode;
pub mod pi;
pub mod shared;

pub trait Adapter {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn notes(&self) -> &'static str {
        ""
    }
    fn experimental(&self) -> bool {
        false
    }

    /// Apakah harness punya daftar provider nonaktif (toggle reversible).
    fn supports_toggle(&self) -> bool {
        false
    }

    /// Seluruh file yang relevan untuk harness ini (urutan menentukan tampilan).
    fn specs(&self) -> Vec<FileSpec>;

    /// Direktori dasar tempat backup disimpan.
    fn backup_base(&self) -> PathBuf;

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot>;

    /// Mutasi dokumen di memori. Tidak menyentuh disk sama sekali.
    fn build(&self, docs: &mut Docs, op: &EditOp) -> Result<Vec<String>>;

    /// API key mentah yang tersimpan untuk `provider_id` di config harness ini.
    ///
    /// Dipakai **hanya di backend** oleh `test_connection` / `fetch_models`:
    /// key tersimpan tidak pernah dikirim ke UI. Nilai yang dikembalikan boleh
    /// berupa referensi environment (`$VAR` / `${VAR}`) - pemanggil yang
    /// me-resolve-nya. Default `None` = harness tidak menyimpan key.
    ///
    /// Harus sepakat dengan `snapshot`: provider yang dilaporkan `has_key`
    /// seharusnya mengembalikan `Some` di sini.
    fn stored_key(&self, docs: &Docs, provider_id: &str) -> Option<String> {
        let _ = (docs, provider_id);
        None
    }

    /// Ringkasan harness: file yang relevan + jumlah provider.
    ///
    /// Jumlah provider dihitung dari snapshot sungguhan supaya halaman awal
    /// bisa menampilkan "N provider terhubung" tanpa panggilan tambahan.
    fn info(&self) -> HarnessInfo {
        let specs = self.specs();
        let files: Vec<ConfigFileInfo> = specs
            .iter()
            .map(|spec| ConfigFileInfo {
                path: spec.path.display().to_string(),
                role: spec.role.to_string(),
                exists: spec.path.exists(),
                sensitive: spec.sensitive,
            })
            .collect();
        let installed = files.iter().any(|f| f.exists);

        let mut provider_count = 0;
        let mut connected_count = 0;
        let mut read_error = None;

        if installed {
            match Docs::load(&specs).and_then(|docs| self.snapshot(&docs)) {
                Ok(snapshot) => {
                    provider_count = snapshot.providers.len();
                    connected_count = snapshot.providers.iter().filter(|p| p.has_key).count();
                }
                Err(error) => read_error = Some(format!("{:#}", error)),
            }
        }

        HarnessInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            installed,
            experimental: self.experimental(),
            files,
            notes: self.notes().to_string(),
            provider_count,
            connected_count,
            read_error,
        }
    }
}

pub fn all() -> Vec<Box<dyn Adapter>> {
    vec![
        Box::new(pi::PiAdapter),
        Box::new(omp::OmpAdapter),
        Box::new(opencode::OpenCodeAdapter),
        Box::new(hermes::HermesAdapter),
    ]
}

pub fn get(id: &str) -> Result<Box<dyn Adapter>> {
    all()
        .into_iter()
        .find(|adapter| adapter.id() == id)
        .ok_or_else(|| anyhow::anyhow!("harness '{}' tidak dikenal", id))
}
