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

    /// Seluruh file yang relevan untuk harness ini (urutan menentukan tampilan).
    fn specs(&self) -> Vec<FileSpec>;

    /// Direktori dasar tempat backup disimpan.
    fn backup_base(&self) -> PathBuf;

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot>;

    /// Mutasi dokumen di memori. Tidak menyentuh disk sama sekali.
    fn build(&self, docs: &mut Docs, op: &EditOp) -> Result<Vec<String>>;

    fn info(&self) -> HarnessInfo {
        let files: Vec<ConfigFileInfo> = self
            .specs()
            .into_iter()
            .map(|spec| ConfigFileInfo {
                path: spec.path.display().to_string(),
                role: spec.role.to_string(),
                exists: spec.path.exists(),
                sensitive: spec.sensitive,
            })
            .collect();
        let installed = files.iter().any(|f| f.exists);
        HarnessInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            installed,
            experimental: self.experimental(),
            files,
            notes: self.notes().to_string(),
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
