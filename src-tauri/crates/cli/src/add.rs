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
    ApplyResult, EditOp, HarnessInfo, KeyMode, ModelDraft, ModelSummary, ProviderDraft,
    ProviderPreset, ProviderSummary,
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
    /// Paksa ganti key saja (provider harus sudah ada).
    pub key_only: bool,
    /// Paksa ganti daftar model saja.
    pub models_only: bool,
    /// Jadikan model ini default harness, tanpa menyentuh yang lain.
    pub default_model: Option<String>,
    /// Lewati menu provider-terpasang dan tulis ulang seluruh provider.
    pub everything: bool,
    /// Jangan tanya/simpan key: pakai kredensial yang sudah ada.
    pub keep_key: bool,
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

/// Identitas lengkap endpoint custom. `id` sudah final (tidak bentrok dengan
/// preset, provider terpasang, atau sentinel `custom`).
struct CustomSpec {
    id: String,
    name: String,
    base_url: String,
    api: String,
}

/// Hasil pemilihan provider: target plus identitas custom bila ada.
struct PickedTarget {
    target: Target,
    custom: Option<CustomSpec>,
}

impl PickedTarget {
    fn preset(preset: &ProviderPreset) -> Self {
        Self {
            target: Target::Preset(Box::new(preset.clone())),
            custom: None,
        }
    }

    fn custom(spec: CustomSpec) -> Self {
        Self {
            target: Target::Custom {
                id: spec.id.clone(),
            },
            custom: Some(spec),
        }
    }
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
    /// "upsert" | "set_key" | "set_models" | "set_default"
    action: String,
    model_count: usize,
    applied: bool,
    files_written: Vec<String>,
    backup_dir: Option<String>,
    warnings: Vec<String>,
    notices: Vec<hakonfig_core::engine::model::Notice>,
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
    let PickedTarget { target, custom } =
        match pick_target(&options, &presets, &installed_ids, interactive)? {
            Some(picked) => picked,
            None => return Err(anyhow::Error::new(Cancelled)),
        };

    let provider_id = target_id(&target).to_string();
    let existing = snapshot
        .providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .cloned();

    // Provider yang sudah ada tidak harus lewat jalur "tulis ulang semuanya":
    // ganti key, ganti model, dan jadikan default adalah tiga aksi terpisah.
    if let Some(existing) = existing.as_ref() {
        let narrow = options.key_only
            || options.models_only
            || options.default_model.is_some()
            || options.everything;
        if interactive && !narrow {
            // `add` menambah, jadi provider yang sudah ada dibuka di pintu
            // kelolanya (`providers`) alih-alih ditawari menu yang berbeda.
            return crate::manage::open_provider(&harness, &provider_id);
        }
        match pick_existing_action(&options, &provider_id)? {
            ExistingChoice::Action(action) => {
                return run_existing_action(
                    &options,
                    action,
                    &harness,
                    &target,
                    existing,
                    interactive,
                );
            }
            // "Change everything" memang lanjut ke alur `add` biasa; Cancel
            // tidak boleh ikut jatuh ke sana — dulu begitu, jadi pengguna yang
            // membatalkan tetap dimintai API key.
            ExistingChoice::Everything => {}
        }
    }

    // Identitas provider custom dilengkapi lebih dulu: tanpa base URL, langkah
    // key/model tidak ada artinya, jadi pesan errornya harus muncul duluan.
    // `--provider <id>` yang bukan preset memakai id itu apa adanya; id yang
    // tidak disebut diturunkan dari nama (slug), dengan akhiran acak bila sudah
    // dipakai preset/provider lain.
    let custom = match (&target, custom) {
        (Target::Custom { id }, None) => Some(resolve_custom(
            Some(id.clone()),
            &options,
            &presets,
            &installed_ids,
            interactive,
        )?),
        (_, spec) => spec,
    };

    let stored_key = api::stored_key(harness.clone(), provider_id.clone())
        .ok()
        .filter(|value| !value.is_empty());
    let key = if options.keep_key {
        None
    } else {
        match options.key.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => Some(value.to_string()),
            _ if interactive => ask_key(&target)?,
            _ => None,
        }
    };

    let (base_url, api_name) = endpoint(&target, &options, custom.as_ref());

    let should_test = options.test.unwrap_or(interactive);
    if should_test
        && !run_test(
            &TestRequest {
                base_url,
                api_name,
                provider_id: provider_id.clone(),
                harness: harness.clone(),
                key: key
                    .clone()
                    .or_else(|| {
                        if options.keep_key {
                            stored_key.clone()
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default(),
            },
            interactive,
            options.yes,
            options.test == Some(true),
        )?
    {
        return Ok(());
    }

    let models = pick_models_with_key_source(
        &options,
        &target,
        custom.as_ref(),
        &harness,
        key.as_deref(),
        ModelSource {
            existing: existing.as_ref(),
            stored_key: options.keep_key && stored_key.is_some(),
        },
        interactive,
    )?;

    // Draft dibangun ulang penuh di sini (validasi custom sudah dijalankan
    // lebih awal supaya pesan errornya muncul sebelum langkah key/model),
    // jadi ini sekadar melengkapi key dan model yang sudah terkumpul.
    let draft = build_draft(
        &target,
        custom.as_ref(),
        key.as_deref(),
        options.key_mode,
        models,
    )?;
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
                action: "upsert".to_string(),
                model_count,
                applied: false,
                files_written: Vec::new(),
                backup_dir: None,
                warnings: preview.warnings.clone(),
                notices: Vec::new(),
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
            harness: harness.clone(),
            provider: provider_id.clone(),
            action: "upsert".to_string(),
            model_count,
            applied: true,
            files_written: applied.files_written.clone(),
            backup_dir: applied.backup_dir.clone(),
            warnings: applied.warnings.clone(),
            notices: applied.notices.clone(),
        })?;
    } else {
        // Warning yang sudah tampil di pratinjau tidak diulang di bawah.
        report(&applied, &preview.warnings);
    }

    // Selesai menambah, langsung buka detail provider-nya — sama seperti GUI
    // yang membuka panel provider begitu provider baru disimpan.
    if interactive && !options.json && !options.yes {
        crate::manage::open_provider(&harness, &provider_id)?;
    }
    Ok(())
}

pub(crate) fn report(applied: &ApplyResult, already_shown: &[String]) {
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

    // Notice berarti config sudah ditulis tetapi harness-nya belum tentu
    // menampilkannya (scope/enabledModels, daftar disable, id bentrok).
    if applied.notices.iter().any(|notice| notice.fix.is_some()) {
        println!("{}", i18n::t(Msg::NoticeHint));
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

/// Hasil pemilihan aksi untuk provider yang sudah terpasang.
enum ExistingChoice {
    /// Jalankan aksi khusus (ganti key/model/default).
    Action(ExistingAction),
    /// Tulis ulang seluruh provider lewat alur `add` biasa.
    Everything,
}

/// Aksi yang dipilih untuk provider yang sudah terpasang.
enum ExistingAction {
    Key,
    Models,
    Default(String),
}

/// Tentukan aksi untuk provider yang sudah ada.
///
/// Tanpa flag dan tanpa terminal, `add` hanya mengganti key kalau memang hanya
/// key yang diberikan: mengganti key tidak boleh memaksa menulis ulang daftar
/// model (itu yang membuat `--key-stdin` dulu menimpa model dengan daftar
/// preset).
fn pick_existing_action(options: &AddOptions, _provider_id: &str) -> Result<ExistingChoice> {
    if let Some(model) = options.default_model.as_deref().map(str::trim) {
        if !model.is_empty() {
            return Ok(ExistingChoice::Action(ExistingAction::Default(
                model.to_string(),
            )));
        }
    }
    if options.key_only {
        return Ok(ExistingChoice::Action(ExistingAction::Key));
    }
    if options.models_only {
        return Ok(ExistingChoice::Action(ExistingAction::Models));
    }
    if options.everything {
        return Ok(ExistingChoice::Everything);
    }

    let changes_endpoint =
        options.name.is_some() || options.base_url.is_some() || options.api.is_some();
    let changes_models = options.models.is_some() || options.sync;

    Ok(if changes_endpoint || changes_models {
        ExistingChoice::Everything
    } else if options.key.is_some() {
        ExistingChoice::Action(ExistingAction::Key)
    } else {
        ExistingChoice::Everything
    })
}

/// Jalankan aksi yang tidak menulis ulang seluruh provider.
fn run_existing_action(
    options: &AddOptions,
    action: ExistingAction,
    harness: &str,
    target: &Target,
    existing: &hakonfig_core::engine::model::ProviderSummary,
    interactive: bool,
) -> Result<()> {
    let provider_id = existing.id.clone();

    let op = match action {
        ExistingAction::Key => {
            let key = match options.key.as_deref().map(str::trim) {
                Some(value) if !value.is_empty() => value.to_string(),
                _ if interactive => ask_key(target)?.ok_or_else(|| {
                    anyhow!(i18n::fill(
                        i18n::t(Msg::KeyRequired),
                        &[("provider", provider_id.clone())]
                    ))
                })?,
                _ => bail!(
                    "{}",
                    i18n::fill(
                        i18n::t(Msg::KeyRequired),
                        &[("provider", provider_id.clone())]
                    )
                ),
            };
            EditOp::SetKey {
                provider_id: provider_id.clone(),
                key,
                key_mode: options.key_mode,
            }
        }
        ExistingAction::Models => {
            // Key yang sudah tersimpan dipakai untuk sync: mengganti daftar
            // model tidak boleh memaksa mengetik ulang API key.
            let (key, stored) = match options.key.as_deref().map(str::trim) {
                Some(value) if !value.is_empty() => (Some(value.to_string()), false),
                _ => (
                    api::stored_key(harness.to_string(), provider_id.clone())
                        .ok()
                        .filter(|value| !value.is_empty()),
                    true,
                ),
            };
            let models = pick_models_with_key_source(
                options,
                target,
                None,
                harness,
                key.as_deref(),
                ModelSource {
                    existing: Some(existing),
                    stored_key: stored,
                },
                interactive,
            )?;
            EditOp::SetModels {
                provider_id: provider_id.clone(),
                models,
            }
        }
        ExistingAction::Default(chosen) => {
            let model_id = if !chosen.is_empty() {
                chosen
            } else if existing.models.is_empty() {
                bail!(
                    "{}",
                    i18n::fill(
                        i18n::t(Msg::NoModelsToPick),
                        &[("provider", provider_id.clone())]
                    )
                );
            } else {
                let choices: Vec<Choice> = existing
                    .models
                    .iter()
                    .map(|model| Choice {
                        id: model.id.clone(),
                        label: model.name.clone().unwrap_or_else(|| model.id.clone()),
                    })
                    .collect();
                let picked =
                    ask(inquire::Select::new(i18n::t(Msg::PickDefaultModel), choices).prompt())?;
                picked.id
            };
            EditOp::SetDefault {
                provider_id: provider_id.clone(),
                model_id,
            }
        }
    };

    let action_label = action_name(&op).to_string();
    let preview =
        api::preview_edit(harness.to_string(), op.clone()).map_err(|error| anyhow!(error))?;
    if !options.json {
        output::print_preview(&preview);
    }
    if options.dry_run {
        if options.json {
            output::print_json(&report_json(
                harness,
                &provider_id,
                &action_label,
                false,
                Vec::new(),
                None,
                preview.warnings.clone(),
            ))?;
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

    let applied = api::apply_edit(harness.to_string(), op).map_err(|error| anyhow!(error))?;
    if options.json {
        output::print_json(&report_json(
            harness,
            &provider_id,
            &action_label,
            true,
            applied.files_written.clone(),
            applied.backup_dir.clone(),
            applied.warnings.clone(),
        ))?;
    } else {
        report(&applied, &preview.warnings);
    }
    Ok(())
}

fn action_name(op: &EditOp) -> &'static str {
    match op {
        EditOp::SetKey { .. } => "set_key",
        EditOp::SetModels { .. } => "set_models",
        EditOp::SetDefault { .. } => "set_default",
        _ => "upsert",
    }
}

fn report_json(
    harness: &str,
    provider: &str,
    action: &str,
    applied: bool,
    files_written: Vec<String>,
    backup_dir: Option<String>,
    warnings: Vec<String>,
) -> AddReport {
    AddReport {
        harness: harness.to_string(),
        provider: provider.to_string(),
        action: action.to_string(),
        model_count: 0,
        applied,
        files_written,
        backup_dir,
        warnings,
        notices: Vec::new(),
    }
}

/// `Ok(None)` = pengguna membatalkan.
fn pick_target(
    options: &AddOptions,
    presets: &[ProviderPreset],
    installed: &[String],
    interactive: bool,
) -> Result<Option<PickedTarget>> {
    if let Some(id) = options.provider.as_deref() {
        let trimmed = id.trim();
        // `custom` berarti id tidak ditentukan: turunkan dari nama.
        if trimmed.eq_ignore_ascii_case(CUSTOM_ID) {
            let spec = resolve_custom(None, options, presets, installed, interactive)?;
            return Ok(Some(PickedTarget::custom(spec)));
        }
        if let Some(preset) = presets
            .iter()
            .find(|preset| preset.id.eq_ignore_ascii_case(trimmed))
        {
            return Ok(Some(PickedTarget::preset(preset)));
        }
        // Bukan preset: kalau pengguna menyebut --name atau --base-url, niatnya
        // jelas custom. Id yang disebut dipakai apa adanya.
        if options.name.is_some() || options.base_url.is_some() {
            let spec = resolve_custom(
                Some(trimmed.to_string()),
                options,
                presets,
                installed,
                interactive,
            )?;
            return Ok(Some(PickedTarget::custom(spec)));
        }
        // Provider custom yang sudah terpasang juga sah: id-nya memang tidak ada
        // di katalog, dan aksi sempit (key/model/default) tidak butuh preset.
        if installed.iter().any(|id| id.eq_ignore_ascii_case(trimmed)) {
            return Ok(Some(PickedTarget {
                target: Target::Custom {
                    id: trimmed.to_string(),
                },
                custom: None,
            }));
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

    // Tanpa --provider: nama/base URL yang diberikan berarti endpoint custom.
    if options.name.is_some() || options.base_url.is_some() {
        let spec = resolve_custom(None, options, presets, installed, interactive)?;
        return Ok(Some(PickedTarget::custom(spec)));
    }

    if !interactive {
        bail!("{}", i18n::t(Msg::NeedProviderFlag));
    }

    let mut options_list = vec![Choice {
        id: CUSTOM_ID.to_string(),
        label: i18n::t(Msg::CustomChoice).to_string(),
    }];
    options_list.extend(presets.iter().map(|preset| Choice {
        id: preset.id.clone(),
        label: provider_label(preset, installed),
    }));

    let picked = ask(
        inquire::Select::new(i18n::t(Msg::PickProvider), options_list)
            .with_help_message(i18n::t(Msg::PickProviderHint))
            .with_page_size(12)
            .prompt(),
    )?;

    if picked.id.eq_ignore_ascii_case(CUSTOM_ID) {
        let spec = resolve_custom(None, options, presets, installed, true)?;
        return Ok(Some(PickedTarget::custom(spec)));
    }

    let preset = presets
        .iter()
        .find(|preset| preset.id == picked.id)
        .ok_or_else(|| anyhow!("provider '{}' vanished from the catalog", picked.id))?;

    Ok(Some(PickedTarget::preset(preset)))
}

/// Tipe API yang bisa ditawarkan wizard untuk endpoint custom.
const CUSTOM_APIS: &[&str] = &[
    "openai-completions",
    "openai-responses",
    "anthropic-messages",
    "google-generative-ai",
];

/// Lengkapi identitas provider custom.
///
/// `explicit_id` = id dari `--provider`. Tanpa itu, id diturunkan dari nama:
/// huruf kecil, spasi/simbol menjadi satu `-`. Kalau id itu sudah dipakai
/// preset, provider terpasang, atau sentinel `custom`, beri akhiran 5 karakter
/// acak (huruf kecil + angka) alih-alih menolak.
fn resolve_custom(
    explicit_id: Option<String>,
    options: &AddOptions,
    presets: &[ProviderPreset],
    installed: &[String],
    interactive: bool,
) -> Result<CustomSpec> {
    let name = match options
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_string(),
        None if interactive => ask_custom_text(Msg::CustomNamePrompt, None, Msg::CustomNeedsName)?,
        None => bail!("{}", i18n::t(Msg::CustomNeedsName)),
    };

    let base_url = match options
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_string(),
        None if interactive => ask_custom_text(
            Msg::CustomBaseUrlPrompt,
            Some(Msg::CustomBaseUrlHint),
            Msg::CustomNeedsBaseUrl,
        )?,
        None => bail!("{}", i18n::t(Msg::CustomNeedsBaseUrl)),
    };

    let api = match options
        .api
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value.to_string(),
        None if interactive => {
            ask(inquire::Select::new(i18n::t(Msg::CustomApiPrompt), CUSTOM_APIS.to_vec()).prompt())?
                .to_string()
        }
        None => draft::DEFAULT_API.to_string(),
    };

    let id = match explicit_id {
        Some(id) => id,
        None => unique_custom_id(&slugify(&name), presets, installed),
    };

    // Validasi yang sama dengan jalur flag: nama/base URL wajib, URL http(s),
    // id tidak boleh sentinel `custom`.
    let draft = draft::draft_custom(
        &id,
        Some(&name),
        Some(&base_url),
        Some(&api),
        None,
        options.key_mode,
        Vec::new(),
    )
    .map_err(|error| anyhow!(error))?;

    Ok(CustomSpec {
        id: draft.id,
        name: draft.name,
        base_url: draft.base_url,
        api: draft.api,
    })
}

fn ask_custom_text(prompt: Msg, hint: Option<Msg>, empty: Msg) -> Result<String> {
    let mut question = inquire::Text::new(i18n::t(prompt));
    if let Some(hint) = hint {
        question = question.with_help_message(i18n::t(hint));
    }
    let value = ask(question.prompt())?;
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{}", i18n::t(empty));
    }
    Ok(value)
}

/// `My GW (prod)` -> `my-gw-prod`. Selalu menghasilkan sesuatu yang tidak
/// kosong supaya id tidak pernah berupa string kosong.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "provider".to_string()
    } else {
        out
    }
}

/// 5 karakter huruf kecil + angka. Cukup acak untuk membedakan id; bukan
/// kredensial, jadi tidak perlu CSPRNG.
fn random_suffix(len: usize) -> String {
    use std::hash::{BuildHasher, Hasher};

    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0),
    );
    let mut out = String::with_capacity(len);
    for index in 0..len {
        hasher.write_usize(index);
        let value = hasher.finish();
        out.push(ALPHABET[(value % ALPHABET.len() as u64) as usize] as char);
    }
    out
}

fn custom_id_taken(id: &str, presets: &[ProviderPreset], installed: &[String]) -> bool {
    id.eq_ignore_ascii_case(CUSTOM_ID)
        || installed.iter().any(|other| other.eq_ignore_ascii_case(id))
        || presets
            .iter()
            .any(|preset| preset.id.eq_ignore_ascii_case(id))
}

fn unique_custom_id(base: &str, presets: &[ProviderPreset], installed: &[String]) -> String {
    if !custom_id_taken(base, presets, installed) {
        return base.to_string();
    }
    for _ in 0..32 {
        let candidate = format!("{base}-{}", random_suffix(5));
        if !custom_id_taken(&candidate, presets, installed) {
            return candidate;
        }
    }
    format!("{base}-{}", random_suffix(16))
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

/// Konteks tambahan saat memilih model: metadata lama (untuk mempertahankan
/// angka yang sudah ada) dan asal key (untuk petunjuk prompt sync).
struct ModelSource<'a> {
    existing: Option<&'a ProviderSummary>,
    stored_key: bool,
}

fn pick_models_with_key_source(
    options: &AddOptions,
    target: &Target,
    custom: Option<&CustomSpec>,
    harness: &str,
    key: Option<&str>,
    source: ModelSource<'_>,
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
        // `--models` non-interaktif tidak boleh berhenti di sini: metadata
        // (reasoning dari katalog/pola, context lama) tetap dilengkapi.
        return finish_models(models, target, source.existing, false);
    }

    if wants_sync(options, preset, key, interactive, source.stored_key)? {
        match sync_models(target, options, custom, harness, key) {
            Ok(live) if !live.is_empty() => {
                let models = draft::models_from_sync(live, preset);
                return finish_models(models, target, source.existing, interactive);
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
            return finish_models(models, target, source.existing, interactive);
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
    finish_models(models, target, source.existing, true)
}

/// Lengkapi metadata model sebelum ditulis (atau dipilih di wizard).
///
/// `reasoning` diambil dari `api::reasoning_hints` — katalog harness lebih dulu,
/// lalu preset, lalu pola id — sama dengan tombol "Detect reasoning" di GUI.
/// Untuk model yang sudah ada di config, `contextWindow`/`maxTokens`/`reasoning`
/// lama dipertahankan bila sumber baru tidak membawanya: `--models` dan tulis
/// ulang tidak boleh menghapus angka yang sudah benar.
fn finish_models(
    models: Vec<ModelDraft>,
    target: &Target,
    existing: Option<&ProviderSummary>,
    interactive: bool,
) -> Result<Vec<ModelDraft>> {
    let mut models = models;
    enrich_models(&mut models, target_id(target), existing);
    if interactive {
        ask_models(&models)
    } else {
        Ok(models)
    }
}

fn enrich_models(models: &mut [ModelDraft], provider_id: &str, existing: Option<&ProviderSummary>) {
    use std::collections::BTreeMap;

    let ids: Vec<String> = models.iter().map(|model| model.id.clone()).collect();
    let hints: BTreeMap<String, bool> = api::reasoning_hints(provider_id.to_string(), ids)
        .unwrap_or_default()
        .into_iter()
        .map(|hint| (hint.id, hint.reasoning))
        .collect();
    let previous: BTreeMap<&str, &ModelSummary> = existing
        .map(|provider| {
            provider
                .models
                .iter()
                .map(|model| (model.id.as_str(), model))
                .collect()
        })
        .unwrap_or_default();

    for model in models.iter_mut() {
        // Hanya menyalakan: config yang sudah menandai reasoning tidak dimatikan
        // hanya karena id-nya tidak cocok dengan pola mana pun.
        if hints.get(&model.id).copied().unwrap_or(false) {
            model.reasoning = true;
        }
        if let Some(old) = previous.get(model.id.as_str()) {
            if model.context_window.is_none() {
                model.context_window = old.context_window;
            }
            if model.max_tokens.is_none() {
                model.max_tokens = old.max_tokens;
            }
            if !model.reasoning {
                model.reasoning = old.reasoning;
            }
        }
    }
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
    stored_key: bool,
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
    let (prompt, hint) = if stored_key {
        (Msg::SyncPromptStored, Msg::SyncHintStored)
    } else {
        (Msg::SyncPrompt, Msg::SyncHint)
    };
    ask(inquire::Confirm::new(i18n::t(prompt))
        .with_default(true)
        .with_help_message(i18n::t(hint))
        .prompt())
}

fn sync_models(
    target: &Target,
    options: &AddOptions,
    custom: Option<&CustomSpec>,
    harness: &str,
    key: Option<&str>,
) -> Result<Vec<hakonfig_core::api::ModelOption>> {
    let (base_url, api_name) = endpoint(target, options, custom);
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
            label: {
                let mut label = model.id.clone();
                if let Some(window) = model.context_window {
                    label.push_str(&format!("  ·  {window} ctx"));
                }
                if model.reasoning {
                    label.push_str("  ·  thinking");
                }
                label
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
    custom: Option<&CustomSpec>,
    key: Option<&str>,
    mode: KeyMode,
    models: Vec<ModelDraft>,
) -> Result<ProviderDraft> {
    match target {
        Target::Preset(preset) => Ok(draft::draft_from_preset(preset, key, mode, models)),
        Target::Custom { id } => {
            let spec = custom.ok_or_else(|| anyhow!("custom provider identity missing"))?;
            draft::draft_custom(
                id,
                Some(&spec.name),
                Some(&spec.base_url),
                Some(&spec.api),
                key,
                mode,
                models,
            )
            .map_err(|error| anyhow!(error))
        }
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
fn endpoint(
    target: &Target,
    options: &AddOptions,
    custom: Option<&CustomSpec>,
) -> (String, String) {
    match target {
        Target::Preset(preset) => (preset.base_url.clone(), preset.api.clone()),
        Target::Custom { .. } => match custom {
            Some(spec) => (spec.base_url.clone(), spec.api.clone()),
            None => (
                options.base_url.clone().unwrap_or_default(),
                options
                    .api
                    .clone()
                    .unwrap_or_else(|| draft::DEFAULT_API.to_string()),
            ),
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> AddOptions {
        AddOptions {
            harness: None,
            provider: None,
            name: None,
            base_url: None,
            api: None,
            key: None,
            key_mode: KeyMode::Env,
            models: None,
            sync: false,
            key_only: false,
            models_only: false,
            default_model: None,
            everything: false,
            keep_key: false,
            dry_run: false,
            yes: false,
            test: None,
            json: false,
        }
    }

    fn choice(options: &AddOptions) -> &'static str {
        match pick_existing_action(options, "deepseek").unwrap() {
            ExistingChoice::Action(ExistingAction::Key) => "key",
            ExistingChoice::Action(ExistingAction::Models) => "models",
            ExistingChoice::Action(ExistingAction::Default(_)) => "default",
            ExistingChoice::Everything => "everything",
        }
    }

    #[test]
    fn scripted_actions_pick_the_narrowest_one() {
        // Hanya key: niatnya rotasi key, model tidak boleh ditimpa preset.
        let mut opts = options();
        opts.key = Some("sk-baru".into());
        assert_eq!(choice(&opts), "key");

        // Flag eksplisit menang atas tebakan.
        opts.models_only = true;
        assert_eq!(choice(&opts), "models");

        opts.models_only = false;
        opts.key_only = true;
        assert_eq!(choice(&opts), "key");

        opts.key_only = false;
        opts.default_model = Some("deepseek-v4-pro".into());
        assert_eq!(choice(&opts), "default");
    }

    #[test]
    fn scripted_model_or_endpoint_changes_stay_a_full_upsert() {
        let mut opts = options();
        opts.models = Some("a,b".into());
        assert_eq!(choice(&opts), "everything");

        let mut opts = options();
        opts.sync = true;
        assert_eq!(choice(&opts), "everything");

        let mut opts = options();
        opts.base_url = Some("https://gw.test/v1".into());
        assert_eq!(choice(&opts), "everything");

        // Tanpa apa pun yang bisa dijadikan aksi sempit: tetap upsert.
        assert_eq!(choice(&options()), "everything");
    }
}
