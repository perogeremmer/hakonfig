//! CLI Hakonfig - jalur kode yang sama dengan aplikasi desktop, tanpa GUI.
//!
//! Semua perintah memanggil `hakonfig_core::api`, jadi perilaku tulis, backup,
//! dan redaksi identik dengan versi desktop.
//!
//! Aturan yang dipegang sejak awal:
//! * default English, `--lang id` / `HAKONFIG_LANG=id` untuk Bahasa Indonesia;
//! * ada flag atau `--json` → non-interaktif, tidak pernah bertanya (skrip & server);
//! * tanpa flag di terminal → wizard;
//! * `--test` eksplisit yang gagal menggagalkan perintah (exit non-zero);
//! * pilihan yang tidak ambigu (hanya satu harness) tidak ditanyakan;
//! * perintah baca tidak pernah menyentuh file.
use std::process::ExitCode;

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use hakonfig_core::api;
use hakonfig_core::engine::model::{HarnessInfo, KeyMode};

mod add;
mod draft;
mod i18n;
mod output;

use i18n::{t, Msg};

#[derive(Parser)]
#[command(
    name = "hakonfig",
    version,
    about = "Manage providers for AI coding harnesses",
    disable_help_subcommand = true
)]
struct Cli {
    /// Alternate root for harness config files.
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<String>,

    /// Print JSON, for scripts.
    #[arg(long, global = true)]
    json: bool,

    /// UI language: en (default) or id.
    #[arg(long, global = true, value_name = "LANG")]
    lang: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Harnesses detected on this machine.
    List,
    /// Config contents of one harness.
    Show {
        /// harness id (e.g. pi, opencode). Optional when only one exists.
        harness: Option<String>,
    },
    /// Search built-in provider presets.
    Providers {
        /// Filter by id or name.
        #[arg(long, short)]
        search: Option<String>,
        /// Only providers badged free or local.
        #[arg(long)]
        free: bool,
    },
    /// Config file paths used by a harness.
    Paths {
        /// harness id. Optional when only one exists.
        harness: Option<String>,
    },
    /// Add or change a provider. Runs as a wizard when called without flags.
    Add {
        /// harness id (e.g. pi, opencode).
        #[arg(long)]
        harness: Option<String>,
        /// catalog provider id, or your own id for a custom endpoint.
        #[arg(long)]
        provider: Option<String>,
        /// Display name (required for a custom provider).
        #[arg(long)]
        name: Option<String>,
        /// Base URL (required for a custom provider).
        #[arg(long)]
        base_url: Option<String>,
        /// API type for a custom provider.
        #[arg(long)]
        api: Option<String>,
        /// Read the API key from stdin instead of prompting.
        #[arg(long)]
        key_stdin: bool,
        /// Store the key as an environment reference ($PROVIDER_API_KEY).
        #[arg(long)]
        key_env: bool,
        /// Comma separated model ids. Default: every preset model.
        #[arg(long, value_name = "ID1,ID2")]
        models: Option<String>,
        /// Test the endpoint even when non-interactive; a failure aborts.
        #[arg(long, conflicts_with = "no_test")]
        test: bool,
        /// Skip the connection test.
        #[arg(long)]
        no_test: bool,
        /// Show the preview only, write nothing.
        #[arg(long)]
        dry_run: bool,
        /// Do not ask for confirmation.
        #[arg(long, short)]
        yes: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Bahasa harus ditentukan sebelum pesan pertama keluar.
    let requested = cli
        .lang
        .clone()
        .or_else(|| std::env::var("HAKONFIG_LANG").ok());
    if let Some(raw) = requested {
        match i18n::parse(&raw) {
            Some(lang) => i18n::set(lang),
            None => {
                eprintln!(
                    "hakonfig: {}",
                    i18n::fill(t(Msg::UnknownLang), &[("raw", raw)])
                );
                return ExitCode::FAILURE;
            }
        }
    }

    // `engine::paths` membaca HAKONFIG_ROOT, jadi --root cukup diteruskan lewat
    // environment: satu pintu, tidak ada jalur kedua yang bisa menyimpang.
    if let Some(root) = &cli.root {
        std::env::set_var("HAKONFIG_ROOT", root);
    }

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.downcast_ref::<add::Cancelled>().is_some() => {
            // Dibatalkan pengguna bukan kegagalan.
            println!("{}", t(Msg::Cancelled));
            ExitCode::SUCCESS
        }
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
        Some(Command::Providers { search, free }) => providers(search.as_deref(), *free, cli.json),
        Some(Command::Paths { harness }) => paths(harness.as_deref(), cli.json),
        Some(Command::Add {
            harness,
            provider,
            name,
            base_url,
            api,
            key_stdin,
            key_env,
            models,
            test,
            no_test,
            dry_run,
            yes,
        }) => add::run(add::AddOptions {
            harness: harness.clone(),
            provider: provider.clone(),
            name: name.clone(),
            base_url: base_url.clone(),
            api: api.clone(),
            key: if *key_stdin {
                Some(add::read_stdin_trimmed()?)
            } else {
                None
            },
            key_mode: if *key_env {
                KeyMode::Env
            } else {
                KeyMode::Inline
            },
            models: models.clone(),
            dry_run: *dry_run,
            yes: *yes,
            test: if *test {
                Some(true)
            } else if *no_test {
                Some(false)
            } else {
                None
            },
            json: cli.json,
        }),
        None => list(cli.json),
    }
}

// ------------------------------------------------------------------ perintah

fn list(json: bool) -> Result<()> {
    let harnesses = api::list_harnesses();

    if json {
        return output::print_json(&harnesses);
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
                    .map(|_| t(Msg::ReadFailed).to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();

    output::print_table(
        &[
            "",
            t(Msg::HeaderId),
            t(Msg::HeaderName),
            t(Msg::HeaderVersion),
            t(Msg::HeaderProviders),
            t(Msg::HeaderNote),
        ],
        rows,
    );

    let installed = harnesses.iter().filter(|entry| entry.installed).count();
    println!(
        "\n{}",
        i18n::fill(
            t(Msg::ListFooter),
            &[
                ("installed", installed.to_string()),
                ("total", harnesses.len().to_string()),
            ]
        )
    );
    if installed == 0 {
        println!("{}", t(Msg::ListNoConfig));
    }
    Ok(())
}

fn show(harness: Option<&str>, json: bool) -> Result<()> {
    let id = resolve_harness(harness)?;
    let snapshot = api::read_harness(id.clone()).map_err(|error| anyhow!(error))?;

    if json {
        return output::print_json(&snapshot);
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
        println!("\n{}", t(Msg::ShowNoProvider));
    } else {
        println!();
        let rows: Vec<Vec<String>> = snapshot
            .providers
            .iter()
            .map(|provider| {
                vec![
                    if provider.enabled {
                        String::new()
                    } else {
                        t(Msg::ProviderDisabled).to_string()
                    },
                    provider.id.clone(),
                    provider.base_url.clone().unwrap_or_else(|| "-".into()),
                    provider.models.len().to_string(),
                    if provider.has_key {
                        t(Msg::KeyPresent).to_string()
                    } else {
                        "-".into()
                    },
                ]
            })
            .collect();
        output::print_table(
            &[
                t(Msg::HeaderStatus),
                t(Msg::HeaderId),
                t(Msg::HeaderBaseUrl),
                t(Msg::HeaderModels),
                t(Msg::HeaderKey),
            ],
            rows,
        );
    }

    if let Some(provider) = &snapshot.default_provider {
        println!(
            "\n{}",
            i18n::fill(
                t(Msg::ShowDefault),
                &[
                    ("provider", provider.clone()),
                    (
                        "model",
                        snapshot
                            .default_model
                            .as_ref()
                            .map(|model| format!("/{model}"))
                            .unwrap_or_default(),
                    ),
                ]
            )
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
            if free_only && !matches!(preset.badge.as_deref(), Some("free") | Some("local")) {
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
        return output::print_json(&presets);
    }

    if presets.is_empty() {
        bail!(
            "{}",
            i18n::fill(t(Msg::ProvidersNone), &[("needle", needle)])
        );
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

    output::print_table(
        &[
            t(Msg::HeaderId),
            t(Msg::HeaderName),
            t(Msg::HeaderPresetModels),
            t(Msg::HeaderBadge),
            t(Msg::HeaderEnv),
        ],
        rows,
    );
    println!(
        "\n{}",
        i18n::fill(
            t(Msg::ProvidersCount),
            &[("count", presets.len().to_string())]
        )
    );
    Ok(())
}

fn paths(harness: Option<&str>, json: bool) -> Result<()> {
    let id = resolve_harness(harness)?;
    let harnesses = api::list_harnesses();
    let entry = harnesses
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| anyhow!("harness '{id}' is not supported"))?;

    if json {
        return output::print_json(&entry.files);
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
///
/// `installed` berarti "file config sudah ada", bukan "harness didukung",
/// jadi id yang eksplisit selalu diterima selama adapter-nya ada.
fn resolve_harness(wanted: Option<&str>) -> Result<String> {
    let harnesses = api::list_harnesses();

    if let Some(id) = wanted {
        let trimmed = id.trim();
        let entry = harnesses
            .iter()
            .find(|entry| entry.id == trimmed)
            .ok_or_else(|| {
                anyhow!(i18n::fill(
                    t(Msg::HarnessUnsupported),
                    &[
                        ("id", trimmed.to_string()),
                        ("choices", output::choices(&harnesses))
                    ],
                ))
            })?;
        return Ok(entry.id.clone());
    }

    let candidates: Vec<&HarnessInfo> = harnesses.iter().filter(|entry| entry.installed).collect();

    match candidates.len() {
        1 => Ok(candidates[0].id.clone()),
        0 => bail!(
            "{}",
            i18n::fill(
                t(Msg::NoHarnessConfig),
                &[("choices", output::choices(&harnesses))]
            )
        ),
        _ => bail!(
            "{}",
            i18n::fill(
                t(Msg::AmbiguousHarness),
                &[
                    ("count", candidates.len().to_string()),
                    ("list", output::choices(&harnesses)),
                ]
            )
        ),
    }
}
