//! CLI Hakonfig - jalur kode yang sama dengan aplikasi desktop, tanpa GUI.
//!
//! Semua perintah di sini memanggil `hakonfig_core::api`, jadi perilaku tulis,
//! backup, dan redaksi identik dengan versi desktop.
//!
//! Aturan yang dipegang sejak awal:
//! * pilihan yang tidak ambigu (hanya satu harness) tidak ditanyakan;
//! * perintah baca tidak pernah menyentuh file;
//! * `--json` untuk skrip, tabel manusia sebagai default.
use std::process::ExitCode;

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use hakonfig_core::api;
use hakonfig_core::engine::model::HarnessInfo;

#[derive(Parser)]
#[command(
    name = "hakonfig",
    version,
    about = "Kelola konfigurasi provider AI coding harness",
    disable_help_subcommand = true
)]
struct Cli {
    /// Root alternatif untuk file konfigurasi harness.
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<String>,

    /// Cetak JSON, untuk dipakai skrip.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Harness yang terdeteksi di mesin ini.
    List,
    /// Isi konfigurasi satu harness.
    Show {
        /// id harness (mis. pi, opencode). Opsional bila hanya ada satu.
        harness: Option<String>,
    },
    /// Cari preset provider siap pakai.
    Providers {
        /// Filter berdasarkan id atau nama.
        #[arg(long, short)]
        search: Option<String>,
        /// Hanya provider berbadge free atau local.
        #[arg(long)]
        free: bool,
    },
    /// Path file konfigurasi yang dipakai harness.
    Paths {
        /// id harness. Opsional bila hanya ada satu yang terpasang.
        harness: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // `engine::paths` membaca HAKONFIG_ROOT, jadi --root cukup diteruskan
    // lewat environment: satu pintu, tidak ada jalur kedua yang bisa menyimpang.
    if let Some(root) = &cli.root {
        std::env::set_var("HAKONFIG_ROOT", root);
    }

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hakonfig: {:#}", error);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Command::List) => list(cli.json),
        Some(Command::Show { harness }) => show(harness.as_deref(), cli.json),
        Some(Command::Providers { search, free }) => {
            providers(search.as_deref(), *free, cli.json)
        }
        Some(Command::Paths { harness }) => paths(harness.as_deref(), cli.json),
        None => list(cli.json),
    }
}

// ------------------------------------------------------------------ perintah

fn list(json: bool) -> Result<()> {
    let harnesses = api::list_harnesses();

    if json {
        return print_json(&harnesses);
    }

    let rows: Vec<Vec<String>> = harnesses
        .iter()
        .map(|entry| {
            vec![
                if entry.installed { "✓" } else { "·" }.to_string(),
                entry.id.clone(),
                entry.name.clone(),
                if entry.installed {
                    api::harness_version(&entry.id).unwrap_or_else(|| "-".into())
                } else {
                    "-".into()
                },
                if entry.installed {
                    entry.provider_count.to_string()
                } else {
                    "-".into()
                },
                entry
                    .read_error
                    .as_ref()
                    .map(|_| "gagal dibaca".to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();

    print_table(&["", "ID", "NAMA", "VERSI", "PROVIDER", "CATATAN"], rows);

    let installed = harnesses.iter().filter(|entry| entry.installed).count();
    println!("\n{installed} terpasang dari {} harness yang didukung.", harnesses.len());
    if installed == 0 {
        println!("Belum ada yang terpasang. Lihat `hakonfig install` (belum tersedia).");
    }
    Ok(())
}

fn show(harness: Option<&str>, json: bool) -> Result<()> {
    let id = resolve_harness(harness, true)?;
    let snapshot = api::read_harness(id.clone()).map_err(|error| anyhow!(error))?;

    if json {
        return print_json(&snapshot);
    }

    println!("{id}");
    for file in &snapshot.files {
        println!(
            "  {} {}  ({})",
            if file.exists { "✓" } else { "·" },
            file.path,
            file.role
        );
    }

    if snapshot.providers.is_empty() {
        println!("\n  belum ada provider.");
    } else {
        println!();
        let rows: Vec<Vec<String>> = snapshot
            .providers
            .iter()
            .map(|provider| {
                vec![
                    if provider.enabled { "" } else { "nonaktif" }.to_string(),
                    provider.id.clone(),
                    provider.base_url.clone().unwrap_or_else(|| "-".into()),
                    provider.models.len().to_string(),
                    if provider.has_key { "ada" } else { "-" }.to_string(),
                ]
            })
            .collect();
        print_table(&["STATUS", "ID", "BASE URL", "MODEL", "KEY"], rows);
    }

    if let Some(provider) = &snapshot.default_provider {
        println!(
            "\ndefault: {}{}",
            provider,
            snapshot
                .default_model
                .as_ref()
                .map(|model| format!("/{model}"))
                .unwrap_or_default()
        );
    }

    for warning in &snapshot.warnings {
        println!("! {warning}");
    }
    Ok(())
}

fn providers(search: Option<&str>, free_only: bool, json: bool) -> Result<()> {
    let needle = search.unwrap_or_default().trim().to_lowercase();
    let presets: Vec<_> = api::list_presets()
        .into_iter()
        .filter(|preset| {
            if free_only
                && !matches!(preset.badge.as_deref(), Some("free") | Some("local"))
            {
                return false;
            }
            if needle.is_empty() {
                return true;
            }
            preset.id.to_lowercase().contains(&needle)
                || preset.name.to_lowercase().contains(&needle)
        })
        .collect();

    if json {
        return print_json(&presets);
    }

    if presets.is_empty() {
        bail!("tidak ada provider yang cocok dengan '{}'", needle);
    }

    let rows: Vec<Vec<String>> = presets
        .iter()
        .map(|preset| {
            vec![
                preset.id.clone(),
                preset.name.clone(),
                preset.models.len().to_string(),
                preset.badge.clone().unwrap_or_default(),
                preset.env_var.clone().unwrap_or_default(),
            ]
        })
        .collect();

    print_table(&["ID", "NAMA", "PRESET MODEL", "BADGE", "ENV"], rows);
    println!("\n{} provider.", presets.len());
    Ok(())
}

fn paths(harness: Option<&str>, json: bool) -> Result<()> {
    let id = resolve_harness(harness, false)?;
    let harnesses = api::list_harnesses();
    let entry = harnesses
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| anyhow!("harness '{id}' tidak dikenal"))?;

    if json {
        return print_json(&entry.files);
    }

    for file in &entry.files {
        println!(
            "{}  {}  ({})",
            if file.exists { "✓" } else { "·" },
            file.path,
            file.role
        );
    }
    Ok(())
}

// ------------------------------------------------------------------ bantuan

/// Pilih harness: sebutan eksplisit menang; kalau kosong dan hanya ada satu
/// kandidat, pakai itu tanpa bertanya; kalau ambigu, sebutkan pilihannya.
fn resolve_harness(wanted: Option<&str>, require_installed: bool) -> Result<String> {
    let harnesses = api::list_harnesses();

    if let Some(id) = wanted {
        let entry = harnesses
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| anyhow!("harness '{id}' tidak dikenal. {}", choices(&harnesses)))?;
        if require_installed && !entry.installed {
            bail!("harness '{id}' belum terpasang atau belum punya config");
        }
        return Ok(entry.id.clone());
    }

    let candidates: Vec<&HarnessInfo> = harnesses
        .iter()
        .filter(|entry| !require_installed || entry.installed)
        .collect();

    match candidates.len() {
        1 => Ok(candidates[0].id.clone()),
        0 => bail!("tidak ada harness terpasang. {}", choices(&harnesses)),
        _ => bail!(
            "ada {} harness; sebutkan salah satu: {}",
            candidates.len(),
            candidates
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn choices(harnesses: &[HarnessInfo]) -> String {
    format!(
        "Pilihan: {}",
        harnesses
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let columns = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.chars().count()).collect();
    for row in &rows {
        for (index, cell) in row.iter().take(columns).enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    let render = |cells: &[String]| {
        let mut line = String::new();
        for (index, cell) in cells.iter().take(columns).enumerate() {
            if index + 1 == columns {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{:<width$}  ", cell, width = widths[index]));
            }
        }
        line.trim_end().to_string()
    };

    let header_cells: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    println!("{}", render(&header_cells));
    for row in rows {
        println!("{}", render(&row));
    }
}
