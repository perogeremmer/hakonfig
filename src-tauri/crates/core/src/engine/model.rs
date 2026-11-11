//! Tipe data bersama antara backend dan frontend.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- discovery

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileInfo {
    pub path: String,
    pub role: String,
    pub exists: bool,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessInfo {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub experimental: bool,
    pub files: Vec<ConfigFileInfo>,
    pub notes: String,
    /// Jumlah provider yang ada di config harness ini.
    pub provider_count: usize,
    /// Jumlah provider yang sudah punya API key terpasang.
    pub connected_count: usize,
    /// Terisi bila config ada tetapi gagal dibaca (mis. JSON rusak).
    pub read_error: Option<String>,
    /// Home yang dipakai, hanya saat target eksplisit (`--home`/`--target`).
    /// Kosong = home milik proses ini, jadi output biasa tidak berubah.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
    /// Pemilik home target dari `/etc/passwd`, bila dikenali.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

// ---------------------------------------------------------------- snapshot

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSummary {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,
    pub api: Option<String>,
    pub has_key: bool,
    pub key_display: Option<String>,
    /// Bagaimana kredensial provider ini disimpan: `stored`, `env`, atau `none`.
    ///
    /// GUI memakainya untuk memilih mode key yang benar saat membuka form edit;
    /// menebak dari tampilan `key_display` rapuh karena mask berbeda per harness.
    #[serde(default)]
    pub key_mode: String,
    /// Nama variabel environment yang dirujuk, kalau `key_mode` = `env`.
    #[serde(default)]
    pub key_env: Option<String>,
    /// true bila kredensialnya tersimpan di file (bisa dihapus dari sini).
    ///
    /// `has_key` juga benar untuk key yang hanya ada di environment; yang itu
    /// tidak bisa dihapus Hakonfig, jadi aksi "lupakan kredensial" harus tahu
    /// bedanya.
    #[serde(default)]
    pub key_stored: bool,
    pub models: Vec<ModelSummary>,
    /// false bila provider dinonaktifkan lewat daftar disable milik harness.
    pub enabled: bool,
    /// true bila harness punya daftar disable (nonaktif = reversible).
    /// Bila false, menonaktifkan berarti menghapus entry dari config.
    pub can_toggle: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessSnapshot {
    pub harness: String,
    pub files: Vec<ConfigFileInfo>,
    pub providers: Vec<ProviderSummary>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub warnings: Vec<String>,
    /// Home yang dibaca, hanya saat target eksplisit (lihat `HarnessInfo`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
    /// Pemilik home target, bila dikenali.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

// ---------------------------------------------------------------- edit input

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    #[default]
    Inline,
    Env,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDraft {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDraft {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub key_mode: Option<KeyMode>,
    #[serde(default)]
    pub models: Vec<ModelDraft>,
}

impl ProviderDraft {
    pub fn key_mode(&self) -> KeyMode {
        self.key_mode.unwrap_or_default()
    }
    /// Key yang benar-benar diisi user (bukan string kosong).
    pub fn key_value(&self) -> Option<&str> {
        self.key.as_deref().map(str::trim).filter(|k| !k.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum EditOp {
    Upsert(ProviderDraft),
    Remove {
        provider_id: String,
    },
    /// Hanya berlaku untuk harness yang punya daftar disable.
    SetEnabled {
        provider_id: String,
        enabled: bool,
    },
    /// Ganti kredensial saja: model, endpoint, dan nama tidak disentuh.
    ///
    /// Dipakai aksi "Change key" di UI, supaya mengganti key tidak perlu
    /// mengirim ulang seluruh draft provider.
    SetKey {
        provider_id: String,
        key: String,
        key_mode: KeyMode,
    },
    /// Hapus kredensial tersimpan milik satu provider.
    ///
    /// Dipakai untuk benar-benar membuat harness berhenti menawarkan provider:
    /// untuk provider bawaan (mis. pi + deepseek) menghapus entri di config kita
    /// tidak cukup, karena harness tetap mengenalinya selama kredensialnya ada.
    /// Destruktif, jadi hanya lewat aksi eksplisit pengguna, dan file tetap
    /// di-backup seperti penulisan lain.
    ForgetKey {
        provider_id: String,
    },
    /// Ganti daftar model saja: kredensial dan endpoint tidak disentuh.
    ///
    /// Daftar ini menjadi isi akhir `models` milik provider, jadi model yang
    /// tidak dicantumkan berarti dihapus (backup tetap dibuat).
    SetModels {
        provider_id: String,
        models: Vec<ModelDraft>,
    },
    /// Ganti default provider/model yang dipakai harness.
    SetDefault {
        provider_id: String,
        model_id: String,
    },
}

// ---------------------------------------------------------------- results

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub role: String,
    pub sensitive: bool,
    pub before: Option<String>,
    pub after: Option<String>,
    pub changed: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub files: Vec<FileDiff>,
    pub warnings: Vec<String>,
    pub will_create: Vec<String>,
    pub will_modify: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub files_written: Vec<String>,
    pub backup_dir: Option<String>,
    pub warnings: Vec<String>,
    /// Temuan yang membuat model yang baru ditulis tidak terlihat harness.
    pub notices: Vec<Notice>,
}

/// Sumber katalog jarak jauh yang di-refresh harness untuk satu provider.
///
/// Dipakai `doctor --online` untuk mengecek hal yang tidak bisa dilihat dari
/// file config: kalau permintaan katalognya gagal, harness diam-diam memakai
/// katalog bawaan dan pengguna melihat "showing cached models".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTarget {
    pub provider: String,
    pub url: String,
}

/// Sesuatu yang membuat provider/model yang sudah ditulis tetap tidak terlihat
/// oleh harness-nya: scope model di pi, daftar disable di omp, atau id yang
/// bentrok dengan provider bawaan harness.
///
/// `code` dipakai `fix_notice` untuk mencocokkan temuan, jadi jangan diubah
/// tanpa memperbarui adapter dan test-nya.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub code: String,
    pub harness: String,
    pub provider: String,
    pub message: String,
    /// Label aksi perbaikan otomatis; `None` = hanya bisa dibereskan manual.
    pub fix: Option<String>,
    /// Aksi ini menghapus sesuatu yang tidak bisa dibuat ulang aplikasi
    /// (kredensial), jadi `doctor --fix` tidak boleh menjalankannya sendiri —
    /// hanya lewat konfirmasi eksplisit di GUI.
    #[serde(default)]
    pub destructive: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub path: String,
    pub role: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub dir: String,
    pub files: Vec<BackupFile>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub status: Option<u16>,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub models_sample: Vec<String>,
}

// ---------------------------------------------------------------- presets

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api: String,
    #[serde(default)]
    pub env_var: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub key_required: bool,
    /// "free" | "local" - ditampilkan sebagai chip di pemilih provider.
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub models: Vec<ModelSummary>,
}
