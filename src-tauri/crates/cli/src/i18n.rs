//! Teks antarmuka CLI.
//!
//! Default **English**; Bahasa Indonesia opsional lewat `--lang id` atau
//! `HAKONFIG_LANG=id`, mengikuti aplikasi desktop yang juga English-first.
//!
//! Pesan yang punya parameter ditulis sebagai template dengan placeholder
//! `{nama}` di kedua bahasa, lalu diisi oleh [`fill`]. Dengan begitu urutan
//! kata tiap bahasa tetap wajar tanpa perlu kode format terpisah.
//!
//! Catatan: `--help` clap sengaja tetap English (dibuat saat kompilasi).
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Id,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Terima "en", "english", "id", "in", "indonesian" (tidak peka huruf besar).
pub fn parse(raw: &str) -> Option<Lang> {
    match raw.trim().to_lowercase().as_str() {
        "en" | "en-us" | "en-gb" | "english" => Some(Lang::En),
        "id" | "id-id" | "in" | "indonesian" | "indonesia" => Some(Lang::Id),
        _ => None,
    }
}

pub fn set(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::En => 0,
            Lang::Id => 1,
        },
        Ordering::Relaxed,
    );
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Lang::Id
    } else {
        Lang::En
    }
}

/// Isi `{placeholder}` pada template.
pub fn fill(template: &str, pairs: &[(&str, String)]) -> String {
    let mut text = template.to_string();
    for (key, value) in pairs {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}

macro_rules! messages {
    ($($variant:ident => ($en:expr, $id:expr)),* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Msg {
            $($variant),*
        }

        impl Msg {
            /// Daftar lengkap, dipakai test kelengkapan terjemahan.
            #[cfg(test)]
            pub const ALL: &'static [Msg] = &[$(Msg::$variant),*];

            fn pair(self) -> (&'static str, &'static str) {
                match self {
                    $(Msg::$variant => ($en, $id)),*
                }
            }
        }
    };
}

messages! {
    // ---- umum
    Choices => ("Options: {list}", "Pilihan: {list}"),
    ReadFailed => ("read failed", "gagal dibaca"),

    // ---- list
    ListFooter => (
        "{installed} of {total} harnesses have a config. `add` can create one for any of them.",
        "{installed} dari {total} harness punya config. `add` bisa membuat yang baru."
    ),
    ListNoConfig => (
        "No config yet. Run `hakonfig-cli add` to create one.",
        "Belum ada config. Jalankan `hakonfig-cli add` untuk membuatnya."
    ),

    // ---- show
    ShowNoProvider => ("  no providers yet.", "  belum ada provider."),
    ShowDefault => ("default: {provider}{model}", "default: {provider}{model}"),
    HeaderStatus => ("STATUS", "STATUS"),
    HeaderId => ("ID", "ID"),
    HeaderBaseUrl => ("BASE URL", "BASE URL"),
    HeaderModels => ("MODELS", "MODEL"),
    HeaderKey => ("KEY", "KEY"),
    ProviderDisabled => ("disabled", "nonaktif"),
    KeyPresent => ("yes", "ada"),

    // ---- providers
    ProviderCountLabel => ("{count} providers", "{count} provider"),
    HeaderName => ("NAME", "NAMA"),
    HeaderVersion => ("VERSION", "VERSI"),
    HeaderProviders => ("PROVIDERS", "PROVIDER"),
    HeaderNote => ("NOTE", "CATATAN"),

    // ---- error pemilihan harness
    HarnessUnsupported => (
        "harness '{id}' is not supported. {choices}",
        "harness '{id}' tidak didukung. {choices}"
    ),
    NoHarnessConfig => (
        "no harness config on this machine; pass --harness <id>. {choices}",
        "belum ada config harness di mesin ini; sebutkan --harness <id>. {choices}"
    ),
    AmbiguousHarness => (
        "{count} harnesses have a config; pick one with --harness: {list}",
        "ada {count} harness punya config; sebutkan salah satu dengan --harness: {list}"
    ),

    // ---- add: langkah
    ConfigWillBeCreated => (
        "! no {name} config yet; the file will be created.",
        "! belum ada config {name}; file akan dibuat."
    ),
    PickHarness => ("Which harness?", "Harness mana?"),
    PickHarnessHint => ("type to search", "ketik untuk mencari"),
    PickProvider => ("Which provider?", "Provider mana?"),
    PickProviderHint => (
        "type to search · ✓ = already configured",
        "ketik untuk mencari · ✓ = sudah terpasang"
    ),
    NoHarnessInstalled => (
        "no harness with a config yet. Run `hakonfig-cli list` to see what is detected.",
        "belum ada harness yang punya config. Jalankan `hakonfig-cli list` untuk lihat yang terdeteksi."
    ),
    HarnessNotInstalled => (
        "'{harness}' is not installed on this machine.",
        "'{harness}' belum terpasang di mesin ini."
    ),
    NoProviderInstalled => (
        "no provider is installed for {harness} yet. Add one with `hakonfig-cli add`.",
        "belum ada provider terpasang di {harness}. Tambahkan dengan `hakonfig-cli add`."
    ),
    PickInstalledProvider => ("Which provider?", "Provider mana?"),
    ManageWhat => ("What do you want to change?", "Mau ubah apa?"),
    ProviderHeader => (
        "{provider}  ·  {name}  ·  {harness}  ·  {models} models  ·  key {key}  ·  {desc}",
        "{provider}  ·  {name}  ·  {harness}  ·  {models} model  ·  key {key}  ·  {desc}"
    ),
    KeySavedLabel => ("saved", "tersimpan"),
    KeyMissingLabel => ("missing", "belum ada"),
    NoModelsYet => ("no models yet", "belum ada model"),
    ProviderGone => (
        "'{provider}' is no longer configured in {harness}.",
        "'{provider}' sudah tidak terpasang di {harness}."
    ),
    ActionEndpoint => ("Change endpoint and name", "Ubah endpoint dan nama"),
    EditName => ("Display name", "Nama tampilan"),
    EditBaseUrl => ("Base URL", "Base URL"),
    ActionRemove => ("Remove this provider", "Hapus provider ini"),
    Back => ("Back", "Kembali"),
    NeedHarnessFlag => (
        "--harness <id> is required. Run `hakonfig-cli list` to see what is installed.",
        "butuh --harness <id>. Jalankan `hakonfig-cli list` untuk lihat yang terpasang."
    ),
    ConfirmRemove => (
        "Remove '{provider}' from the config?",
        "Hapus '{provider}' dari config?"
    ),
    ConfirmRemoveHint => (
        "a backup is written first; the key entry is removed too",
        "backup dibuat lebih dulu; entry key ikut dihapus"
    ),
    ActionKey => ("Change the API key only", "Ganti API key saja"),
    ActionModels => ("Choose models (key stays)", "Pilih model (key tidak diubah)"),
    ActionDefault => ("Set the default model", "Jadikan model default"),
    KeyRequired => (
        "an API key is required for '{provider}'. Pipe it with --key-stdin, or run without flags in a terminal.",
        "butuh API key untuk '{provider}'. Kirim lewat --key-stdin, atau jalankan tanpa flag di terminal."
    ),
    PickDefaultModel => ("Which model becomes the default?", "Model mana yang jadi default?"),
    NoModelsToPick => (
        "'{provider}' has no model list yet; add models first",
        "'{provider}' belum punya daftar model; tambahkan model dulu"
    ),
    Cancelled => ("cancelled.", "dibatalkan."),
    NeedProviderFlag => (
        "--provider <id> is required. List presets with `hakonfig-cli providers`, \
         or use a custom endpoint: --provider <id> --name <name> --base-url <url>",
        "butuh --provider <id>. Lihat daftar dengan `hakonfig-cli providers`, \
         atau pakai endpoint custom: --provider <id> --name <nama> --base-url <url>"
    ),
    NeedCustomId => (
        "--provider custom needs its own id, e.g. \
         --provider toko-saya --name Toko --base-url https://...",
        "--provider custom butuh id sendiri, mis. \
         --provider toko-saya --name Toko --base-url https://..."
    ),
    KeyPrompt => ("API key for {name}", "API key {name}"),
    KeyHintEnv => ("leave empty to use ${env}", "kosongkan untuk memakai ${env}"),
    KeyStdinNeedsPipe => (
        "--key-stdin needs piped input, e.g. \
         printf '%s' \"$KEY\" | hakonfig-cli add ...; it cannot read from a terminal.",
        "--key-stdin butuh input dari pipe, mis. \
         printf '%s' \"$KEY\" | hakonfig-cli add ...; tidak bisa dibaca dari terminal."
    ),
    StdinFailed => ("failed to read stdin: {error}", "gagal membaca stdin: {error}"),

    // ---- doctor
    DoctorHarness => ("harness", "harness"),
    DoctorProvider => ("provider", "provider"),
    DoctorIssue => ("issue", "masalah"),
    DoctorFix => ("fix", "perbaikan"),
    DoctorManual => ("manual", "manual"),
    DoctorClean => (
        "no hidden config: every written provider is reachable in its harness.",
        "tidak ada config tersembunyi: semua provider yang tertulis bisa dilihat harness-nya."
    ),
    DoctorFixed => ("fixed: {detail}", "diperbaiki: {detail}"),
    DoctorNothingToFix => (
        "nothing needed fixing.",
        "tidak ada yang perlu diperbaiki."
    ),
    DoctorFixFailed => (
        "! could not fix {harness}/{provider}: {error}",
        "! gagal memperbaiki {harness}/{provider}: {error}"
    ),
    DoctorRemaining => (
        "some issues are left; fix them manually.",
        "masih ada masalah yang tersisa; perbaiki manual."
    ),
    DoctorHintFix => (
        "{count} issue(s) hide config from its harness: run 'hakonfig-cli doctor --fix'.",
        "{count} masalah membuat config tidak terlihat harness-nya: jalankan 'hakonfig-cli doctor --fix'."
    ),
    UnknownHarness => (
        "unknown harness '{raw}'. Run 'hakonfig-cli list' to see the ids.",
        "harness '{raw}' tidak dikenal. Jalankan 'hakonfig-cli list' untuk melihat id-nya."
    ),
    NoticeHint => (
        "run 'hakonfig-cli doctor' to see why, and 'doctor --fix' to fix it.",
        "jalankan 'hakonfig-cli doctor' untuk melihat sebabnya, dan 'doctor --fix' untuk memperbaikinya."
    ),

    // ---- add: model
    PickModels => ("Which models?", "Model mana yang dipakai?"),
    PickModelsHintAll => (
        "space to toggle · enter to accept · type to search",
        "spasi untuk memilih · enter untuk lanjut · ketik untuk mencari"
    ),
    NeedModelsFlag => (
        "this provider has no model list yet. Add --models id1,id2 \
         (or run once in a terminal to sync from the endpoint).",
        "provider ini belum punya daftar model. Tambahkan --models id1,id2 \
         (atau jalankan sekali di terminal untuk sync dari endpoint)."
    ),
    SyncPromptStored => (
        "Fetch the model list from the endpoint now?",
        "Ambil daftar model dari endpoint sekarang?"
    ),
    SyncHintStored => (
        "needs network; uses the API key already saved for this provider",
        "butuh jaringan; memakai API key yang sudah tersimpan untuk provider ini"
    ),
    SyncPrompt => (
        "Fetch the model list from the endpoint now?",
        "Ambil daftar model dari endpoint sekarang?"
    ),
    SyncHint => (
        "needs network; uses the key you just entered",
        "butuh jaringan; memakai key yang baru dimasukkan"
    ),
    SyncEmpty => ("! endpoint returned no models", "! endpoint tidak mengembalikan model"),
    SyncFailed => ("! model sync failed: {error}", "! gagal sync model: {error}"),
    // Dipakai `--sync`: pengguna memang meminta daftar live, jadi kegagalan
    // menghentikan perintah alih-alih diam-diam memakai daftar preset.
    SyncHardEmpty => (
        "--sync asked the endpoint for models but it returned none",
        "--sync meminta daftar model ke endpoint, tetapi tidak ada yang dikembalikan"
    ),
    SyncHardFailed => ("model sync failed: {error}", "gagal sync model: {error}"),
    ModelsTextPrompt => ("model ids (comma separated)", "id model (pisah koma)"),
    ModelsTextHint => (
        "example: llama-3.3-70b-versatile,llama-3.1-8b-instant",
        "contoh: llama-3.3-70b-versatile,llama-3.1-8b-instant"
    ),
    ModelsEmpty => (
        "no models selected; a provider without models is not useful.",
        "tidak ada model dipakai; provider tanpa model tidak berguna."
    ),
    ModelsFlagEmpty => (
        "--models did not contain a single model id.",
        "--models tidak berisi satu pun id model."
    ),

    // ---- add: terapkan
    ConfirmApply => ("Apply these changes?", "Terapkan perubahan?"),
    ConfirmApplyHint => (
        "a backup is created before writing",
        "backup otomatis dibuat sebelum menulis"
    ),
    DryRun => ("(dry-run: nothing was written)", "(dry-run: tidak ada yang ditulis)"),
    Wrote => ("wrote: {path}", "ditulis: {path}"),
    Backup => ("backup: {path}", "backup: {path}"),
    NothingToWrite => (
        "nothing to write, the config already matches.",
        "tidak ada perubahan yang perlu ditulis."
    ),
    TestOk => ("✓ connection ok{latency}", "✓ koneksi ok{latency}"),
    TestFailed => ("! connection test failed: {message}", "! uji koneksi gagal: {message}"),
    TestFailedAbort => (
        "connection test failed; aborting because --test was requested (skip it with --no-test).",
        "uji koneksi gagal; dibatalkan karena --test diminta (lewati dengan --no-test)."
    ),
    ConfirmAfterFailedTest => (
        "Write the config anyway?",
        "Tetap tulis konfigurasi?"
    ),

    // ---- pratinjau
    PreviewNoChanges => ("no changes.", "tidak ada perubahan."),
    PreviewNew => ("new: {path}", "baru: {path}"),
    PreviewModified => ("modified: {path}", "diubah: {path}"),
    PreviewSensitive => (
        "  (secret values are redacted)",
        "  (isinya rahasia — disamarkan)"
    ),
    PreviewTruncated => ("  ... {count} more lines", "  ... {count} baris lagi"),

    // ---- draft / validasi
    EmptyId => ("provider id must not be empty", "id provider tidak boleh kosong"),
    CustomNeedsName => (
        "--name is required for a custom provider",
        "--name wajib untuk provider custom"
    ),
    CustomNeedsBaseUrl => (
        "--base-url is required for a custom provider",
        "--base-url wajib untuk provider custom"
    ),
    BaseUrlMustBeHttp => (
        "base URL must start with http:// or https://: {url}",
        "base URL harus http(s): {url}"
    ),
    UnknownProviderSuggest => (
        "provider '{id}' is not in the catalog. Did you mean '{candidate}'?",
        "provider '{id}' tidak ada di katalog. Mungkin maksudmu '{candidate}'?"
    ),
    UnknownProvider => (
        "provider '{id}' is not in the catalog. See `hakonfig-cli providers`.",
        "provider '{id}' tidak ada di katalog. Lihat `hakonfig-cli providers`."
    ),
    UnknownLang => (
        "unknown language '{raw}'; use en or id.",
        "bahasa '{raw}' tidak dikenal; pakai en atau id."
    ),
}

/// Teks menurut bahasa aktif.
pub fn t(key: Msg) -> &'static str {
    let (en, id) = key.pair();
    match current() {
        Lang::En => en,
        Lang::Id => id,
    }
}

/// Ambil placeholder `{...}` dari sebuah template (untuk test invarian).
#[cfg(test)]
fn placeholders(template: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        found.push(rest[start + 1..start + end].to_string());
        rest = &rest[start + end + 1..];
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Bahasa aktif bersifat global per proses, jadi test yang mengubahnya
    /// harus berurutan.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_is_english() {
        let _guard = LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set(Lang::En);
        assert_eq!(
            t(Msg::NothingToWrite),
            "nothing to write, the config already matches."
        );
        assert_eq!(t(Msg::PickProvider), "Which provider?");
    }

    #[test]
    fn can_switch_to_indonesian() {
        let _guard = LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set(Lang::Id);
        assert_eq!(
            t(Msg::NothingToWrite),
            "tidak ada perubahan yang perlu ditulis."
        );
        set(Lang::En);
    }

    #[test]
    fn parse_accepts_various_forms() {
        assert_eq!(parse("en"), Some(Lang::En));
        assert_eq!(parse("EN-US"), Some(Lang::En));
        assert_eq!(parse(" id "), Some(Lang::Id));
        assert_eq!(parse("indonesian"), Some(Lang::Id));
        assert_eq!(parse("jawa"), None);
    }

    #[test]
    fn fill_replaces_placeholders() {
        let text = fill("{count} providers.", &[("count", "3".to_string())]);
        assert_eq!(text, "3 providers.");
    }

    #[test]
    fn every_message_has_both_languages_and_the_same_placeholders() {
        for key in Msg::ALL {
            let (en, id) = key.pair();
            assert!(!en.trim().is_empty(), "{key:?} tanpa teks English");
            assert!(!id.trim().is_empty(), "{key:?} tanpa teks Indonesia");
            assert_eq!(
                placeholders(en),
                placeholders(id),
                "{key:?}: placeholder English dan Indonesia harus sama"
            );
        }
    }
}
