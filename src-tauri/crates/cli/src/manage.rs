//! `providers`, `default`, `set`, `remove`, dan `catalog`.
//!
//! `providers` adalah pintu kelola provider yang **sudah terpasang** — sama
//! seperti layar provider di aplikasi desktop: pilih harness, lihat provider,
//! buka detailnya, lalu ubah satu hal (key, model, default, endpoint) atau
//! hapus. `catalog` tetap urusan "apa yang didukung".

use std::io::IsTerminal;

use anyhow::{anyhow, bail, Result};
use hakonfig_core::api;
use hakonfig_core::engine::model::EditOp;
use hakonfig_core::engine::model::KeyMode;
use hakonfig_core::engine::model::{ApplyResult, HarnessSnapshot, ProviderPreset, ProviderSummary};
use inquire::Select;

use crate::add::{self, AddOptions};
use crate::i18n::{self, Msg};
use crate::output;

/// Hasil satu baris tabel `providers`.
struct Installed {
    harness: String,
    provider: ProviderSummary,
    is_default: bool,
}

pub struct ManageOptions {
    pub harness: Option<String>,
    pub provider: Option<String>,
    pub json: bool,
}

/// `hakonfig-cli providers` — provider yang terpasang, bukan katalog.
pub fn run(options: ManageOptions) -> Result<()> {
    let interactive =
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !options.json;

    let harnesses = installed_harnesses(options.harness.as_deref())?;
    if harnesses.is_empty() {
        bail!("{}", i18n::t(Msg::NoHarnessInstalled));
    }

    let rows = collect(&harnesses)?;

    if options.json {
        output::print_json(&rows_json(&rows))?;
        return Ok(());
    }

    if !interactive {
        print_rows(&rows);
        return Ok(());
    }

    // Interaktif: pilih harness (kalau lebih dari satu), lalu provider, lalu
    // buka detail — persis alur aplikasi desktop.
    let harness = match options.harness.clone() {
        Some(id) => id,
        None if harnesses.len() == 1 => harnesses[0].clone(),
        None => {
            let picked = ask(Select::new(i18n::t(Msg::PickHarness), harnesses.clone()).prompt())?;
            picked
        }
    };

    let provider_id = match options.provider.clone() {
        Some(id) => id,
        None => {
            let snapshot = api::read_harness(harness.clone()).map_err(|error| anyhow!(error))?;
            if snapshot.providers.is_empty() {
                bail!(
                    "{}",
                    i18n::fill(
                        i18n::t(Msg::NoProviderInstalled),
                        &[("harness", harness.clone())]
                    )
                );
            }
            let choices: Vec<Choice> = snapshot
                .providers
                .iter()
                .map(|provider| Choice {
                    id: provider.id.clone(),
                    label: provider_label(&snapshot, provider),
                })
                .collect();
            let picked = ask(Select::new(i18n::t(Msg::PickInstalledProvider), choices).prompt())?;
            picked.id
        }
    };

    open_provider(&harness, &provider_id)
}

/// Menu detail satu provider — dipakai `providers` dan `add` (setelah provider
/// baru ditulis, detailnya langsung dibuka seperti di GUI).
pub fn open_provider(harness: &str, provider_id: &str) -> Result<()> {
    loop {
        let snapshot = api::read_harness(harness.to_string()).map_err(|error| anyhow!(error))?;
        let Some(provider) = snapshot
            .providers
            .iter()
            .find(|entry| entry.id == provider_id)
            .cloned()
        else {
            bail!(
                "{}",
                i18n::fill(
                    i18n::t(Msg::ProviderGone),
                    &[
                        ("provider", provider_id.to_string()),
                        ("harness", harness.to_string())
                    ]
                )
            );
        };

        println!();
        println!("{}", provider_header(&snapshot, &provider));

        let choices = vec![
            i18n::t(Msg::ActionKey).to_string(),
            i18n::t(Msg::ActionModels).to_string(),
            i18n::t(Msg::ActionDefault).to_string(),
            i18n::t(Msg::ActionEndpoint).to_string(),
            i18n::t(Msg::ActionRemove).to_string(),
            i18n::t(Msg::Back).to_string(),
        ];
        let picked = ask(Select::new(i18n::t(Msg::ManageWhat), choices).prompt())?;

        if picked == i18n::t(Msg::Back) {
            return Ok(());
        }
        if picked == i18n::t(Msg::ActionRemove) {
            remove_provider(harness, &provider, false)?;
            return Ok(());
        }
        if picked == i18n::t(Msg::ActionDefault) {
            set_default_for(harness, &provider)?;
            continue;
        }

        // Endpoint/nama tidak punya alur sendiri di `add` (nilainya flag), jadi
        // ditanyakan di sini dengan nilai sekarang sebagai default.
        let (name, base_url) = if picked == i18n::t(Msg::ActionEndpoint) {
            let name = prompt_text(Msg::EditName, &provider.name)?;
            let base_url = prompt_text(
                Msg::EditBaseUrl,
                provider.base_url.as_deref().unwrap_or_default(),
            )?;
            (Some(name), Some(base_url))
        } else {
            // Aksi sempit pada provider custom butuh nilai ini supaya id-nya
            // diterima sebagai target.
            (Some(provider.name.clone()), provider.base_url.clone())
        };

        // Aksi lain memakai jalur `add` yang sudah ada, dengan flag sempit
        // supaya tepat satu hal yang berubah.
        let options = AddOptions {
            harness: Some(harness.to_string()),
            provider: Some(provider_id.to_string()),
            name,
            base_url,
            api: provider.api.clone(),
            key: None,
            key_mode: KeyMode::Inline,
            models: None,
            sync: false,
            key_only: picked == i18n::t(Msg::ActionKey),
            models_only: picked == i18n::t(Msg::ActionModels),
            default_model: None,
            everything: picked == i18n::t(Msg::ActionEndpoint),
            keep_key: picked == i18n::t(Msg::ActionEndpoint),
            dry_run: false,
            yes: false,
            test: None,
            json: false,
        };
        add::run(options)?;
    }
}

/// `hakonfig-cli default` — default yang sedang dipakai tiap harness.
pub fn run_default(harness: Option<String>, json: bool) -> Result<()> {
    let harnesses = installed_harnesses(harness.as_deref())?;
    if harnesses.is_empty() {
        bail!("{}", i18n::t(Msg::NoHarnessInstalled));
    }

    let mut rows = Vec::new();
    for id in &harnesses {
        let snapshot = api::read_harness(id.clone()).map_err(|error| anyhow!(error))?;
        rows.push((
            snapshot.harness.clone(),
            snapshot.default_provider.clone(),
            snapshot.default_model.clone(),
        ));
    }

    let target = target_identity();

    if json {
        let payload: Vec<serde_json::Value> = rows
            .iter()
            .map(|(harness, provider, model)| {
                let mut value = serde_json::json!({
                    "harness": harness,
                    "defaultProvider": provider,
                    "defaultModel": model,
                });
                if let Some((home, user)) = &target {
                    value["home"] = serde_json::json!(home);
                    if let Some(user) = user {
                        value["user"] = serde_json::json!(user);
                    }
                }
                value
            })
            .collect();
        output::print_json(&payload)?;
        return Ok(());
    }

    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|(harness, provider, model)| {
            let mut line = vec![
                harness.clone(),
                provider.clone().unwrap_or_else(|| "-".to_string()),
                model.clone().unwrap_or_else(|| "-".to_string()),
            ];
            if let Some((home, user)) = &target {
                line.push(environment_label(home, user.as_deref()));
            }
            line
        })
        .collect();
    let mut headers = vec![
        i18n::t(Msg::HeaderHarness).to_string(),
        i18n::t(Msg::HeaderDefaultProvider).to_string(),
        i18n::t(Msg::HeaderDefaultModel).to_string(),
    ];
    if target.is_some() {
        headers.push(i18n::t(Msg::HeaderEnv).to_string());
    }
    output::print_table(&headers, table);
    Ok(())
}

/// `hakonfig-cli set default --harness <id> --provider <id> [--model <id>]`.
pub fn run_set_default(
    harness: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    json: bool,
) -> Result<()> {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !json;

    let harnesses = installed_harnesses(harness.as_deref())?;
    if harnesses.is_empty() {
        bail!("{}", i18n::t(Msg::NoHarnessInstalled));
    }
    // Default itu milik harness, jadi harness-nya harus dipilih lebih dulu.
    let harness = match harness {
        Some(id) => id,
        None if harnesses.len() == 1 => harnesses[0].clone(),
        None if interactive => ask(Select::new(i18n::t(Msg::PickHarness), harnesses).prompt())?,
        None => bail!("{}", i18n::t(Msg::NeedHarnessFlag)),
    };

    let snapshot = api::read_harness(harness.clone()).map_err(|error| anyhow!(error))?;
    if snapshot.providers.is_empty() {
        bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::NoProviderInstalled),
                &[("harness", harness.clone())]
            )
        );
    }

    let provider_id = match provider {
        Some(id) => id,
        None if interactive => {
            let choices: Vec<Choice> = snapshot
                .providers
                .iter()
                .map(|entry| Choice {
                    id: entry.id.clone(),
                    label: provider_label(&snapshot, entry),
                })
                .collect();
            ask(Select::new(i18n::t(Msg::PickInstalledProvider), choices).prompt())?.id
        }
        None => bail!("{}", i18n::t(Msg::NeedProviderFlag)),
    };

    let Some(provider) = snapshot
        .providers
        .iter()
        .find(|entry| entry.id == provider_id)
        .cloned()
    else {
        bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::ProviderGone),
                &[
                    ("provider", provider_id.clone()),
                    ("harness", harness.clone())
                ]
            )
        );
    };

    let model_id = match model {
        Some(id) => id,
        None => pick_model(&provider, &snapshot)?,
    };

    apply(
        &harness,
        EditOp::SetDefault {
            provider_id,
            model_id,
        },
        json,
        interactive,
    )
}

/// `hakonfig-cli set key --harness <id> --provider <id>`.
pub fn run_set_key(
    harness: Option<String>,
    provider: Option<String>,
    key: Option<String>,
    key_mode: KeyMode,
    json: bool,
) -> Result<()> {
    let (harness, provider_id) = resolve_target(harness, provider, json)?;
    let key = match key.map(|value| value.trim().to_string()) {
        Some(value) if !value.is_empty() => value,
        _ => add::read_stdin_trimmed()?,
    };
    apply(
        &harness,
        EditOp::SetKey {
            provider_id,
            key,
            key_mode,
        },
        json,
        false,
    )
}

/// `hakonfig-cli set models --harness <id> --provider <id> [--models a,b] [--sync]`.
pub fn run_set_models(
    harness: Option<String>,
    provider: Option<String>,
    models: Option<String>,
    sync: bool,
    json: bool,
) -> Result<()> {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !json;
    let (harness, provider_id) = resolve_target(harness, provider, json)?;

    // Jalur `add` sudah punya pemilih model lengkap (sync/checkbox/manual),
    // jadi flag sempit ini cukup untuk meneruskannya.
    add::run(AddOptions {
        harness: Some(harness),
        provider: Some(provider_id),
        name: None,
        base_url: None,
        api: None,
        key: None,
        key_mode: KeyMode::Inline,
        models,
        sync,
        key_only: false,
        models_only: true,
        default_model: None,
        everything: false,
        keep_key: false,
        dry_run: false,
        yes: false,
        test: Some(false),
        json: json && interactive,
    })
}

/// `hakonfig-cli remove provider --harness <id> --provider <id>`.
pub fn run_remove(
    harness: Option<String>,
    provider: Option<String>,
    json: bool,
    yes: bool,
) -> Result<()> {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !json;
    let (harness, provider_id) = resolve_target(harness, provider, json)?;
    let snapshot = api::read_harness(harness.clone()).map_err(|error| anyhow!(error))?;
    let Some(provider) = snapshot
        .providers
        .iter()
        .find(|entry| entry.id == provider_id)
        .cloned()
    else {
        bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::ProviderGone),
                &[
                    ("provider", provider_id.clone()),
                    ("harness", harness.clone())
                ]
            )
        );
    };
    let _ = interactive;
    remove_provider(&harness, &provider, json || yes)
}

/// `hakonfig-cli catalog` — katalog preset yang didukung.
pub fn run_catalog(search: Option<String>, free: bool, json: bool) -> Result<()> {
    let presets = api::list_presets();
    let needle = search.as_deref().unwrap_or_default().trim().to_lowercase();
    let filtered: Vec<ProviderPreset> = presets
        .into_iter()
        .filter(|preset| {
            needle.is_empty()
                || preset.id.to_lowercase().contains(&needle)
                || preset.name.to_lowercase().contains(&needle)
        })
        .filter(|preset| {
            !free || {
                let badge = preset.badge.as_deref().unwrap_or_default().to_lowercase();
                badge == "free" || badge == "local"
            }
        })
        .collect();

    if json {
        output::print_json(&filtered)?;
        return Ok(());
    }

    let rows: Vec<Vec<String>> = filtered
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
    output::print_table(&["ID", "NAME", "PRESET MODELS", "BADGE", "ENV"], rows);
    Ok(())
}

// ------------------------------------------------------------------ helpers

fn installed_harnesses(only: Option<&str>) -> Result<Vec<String>> {
    let harnesses = api::list_harnesses();
    let mut ids: Vec<String> = harnesses
        .iter()
        .filter(|entry| entry.installed)
        .map(|entry| entry.id.clone())
        .collect();

    if let Some(requested) = only {
        let snapshot = api::read_harness(requested.to_string()).map_err(|error| anyhow!(error))?;
        if !harnesses
            .iter()
            .any(|entry| entry.id == snapshot.harness && entry.installed)
        {
            bail!(
                "{}",
                i18n::fill(
                    i18n::t(Msg::HarnessNotInstalled),
                    &[("harness", requested.to_string())]
                )
            );
        }
        ids.retain(|id| id == requested);
    }
    Ok(ids)
}

fn collect(harnesses: &[String]) -> Result<Vec<Installed>> {
    let mut rows = Vec::new();
    for id in harnesses {
        let snapshot = api::read_harness(id.clone()).map_err(|error| anyhow!(error))?;
        let default_provider = snapshot.default_provider.clone();
        for provider in &snapshot.providers {
            let is_default = default_provider.as_deref() == Some(provider.id.as_str());
            rows.push(Installed {
                harness: snapshot.harness.clone(),
                provider: provider.clone(),
                is_default,
            });
        }
    }
    Ok(rows)
}

fn rows_json(rows: &[Installed]) -> Vec<serde_json::Value> {
    let target = target_identity();
    rows.iter()
        .map(|row| {
            let mut value = serde_json::json!({
                "harness": row.harness,
                "id": row.provider.id,
                "name": row.provider.name,
                "baseUrl": row.provider.base_url,
                "api": row.provider.api,
                "hasKey": row.provider.has_key,
                "keyDisplay": row.provider.key_display,
                // Dari mana key datang: `stored` (file), `env` (variabel), atau
                // `none`. Skrip butuh ini untuk membedakan keduanya.
                "keyMode": row.provider.key_mode,
                "keyEnv": row.provider.key_env,
                "enabled": row.provider.enabled,
                "isDefault": row.is_default,
                "models": row.provider.models.iter().map(|model| model.id.clone()).collect::<Vec<_>>(),
            });
            // Home target hanya muncul saat memang menunjuk home lain, supaya
            // skrip yang membaca output biasa tidak menerima field baru.
            if let Some((home, user)) = &target {
                value["home"] = serde_json::json!(home);
                if let Some(user) = user {
                    value["user"] = serde_json::json!(user);
                }
            }
            value
        })
        .collect()
}

/// Identitas target sekali untuk seluruh tabel: `(home, user)`.
fn target_identity() -> Option<(String, Option<String>)> {
    let home = hakonfig_core::engine::paths::explicit_home()?;
    let user = hakonfig_core::engine::targets::user_for_home(&home);
    Some((home.display().to_string(), user))
}

fn environment_label(home: &str, user: Option<&str>) -> String {
    match user {
        Some(user) => format!("{user}@{home}"),
        None => home.to_string(),
    }
}

fn print_rows(rows: &[Installed]) {
    let target = target_identity();
    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let mut line = vec![
                row.harness.clone(),
                row.provider.id.clone(),
                row.provider.name.clone(),
                row.provider.models.len().to_string(),
                if row.provider.has_key { "yes" } else { "-" }.to_string(),
                if row.is_default { "★" } else { "" }.to_string(),
            ];
            if let Some((home, user)) = &target {
                line.push(environment_label(home, user.as_deref()));
            }
            line
        })
        .collect();
    let mut headers = vec![
        i18n::t(Msg::HeaderHarness).to_string(),
        "ID".to_string(),
        "NAME".to_string(),
        i18n::t(Msg::HeaderModels).to_string(),
        i18n::t(Msg::HeaderKey).to_string(),
        i18n::t(Msg::HeaderDefault).to_string(),
    ];
    if target.is_some() {
        headers.push(i18n::t(Msg::HeaderEnv).to_string());
    }
    output::print_table(&headers, table);
}

fn provider_label(snapshot: &HarnessSnapshot, provider: &ProviderSummary) -> String {
    let mut parts = vec![provider.id.clone(), provider.name.clone()];
    if provider.models.is_empty() {
        parts.push(i18n::t(Msg::NoModelsYet).to_string());
    } else {
        parts.push(i18n::fill(
            i18n::t(Msg::ProviderCountLabel),
            &[("count", provider.models.len().to_string())],
        ));
    }
    if provider.has_key {
        parts.push("✓".to_string());
    }
    let is_default = snapshot.default_provider.as_deref() == Some(provider.id.as_str());
    if is_default {
        parts.push("★".to_string());
    }
    parts.join("  ·  ")
}

fn provider_header(snapshot: &HarnessSnapshot, provider: &ProviderSummary) -> String {
    i18n::fill(
        i18n::t(Msg::ProviderHeader),
        &[
            ("provider", provider.id.clone()),
            ("name", provider.name.clone()),
            ("harness", snapshot.harness.clone()),
            ("models", provider.models.len().to_string()),
            (
                "key",
                if provider.has_key {
                    i18n::t(Msg::KeySavedLabel).to_string()
                } else {
                    i18n::t(Msg::KeyMissingLabel).to_string()
                },
            ),
            (
                "desc",
                match &snapshot.default_model {
                    Some(model) => format!("★ {model}"),
                    None => "-".to_string(),
                },
            ),
        ],
    )
}

fn pick_model(provider: &ProviderSummary, snapshot: &HarnessSnapshot) -> Result<String> {
    if provider.models.is_empty() {
        bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::NoModelsToPick),
                &[("provider", provider.id.clone())]
            )
        );
    }
    let choices: Vec<Choice> = provider
        .models
        .iter()
        .map(|model| Choice {
            id: model.id.clone(),
            label: match &snapshot.default_model {
                Some(current) if current.ends_with(&format!("/{}", model.id)) => {
                    format!("{}  ★", model.id)
                }
                _ => model.id.clone(),
            },
        })
        .collect();
    Ok(ask(Select::new(i18n::t(Msg::PickDefaultModel), choices).prompt())?.id)
}

fn set_default_for(harness: &str, provider: &ProviderSummary) -> Result<()> {
    let snapshot = api::read_harness(harness.to_string()).map_err(|error| anyhow!(error))?;
    let model_id = pick_model(provider, &snapshot)?;
    apply(
        harness,
        EditOp::SetDefault {
            provider_id: provider.id.clone(),
            model_id,
        },
        false,
        true,
    )
}

fn remove_provider(harness: &str, provider: &ProviderSummary, quiet: bool) -> Result<()> {
    let op = EditOp::Remove {
        provider_id: provider.id.clone(),
    };
    let preview =
        api::preview_edit(harness.to_string(), op.clone()).map_err(|error| anyhow!(error))?;

    if !quiet {
        output::print_preview(&preview);
        let question = i18n::fill(
            i18n::t(Msg::ConfirmRemove),
            &[("provider", provider.id.clone())],
        )
        .to_string();
        let proceed = ask(inquire::Confirm::new(&question)
            .with_default(false)
            .with_help_message(i18n::t(Msg::ConfirmRemoveHint))
            .prompt())?;
        if !proceed {
            return Err(anyhow::Error::new(add::Cancelled));
        }
    }

    let applied = api::apply_edit(harness.to_string(), op).map_err(|error| anyhow!(error))?;
    if quiet {
        let payload = serde_json::json!({
            "harness": harness,
            "provider": provider.id,
            "action": "remove",
            "applied": true,
            "filesWritten": applied.files_written,
            "backupDir": applied.backup_dir,
            "warnings": applied.warnings,
        });
        output::print_json(&payload)?;
    } else {
        add::report(&applied, &[]);
    }
    Ok(())
}

fn resolve_target(
    harness: Option<String>,
    provider: Option<String>,
    json: bool,
) -> Result<(String, String)> {
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !json;
    let harnesses = installed_harnesses(harness.as_deref())?;
    if harnesses.is_empty() {
        bail!("{}", i18n::t(Msg::NoHarnessInstalled));
    }
    let harness = match harness {
        Some(id) => id,
        None if harnesses.len() == 1 => harnesses[0].clone(),
        None if interactive => ask(Select::new(i18n::t(Msg::PickHarness), harnesses).prompt())?,
        None => bail!("{}", i18n::t(Msg::NeedHarnessFlag)),
    };

    let snapshot = api::read_harness(harness.clone()).map_err(|error| anyhow!(error))?;
    if snapshot.providers.is_empty() {
        bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::NoProviderInstalled),
                &[("harness", harness.clone())]
            )
        );
    }
    let provider_id = match provider {
        Some(id) => id,
        None if interactive => {
            let choices: Vec<Choice> = snapshot
                .providers
                .iter()
                .map(|entry| Choice {
                    id: entry.id.clone(),
                    label: provider_label(&snapshot, entry),
                })
                .collect();
            ask(Select::new(i18n::t(Msg::PickInstalledProvider), choices).prompt())?.id
        }
        None => bail!("{}", i18n::t(Msg::NeedProviderFlag)),
    };
    Ok((harness, provider_id))
}

fn apply(harness: &str, op: EditOp, json: bool, interactive: bool) -> Result<()> {
    let action = match &op {
        EditOp::SetKey { .. } => "set_key",
        EditOp::SetModels { .. } => "set_models",
        EditOp::SetDefault { .. } => "set_default",
        EditOp::SetEnabled { .. } => "set_enabled",
        EditOp::ForgetKey { .. } => "forget_key",
        EditOp::Remove { .. } => "remove",
        _ => "upsert",
    };
    let provider_id = match &op {
        EditOp::SetKey { provider_id, .. }
        | EditOp::SetModels { provider_id, .. }
        | EditOp::SetDefault { provider_id, .. }
        | EditOp::SetEnabled { provider_id, .. }
        | EditOp::ForgetKey { provider_id }
        | EditOp::Remove { provider_id } => provider_id.clone(),
        EditOp::Upsert(draft) => draft.id.clone(),
    };

    let preview =
        api::preview_edit(harness.to_string(), op.clone()).map_err(|error| anyhow!(error))?;
    if !json {
        output::print_preview(&preview);
    }

    if interactive && !json {
        let proceed = ask(inquire::Confirm::new(i18n::t(Msg::ConfirmApply))
            .with_default(true)
            .with_help_message(i18n::t(Msg::ConfirmApplyHint))
            .prompt())?;
        if !proceed {
            return Err(anyhow::Error::new(add::Cancelled));
        }
    }

    let applied: ApplyResult =
        api::apply_edit(harness.to_string(), op).map_err(|error| anyhow!(error))?;
    if json {
        let payload = serde_json::json!({
            "harness": harness,
            "provider": provider_id,
            "action": action,
            "applied": true,
            "filesWritten": applied.files_written,
            "backupDir": applied.backup_dir,
            "warnings": applied.warnings,
        });
        output::print_json(&payload)?;
    } else {
        add::report(&applied, &preview.warnings);
    }
    Ok(())
}

fn prompt_text(message: Msg, current: &str) -> Result<String> {
    let answer = ask(inquire::Text::new(i18n::t(message))
        .with_default(current)
        .prompt())?;
    Ok(answer.trim().to_string())
}

fn ask<T>(result: Result<T, inquire::InquireError>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => {
            Err(anyhow::Error::new(add::Cancelled))
        }
        Err(error) => Err(anyhow!(error)),
    }
}

/// Pilihan dengan id terpisah dari label (label boleh memuat badge ✓/★).
struct Choice {
    id: String,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}
