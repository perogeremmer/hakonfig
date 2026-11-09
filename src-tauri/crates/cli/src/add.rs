//! `add`: tambah atau ubah provider.
//!
//! Dua mode, satu jalur kode:
//! * ada flag atau `--json` → non-interaktif, tidak pernah bertanya (untuk
//!   skrip & server);
//! * dijalankan di terminal tanpa flag → wizard memakai `inquire`.
//!
//! Keduanya berakhir di `EditOp::Upsert` yang sama, jadi perilaku tulis,
//! backup, dan redaksi identik.
use std::io::{IsTerminal, Read};

use anyhow::{anyhow, bail, Result};
use hakonfig_core::api;
use hakonfig_core::engine::model::{
    ApplyResult, EditOp, HarnessInfo, KeyMode, ModelDraft, ProviderDraft, ProviderPreset,
};

use crate::draft::{self, CUSTOM_ID};
use crate::i18n::{self, Msg};
use crate::output;

pub struct AddOptions {
    pub harness: Option<String>,
    pub provider: Option<String>,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api: Option<String>,
    pub key: Option<String>,
    pub key_mode: KeyMode,
    pub models: Option<String>,
    /// Ambil daftar model dari endpoint, bukan dari preset.
    pub sync: bool,
    pub dry_run: bool,
    pub yes: bool,
    /// None = ikut mode (uji koneksi hanya saat interaktif).
    pub test: Option<bool>,
    pub json: bool,
}

/// Provider yang jadi target: dari katalog, atau endpoint custom.
enum Target {
    Preset(Box<ProviderPreset>),
    Custom { id: String },
}

/// Penanda "pengguna membatalkan". Bukan kegagalan: exit code tetap 0.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AddReport {
    harness: String,
    provider: String,
    model_count: usize,
    applied: bool,
    files_written: Vec<String>,
    backup_dir: Option<String>,
    warnings: Vec<String>,
}

pub fn run(options: AddOptions) -> Result<()> {
    // Satu-satunya tempat yang memutuskan interaktif atau tidak. `--json`
    // berarti output disiapkan untuk skrip, dan stdout yang dipipe berarti
    // pertanyaan tidak akan terbaca manusia; keduanya memaksa non-interaktif.
    // Test otomatis menjalankan binary dengan stdin pipa, jadi otomatis lewat
    // jalur flag.
    let interactive =
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !options.json;

    let harnesses = api::list_harnesses();
    let harness = pick_harness(options.harness.as_deref(), &harnesses, interactive)?;

    let snapshot = api::read_harness(harness.clone()).map_err(|error| anyhow!(error))?;
    let installed_ids: Vec<String> = snapshot
        .providers
        .iter()
        .map(|provider| provider.id.clone())
        .collect();

    let presets = api::list_presets();
    let target = match pick_target(&options, &presets, &installed_ids, interactive)? {
        Some(target) => target,
        None => return Err(anyhow::Error::new(Cancelled)),
    };

    // Identitas provider custom divalidasi lebih dulu: tanpa base URL, langkah
    // key/model tidak ada artinya, jadi pesan errornya harus muncul duluan.
    if matches!(target, Target::Custom { .. }) {
        draft::draft_custom(
            target_id(&target),
            options.name.as_deref(),
            options.base_url.as_deref(),
            options.api.as_deref(),
            None,
            options.key_mode,
            Vec::new(),
        )
        .map_err(|error| anyhow!(error))?;
    }

    let key = match options.key.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => Some(value.to_string()),
        _ if interactive => ask_key(&target)?,
        _ => None,
    };

    let provider_id = target_id(&target).to_string();
    let (base_url, api_name) = endpoint(&target, &options);

    let should_test = options.test.unwrap_or(interactive);
    if should_test
        && !run_test(
            &TestRequest {
                base_url,
                api_name,
                provider_id: provider_id.clone(),
                harness: harness.clone(),
                key: key.clone().unwrap_or_default(),
            },
            interactive,
            options.yes,
            options.test == Some(true),
        )?
    {
        return Ok(());
    }

    let models = pick_models(&options, &target, &harness, key.as_deref(), interactive)?;

    // Draft dibangun ulang penuh di sini (validasi custom sudah dijalankan
    // lebih awal supaya pesan errornya muncul sebelum langkah key/model),
    // jadi ini sekadar melengkapi key dan model yang sudah terkumpul.
    let draft = build_draft(&target, key.as_deref(), options.key_mode, models, &options)?;
    let model_count = draft.models.len();

    let op = EditOp::Upsert(draft);
    let preview = api::preview_edit(harness.clone(), op.clone()).map_err(|error| anyhow!(error))?;

    if !options.json {
        output::print_preview(&preview);
    }

    if options.dry_run {
        if options.json {
            output::print_json(&AddReport {
                harness: harness.clone(),
                provider: provider_id,
                model_count,
                applied: false,
                files_written: Vec::new(),
                backup_dir: None,
                warnings: preview.warnings.clone(),
            })?;
        } else {
            println!("\n{}", i18n::t(Msg::DryRun));
        }
        return Ok(());
    }

    if interactive && !options.yes {
        let proceed = ask(inquire::Confirm::new(i18n::t(Msg::ConfirmApply))
            .with_default(true)
            .with_help_message(i18n::t(Msg::ConfirmApplyHint))
            .prompt())?;
        if !proceed {
            return Err(anyhow::Error::new(Cancelled));
        }
    }

    let applied = api::apply_edit(harness.clone(), op).map_err(|error| anyhow!(error))?;

    if options.json {
        output::print_json(&AddReport {
            harness,
            provider: provider_id,
            model_count,
            applied: true,
            files_written: applied.files_written.clone(),
            backup_dir: applied.backup_dir.clone(),
            warnings: applied.warnings.clone(),
        })?;
    } else {
        // Warning yang sudah tampil di pratinjau tidak diulang di bawah.
        report(&applied, &preview.warnings);
    }
    Ok(())
}

fn report(applied: &ApplyResult, already_shown: &[String]) {
    if applied.files_written.is_empty() {
        println!("{}", i18n::t(Msg::NothingToWrite));
    } else {
        for path in &applied.files_written {
            println!(
                "{}",
                i18n::fill(i18n::t(Msg::Wrote), &[("path", path.clone())])
            );
        }
        if let Some(backup) = &applied.backup_dir {
            println!(
                "{}",
                i18n::fill(i18n::t(Msg::Backup), &[("path", backup.clone())])
            );
        }
    }
    for warning in &applied.warnings {
        if already_shown.contains(warning) {
            continue;
        }
        println!("! {warning}");
    }
}

// ------------------------------------------------------------------ langkah

/// Pilih harness.
///
/// Penting: `add` boleh membuat config yang belum ada (server baru belum
/// pernah menjalankan harness-nya), jadi yang dicek adalah apakah harness
/// **didukung**, bukan apakah file-nya sudah ada.
fn pick_harness(
    wanted: Option<&str>,
    harnesses: &[HarnessInfo],
    interactive: bool,
) -> Result<String> {
    if let Some(id) = wanted {
        let trimmed = id.trim();
        let entry = harnesses
            .iter()
            .find(|entry| entry.id == trimmed)
            .ok_or_else(|| {
                anyhow!(i18n::fill(
                    i18n::t(Msg::HarnessUnsupported),
                    &[
                        ("id", trimmed.to_string()),
                        ("choices", output::choices(harnesses))
                    ],
                ))
            })?;
        if !entry.installed {
            eprintln!(
                "{}",
                i18n::fill(
                    i18n::t(Msg::ConfigWillBeCreated),
                    &[("name", entry.name.clone())]
                )
            );
        }
        return Ok(entry.id.clone());
    }

    let with_config: Vec<&HarnessInfo> = harnesses.iter().filter(|entry| entry.installed).collect();
    match with_config.len() {
        // Pilihan yang tidak ambigu tidak perlu ditanyakan, bahkan tanpa TTY.
        1 => Ok(with_config[0].id.clone()),
        0 => bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::NoHarnessConfig),
                &[("choices", output::choices(harnesses))]
            )
        ),
        _ if interactive => {
            let options: Vec<Choice> = with_config
                .iter()
                .map(|entry| Choice {
                    id: entry.id.clone(),
                    label: format!(
                        "{} — {}",
                        entry.name,
                        i18n::fill(
                            i18n::t(Msg::ProviderCountLabel),
                            &[("count", entry.provider_count.to_string())]
                        )
                    ),
                })
                .collect();
            let picked = ask(inquire::Select::new(i18n::t(Msg::PickHarness), options)
                .with_help_message(i18n::t(Msg::PickHarnessHint))
                .with_page_size(10)
                .prompt())?;
            Ok(picked.id)
        }
        _ => bail!(
            "{}",
            i18n::fill(
                i18n::t(Msg::AmbiguousHarness),
                &[
                    ("count", with_config.len().to_string()),
                    (
                        "list",
                        with_config
                            .iter()
                            .map(|entry| entry.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ]
            )
        ),
    }
}

/// `Ok(None)` = pengguna membatalkan.
fn pick_target(
    options: &AddOptions,
    presets: &[ProviderPreset],
    installed: &[String],
    interactive: bool,
) -> Result<Option<Target>> {
    if let Some(id) = options.provider.as_deref() {
        let trimmed = id.trim();
        if let Some(preset) = presets
            .iter()
            .find(|preset| preset.id.eq_ignore_ascii_case(trimmed))
        {
            return Ok(Some(Target::Preset(Box::new(preset.clone()))));
        }
        // Bukan preset: kalau pengguna menyebut --name atau --base-url, niatnya
        // jelas custom, jadi biarkan validasi custom yang memberi pesan spesifik
        // ("--base-url wajib") alih-alih "provider tidak ada di katalog".
        if trimmed != CUSTOM_ID && (options.name.is_some() || options.base_url.is_some()) {
            return Ok(Some(Target::Custom {
                id: trimmed.to_string(),
            }));
        }
        if trimmed == CUSTOM_ID {
            bail!("{}", i18n::t(Msg::NeedCustomId));
        }
        // Bukan preset dan bukan custom: laporkan dengan saran kandidat
        // terdekat dari katalog. Cabang `Ok` tidak mungkin (id sudah
        // dicocokkan dengan `eq_ignore_ascii_case` di atas), tapi fallback-nya
        // tetap pesan utuh, bukan string kosong.
        let message = draft::find_preset(presets, trimmed)
            .err()
            .unwrap_or_else(|| {
                i18n::fill(
                    i18n::t(Msg::UnknownProvider),
                    &[("id", trimmed.to_string())],
                )
            });
        return Err(anyhow!(message));
    }

    if !interactive {
        bail!("{}", i18n::t(Msg::NeedProviderFlag));
    }

    let options_list: Vec<Choice> = presets
        .iter()
        .map(|preset| Choice {
            id: preset.id.clone(),
            label: provider_label(preset, installed),
        })
        .collect();

    let picked = ask(
        inquire::Select::new(i18n::t(Msg::PickProvider), options_list)
            .with_help_message(i18n::t(Msg::PickProviderHint))
            .with_page_size(12)
            .prompt(),
    )?;

    let preset = presets
        .iter()
        .find(|preset| preset.id == picked.id)
        .ok_or_else(|| anyhow!("provider '{}' vanished from the catalog", picked.id))?;

    if installed.iter().any(|id| id == &preset.id) {
        let choices = vec![
            i18n::t(Msg::ProviderExistsEdit).to_string(),
            i18n::t(Msg::Cancel).to_string(),
        ];
        let picked_action =
            ask(inquire::Select::new(i18n::t(Msg::ProviderExists), choices).prompt())?;
        if picked_action == i18n::t(Msg::Cancel) {
            return Ok(None);
        }
    }

    Ok(Some(Target::Preset(Box::new(preset.clone()))))
}

fn provider_label(preset: &ProviderPreset, installed: &[String]) -> String {
    let mut parts = vec![preset.id.clone(), preset.name.clone()];
    if let Some(badge) = &preset.badge {
        parts.push(badge.clone());
    }
    if !preset.models.is_empty() {
        parts.push(i18n::fill(
            i18n::t(Msg::ProviderCountLabel),
            &[("count", preset.models.len().to_string())],
        ));
    }
    if installed.iter().any(|id| id == &preset.id) {
        parts.push("✓".to_string());
    }
    parts.join("  ·  ")
}

fn ask_key(target: &Target) -> Result<Option<String>> {
    let (name, hint) = match target {
        Target::Preset(preset) => {
            let mut hint = String::new();
            if let Some(env) = &preset.env_var {
                hint.push_str(&i18n::fill(
                    i18n::t(Msg::KeyHintEnv),
                    &[("env", env.clone())],
                ));
            }
            if let Some(docs) = &preset.docs_url {
                if !hint.is_empty() {
                    hint.push_str(" · ");
                }
                hint.push_str(docs);
            }
            (preset.name.clone(), hint)
        }
        Target::Custom { id } => (id.clone(), String::new()),
    };

    let question = i18n::fill(i18n::t(Msg::KeyPrompt), &[("name", name)]);
    let mut prompt = inquire::Password::new(&question)
        .without_confirmation()
        .with_display_mode(inquire::PasswordDisplayMode::Masked);
    if !hint.is_empty() {
        prompt = prompt.with_help_message(&hint);
    }

    let value = ask(prompt.prompt())?;
    let trimmed = value.trim();
    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}

/// Semua yang dibutuhkan satu kali uji koneksi, dikumpulkan supaya
/// `run_test` tidak berjumbai dengan delapan argumen.
struct TestRequest {
    base_url: String,
    api_name: String,
    provider_id: String,
    harness: String,
    key: String,
}

/// Uji koneksi. `false` = pengguna memilih berhenti.
///
/// `explicit` benar jika `--test` disebut sendiri: automation memintanya
/// sebagai gerbang sebelum menulis, jadi kegagalan harus menggagalkan
/// perintah, bukan diam-diam lanjut.
fn run_test(request: &TestRequest, interactive: bool, yes: bool, explicit: bool) -> Result<bool> {
    if request.base_url.is_empty() {
        return Ok(true);
    }

    let result = block_on(api::test_connection(
        request.base_url.clone(),
        request.api_name.clone(),
        request.key.clone(),
        Some(request.harness.clone()),
        Some(request.provider_id.clone()),
    ));

    match result {
        Ok(test) if test.ok => {
            if interactive {
                println!(
                    "{}",
                    i18n::fill(
                        i18n::t(Msg::TestOk),
                        &[(
                            "latency",
                            test.latency_ms
                                .map(|ms| format!(" · {ms} ms"))
                                .unwrap_or_default()
                        )]
                    )
                );
            }
            Ok(true)
        }
        Ok(test) => {
            eprintln!(
                "{}",
                i18n::fill(i18n::t(Msg::TestFailed), &[("message", test.message)])
            );
            continue_after_failed_test(interactive, yes, explicit)
        }
        Err(error) => {
            eprintln!(
                "{}",
                i18n::fill(i18n::t(Msg::TestFailed), &[("message", error)])
            );
            continue_after_failed_test(interactive, yes, explicit)
        }
    }
}

fn continue_after_failed_test(interactive: bool, yes: bool, explicit: bool) -> Result<bool> {
    if !interactive {
        if explicit {
            // `--test` eksplisit adalah syarat yang diminta automation;
            // gagal berarti gagal, bukan lanjut menulis config rusak.
            bail!("{}", i18n::t(Msg::TestFailedAbort));
        }
        // Default non-interaktif (tanpa --test): jangan menggagalkan
        // automation hanya karena endpoint sedang tidak bisa dihubungi.
        // Peringatan sudah dicetak ke stderr.
        return Ok(true);
    }
    if yes {
        return Ok(true);
    }
    let proceed = ask(inquire::Confirm::new(i18n::t(Msg::ConfirmAfterFailedTest))
        .with_default(false)
        .prompt())?;
    Ok(proceed)
}

fn pick_models(
    options: &AddOptions,
    target: &Target,
    harness: &str,
    key: Option<&str>,
    interactive: bool,
) -> Result<Vec<ModelDraft>> {
    let preset = match target {
        Target::Preset(preset) => Some(preset.as_ref()),
        Target::Custom { .. } => None,
    };

    // `--models` selalu menang: id yang disebut eksplisit tidak butuh jaringan.
    if let Some(raw) = &options.models {
        let models = draft::models_from_list(raw, preset);
        if models.is_empty() {
            bail!("{}", i18n::t(Msg::ModelsFlagEmpty));
        }
        return Ok(models);
    }

    if wants_sync(options, preset, key, interactive)? {
        match sync_models(target, options, harness, key) {
            Ok(live) if !live.is_empty() => {
                let models = draft::models_from_sync(live, preset);
                return if interactive {
                    ask_models(&models)
                } else {
                    Ok(models)
                };
            }
            Ok(_) => {
                // Endpoint hidup tapi tidak mengembalikan model. Interaktif
                // masih bisa lanjut dengan daftar preset/manual; `--sync`
                // berarti pengguna memang meminta daftar live, jadi gagal.
                if !interactive {
                    bail!("{}", i18n::t(Msg::SyncHardEmpty));
                }
                eprintln!("{}", i18n::t(Msg::SyncEmpty));
            }
            Err(error) => {
                if !interactive {
                    bail!(
                        "{}",
                        i18n::fill(
                            i18n::t(Msg::SyncHardFailed),
                            &[("error", error.to_string())]
                        )
                    );
                }
                eprintln!(
                    "{}",
                    i18n::fill(i18n::t(Msg::SyncFailed), &[("error", error.to_string())])
                );
            }
        }
    }

    // Tidak sync (ditolak, gagal, atau non-interaktif tanpa `--sync`): pakai
    // daftar model preset kalau ada.
    if let Some(preset) = preset {
        if !preset.models.is_empty() {
            let models = draft::models_from_preset(preset);
            return if interactive {
                ask_models(&models)
            } else {
                Ok(models)
            };
        }
    }

    // Preset tanpa daftar model dan tidak ada yang bisa di-sync.
    if !interactive {
        bail!("{}", i18n::t(Msg::NeedModelsFlag));
    }

    let raw = ask(inquire::Text::new(i18n::t(Msg::ModelsTextPrompt))
        .with_help_message(i18n::t(Msg::ModelsTextHint))
        .prompt())?;
    let models = draft::models_from_list(&raw, preset);
    if models.is_empty() {
        bail!("{}", i18n::t(Msg::ModelsEmpty));
    }
    Ok(models)
}

/// Perlu ambil daftar model dari endpoint?
///
/// `--sync` memintanya tanpa bertanya. Tanpa flag, keputusan diambil di
/// terminal: key yang baru ditempel dan endpoint yang bisa dihubungi sudah
/// cukup alasan untuk menawarkan daftar terbaru, karena daftar preset gampang
/// tertinggal dari katalog provider.
fn wants_sync(
    options: &AddOptions,
    preset: Option<&ProviderPreset>,
    key: Option<&str>,
    interactive: bool,
) -> Result<bool> {
    if options.sync {
        return Ok(true);
    }
    if !interactive {
        return Ok(false);
    }
    // Provider ber-key butuh key untuk sync; tanpa key permintaannya pasti 401,
    // jadi lebih baik langsung ke daftar preset.
    let needs_key = preset.map(|preset| preset.key_required).unwrap_or(false);
    if key.is_none() && needs_key {
        return Ok(false);
    }
    ask(inquire::Confirm::new(i18n::t(Msg::SyncPrompt))
        .with_default(true)
        .with_help_message(i18n::t(Msg::SyncHint))
        .prompt())
}

fn sync_models(
    target: &Target,
    options: &AddOptions,
    harness: &str,
    key: Option<&str>,
) -> Result<Vec<hakonfig_core::api::ModelOption>> {
    let (base_url, api_name) = endpoint(target, options);
    block_on(api::fetch_models(
        base_url,
        api_name,
        key.unwrap_or_default().to_string(),
        Some(harness.to_string()),
        Some(target_id(target).to_string()),
    ))
    .map_err(|error| anyhow!(error))
}

/// MultiSelect dengan semua model terpilih — perilaku yang sama dengan GUI,
/// yang mencentang setiap model hasil sync.
fn ask_models(models: &[ModelDraft]) -> Result<Vec<ModelDraft>> {
    let choices: Vec<Choice> = models
        .iter()
        .map(|model| Choice {
            id: model.id.clone(),
            label: match model.context_window {
                Some(window) => format!("{}  ·  {} ctx", model.id, window),
                None => model.id.clone(),
            },
        })
        .collect();
    let picked = ask(inquire::MultiSelect::new(i18n::t(Msg::PickModels), choices)
        .with_all_selected_by_default()
        .with_help_message(i18n::t(Msg::PickModelsHintAll))
        .prompt())?;
    if picked.is_empty() {
        bail!("{}", i18n::t(Msg::ModelsEmpty));
    }
    // Metadata dari daftar asal dibawa, bukan dibuat ulang dari id.
    Ok(picked
        .into_iter()
        .filter_map(|choice| models.iter().find(|model| model.id == choice.id).cloned())
        .collect())
}

fn build_draft(
    target: &Target,
    key: Option<&str>,
    mode: KeyMode,
    models: Vec<ModelDraft>,
    options: &AddOptions,
) -> Result<ProviderDraft> {
    match target {
        Target::Preset(preset) => Ok(draft::draft_from_preset(preset, key, mode, models)),
        Target::Custom { id } => draft::draft_custom(
            id,
            options.name.as_deref(),
            options.base_url.as_deref(),
            options.api.as_deref(),
            key,
            mode,
            models,
        )
        .map_err(|error| anyhow!(error)),
    }
}

// ------------------------------------------------------------------ bantuan

/// Bungkus prompt supaya Esc/Ctrl-C menjadi [`Cancelled`], bukan error biasa.
fn ask<T>(result: Result<T, inquire::InquireError>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Err(anyhow::Error::new(Cancelled)),
        Err(error) => Err(anyhow::Error::new(error)),
    }
}

fn target_id(target: &Target) -> &str {
    match target {
        Target::Preset(preset) => &preset.id,
        Target::Custom { id } => id,
    }
}

/// Endpoint yang akan dipakai: dari preset, atau dari flag untuk custom.
fn endpoint(target: &Target, options: &AddOptions) -> (String, String) {
    match target {
        Target::Preset(preset) => (preset.base_url.clone(), preset.api.clone()),
        Target::Custom { .. } => (
            options.base_url.clone().unwrap_or_default(),
            options
                .api
                .clone()
                .unwrap_or_else(|| draft::DEFAULT_API.to_string()),
        ),
    }
}

#[derive(Clone)]
struct Choice {
    id: String,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

/// Baca stdin sampai habis (untuk `--key-stdin`).
///
/// Kalau stdin sebuah terminal, perintah ini akan menggantung tanpa penjelasan,
/// jadi ditolak lebih awal dengan pesan yang menunjukkan cara yang benar.
pub fn read_stdin_trimmed() -> Result<String> {
    if std::io::stdin().is_terminal() {
        bail!("{}", i18n::t(Msg::KeyStdinNeedsPipe));
    }
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| {
            anyhow!(i18n::fill(
                i18n::t(Msg::StdinFailed),
                &[("error", error.to_string())]
            ))
        })?;
    Ok(buffer.trim().to_string())
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime")
        .block_on(future)
}
