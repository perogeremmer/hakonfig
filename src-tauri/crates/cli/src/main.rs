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
mod manage;
mod output;

use i18n::{t, Msg};

#[derive(Subcommand)]
enum SetWhat {
    /// Replace the API key only.
    Key {
        /// harness id.
        #[arg(long)]
        harness: Option<String>,
        /// Provider id.
        #[arg(long)]
        provider: Option<String>,
        /// Read the key from stdin instead of the environment.
        #[arg(long)]
        key_stdin: bool,
        /// New key value (prefer --key-stdin so it stays out of history).
        #[arg(long, value_name = "KEY")]
        key: Option<String>,
        /// Keep the key in the environment instead of the config file.
        #[arg(long)]
        env: bool,
    },
    /// Replace the model list of a provider.
    Models {
        /// harness id.
        #[arg(long)]
        harness: Option<String>,
        /// Provider id.
        #[arg(long)]
        provider: Option<String>,
        /// Comma-separated model ids.
        #[arg(long, value_name = "IDS")]
        models: Option<String>,
        /// Ask the endpoint for the model list (uses the saved key).
        #[arg(long)]
        sync: bool,
    },
    /// Make one model of a provider the harness default.
    Default {
        /// harness id.
        #[arg(long)]
        harness: Option<String>,
        /// Provider id.
        #[arg(long)]
        provider: Option<String>,
        /// Model id that becomes the default.
        #[arg(long, value_name = "MODEL_ID")]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum RemoveWhat {
    /// Remove a provider from a harness config.
    Provider {
        /// harness id.
        #[arg(long)]
        harness: Option<String>,
        /// Provider id.
        #[arg(long)]
        provider: Option<String>,
        /// Skip the confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Parser)]
#[command(
    name = "hakonfig",
    version,
    about = "Manage providers for AI coding harnesses",
    disable_help_subcommand = true
)]
struct Cli {
    /// Alternate root for harness config files.
    ///
    /// Test layout: files resolve as <DIR>/<harness>/... Use --home to point
    /// at a real home directory instead.
    #[arg(long, global = true, value_name = "DIR")]
    root: Option<String>,

    /// HOME of the target user, e.g. an agent's dedicated home (/var/lib/openbot).
    ///
    /// All harness directories resolve inside it with their real layout
    /// (<DIR>/.omp, <DIR>/.pi, ...), and version probes use its .local/bin.
    #[arg(long, global = true, value_name = "DIR")]
    home: Option<String>,

    /// Target by username (resolved via /etc/passwd) or home directory.
    ///
    /// Shorthand for --home with user lookup, e.g. --target openbot.
    #[arg(long, global = true, value_name = "USER|DIR", conflicts_with = "home")]
    target: Option<String>,

    /// Run as the owner of the harness home, switching user when needed.
    ///
    /// Re-executes the same command via `sudo -n -u <USER> -H`, so files are
    /// written with the right owner and the harness sees them. Without
    /// passwordless sudo the exact command is printed instead.
    #[arg(
        long,
        global = true,
        value_name = "USER",
        conflicts_with_all = ["home", "target"]
    )]
    as_user: Option<String>,

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
    /// Providers installed on this machine — the door to managing them.
    ///
    /// With a terminal it opens the provider, so a key, model list, default
    /// model, endpoint, or the provider itself can be changed one at a time.
    Providers {
        /// Limit to one harness.
        #[arg(long)]
        harness: Option<String>,
        /// Open this provider straight away.
        #[arg(long)]
        provider: Option<String>,
        /// Print the built-in catalog instead (moved to `catalog`).
        #[arg(long, hide = true)]
        catalog: bool,
        /// Filter the catalog by id or name (implies --catalog).
        #[arg(long, short)]
        search: Option<String>,
        /// Only catalog providers badged free or local (implies --catalog).
        #[arg(long)]
        free: bool,
    },
    /// Provider presets this build supports (the catalog).
    #[command(visible_alias = "presets", visible_alias = "provider-list")]
    Catalog {
        /// Filter by id or name.
        #[arg(long, short)]
        search: Option<String>,
        /// Only providers badged free or local.
        #[arg(long)]
        free: bool,
    },
    /// Default provider and model of each harness.
    Default {
        /// Limit to one harness.
        #[arg(long)]
        harness: Option<String>,
    },
    /// Change one thing on a provider that is already installed.
    Set {
        #[command(subcommand)]
        what: SetWhat,
    },
    /// Remove a provider from a harness config.
    Remove {
        #[command(subcommand)]
        what: RemoveWhat,
    },
    /// Config file paths used by a harness.
    Paths {
        /// harness id. Optional when only one exists.
        harness: Option<String>,
    },
    /// Homes of other users (ADE / VPS) with harnesses installed.
    Targets,
    /// Find config the harness will not show, and optionally fix it.
    Doctor {
        /// Limit the check to one harness.
        #[arg(long)]
        harness: Option<String>,
        /// Apply the automatic fixes.
        #[arg(long)]
        fix: bool,
        /// Also check the remote model catalogs the harness refreshes (network).
        #[arg(long)]
        online: bool,
    },
    /// Add a provider (wizard without flags). Opens its settings if it already exists.
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
        /// Ask the endpoint which models exist and use that list instead of the
        /// preset list. Needs network access and, usually, the key.
        #[arg(long)]
        sync: bool,
        /// Only replace the API key of an existing provider; models stay as they
        /// are.
        #[arg(long)]
        key_only: bool,
        /// Only replace the model list of an existing provider; the key stays.
        #[arg(long)]
        models_only: bool,
        /// Only set the default model of the harness (no other change).
        #[arg(long, value_name = "MODEL_ID")]
        default_model: Option<String>,
        /// Rewrite the whole provider even though it already exists.
        #[arg(long)]
        everything: bool,
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

    // `engine::paths` membaca HAKONFIG_ROOT/HAKONFIG_HOME, jadi --root dan
    // --home cukup diteruskan lewat environment: satu pintu, tidak ada jalur
    // kedua yang bisa menyimpang.
    if let Some(root) = &cli.root {
        std::env::set_var("HAKONFIG_ROOT", root);
    }
    if let Some(home) = &cli.home {
        let target = match absolute_home(home) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("hakonfig: {error}");
                return ExitCode::FAILURE;
            }
        };
        if !target.is_dir() {
            eprintln!(
                "hakonfig: {}",
                i18n::fill(
                    t(Msg::HomeMissing),
                    &[("path", target.display().to_string())]
                )
            );
        }
        // Path absolut: path relatif akan tampil relatif di seluruh output dan
        // berubah artinya kalau cwd berpindah (mis. lewat `sudo -u ... -H`).
        std::env::set_var("HAKONFIG_HOME", &target);
        // Selalu konfirmasi home mana yang dipakai: kesalahan ketik di sini
        // persis seperti jebakan --root — gagal senyap tanpa konfirmasi.
        eprintln!(
            "{}",
            i18n::fill(
                t(Msg::TargetHome),
                &[("path", target.display().to_string())]
            )
        );
    }
    if let Some(target) = &cli.target {
        match api::resolve_target(target.trim()) {
            Ok(home) => {
                std::env::set_var("HAKONFIG_HOME", &home);
                // Konfirmasi hasil resolusi, sama seperti --home: tanpa ini,
                // salah target berarti gagal senyap.
                eprintln!("{}", i18n::fill(t(Msg::TargetHome), &[("path", home)]));
            }
            Err(error) => {
                eprintln!("hakonfig: {error}");
                return ExitCode::FAILURE;
            }
        }
    }

    if let Some(user) = &cli.as_user {
        return switch_user(user, &cli);
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

/// `--as-user <user>`: pastikan perintah berjalan sebagai pemilik home.
///
/// Menulis ke home user lain dari uid yang salah menghasilkan file milik uid
/// itu: harness bisa kehilangan akses ke key-nya sendiri, dan gejalanya baru
/// terlihat jauh kemudian. Jadi kalau bukan user tersebut, perintah ini
/// menjalankan ulang dirinya lewat `sudo -n -u <user> -H` (tanpa prompt, supaya
/// skrip tidak menggantung), dan kalau sudo butuh password ia mencetak perintah
/// siap-tempel alih-alih gagal tanpa penjelasan.
fn switch_user(user: &str, cli: &Cli) -> ExitCode {
    let Some((home, uid)) = hakonfig_core::engine::targets::passwd_user(user.trim()) else {
        eprintln!(
            "hakonfig: {}",
            i18n::fill(t(Msg::AsUserUnknown), &[("user", user.trim().to_string())])
        );
        return ExitCode::FAILURE;
    };
    std::env::set_var("HAKONFIG_HOME", &home);

    let home_label = home.display().to_string();
    if hakonfig_core::engine::targets::current_uid() == Some(uid) {
        // Sudah user yang benar: perilakunya sama dengan --home.
        eprintln!(
            "{}",
            i18n::fill(t(Msg::TargetHome), &[("path", home_label.clone())])
        );
        return finish(run(cli));
    }

    let command = manual_command(user.trim(), &home_label, cli);
    if !sudo_without_password() {
        eprintln!(
            "{}",
            i18n::fill(
                t(Msg::AsUserNeedsSudo),
                &[("user", user.trim().to_string()), ("command", command)]
            )
        );
        return ExitCode::FAILURE;
    }

    eprintln!(
        "{}",
        i18n::fill(
            t(Msg::AsUserSwitch),
            &[
                ("user", user.trim().to_string()),
                ("path", home_label.clone())
            ]
        )
    );
    match reexec(user.trim(), cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!(
                "hakonfig: {}",
                i18n::fill(
                    t(Msg::AsUserFailed),
                    &[("error", format!("{error:#}")), ("command", command)]
                )
            );
            ExitCode::FAILURE
        }
    }
}

/// Selesai dengan exit code yang sama seperti perintah biasa.
fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.downcast_ref::<add::Cancelled>().is_some() => {
            println!("{}", t(Msg::Cancelled));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("hakonfig: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Perintah siap-tempel untuk dijalankan sendiri oleh pengguna.
///
/// Argumen asli ikut disertakan (tanpa `--as-user`) supaya bisa langsung
/// disalin: perintah yang menyuruh "jalankan ulang" tanpa menyebut apa yang
/// dijalankan hanya menambah teka-teki.
fn manual_command(user: &str, home: &str, cli: &Cli) -> String {
    let binary = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "hakonfig-cli".to_string());
    let args: Vec<String> = forwarded_args(cli)
        .into_iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect();
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    format!("sudo -u {user} -H env PATH=\"{home}/.local/bin:$PATH\" {binary}{suffix}")
}

/// `sudo -n true` = bisa sudo tanpa password. Kalau tidak, jangan menyerah pada
/// prompt: skrip dan server akan menggantung menunggu input.
fn sudo_without_password() -> bool {
    std::process::Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Jalankan ulang dirinya sebagai `user`, tanpa flag `--as-user` (kalau tidak,
/// setiap anak akan mencoba berpindah lagi).
fn reexec(user: &str, cli: &Cli) -> Result<ExitCode> {
    let binary = std::env::current_exe()?;
    let home = std::env::var("HAKONFIG_HOME").unwrap_or_default();
    let mut command = std::process::Command::new("sudo");
    command
        .arg("-n")
        .arg("-u")
        .arg(user)
        .arg("-H")
        .arg("env")
        .arg(format!(
            "PATH={home}/.local/bin:{}",
            std::env::var("PATH").unwrap_or_default()
        ))
        .arg(binary)
        .args(forwarded_args(cli));

    let status = command.status()?;
    Ok(ExitCode::from(
        status.code().unwrap_or(1).clamp(0, 255) as u8
    ))
}

/// Argumen asli tanpa pasangan `--as-user <user>` (bentuk `--flag=nilai` juga).
fn forwarded_args(cli: &Cli) -> Vec<std::ffi::OsString> {
    let raw: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let mut kept = Vec::with_capacity(raw.len());
    let mut skip_next = false;
    for arg in raw {
        if skip_next {
            skip_next = false;
            continue;
        }
        let text = arg.to_string_lossy().to_string();
        if text == "--as-user" {
            skip_next = true;
            continue;
        }
        if text.starts_with("--as-user=") {
            continue;
        }
        kept.push(arg);
    }
    let _ = cli;
    kept
}

/// `--home` harus absolut: relatif akan tampil relatif di seluruh output dan
/// artinya berubah kalau cwd berpindah. Path yang ada di-canonicalize supaya
/// symlink tidak menghasilkan dua nama untuk home yang sama.
fn absolute_home(raw: &str) -> Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        bail!("--home needs a directory");
    }
    let absolut = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| anyhow!("cannot resolve relative --home: {error}"))?
            .join(path)
    };
    if absolut.is_file() {
        bail!("--home {} is a file, not a directory", absolut.display());
    }
    Ok(absolut.canonicalize().unwrap_or(absolut))
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Command::List) => list(cli.json),
        Some(Command::Show { harness }) => show(harness.as_deref(), cli.json),
        Some(Command::Providers {
            harness,
            provider,
            catalog,
            search,
            free,
        }) => {
            // `--search`/`--free`/`--catalog` menjaga skrip lama tetap jalan.
            if *catalog || search.is_some() || *free {
                manage::run_catalog(search.clone(), *free, cli.json)
            } else {
                manage::run(manage::ManageOptions {
                    harness: harness.clone(),
                    provider: provider.clone(),
                    json: cli.json,
                })
            }
        }
        Some(Command::Catalog { search, free }) => {
            manage::run_catalog(search.clone(), *free, cli.json)
        }
        Some(Command::Default { harness }) => manage::run_default(harness.clone(), cli.json),
        Some(Command::Set { what }) => match what {
            SetWhat::Key {
                harness,
                provider,
                key,
                env,
                ..
            } => manage::run_set_key(
                harness.clone(),
                provider.clone(),
                key.clone(),
                if *env { KeyMode::Env } else { KeyMode::Inline },
                cli.json,
            ),
            SetWhat::Models {
                harness,
                provider,
                models,
                sync,
            } => manage::run_set_models(
                harness.clone(),
                provider.clone(),
                models.clone(),
                *sync,
                cli.json,
            ),
            SetWhat::Default {
                harness,
                provider,
                model,
            } => {
                manage::run_set_default(harness.clone(), provider.clone(), model.clone(), cli.json)
            }
        },
        Some(Command::Remove { what }) => match what {
            RemoveWhat::Provider {
                harness,
                provider,
                yes,
            } => manage::run_remove(harness.clone(), provider.clone(), cli.json, *yes),
        },
        Some(Command::Paths { harness }) => paths(harness.as_deref(), cli.json),
        Some(Command::Targets) => targets_cmd(cli.json),
        Some(Command::Doctor {
            harness,
            fix,
            online,
        }) => doctor(harness.as_deref(), *fix, *online, cli.json),
        Some(Command::Add {
            harness,
            provider,
            name,
            base_url,
            api,
            key_stdin,
            key_env,
            models,
            sync,
            key_only,
            models_only,
            default_model,
            everything,
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
            sync: *sync,
            key_only: *key_only,
            models_only: *models_only,
            default_model: default_model.clone(),
            everything: *everything,
            keep_key: false,
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

    // Saat target eksplisit, tabel menyebut targetnya sekali di kolom sendiri:
    // tanpa itu hasil home lain terlihat identik dengan home sendiri.
    let target = harnesses.iter().find_map(|entry| entry.home.clone());
    let rows: Vec<Vec<String>> = harnesses
        .iter()
        .map(|entry| {
            let mut row = vec![
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
            ];
            if target.is_some() {
                row.push(environment_label(entry));
            }
            row
        })
        .collect();

    let mut headers = vec![
        String::new(),
        t(Msg::HeaderId).to_string(),
        t(Msg::HeaderName).to_string(),
        t(Msg::HeaderVersion).to_string(),
        t(Msg::HeaderProviders).to_string(),
        t(Msg::HeaderNote).to_string(),
    ];
    if target.is_some() {
        headers.push(t(Msg::HeaderEnv).to_string());
    }
    output::print_table(&headers, rows);

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

/// Label target untuk kolom `ENV`: `openbot@/var/lib/openbot` bila user-nya
/// dikenali, `home <path>` bila tidak.
fn environment_label(entry: &hakonfig_core::engine::model::HarnessInfo) -> String {
    let home = entry.home.clone().unwrap_or_default();
    match entry.user.as_deref() {
        Some(user) => format!("{user}@{home}"),
        None => i18n::fill(t(Msg::EnvHomeOnly), &[("home", home)]),
    }
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

/// `targets`: home user lain (ADE / VPS) yang harness-nya terdeteksi.
///
/// Perintah baca: tidak pernah menulis apa pun. Output menunjuk cara
/// menargetkan tiap home dengan `--home`.
fn targets_cmd(json: bool) -> Result<()> {
    let found = api::list_targets();

    if json {
        return output::print_json(&found);
    }

    if found.is_empty() {
        println!("{}", t(Msg::TargetsNone));
        return Ok(());
    }

    let rows: Vec<Vec<String>> = found
        .iter()
        .map(|entry| {
            vec![
                entry.user.clone(),
                entry.home.clone(),
                entry.harnesses.join(", "),
            ]
        })
        .collect();
    output::print_table(
        &[
            t(Msg::HeaderUser),
            t(Msg::HeaderHome),
            t(Msg::HeaderHarnesses),
        ],
        rows,
    );
    println!(
        "\n{}",
        i18n::fill(t(Msg::TargetsCount), &[("count", found.len().to_string())])
    );
    Ok(())
}

fn doctor(harness: Option<&str>, fix: bool, online: bool, json: bool) -> Result<()> {
    let requested = match harness {
        Some(id) => Some(pick_harness_id(id)?),
        None => None,
    };

    let mut notices = api::list_notices(requested.clone()).map_err(|error| anyhow!(error))?;
    let mut catalog_notices = if online {
        api::probe_catalogs(requested.clone()).map_err(|error| anyhow!(error))?
    } else {
        Vec::new()
    };

    if fix {
        let mut fixed = Vec::new();
        for notice in notices.iter().filter(|notice| notice.fix.is_some()) {
            match api::fix_notice(
                notice.harness.clone(),
                notice.code.clone(),
                notice.provider.clone(),
            ) {
                Ok(result) => {
                    let detail = result
                        .warnings
                        .first()
                        .cloned()
                        .unwrap_or_else(|| notice.fix.clone().unwrap_or_default());
                    fixed.push((notice.clone(), detail));
                }
                Err(error) => {
                    eprintln!(
                        "{}",
                        i18n::fill(
                            t(Msg::DoctorFixFailed),
                            &[
                                ("harness", notice.harness.clone()),
                                ("provider", notice.provider.clone()),
                                ("error", error),
                            ],
                        )
                    );
                }
            }
        }
        // Sesudah diperbaiki, tampilkan kondisi terbaru (sisa notice).
        notices = api::list_notices(requested.clone()).map_err(|error| anyhow!(error))?;
        if online {
            catalog_notices = api::probe_catalogs(requested).map_err(|error| anyhow!(error))?;
        }

        if json {
            output::print_json(&serde_json::json!({
                "fixed": fixed
                    .iter()
                    .map(|(notice, detail)| serde_json::json!({
                        "code": notice.code,
                        "harness": notice.harness,
                        "provider": notice.provider,
                        "detail": detail,
                    }))
                    .collect::<Vec<_>>(),
                "remaining": notices,
            }))?;
            return if notices.iter().any(|notice| notice.fix.is_some()) {
                Err(anyhow!(t(Msg::DoctorRemaining)))
            } else {
                Ok(())
            };
        }

        for (_, detail) in &fixed {
            println!(
                "{}",
                i18n::fill(t(Msg::DoctorFixed), &[("detail", detail.clone())])
            );
        }
        if fixed.is_empty() {
            println!("{}", t(Msg::DoctorNothingToFix));
        }
    } else if json {
        notices.extend(catalog_notices);
        output::print_json(&notices)?;
        return if notices.iter().any(|notice| notice.fix.is_some()) {
            Err(anyhow!(t(Msg::DoctorRemaining)))
        } else {
            Ok(())
        };
    }

    if notices.is_empty() && catalog_notices.is_empty() {
        println!("{}", t(Msg::DoctorClean));
        return Ok(());
    }
    notices.extend(catalog_notices);

    let rows: Vec<Vec<String>> = notices
        .iter()
        .map(|notice| {
            vec![
                notice.harness.clone(),
                if notice.provider.is_empty() {
                    "-".to_string()
                } else {
                    notice.provider.clone()
                },
                notice.message.clone(),
                notice
                    .fix
                    .clone()
                    .unwrap_or_else(|| t(Msg::DoctorManual).to_string()),
            ]
        })
        .collect();
    output::print_table(
        &[
            t(Msg::DoctorHarness),
            t(Msg::DoctorProvider),
            t(Msg::DoctorIssue),
            t(Msg::DoctorFix),
        ],
        rows,
    );

    let fixable = notices.iter().filter(|notice| notice.fix.is_some()).count();
    if fixable == 0 {
        return Ok(());
    }

    // Config yang tertulis tetapi tidak terlihat harness = kegagalan nyata, jadi
    // exit code non-zero supaya bisa dipakai di skrip. Notice yang hanya bisa
    // dibereskan manual (mis. id bentrok) tidak menggagalkan apa pun.
    if fix {
        Err(anyhow!(t(Msg::DoctorRemaining)))
    } else {
        Err(anyhow!(i18n::fill(
            t(Msg::DoctorHintFix),
            &[("count", fixable.to_string())]
        )))
    }
}

/// Validasi id harness yang diminta pengguna.
fn pick_harness_id(raw: &str) -> Result<String> {
    let id = raw.trim().to_lowercase();
    if api::list_harnesses().iter().any(|harness| harness.id == id) {
        Ok(id)
    } else {
        bail!(
            "{}",
            i18n::fill(t(Msg::UnknownHarness), &[("raw", raw.to_string())])
        )
    }
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
