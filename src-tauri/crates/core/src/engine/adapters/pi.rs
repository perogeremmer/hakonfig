//! Adapter Pi (`~/.pi/agent`).
//!
//! Pi menyimpan katalog di `models.json` (JSONC, mendukung komentar) dan
//! kredensial di `auth.json` - dua file terpisah yang harus konsisten:
//!   * katalog : `providers.<id>` = { name, baseUrl, api, apiKey?, models[] }
//!   * kredensial: `auth.json.<id>` = { type: "api_key", key }
//!
//! Pi mendahulukan kredensial tersimpan di atas referensi environment, jadi
//! Hakonfig tidak pernah menulis keduanya sekaligus.
use anyhow::Result;
use serde_json::{Map, Value};
use std::path::PathBuf;

use super::shared;
use super::Adapter;
use crate::engine::doc::{get_str, mask_key, Docs, FileSpec, Format};
use crate::engine::glob;
use crate::engine::model::{
    CatalogTarget, ConfigFileInfo, EditOp, HarnessSnapshot, KeyMode, Notice, ProviderSummary,
};
use crate::engine::paths;

pub struct PiAdapter;

/// Seberapa jauh scope `enabledModels` boleh dilebarkan saat config berubah.
#[derive(Clone, Copy)]
enum Ensure<'a> {
    /// Hanya buang pola yang sudah mati.
    Nothing,
    /// Provider yang disentuh dijamin tercakup seluruhnya (`<id>/**`).
    Provider,
    /// Hanya model tertentu yang dijamin tercakup (`<id>/<model>`).
    Models(&'a [String]),
}

impl PiAdapter {
    fn agent_dir() -> PathBuf {
        paths::pi_dir().join("agent")
    }

    /// Pola `enabledModels` yang dipakai pi, kalau pengguna memakai scope.
    ///
    /// pi menyimpan daftar ini di `settings.json` saat pengguna menekan Ctrl+S
    /// di pemilih model. Key yang tidak ada berarti "semua model", bukan "tidak
    /// ada model", jadi bedanya penting.
    fn enabled_models(settings: &Map<String, Value>) -> Option<Vec<String>> {
        let list = settings.get("enabledModels")?.as_array()?;
        let patterns: Vec<String> = list
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .map(str::to_string)
            .collect();
        if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        }
    }

    /// Jaga `defaultProvider`/`defaultModel` tetap menunjuk provider yang aktif.
    ///
    /// pi memilih model awal dari default ini. Kalau default menunjuk provider
    /// yang dinonaktifkan, pi diam-diam jatuh ke model lain dan pengguna tidak
    /// tahu kenapa; kalau semua provider nonaktif, default yang tertinggal cuma
    /// janji kosong di config. Jadi default ikut dipindah (atau dikosongkan)
    /// setiap kali toggle mengubah kumpulan provider aktif.
    fn sync_default(docs: &mut Docs, touched: &str, enabled: bool) -> Result<Vec<String>> {
        let Some(exists) = docs.get("settings").map(|doc| doc.exists) else {
            return Ok(Vec::new());
        };
        if !exists {
            return Ok(Vec::new());
        }

        let current = docs
            .get("settings")
            .and_then(|doc| doc.data().get("defaultProvider"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        // Provider aktif sesudah edit, beserta model pertamanya.
        let first_active = || {
            Self::active_models(docs)
                .into_iter()
                .find(|(_, models)| !models.is_empty())
                .map(|(id, models)| (id, models[0].clone()))
        };

        let mut messages = Vec::new();
        let replacement = if enabled {
            // Default yang kosong diisi supaya harness punya titik awal lagi.
            if current.is_some() {
                None
            } else {
                let active = Self::active_models(docs);
                active
                    .get(touched)
                    .filter(|models| !models.is_empty())
                    .map(|models| (touched.to_string(), models[0].clone()))
            }
        } else if current.as_deref() == Some(touched) {
            first_active()
        } else {
            None
        };

        let mut scope_for: Option<(String, String)> = None;
        {
            let settings = docs.require_mut("settings")?;
            let root = settings.root_object();
            match replacement {
                Some((provider, model)) => {
                    root.insert(
                        "defaultProvider".to_string(),
                        Value::String(provider.clone()),
                    );
                    root.insert(
                        "defaultModel".to_string(),
                        Value::String(format!("{provider}/{model}")),
                    );
                    messages.push(if enabled {
                        format!("Default set to '{provider}/{model}' for the re-enabled provider.")
                    } else {
                        format!(
                            "Default moved to '{provider}/{model}' because '{touched}' was disabled."
                        )
                    });
                    scope_for = Some((provider, model));
                }
                None => {
                    if !enabled && current.as_deref() == Some(touched) {
                        root.remove("defaultProvider");
                        root.remove("defaultModel");
                        messages.push(format!(
                            "No provider is active any more, so the default pointing at '{touched}' was cleared. pi will choose its own model."
                        ));
                    }
                }
            }
        }

        // Default baru harus terlihat oleh scope `enabledModels`, kalau pengguna
        // memakai scope.
        if let Some((provider, model)) = scope_for {
            if let Some(summary) = Self::reconcile_scope(docs, &provider, Ensure::Models(&[model]))?
            {
                messages.push(summary);
            }
        }

        Ok(messages)
    }

    /// Peringatan bahwa provider ini juga dikenali pi secara bawaan.
    ///
    /// Katalog pi berisi puluhan provider (deepseek, openai, anthropic, groq,
    /// ...). Menonaktifkan entri kita hanya menghapus *konfigurasi* kita;
    /// provider bawaannya tetap ada selama kredensialnya ada, dan itu keputusan
    /// pi, bukan sesuatu yang bisa kita tulis ke models.json.
    fn builtin_warning(provider: &str) -> Option<String> {
        match crate::engine::version::knows_provider("pi", provider) {
            Some(true) => Some(format!(
                "pi also knows '{provider}' as a built-in provider, so it stays in pi's model list as long as a credential exists (auth.json or an environment variable). Hakonfig only removed its own entry in models.json."
            )),
            _ => None,
        }
    }

    /// Notice: default menunjuk provider yang dinonaktifkan.
    ///
    /// Bisa terjadi kalau config diedit di luar Hakonfig (atau versi lama
    /// menonaktifkan provider tanpa memindahkan default).
    fn default_notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let Some(settings) = docs.get("settings") else {
            return Vec::new();
        };
        let Some(current) = settings
            .data()
            .get("defaultProvider")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Vec::new();
        };
        let parked = docs
            .get("models")
            .map(|models| models.parked_ids())
            .unwrap_or_default();
        if !parked.iter().any(|id| id == current) {
            return Vec::new();
        }
        // Hanya laporkan kalau provider itu memang yang sedang diminta.
        if !providers.is_empty() && !providers.iter().any(|entry| entry.id == current) {
            return Vec::new();
        }

        vec![Notice {
            code: "pi.default_disabled".to_string(),
            harness: "pi".to_string(),
            provider: current.to_string(),
            message: format!(
                "settings.json still uses '{current}' as the default provider, but it is disabled in models.json. pi will silently fall back to another model."
            ),
            fix: Some("Move the default to an active provider".to_string()),
        }]
    }

    /// Referensi model yang ditulis untuk satu provider: `id/model`.
    fn model_references(models: &crate::engine::doc::Doc, provider: &str) -> Vec<String> {
        let parked = models.parked_value(provider);
        let Some(entry) = models
            .data()
            .get("providers")
            .and_then(|map| map.get(provider))
            .or(parked.as_ref())
        else {
            return Vec::new();
        };
        entry
            .get("models")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|model| get_str(model, "id"))
                    .map(|id| format!("{}/{}", provider, id.trim()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn scope_notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let Some(settings) = docs.get("settings") else {
            return Vec::new();
        };
        let Some(object) = settings.data().as_object() else {
            return Vec::new();
        };
        // Tanpa `enabledModels` pi menampilkan semua model yang punya kredensial,
        // jadi tidak ada yang perlu diperingatkan.
        let Some(patterns) = Self::enabled_models(object) else {
            return Vec::new();
        };
        let Some(models) = docs.get("models") else {
            return Vec::new();
        };

        let mut notices = Vec::new();
        for provider in providers {
            let provider = provider.id;
            let references = Self::model_references(models, provider);
            if references.is_empty() {
                continue;
            }
            let outside: Vec<&String> = references
                .iter()
                .filter(|reference| !glob::covers(&patterns, reference))
                .collect();
            if outside.is_empty() {
                continue;
            }
            let sample = outside
                .iter()
                .take(3)
                .map(|reference| reference.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let narrow = patterns.iter().any(|pattern| {
                pattern.eq_ignore_ascii_case(&glob::provider_narrow_pattern(provider))
            });
            let hint = if narrow {
                format!(
                    " The pattern '{}' only matches model ids without a slash; use '{}' instead.",
                    glob::provider_narrow_pattern(provider),
                    glob::provider_wide_pattern(provider)
                )
            } else {
                String::new()
            };
            notices.push(Notice {
                code: "pi.scope".to_string(),
                harness: "pi".to_string(),
                provider: provider.to_string(),
                message: format!(
                    "{} of {} models of '{provider}' are outside pi's enabledModels scope, so the model picker hides them ({sample}).{hint}",
                    outside.len(),
                    references.len()
                ),
                fix: Some(format!(
                    "Add '{}' to enabledModels",
                    glob::provider_wide_pattern(provider)
                )),
            });
        }
        notices
    }

    /// Provider aktif di `models.json` beserta id modelnya.
    ///
    /// Dibaca dari dokumen yang sudah dimutasi, jadi hasilnya keadaan *sesudah*
    /// edit: provider yang dinonaktifkan (diparkir) tidak termasuk.
    fn active_models(docs: &Docs) -> std::collections::BTreeMap<String, Vec<String>> {
        let mut catalog: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        let Some(models) = docs.get("models") else {
            return catalog;
        };
        let Some(providers) = models.data().get("providers").and_then(Value::as_object) else {
            return catalog;
        };
        for (id, entry) in providers {
            let ids = entry
                .get("models")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|model| get_str(model, "id"))
                        .map(|value| value.trim().to_string())
                        .collect()
                })
                .unwrap_or_default();
            catalog.insert(id.clone(), ids);
        }
        catalog
    }

    /// Buang sufiks thinking (`:high`, `:max`, …) dari satu referensi model.
    fn strip_thinking(reference: &str) -> &str {
        const LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];
        match reference.rsplit_once(':') {
            Some((head, suffix)) if LEVELS.contains(&suffix.to_lowercase().as_str()) => head,
            _ => reference,
        }
    }

    /// Apakah pola `enabledModels` masih menunjuk sesuatu yang ada.
    fn pattern_is_live(
        pattern: &str,
        catalog: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> bool {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return false;
        }
        // Tanpa garis miring: pola global (`*`) atau pola id model lintas
        // provider (`*sonnet*`). Tidak bisa dinilai, jadi dipertahankan.
        let Some((provider, reference)) = pattern.split_once('/') else {
            return true;
        };
        if provider.contains('*') || provider.contains('?') || provider.contains('[') {
            return true;
        }
        let Some(models) = catalog.get(provider) else {
            // Provider sudah tidak ada (dihapus atau dinonaktifkan).
            return false;
        };
        if reference.contains('*') || reference.contains('?') || reference.contains('[') {
            return true;
        }
        let model = Self::strip_thinking(reference);
        models.iter().any(|id| id == model)
    }

    /// Selaraskan `enabledModels` dengan isi `models.json` sesudah mutasi.
    ///
    /// pi tidak punya saklar "nonaktifkan provider": provider yang dimatikan
    /// berarti entry-nya tidak ada. Kalau scope masih menyebut provider itu, pi
    /// menyimpan pola yang tidak cocok dengan apa pun (muncul sebagai peringatan
    /// di pemilih model) dan pengguna harus memilih ulang lalu `Ctrl+S`.
    /// Karena Hakonfig yang mengubah config, Hakonfig juga yang menjaga scope-nya
    /// masuk akal:
    ///
    /// * pola yang menunjuk provider/model yang tidak ada lagi dibuang;
    /// * provider yang baru ditulis atau baru diaktifkan dijamin tercakup
    ///   (`<id>/**`), menggantikan pola sempit `<id>/*`;
    /// * scope tidak pernah dikarang kalau belum ada, dan scope yang jadi kosong
    ///   dihapus supaya pi kembali menampilkan semua model.
    fn reconcile_scope(
        docs: &mut Docs,
        touched: &str,
        ensure: Ensure<'_>,
    ) -> Result<Option<String>> {
        // File settings yang belum ada tidak dibuat hanya untuk urusan scope.
        let Some(exists) = docs.get("settings").map(|doc| doc.exists) else {
            return Ok(None);
        };
        if !exists {
            return Ok(None);
        }

        let catalog = Self::active_models(docs);
        let settings = docs.require_mut("settings")?;
        let object = settings.root_object();
        let Some(existing) = object.get("enabledModels").and_then(Value::as_array) else {
            return Ok(None);
        };
        let mut patterns: Vec<String> = existing
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        if patterns.is_empty() {
            return Ok(None);
        }

        let original = patterns.clone();
        patterns.retain(|pattern| Self::pattern_is_live(pattern, &catalog));

        let mut changes: Vec<String> = original
            .iter()
            .filter(|pattern| !patterns.contains(pattern))
            .map(|pattern| format!("dropped '{pattern}'"))
            .collect();

        if catalog.contains_key(touched) {
            let narrow = glob::provider_narrow_pattern(touched);
            match ensure {
                Ensure::Nothing => {}
                // Provider-wide: provider baru ditulis atau baru diaktifkan, jadi
                // seluruh modelnya memang dimaksudkan terlihat.
                Ensure::Provider => {
                    let wide = glob::provider_wide_pattern(touched);
                    let reference = format!("{touched}/contoh");
                    let had_narrow = patterns.iter().any(|p| p.eq_ignore_ascii_case(&narrow));
                    patterns.retain(|p| !p.eq_ignore_ascii_case(&narrow));
                    if had_narrow {
                        changes.push(format!("replaced '{narrow}'"));
                    }
                    if !glob::covers(&patterns, &reference) {
                        patterns.push(wide.clone());
                        changes.push(format!("added '{wide}'"));
                    }
                }
                // Pemilihan model eksplisit: jangan melebar ke seluruh provider,
                // cukup pastikan model yang ditulis tercakup namanya.
                Ensure::Models(ids) => {
                    let had_narrow = patterns.iter().any(|p| p.eq_ignore_ascii_case(&narrow));
                    if had_narrow {
                        patterns.retain(|p| !p.eq_ignore_ascii_case(&narrow));
                        changes.push(format!("replaced '{narrow}'"));
                    }
                    for id in ids {
                        let id = id.trim();
                        if id.is_empty() {
                            continue;
                        }
                        let reference = format!("{touched}/{id}");
                        if glob::covers(&patterns, &reference) {
                            continue;
                        }
                        patterns.push(reference.clone());
                        changes.push(format!("added '{reference}'"));
                    }
                }
            }
        }

        if changes.is_empty() {
            return Ok(None);
        }

        if patterns.is_empty() {
            // Scope kosong = "semua model"; key-nya dibuang supaya pi tidak
            // menyimpan daftar yang tidak berarti.
            object.remove("enabledModels");
            changes.push("removed empty enabledModels".to_string());
        } else {
            object.insert(
                "enabledModels".to_string(),
                Value::Array(patterns.into_iter().map(Value::String).collect()),
            );
        }

        Ok(Some(format!(
            "settings.json enabledModels: {}",
            changes.join(", ")
        )))
    }

    /// Env var yang dipakai pi untuk kredensial provider bawaan.
    ///
    /// pi membaca kredensial dari `auth.json` **atau** env; yang dari env tidak
    /// terlihat di file mana pun, jadi daftar kecil ini dipakai untuk menebak
    /// provider mana yang akan di-refresh katalognya.
    fn env_api_keys() -> &'static [(&'static str, &'static str)] {
        &[
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("cerebras", "CEREBRAS_API_KEY"),
            ("deepseek", "DEEPSEEK_API_KEY"),
            ("fireworks", "FIREWORKS_API_KEY"),
            ("google", "GEMINI_API_KEY"),
            ("groq", "GROQ_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("openai", "OPENAI_API_KEY"),
            ("openrouter", "OPENROUTER_API_KEY"),
            ("xai", "XAI_API_KEY"),
            ("zai", "ZAI_API_KEY"),
        ]
    }

    /// Provider yang punya kredensial: dari `auth.json`, atau dari environment
    /// **bila yang dikelola adalah home proses ini sendiri**.
    ///
    /// Environment yang dibaca adalah environment *runner*, bukan environment
    /// user target. Saat `--home` menunjuk home user lain (mis. agent ADE di
    /// `/var/lib/openbot`), env runner tidak ada hubungannya dengan harness
    /// itu: membaca `DEEPSEEK_API_KEY` milik operator akan melaporkan
    /// kredensial yang tidak pernah dimiliki agent tersebut. Jadi inferensi
    /// dari env hanya dilakukan untuk home sendiri.
    fn credentialed_providers(docs: &Docs) -> Vec<String> {
        let mut ids: Vec<String> = docs
            .get("auth")
            .and_then(|doc| doc.data().as_object())
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        if crate::engine::paths::explicit_home().is_none() {
            for (id, variable) in Self::env_api_keys() {
                let from_env = std::env::var(variable)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);
                if from_env && !ids.iter().any(|known| known == id) {
                    ids.push((*id).to_string());
                }
            }
        }
        ids.sort();
        ids
    }

    /// Provider yang tersimpan di `models.json`, termasuk yang diparkir.
    fn configured_providers(docs: &Docs) -> Vec<String> {
        let Some(models) = docs.get("models") else {
            return Vec::new();
        };
        let mut ids: Vec<String> = models
            .data()
            .get("providers")
            .and_then(Value::as_object)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        ids.extend(models.parked_ids());
        ids.sort();
        ids.dedup();
        ids
    }

    /// Cache katalog pi.dev (`models-store.json`), dibaca langsung karena
    /// file itu milik pi dan tidak pernah ditulis Hakonfig.
    fn catalog_cache() -> serde_json::Map<String, Value> {
        let path = Self::agent_dir().join("models-store.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    /// Notice yang tidak butuh jaringan: kredensial yang memicu refresh tanpa
    /// provider di config, dan cache katalog yang tersimpan kosong.
    fn catalog_notices(&self, docs: &Docs) -> Vec<Notice> {
        let credentialed = Self::credentialed_providers(docs);
        let configured = Self::configured_providers(docs);
        let cache = Self::catalog_cache();
        let mut notices = Vec::new();

        for provider in &credentialed {
            if !configured.iter().any(|id| id == provider) {
                notices.push(Notice {
                    code: "catalog.credential-without-provider".to_string(),
                    harness: self.id().to_string(),
                    provider: provider.clone(),
                    message: format!(
                        "pi has a credential for '{provider}', so it refreshes that provider's catalog from pi.dev, but there is no '{provider}' entry in models.json — only the built-in catalog is usable, and a failed refresh shows up as \"Could not refresh {provider}; showing cached models\"."
                    ),
                    fix: None,
                });
            }
        }

        for (provider, entry) in &cache {
            let empty = entry
                .get("models")
                .and_then(Value::as_array)
                .map(|models| models.is_empty())
                .unwrap_or(false);
            let no_timestamp = entry
                .get("lastModified")
                .and_then(Value::as_i64)
                .map(|stamp| stamp == 0)
                .unwrap_or(true);
            if empty || no_timestamp {
                notices.push(Notice {
                    code: "catalog.empty-cache".to_string(),
                    harness: self.id().to_string(),
                    provider: provider.clone(),
                    message: format!(
                        "the stored pi.dev catalog for '{provider}' is empty or has no timestamp, so pi falls back to the list shipped with the installed version. The last catalog request for it never succeeded."
                    ),
                    fix: None,
                });
            }
        }
        notices
    }
}

impl Adapter for PiAdapter {
    fn id(&self) -> &'static str {
        "pi"
    }

    fn name(&self) -> &'static str {
        "Pi"
    }

    fn notes(&self) -> &'static str {
        "Katalog di models.json, API key di auth.json. Menonaktifkan provider = mengomentarinya di models.json."
    }

    fn supports_toggle(&self) -> bool {
        true
    }

    fn specs(&self) -> Vec<FileSpec> {
        let dir = Self::agent_dir();
        vec![
            FileSpec {
                role: "models",
                path: dir.join("models.json"),
                format: Format::Jsonc,
                sensitive: false,
            },
            FileSpec {
                role: "auth",
                path: dir.join("auth.json"),
                format: Format::Json,
                sensitive: true,
            },
            FileSpec {
                role: "settings",
                path: dir.join("settings.json"),
                format: Format::Json,
                sensitive: false,
            },
        ]
    }

    fn backup_base(&self) -> PathBuf {
        Self::agent_dir()
    }

    fn snapshot(&self, docs: &Docs) -> Result<HarnessSnapshot> {
        let models = docs.require("models")?;
        let auth = docs.get("auth");
        let settings = docs.get("settings");

        let mut warnings = Vec::new();
        if models.has_comments {
            warnings.push(
                "models.json contains comments. Comments are not preserved when Hakonfig writes (a backup is created automatically)."
                    .to_string(),
            );
        }

        let mut providers = Vec::new();
        let mut active_ids: Vec<String> = Vec::new();
        if let Some(map) = models.data().get("providers").and_then(Value::as_object) {
            for (id, entry) in map {
                active_ids.push(id.clone());
                let config_key = get_str(entry, "apiKey");
                let stored_key = auth
                    .and_then(|doc| doc.data().get(id))
                    .and_then(|credential| get_str(credential, "key"));

                let inline_key = config_key
                    .as_deref()
                    .filter(|value| !value.trim_start().starts_with('$'));
                let (has_key, key_display) = match (&stored_key, &config_key) {
                    (Some(key), _) => (true, Some(mask_key(key))),
                    (None, Some(key)) => (true, Some(mask_key(key))),
                    (None, None) => (false, None),
                };
                // Yang bisa dihapus: entri auth.json atau apiKey literal.
                let key_stored = stored_key.is_some() || inline_key.is_some();

                providers.push(ProviderSummary {
                    id: id.clone(),
                    name: get_str(entry, "name").unwrap_or_else(|| id.clone()),
                    base_url: get_str(entry, "baseUrl"),
                    api: get_str(entry, "api"),
                    has_key,
                    key_display,
                    key_stored,
                    models: shared::models_from_pi_style(entry),
                    enabled: true,
                    can_toggle: self.supports_toggle(),
                });
            }
        }
        // Provider yang dinonaktifkan tetap tampil (dari blok komentar).
        for id in models.parked_ids() {
            if active_ids.contains(&id) {
                continue;
            }
            let entry = models.parked_value(&id).unwrap_or(Value::Null);
            let stored_key = auth
                .and_then(|doc| doc.data().get(&id))
                .and_then(|credential| get_str(credential, "key"));
            let config_key = get_str(&entry, "apiKey");
            let inline_key = config_key
                .as_deref()
                .filter(|value| !value.trim_start().starts_with('$'));
            let (has_key, key_display) = match (&stored_key, &config_key) {
                (Some(key), _) => (true, Some(mask_key(key))),
                (None, Some(key)) => (true, Some(mask_key(key))),
                (None, None) => (false, None),
            };
            let key_stored = stored_key.is_some() || inline_key.is_some();

            providers.push(ProviderSummary {
                id: id.clone(),
                name: get_str(&entry, "name").unwrap_or_else(|| id.clone()),
                base_url: get_str(&entry, "baseUrl"),
                api: get_str(&entry, "api"),
                has_key,
                key_display,
                // Provider nonaktif: kredensialnya masih tersimpan.
                key_stored,
                models: shared::models_from_pi_style(&entry),
                enabled: false,
                can_toggle: true,
            });
        }

        providers.sort_by(|a, b| a.id.cmp(&b.id));

        let default_provider = settings.and_then(|doc| get_str(doc.data(), "defaultProvider"));
        let default_model = settings.and_then(|doc| get_str(doc.data(), "defaultModel"));

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

        Ok(HarnessSnapshot {
            harness: self.id().to_string(),
            home: None,
            user: None,
            files,
            providers,
            default_provider,
            default_model,
            warnings,
        })
    }

    fn build(&self, docs: &mut Docs, op: &EditOp) -> Result<Vec<String>> {
        match op {
            EditOp::Upsert(draft) => {
                shared::validate_draft(draft)?;
                let mut warnings = Vec::new();

                let existing_entry = docs
                    .require("models")?
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&draft.id))
                    .cloned();
                let is_new = existing_entry.is_none();

                let key_value = draft.key_value().map(str::to_string);
                let env_var = crate::engine::doc::env_name_from(draft.id.trim());
                let mode = shared::key_mode(draft);

                // --- models.json
                let api_key_field = match (&key_value, mode) {
                    (Some(_), KeyMode::Env) => Some(format!("${}", env_var)),
                    // Key inline disimpan di auth.json; jangan sisakan apiKey
                    // lama yang bisa menimpa atau membingungkan.
                    (Some(_), KeyMode::Inline) => None,
                    // Edit tanpa key baru: **pertahankan** apiKey yang sudah ada.
                    // Menghapusnya di sini akan membuang kunci milik pengguna
                    // hanya karena ia mengganti nama atau model.
                    (None, _) => existing_entry
                        .as_ref()
                        .and_then(|entry| get_str(entry, "apiKey")),
                };

                let models = docs.require_mut("models")?;
                let entry = shared::pi_style_entry(draft, existing_entry.as_ref(), api_key_field);
                models
                    .object_at("providers")
                    .insert(draft.id.trim().to_string(), entry);

                // --- auth.json
                if let Some(key) = &key_value {
                    let auth = docs.require_mut("auth")?;
                    match mode {
                        KeyMode::Inline => {
                            let entry = serde_json::json!({
                                "type": "api_key",
                                "key": key,
                            });
                            auth.root_object()
                                .insert(draft.id.trim().to_string(), entry);
                        }
                        KeyMode::Env => {
                            if auth.data().get(draft.id.trim()).is_some() {
                                auth.root_object().remove(draft.id.trim());
                            }
                            warnings.push(format!(
                                "Environment mode: the key is not written. Set {} in your shell.",
                                env_var
                            ));
                        }
                    }
                } else if is_new {
                    warnings.push(
                        "Provider created without an API key. Fill the key in via Change so Pi can authenticate."
                            .to_string(),
                    );
                }

                // --- settings.json
                // Kalau pengguna pernah menyimpan scope model (Ctrl+S di pemilih
                // model), scope itu yang menentukan apa yang terlihat — model
                // yang baru ditulis bisa tidak muncul sama sekali. Karena kita
                // yang baru menulis modelnya, kita juga yang melebarkan scope-nya.
                if let Some(summary) =
                    Self::reconcile_scope(docs, draft.id.trim(), Ensure::Provider)?
                {
                    warnings.push(summary);
                }

                Ok(warnings)
            }
            EditOp::SetKey {
                provider_id,
                key,
                key_mode,
            } => {
                let target = provider_id.trim().to_string();
                let key = key.trim().to_string();
                if key.is_empty() {
                    anyhow::bail!("API key is empty.");
                }
                let env_var = crate::engine::doc::env_name_from(&target);
                let mut warnings = Vec::new();

                // Provider harus sudah ada: mengganti key bukan cara membuat
                // provider baru, dan tanpa entry modelnya tidak ada artinya.
                let models = docs.require("models")?;
                if models
                    .data()
                    .get("providers")
                    .and_then(|map| map.get(&target))
                    .is_none()
                {
                    anyhow::bail!("Provider '{}' not found in models.json.", target);
                }

                match key_mode {
                    KeyMode::Inline => {
                        let auth = docs.require_mut("auth")?;
                        auth.root_object().insert(
                            target.clone(),
                            serde_json::json!({ "type": "api_key", "key": key }),
                        );
                        // apiKey lama di models.json bisa menimpa kredensial
                        // tersimpan, jadi dibersihkan — tetapi hanya kalau ada,
                        // supaya `models.json` tidak ikut ditulis (dan di-backup)
                        // ketika tidak ada yang berubah.
                        let has_stale = docs
                            .get("models")
                            .and_then(|models| models.data().get("providers"))
                            .and_then(|providers| providers.get(&target))
                            .and_then(|entry| entry.get("apiKey"))
                            .is_some();
                        if has_stale {
                            if let Some(models) = docs.get_mut("models") {
                                if let Some(entry) = models
                                    .object_at("providers")
                                    .get_mut(&target)
                                    .and_then(Value::as_object_mut)
                                {
                                    entry.remove("apiKey");
                                }
                            }
                        }
                    }
                    KeyMode::Env => {
                        let models = docs.require_mut("models")?;
                        if let Some(entry) = models
                            .object_at("providers")
                            .get_mut(&target)
                            .and_then(Value::as_object_mut)
                        {
                            entry.insert(
                                "apiKey".to_string(),
                                Value::String(format!("${}", env_var)),
                            );
                        }
                        if let Some(auth) = docs.get_mut("auth") {
                            if auth.data().get(&target).is_some() {
                                auth.root_object().remove(&target);
                            }
                        }
                        warnings.push(format!(
                            "Environment mode: the key is not written. Set {} in your shell.",
                            env_var
                        ));
                    }
                }

                Ok(warnings)
            }
            EditOp::ForgetKey { provider_id } => {
                let target = provider_id.trim().to_string();
                if target.is_empty() {
                    anyhow::bail!("Provider is required.");
                }

                let mut warnings = Vec::new();
                let mut removed = false;

                // Kredensial tersimpan.
                if let Some(auth) = docs.get_mut("auth") {
                    if auth.data().get(&target).is_some() {
                        auth.root_object().remove(&target);
                        removed = true;
                    }
                }
                // apiKey literal di models.json menang atas auth.json, jadi ikut
                // dibersihkan kalau ada. Referensi `$ENV` dibiarkan: nilainya di
                // shell pengguna, bukan di file ini, dan menghapus referensinya
                // tidak mengubah apa pun bagi harness.
                let mut drop_key = false;
                if let Some(models) = docs.get_mut("models") {
                    if let Some(entry) = models
                        .object_at("providers")
                        .get_mut(&target)
                        .and_then(Value::as_object_mut)
                    {
                        let inline = entry
                            .get("apiKey")
                            .and_then(Value::as_str)
                            .map(|value| !value.trim_start().starts_with('$'))
                            .unwrap_or(false);
                        if inline {
                            entry.remove("apiKey");
                            drop_key = true;
                            removed = true;
                        }
                    }
                }
                let _ = drop_key;

                if !removed {
                    anyhow::bail!(
                        "No stored credential for '{}'. It may use an environment variable.",
                        target
                    );
                }

                warnings.push(format!(
                    "Stored credential for '{}' removed (auth.json / models.json). The value is still in the backup file.",
                    target
                ));

                // Env var tidak bisa dihapus dari sini, dan itu justru yang
                // membuat provider tetap hidup di harness.
                let env_name = format!(
                    "{}_API_KEY",
                    target.to_ascii_uppercase().replace(['-', '.'], "_")
                );
                if std::env::var_os(&env_name).is_some() {
                    warnings.push(format!(
                        "{env_name} is still set in your environment, so pi keeps offering '{target}'. Unset it to finish disabling the provider."
                    ));
                }
                Ok(warnings)
            }
            EditOp::SetModels {
                provider_id,
                models,
            } => {
                let target = provider_id.trim().to_string();
                if models.is_empty() {
                    anyhow::bail!("Tambahkan minimal satu model.");
                }

                let document = docs.require_mut("models")?;
                let Some(entry) = document
                    .object_at("providers")
                    .get_mut(&target)
                    .and_then(Value::as_object_mut)
                else {
                    anyhow::bail!("Provider '{}' not found in models.json.", target);
                };
                entry.insert(
                    "models".to_string(),
                    Value::Array(models.iter().map(model_to_value).collect()),
                );

                let written: Vec<String> = models.iter().map(|model| model.id.clone()).collect();
                let mut warnings = vec![format!(
                    "{} model ditulis untuk '{}'.",
                    models.len(),
                    target
                )];
                // Daftar model berubah: scope ikut menyesuaikan, tanpa melebar
                // ke model yang tidak dipilih.
                if let Some(summary) =
                    Self::reconcile_scope(docs, &target, Ensure::Models(&written))?
                {
                    warnings.push(summary);
                }
                Ok(warnings)
            }
            EditOp::SetEnabled {
                provider_id,
                enabled,
            } => {
                let target = provider_id.trim().to_string();
                let models = docs.require_mut("models")?;
                let existing = models
                    .data()
                    .get("providers")
                    .and_then(|p| p.get(&target))
                    .cloned();

                if *enabled {
                    let Some(value) = models.parked_value(&target) else {
                        if existing.is_some() {
                            return Ok(vec![]); // sudah aktif
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    models.object_at("providers").insert(target.clone(), value);
                    models.drop_parked(&target);
                    // Scope perlu mencakup provider yang baru hidup lagi, kalau
                    // tidak modelnya kembali ada tapi tetap tersembunyi.
                    let mut warnings = vec![format!("Provider '{}' re-enabled.", target)];
                    if let Some(summary) = Self::reconcile_scope(docs, &target, Ensure::Provider)? {
                        warnings.push(summary);
                    }
                    // Provider hidup lagi tapi default kosong: isi supaya harness
                    // punya titik awal tanpa harus memilih ulang.
                    warnings.extend(Self::sync_default(docs, &target, true)?);
                    Ok(warnings)
                } else {
                    let Some(entry) = existing else {
                        if models.parked_value(&target).is_some() {
                            return Ok(vec![]); // sudah nonaktif
                        }
                        anyhow::bail!("Provider '{}' not found.", target);
                    };
                    let removed = models
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                        .and_then(|map| map.remove(&target))
                        .is_some();
                    if removed {
                        models.park(&target, &entry);
                    }
                    let mut warnings = vec![format!(
                        "Provider '{}' disabled (commented out in models.json).",
                        target
                    )];
                    // Provider yang dinonaktifkan tidak punya model yang bisa
                    // dicocokkan, jadi polanya di scope ikut dibuang.
                    if let Some(summary) = Self::reconcile_scope(docs, &target, Ensure::Nothing)? {
                        warnings.push(summary);
                    }
                    // Default yang menunjuk provider ini tidak boleh tertinggal.
                    warnings.extend(Self::sync_default(docs, &target, false)?);
                    // Kalau pi sendiri mengenal provider ini, entri kita bukan
                    // satu-satunya sumber: katakan apa adanya.
                    if let Some(warning) = Self::builtin_warning(&target) {
                        warnings.push(warning);
                    }
                    Ok(warnings)
                }
            }
            EditOp::SetDefault {
                provider_id,
                model_id,
            } => {
                let provider = provider_id.trim().to_string();
                let model = model_id.trim().to_string();
                if provider.is_empty() || model.is_empty() {
                    anyhow::bail!("Provider and model are required.");
                }
                let settings = docs.require_mut("settings")?;
                let root = settings.root_object();
                root.insert(
                    "defaultProvider".to_string(),
                    Value::String(provider.clone()),
                );
                let reference = format!("{}/{}", provider, model);
                root.insert("defaultModel".to_string(), Value::String(reference.clone()));

                // Pi memakai model PERTAMA dari scope `enabledModels` bila scope
                // tidak kosong, dan hanya jatuh ke defaultModel kalau scope kosong
                // (`findInitialModel` di dist/core/model-resolver.js). Jadi default
                // harus ikut jadi entri pertama scope, kalau tidak pilihan user
                // tidak pernah terpakai.
                let mut warnings = Vec::new();
                let mut rescoped = false;
                if let Some(list) = root.get_mut("enabledModels").and_then(Value::as_array_mut) {
                    if !list.is_empty() {
                        let before = list.len();
                        list.retain(|item| item.as_str() != Some(reference.as_str()));
                        list.insert(0, Value::String(reference.clone()));
                        rescoped = true;
                        if before == list.len() - 1 && before > 0 {
                            warnings.push(format!(
                                "'{}' moved to the first position of enabledModels: Pi uses the first model in the scope, not defaultModel, as long as the scope is not empty.",
                                reference
                            ));
                        } else {
                            warnings.push(format!(
                                "'{}' added as the first enabledModels entry: Pi uses the first model in the scope, not defaultModel, as long as the scope is not empty.",
                                reference
                            ));
                        }
                    }
                }

                warnings.insert(
                    0,
                    format!(
                        "Pi default: {}/{} (written to settings.json{}).",
                        provider,
                        model,
                        if rescoped {
                            " dan urutan enabledModels"
                        } else {
                            ""
                        }
                    ),
                );
                Ok(warnings)
            }
            EditOp::Remove { provider_id } => {
                let target = provider_id.trim().to_string();
                let mut removed = false;

                {
                    let models = docs.require_mut("models")?;
                    if let Some(map) = models
                        .tree_mut()
                        .get_mut("providers")
                        .and_then(Value::as_object_mut)
                    {
                        removed = map.remove(&target).is_some();
                    }
                }

                if let Some(auth) = docs.get_mut("auth") {
                    if auth.data().get(&target).is_some() {
                        auth.root_object().remove(&target);
                    }
                }
                if let Some(models) = docs.get_mut("models") {
                    if models.parked_value(&target).is_some() {
                        models.drop_parked(&target);
                    }
                }

                let mut warnings = Vec::new();
                if !removed {
                    warnings.push(format!("Provider '{}' not found in models.json.", target));
                }
                if let Some(summary) = Self::reconcile_scope(docs, &target, Ensure::Nothing)? {
                    warnings.push(summary);
                }
                Ok(warnings)
            }
        }
    }

    /// Pi mendahulukan kredensial di `auth.json` di atas `apiKey` di
    /// `models.json` - urutan yang sama dengan `snapshot`.
    fn builtin_providers(&self) -> &'static [(&'static str, &'static str)] {
        shared::BUILTIN_PROVIDERS
    }

    fn notices(&self, docs: &Docs, providers: &[shared::ProviderRef<'_>]) -> Vec<Notice> {
        let mut notices =
            shared::builtin_id_notices(self.id(), self.builtin_providers(), providers);
        notices.extend(self.scope_notices(docs, providers));
        notices.extend(self.default_notices(docs, providers));
        notices.extend(self.catalog_notices(docs));
        notices
    }

    fn catalog_targets(&self, docs: &Docs) -> Vec<CatalogTarget> {
        let base = std::env::var("HAKONFIG_PI_CATALOG_BASE")
            .unwrap_or_else(|_| "https://pi.dev".to_string());
        Self::credentialed_providers(docs)
            .into_iter()
            // Gateway custom tidak punya entri di pi.dev, jadi pi tidak pernah
            // me-refresh katalognya; memeriksanya hanya menghasilkan alarm palsu.
            .filter(|provider| {
                self.builtin_providers()
                    .iter()
                    .any(|(id, _)| id == provider)
            })
            .map(|provider| CatalogTarget {
                url: format!(
                    "{}/api/models/providers/{}?types=chat,image,classifier",
                    base.trim_end_matches('/'),
                    provider
                ),
                provider,
            })
            .collect()
    }

    fn fix_notice(&self, docs: &mut Docs, notice: &Notice) -> Result<Option<String>> {
        match notice.code.as_str() {
            "pi.scope" => Self::reconcile_scope(docs, &notice.provider, Ensure::Provider),
            "pi.default_disabled" => Ok(Self::sync_default(docs, &notice.provider, false)?
                .into_iter()
                .next()),
            other => anyhow::bail!("pi has no automatic fix for '{other}'"),
        }
    }

    fn stored_key(&self, docs: &Docs, provider_id: &str) -> Option<String> {
        let target = provider_id.trim();
        if target.is_empty() {
            return None;
        }
        if let Some(key) = docs
            .get("auth")
            .and_then(|doc| doc.data().get(target))
            .and_then(|credential| get_str(credential, "key"))
        {
            return Some(key);
        }
        let models = docs.get("models")?;
        let entry = models
            .data()
            .get("providers")
            .and_then(|map| map.get(target))
            .cloned()
            // provider yang dinonaktifkan tetap punya key tersimpan (blok komentar)
            .or_else(|| models.parked_value(target))?;
        get_str(&entry, "apiKey")
    }
}

/// Satu entri model untuk `models.json`.
fn model_to_value(model: &crate::engine::model::ModelDraft) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("id".to_string(), Value::String(model.id.trim().to_string()));
    if let Some(name) = model
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        object.insert("name".to_string(), Value::String(name.to_string()));
    }
    if let Some(window) = model.context_window.filter(|value| *value > 0) {
        object.insert("contextWindow".to_string(), Value::from(window));
    }
    if let Some(max) = model.max_tokens.filter(|value| *value > 0) {
        object.insert("maxTokens".to_string(), Value::from(max));
    }
    if model.reasoning {
        object.insert("reasoning".to_string(), Value::Bool(true));
    }
    Value::Object(object)
}
