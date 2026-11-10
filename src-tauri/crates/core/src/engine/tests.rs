//! Test integrasi engine pada sandbox (`HAKONFIG_ROOT`), sehingga config asli
//! di mesin tidak pernah tersentuh.

use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::api;
use crate::engine::adapters;
use crate::engine::adapters::Adapter;
use crate::engine::doc::Docs;
use crate::engine::model::{EditOp, KeyMode, ModelDraft, ProviderDraft};
use crate::engine::testutil::sandbox;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn read_json_value(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn draft(id: &str, key: Option<&str>, mode: KeyMode) -> ProviderDraft {
    ProviderDraft {
        id: id.to_string(),
        name: format!("Provider {id}"),
        base_url: "https://api.example.com/v1".to_string(),
        api: "openai-completions".to_string(),
        key: key.map(str::to_string),
        key_mode: Some(mode),
        models: vec![ModelDraft {
            id: "model-a".to_string(),
            name: Some("Model A".to_string()),
            context_window: Some(128000),
            max_tokens: Some(8192),
            reasoning: true,
        }],
    }
}

fn upsert(id: &str, key: Option<&str>, mode: KeyMode) -> EditOp {
    EditOp::Upsert(draft(id, key, mode))
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn yaml(path: &Path) -> Value {
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// Membaca snapshot lewat adapter (jalur yang sama dengan UI).
fn snapshot(harness: &str) -> crate::engine::model::HarnessSnapshot {
    let adapter = adapters::get(harness).unwrap();
    let docs = Docs::load(&adapter.specs()).unwrap();
    adapter.snapshot(&docs).unwrap()
}

/// Key mentah yang dibaca backend untuk `test_connection` / `fetch_models`.
fn stored_key(harness: &str, provider_id: &str) -> Option<String> {
    let adapter = adapters::get(harness).unwrap();
    let docs = Docs::load(&adapter.specs()).unwrap();
    adapter.stored_key(&docs, provider_id)
}

#[test]
fn change_default_model_per_harness() {
    let (_lock, root) = sandbox("defaults");
    let pi_dir = root.join("pi/agent");
    let omp_dir = root.join("omp/agent");
    let oc_dir = root.join("opencode/config");
    let oc_data = root.join("opencode/data");
    let hm_dir = root.join("hermes");

    // --- Pi: settings.json
    write(
        &pi_dir.join("settings.json"),
        r#"{"theme":"dark","defaultProvider":"lama","defaultModel":"lama/m0"}"#,
    );
    api::apply_edit(
        "pi".into(),
        EditOp::SetDefault {
            provider_id: "groq".to_string(),
            model_id: "llama-3.3-70b-versatile".to_string(),
        },
    )
    .unwrap();
    let settings = json(&pi_dir.join("settings.json"));
    assert_eq!(settings["defaultProvider"], "groq");
    assert_eq!(
        settings["defaultModel"], "llama-3.3-70b-versatile",
        "pi menyimpan id model saja, bukan provider/model"
    );
    assert_eq!(settings["theme"], "dark", "setelan lain tidak boleh hilang");
    assert_eq!(snapshot("pi").default_provider.as_deref(), Some("groq"));

    // --- oh-my-pi: config.yml modelRoles.default
    write(
        &omp_dir.join("config.yml"),
        "setupVersion: 2\nmodelRoles:\n  advisor: a/b\n  default: lama/m0\n",
    );
    api::apply_edit(
        "omp".into(),
        EditOp::SetDefault {
            provider_id: "opencode-go".to_string(),
            model_id: "kimi-k3".to_string(),
        },
    )
    .unwrap();
    let cfg = yaml(&omp_dir.join("config.yml"));
    assert_eq!(cfg["modelRoles"]["default"], "opencode-go/kimi-k3");
    assert_eq!(cfg["modelRoles"]["advisor"], "a/b", "role lain tetap");

    // --- OpenCode: root "model"
    write(
        &oc_dir.join("opencode.json"),
        r#"{"model":"lama/m0","theme":"x"}"#,
    );
    write(&oc_data.join("auth.json"), "{}");
    api::apply_edit(
        "opencode".into(),
        EditOp::SetDefault {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
        },
    )
    .unwrap();
    let cfg = json(&oc_dir.join("opencode.json"));
    assert_eq!(cfg["model"], "deepseek/deepseek-chat");
    assert_eq!(cfg["theme"], "x");

    // --- Hermes: model.provider "custom:<id>" + default + base_url
    write(
        &hm_dir.join("config.yaml"),
        "model:\n  default: lama/m0\n  provider: custom:lama\nproviders:\n  groq:\n    name: Groq\n    base_url: https://api.groq.com/openai/v1\n    key_env: GROQ_API_KEY\n    api_mode: chat_completions\n",
    );
    write(&hm_dir.join(".env"), "");
    api::apply_edit(
        "hermes".into(),
        EditOp::SetDefault {
            provider_id: "groq".to_string(),
            model_id: "llama-3.3-70b-versatile".to_string(),
        },
    )
    .unwrap();
    let cfg = yaml(&hm_dir.join("config.yaml"));
    assert_eq!(cfg["model"]["provider"], "custom:groq");
    assert_eq!(cfg["model"]["default"], "llama-3.3-70b-versatile");
    assert_eq!(cfg["model"]["base_url"], "https://api.groq.com/openai/v1");
    assert_eq!(
        cfg["providers"]["groq"]["key_env"], "GROQ_API_KEY",
        "provider tidak boleh ikut berubah"
    );
}

#[test]
fn pi_merge_backup_and_redaction() {
    let (_lock, root) = sandbox("pi_merge_backup_and_redaction");
    let dir = root.join("pi/agent");

    write(
        &dir.join("models.json"),
        r#"{
  // komentar pengguna tidak boleh hilang diam-diam
  "$schema": "https://pi.dev/schemas/models.schema.json",
  "providers": {
    "existing": {
      "name": "Existing",
      "baseUrl": "https://example.com/v1",
      "api": "openai-completions",
      "apiKey": "$EXISTING_API_KEY",
      "modelOverrides": { "m1": { "contextWindow": 1000 } },
      "models": [{ "id": "m1", "name": "M1", "extraField": true, "thinking": { "mode": "effort" } }]
    }
  }
}
"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"existing","defaultModel":"existing/m1"}"#,
    );

    // --- snapshot awal
    let snap = snapshot("pi");
    assert_eq!(snap.providers.len(), 1);
    assert_eq!(snap.default_provider.as_deref(), Some("existing"));
    assert!(
        snap.warnings.iter().any(|w| w.contains("comments")),
        "harus ada peringatan komentar: {:?}",
        snap.warnings
    );

    // --- tulis provider baru (key inline)
    let result = api::apply_edit(
        "pi".into(),
        upsert("groq", Some("sk-rahasia-1234"), KeyMode::Inline),
    )
    .expect("apply harus berhasil");
    assert!(result.backup_dir.is_some(), "backup wajib dibuat");
    assert!(result
        .files_written
        .iter()
        .any(|p| p.ends_with("models.json")));
    assert!(result
        .files_written
        .iter()
        .any(|p| p.ends_with("auth.json")));

    // --- field tak dikenal tetap utuh
    let models = json(&dir.join("models.json"));
    assert_eq!(
        models["providers"]["existing"]["modelOverrides"]["m1"]["contextWindow"],
        1000
    );
    assert_eq!(
        models["providers"]["existing"]["models"][0]["extraField"],
        true
    );
    assert_eq!(
        models["providers"]["existing"]["models"][0]["thinking"]["mode"],
        "effort"
    );
    assert_eq!(
        models["providers"]["existing"]["apiKey"],
        "$EXISTING_API_KEY"
    );
    assert_eq!(models["providers"]["existing"]["name"], "Existing");

    // --- provider baru
    assert_eq!(
        models["providers"]["groq"]["baseUrl"],
        "https://api.example.com/v1"
    );
    assert_eq!(models["providers"]["groq"]["api"], "openai-completions");
    assert!(models["providers"]["groq"].get("apiKey").is_none());
    assert_eq!(
        models["providers"]["groq"]["models"][0]["contextWindow"],
        128000
    );
    assert_eq!(models["providers"]["groq"]["models"][0]["reasoning"], true);

    // --- key masuk ke auth.json, bukan models.json
    let auth = json(&dir.join("auth.json"));
    assert_eq!(auth["groq"]["type"], "api_key");
    assert_eq!(auth["groq"]["key"], "sk-rahasia-1234");

    // --- pratinjau tidak boleh membocorkan key
    let preview = api::preview_edit(
        "pi".into(),
        upsert("groq", Some("sk-baru-9999"), KeyMode::Inline),
    )
    .expect("preview harus berhasil");
    let auth_diff = preview
        .files
        .iter()
        .find(|file| file.role == "auth")
        .expect("harus ada diff auth.json");
    assert!(auth_diff.changed);
    let after = auth_diff.after.as_deref().unwrap_or_default();
    let before = auth_diff.before.as_deref().unwrap_or_default();
    assert!(
        !after.contains("sk-baru-9999"),
        "key baru bocor di pratinjau"
    );
    assert!(
        !before.contains("sk-rahasia-1234"),
        "key lama bocor di pratinjau"
    );
    assert!(preview.warnings.iter().any(|w| w.contains("redacted")));

    // --- endpoint junction: mode env menulis referensi $VAR
    api::apply_edit(
        "pi".into(),
        upsert("groq-env", Some("apa-saja"), KeyMode::Env),
    )
    .unwrap();
    let models = json(&dir.join("models.json"));
    assert_eq!(
        models["providers"]["groq-env"]["apiKey"],
        "$GROQ_ENV_API_KEY"
    );
    let auth = json(&dir.join("auth.json"));
    assert!(auth.get("groq-env").is_none());

    // --- hapus provider
    api::apply_edit(
        "pi".into(),
        EditOp::Remove {
            provider_id: "groq".to_string(),
        },
    )
    .unwrap();
    let models = json(&dir.join("models.json"));
    assert!(models["providers"].get("groq").is_none());
    assert!(models["providers"].get("existing").is_some());
    let auth = json(&dir.join("auth.json"));
    assert!(auth.get("groq").is_none());

    // --- hapus provider yang tidak ada: tidak boleh mengubah apa pun
    let preview = api::preview_edit(
        "pi".into(),
        EditOp::Remove {
            provider_id: "tidak-ada".to_string(),
        },
    )
    .unwrap();
    assert!(preview.files.iter().all(|file| !file.changed));

    // --- backup bisa dipulihkan
    let backups = api::list_backups("pi".into()).unwrap();
    assert!(!backups.is_empty(), "harus ada backup");
    let restored = api::restore_backup("pi".into(), backups[0].id.clone()).unwrap();
    assert!(!restored.files_written.is_empty());
    // backup restore sendiri juga memicu backup baru
    assert!(restored.backup_dir.is_some());
}

#[test]
fn omp_yaml_merge() {
    let (_lock, root) = sandbox("pi_merge_backup_and_redaction");
    let dir = root.join("omp/agent");

    write(
        &dir.join("models.yml"),
        r#"# komentar omp
providers:
  existing:
    baseUrl: https://example.com/v1
    api: openai-completions
    modelOverrides:
      deepseek-v4-pro:
        input: [text, image]
    models:
      - id: m1
        name: M1
        custom: yes-kita-pertahankan
  "#,
    );
    write(
        &dir.join("config.yml"),
        "modelRoles:\n  default: existing/m1\n",
    );

    let snap = snapshot("omp");
    assert_eq!(snap.providers.len(), 1);
    assert_eq!(snap.default_provider.as_deref(), Some("existing"));

    api::apply_edit(
        "omp".into(),
        upsert("groq", Some("gsk-rahasia"), KeyMode::Inline),
    )
    .unwrap();

    let models = yaml(&dir.join("models.yml"));
    assert_eq!(
        models["providers"]["existing"]["modelOverrides"]["deepseek-v4-pro"]["input"][1],
        "image"
    );
    assert_eq!(
        models["providers"]["existing"]["models"][0]["custom"],
        "yes-kita-pertahankan"
    );
    assert_eq!(models["providers"]["groq"]["apiKey"], "gsk-rahasia");
    assert_eq!(models["providers"]["groq"]["models"][0]["id"], "model-a");

    // file yang memuat key harus 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.join("models.yml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "models.yml berisi key, harus 0600");
    }

    // pratinjau menyamarkan apiKey
    let preview = api::preview_edit(
        "omp".into(),
        upsert("groq", Some("gsk-baru"), KeyMode::Inline),
    )
    .unwrap();
    let diff = preview
        .files
        .iter()
        .find(|file| file.role == "models")
        .unwrap();
    let after = diff.after.as_deref().unwrap_or_default();
    assert!(!after.contains("gsk-baru"), "key bocor di pratinjau YAML");
    assert!(
        !after.contains("gsk-rahasia"),
        "key lama bocor di pratinjau YAML"
    );
}

#[test]
fn omp_disable_toggle_is_reversible() {
    let (_lock, root) = sandbox("omp-toggle");
    let dir = root.join("omp/agent");

    write(
        &dir.join("models.yml"),
        "providers:\n  groq:\n    baseUrl: https://api.groq.com/openai/v1\n    api: openai-completions\n",
    );
    write(&dir.join("config.yml"), "modelRoles:\n  default: groq/m\n");

    // semua provider aktif secara default
    let snap = snapshot("omp");
    assert!(snap.providers[0].enabled);
    assert!(snap.providers[0].can_toggle);

    // nonaktifkan → tercatat di disabledProviders, entry tetap ada
    api::apply_edit(
        "omp".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["disabledProviders"][0], "groq");
    let models = yaml(&dir.join("models.yml"));
    assert!(
        models["providers"].get("groq").is_some(),
        "entry tidak boleh hilang"
    );
    assert!(!snapshot("omp").providers[0].enabled);

    // nonaktifkan dua kali tidak menduplikasi
    api::apply_edit(
        "omp".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["disabledProviders"].as_array().unwrap().len(), 1);

    // aktifkan lagi → daftar bersih
    api::apply_edit(
        "omp".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["disabledProviders"].as_array().unwrap().len(), 0);
    assert!(snapshot("omp").providers[0].enabled);
}

#[test]
fn pi_disable_comments_out_instead_of_removing() {
    let (_lock, root) = sandbox("pi-park");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "groq": {
      "name": "Groq",
      "baseUrl": "https://api.groq.com/openai/v1",
      "api": "openai-completions",
      "models": [{ "id": "m1", "name": "M1" }]
    }
  }
}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"groq":{"type":"api_key","key":"gsk-abc"}}"#,
    );

    let snap = snapshot("pi");
    assert!(snap.providers[0].enabled);
    assert!(snap.providers[0].can_toggle, "Pi sekarang mendukung toggle");

    // nonaktifkan
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let raw = std::fs::read_to_string(dir.join("models.json")).unwrap();
    assert!(
        raw.contains("hakonfig:disabled groq"),
        "harus ada penanda parkir: {raw}"
    );
    assert!(
        raw.contains("Groq") && raw.contains("//"),
        "entri harus dikomentari, bukan hilang: {raw}"
    );
    // file tetap JSONC valid (bisa diparse) dan provider tidak lagi aktif
    let parsed = crate::engine::doc::parse_json_like(&raw).unwrap();
    assert!(parsed["providers"].get("groq").is_none());
    // key di auth.json tidak dihapus
    let auth = json(&dir.join("auth.json"));
    assert_eq!(auth["groq"]["key"], "gsk-abc");

    // tetap tampil di daftar sebagai nonaktif
    let snap = snapshot("pi");
    let groq = snap.providers.iter().find(|p| p.id == "groq").unwrap();
    assert!(!groq.enabled);
    assert!(groq.can_toggle);
    assert_eq!(groq.name, "Groq");
    assert_eq!(
        groq.models.len(),
        1,
        "model ikut terbaca dari blok komentar"
    );
    assert!(groq.has_key);

    // aktifkan lagi -> kembali ke tree, komentar hilang
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let raw = std::fs::read_to_string(dir.join("models.json")).unwrap();
    assert!(!raw.contains("hakonfig:disabled"), "penanda hilang: {raw}");
    let parsed = crate::engine::doc::parse_json_like(&raw).unwrap();
    assert_eq!(parsed["providers"]["groq"]["name"], "Groq");
    assert!(snapshot("pi").providers[0].enabled);
}

#[test]
fn removing_disabled_provider_drops_comment_block() {
    let (_lock, root) = sandbox("pi-park-delete");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"groq":{"name":"Groq","baseUrl":"https://x/v1","api":"openai-completions"}}}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    assert!(std::fs::read_to_string(dir.join("models.json"))
        .unwrap()
        .contains("hakonfig:disabled"));

    // hapus = benar-benar buang, termasuk blok komentarnya
    api::apply_edit(
        "pi".into(),
        EditOp::Remove {
            provider_id: "groq".to_string(),
        },
    )
    .unwrap();
    let raw = std::fs::read_to_string(dir.join("models.json")).unwrap();
    assert!(
        !raw.contains("hakonfig:disabled"),
        "blok parkir harus hilang: {raw}"
    );
    assert!(!raw.contains("Groq"), "nama provider harus hilang: {raw}");
    assert!(snapshot("pi").providers.is_empty());
}

#[test]
fn hermes_disable_comments_out_in_yaml() {
    let (_lock, root) = sandbox("hermes-park");
    let dir = root.join("hermes");
    write(
        &dir.join("config.yaml"),
        "providers:\n  groq:\n    name: Groq\n    base_url: https://api.groq.com/openai/v1\n    key_env: GROQ_API_KEY\n    api_mode: chat_completions\n",
    );
    write(&dir.join(".env"), "GROQ_API_KEY=gsk-abc\n");

    api::apply_edit(
        "hermes".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let raw = std::fs::read_to_string(dir.join("config.yaml")).unwrap();
    assert!(
        raw.contains("# hakonfig:disabled groq"),
        "yaml parkir: {raw}"
    );
    let parsed: serde_json::Value = serde_yaml::from_str(&raw).unwrap();
    assert!(parsed["providers"].get("groq").is_none());
    // key di .env tetap
    assert!(std::fs::read_to_string(dir.join(".env"))
        .unwrap()
        .contains("GROQ_API_KEY=gsk-abc"));

    let snap = snapshot("hermes");
    assert!(!snap.providers[0].enabled);

    api::apply_edit(
        "hermes".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let raw = std::fs::read_to_string(dir.join("config.yaml")).unwrap();
    assert!(!raw.contains("hakonfig:disabled"), "penanda hilang: {raw}");
    let parsed: serde_json::Value = serde_yaml::from_str(&raw).unwrap();
    assert_eq!(parsed["providers"]["groq"]["name"], "Groq");
}

#[test]
fn opencode_provider_and_auth() {
    let (_lock, root) = sandbox("pi_merge_backup_and_redaction");
    let config_dir = root.join("opencode/config");
    let data_dir = root.join("opencode/data");

    write(
        &config_dir.join("opencode.json"),
        r#"{
  "$schema": "https://opencode.ai/config.json",
  "model": "existing/m1",
  "provider": {
    "existing": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Existing",
      "options": { "baseURL": "https://example.com/v1", "timeout": 5000 },
      "models": {
        "m1": { "name": "M1", "limit": { "context": 1000, "output": 100 }, "attachment": true }
      }
    }
  }
}"#,
    );
    write(
        &data_dir.join("auth.json"),
        r#"{"existing":{"type":"api","key":"key-lama-jangan-hilang"}}"#,
    );

    let snap = snapshot("opencode");
    assert_eq!(snap.providers.len(), 1);
    assert_eq!(snap.providers[0].models[0].id, "m1");
    assert_eq!(snap.default_model.as_deref(), Some("m1"));

    api::apply_edit(
        "opencode".into(),
        upsert("groq", Some("gsk-oc-rahasia"), KeyMode::Inline),
    )
    .unwrap();

    let config = json(&config_dir.join("opencode.json"));
    assert_eq!(
        config["provider"]["groq"]["npm"],
        "@ai-sdk/openai-compatible"
    );
    assert_eq!(
        config["provider"]["groq"]["options"]["baseURL"],
        "https://api.example.com/v1"
    );
    assert!(config["provider"]["groq"]["options"]
        .get("apiKey")
        .is_none());
    assert_eq!(
        config["provider"]["groq"]["models"]["model-a"]["limit"]["context"],
        128000
    );
    assert_eq!(
        config["provider"]["groq"]["models"]["model-a"]["limit"]["output"],
        8192
    );

    // provider lama utuh, termasuk setelan tak dikenal
    assert_eq!(config["provider"]["existing"]["options"]["timeout"], 5000);
    assert_eq!(
        config["provider"]["existing"]["models"]["m1"]["attachment"],
        true
    );

    let auth = json(&data_dir.join("auth.json"));
    assert_eq!(auth["groq"]["type"], "api");
    assert_eq!(auth["groq"]["key"], "gsk-oc-rahasia");
    assert_eq!(auth["existing"]["key"], "key-lama-jangan-hilang");

    // toggle nonaktif: entry tetap ada, tercatat di disabled_providers
    assert!(snapshot("opencode").providers.iter().all(|p| p.enabled));
    api::apply_edit(
        "opencode".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let config = json(&config_dir.join("opencode.json"));
    assert_eq!(config["disabled_providers"][0], "groq");
    assert!(config["provider"].get("groq").is_some());
    let groq = snapshot("opencode")
        .providers
        .into_iter()
        .find(|p| p.id == "groq")
        .unwrap();
    assert!(!groq.enabled);
    assert!(groq.can_toggle);

    api::apply_edit(
        "opencode".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let config = json(&config_dir.join("opencode.json"));
    assert_eq!(config["disabled_providers"].as_array().unwrap().len(), 0);

    // hapus
    api::apply_edit(
        "opencode".into(),
        EditOp::Remove {
            provider_id: "groq".to_string(),
        },
    )
    .unwrap();
    let config = json(&config_dir.join("opencode.json"));
    assert!(config["provider"].get("groq").is_none());
    let auth = json(&data_dir.join("auth.json"));
    assert!(auth.get("groq").is_none());
    assert!(auth.get("existing").is_some());
}

#[test]
fn hermes_config_and_env() {
    let (_lock, root) = sandbox("pi_merge_backup_and_redaction");
    let dir = root.join("hermes");

    write(
        &dir.join("config.yaml"),
        r#"model:
  default: existing/m1
  provider: "custom:existing"
providers:
  existing:
    name: "Existing"
    base_url: "https://example.com/v1"
    key_env: "EXISTING_API_KEY"
    api_mode: "chat_completions"
"#,
    );
    write(
        &dir.join(".env"),
        "# kredensial hermes\nEXISTING_API_KEY=key-lama\nOTHER=biarkan\n",
    );

    let snap = snapshot("hermes");
    assert_eq!(snap.providers.len(), 1);
    assert_eq!(snap.default_provider.as_deref(), Some("existing"));
    assert!(snap.providers[0].has_key);

    api::apply_edit(
        "hermes".into(),
        upsert("groq", Some("gsk-hermes"), KeyMode::Inline),
    )
    .unwrap();

    let config = yaml(&dir.join("config.yaml"));
    assert_eq!(config["providers"]["groq"]["name"], "Provider groq");
    assert_eq!(
        config["providers"]["groq"]["base_url"],
        "https://api.example.com/v1"
    );
    assert_eq!(config["providers"]["groq"]["key_env"], "GROQ_API_KEY");
    assert_eq!(config["providers"]["groq"]["api_mode"], "chat_completions");
    // provider lama utuh
    assert_eq!(
        config["providers"]["existing"]["key_env"],
        "EXISTING_API_KEY"
    );

    let env = fs::read_to_string(dir.join(".env")).unwrap();
    assert!(env.contains("GROQ_API_KEY=gsk-hermes"), "env: {env}");
    assert!(env.contains("EXISTING_API_KEY=key-lama"), "env: {env}");
    assert!(env.contains("OTHER=biarkan"), "env: {env}");
    assert!(
        env.contains("# kredensial hermes"),
        "komentar .env harus awet: {env}"
    );

    // mengganti key yang sama tidak menduplikasi baris
    api::apply_edit(
        "hermes".into(),
        upsert("groq", Some("gsk-hermes-baru"), KeyMode::Inline),
    )
    .unwrap();
    let env = fs::read_to_string(dir.join(".env")).unwrap();
    assert_eq!(env.matches("GROQ_API_KEY=").count(), 1, "env: {env}");
    assert!(env.contains("GROQ_API_KEY=gsk-hermes-baru"));

    // hapus provider ikut membersihkan env-nya
    api::apply_edit(
        "hermes".into(),
        EditOp::Remove {
            provider_id: "groq".to_string(),
        },
    )
    .unwrap();
    let config = yaml(&dir.join("config.yaml"));
    assert!(config["providers"].get("groq").is_none());
    let env = fs::read_to_string(dir.join(".env")).unwrap();
    assert!(!env.contains("GROQ_API_KEY"), "env: {env}");
    assert!(env.contains("OTHER=biarkan"));
}

#[test]
fn invalid_input_is_rejected() {
    let (_lock, _root) = sandbox("validasi");
    let mut bad = draft("Groq Salah!", Some("k"), KeyMode::Inline);
    assert!(api::preview_edit("pi".into(), EditOp::Upsert(bad.clone())).is_err());

    bad = draft("groq", Some("k"), KeyMode::Inline);
    bad.base_url = "api.example.com".to_string();
    assert!(api::preview_edit("pi".into(), EditOp::Upsert(bad.clone())).is_err());

    bad = draft("groq", Some("k"), KeyMode::Inline);
    bad.models.clear();
    assert!(api::preview_edit("pi".into(), EditOp::Upsert(bad)).is_err());
}

// ------------------------------------------------------------ stored key

#[test]
fn stored_key_is_read_per_adapter() {
    let (_lock, root) = sandbox("stored_key");

    // --- Pi: auth.json menang atas apiKey; apiKey boleh berupa $VAR.
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{
  "providers": {
    "groq": { "name": "Groq", "baseUrl": "https://api.groq.com/openai/v1", "api": "openai-completions", "apiKey": "pi-apiKey", "models": [{ "id": "m1" }] },
    "openai": { "name": "OpenAI", "baseUrl": "https://api.openai.com/v1", "api": "openai-completions", "apiKey": "$OPENAI_API_KEY", "models": [{ "id": "gpt" }] },
    "tanpa-key": { "name": "Tanpa", "baseUrl": "https://x.test/v1", "api": "openai-completions", "models": [{ "id": "m" }] }
  }
}"#,
    );
    write(
        &pi.join("auth.json"),
        r#"{ "groq": { "type": "api_key", "key": "pi-auth" } }"#,
    );
    assert_eq!(stored_key("pi", "groq").as_deref(), Some("pi-auth"));
    assert_eq!(
        stored_key("pi", "openai").as_deref(),
        Some("$OPENAI_API_KEY"),
        "referensi environment diteruskan apa adanya"
    );
    assert_eq!(stored_key("pi", "tanpa-key"), None);
    assert_eq!(stored_key("pi", "tidak-ada"), None);

    // Provider yang dinonaktifkan (dikomentari) tetap punya key tersimpan.
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    assert_eq!(
        stored_key("pi", "groq").as_deref(),
        Some("pi-auth"),
        "provider nonaktif tetap bisa dites"
    );

    // --- oh-my-pi: key ada di apiKey models.yml.
    let omp = root.join("omp/agent");
    write(
        &omp.join("models.yml"),
        "providers:\n  groq:\n    baseUrl: https://api.groq.com/openai/v1\n    api: openai-completions\n    apiKey: omp-key\n    models:\n      - id: m1\n",
    );
    assert_eq!(stored_key("omp", "groq").as_deref(), Some("omp-key"));

    // --- OpenCode: auth.json → options.apiKey → env[0].
    let oc = root.join("opencode/config");
    write(
        &oc.join("opencode.json"),
        r#"{
  "provider": {
    "langsung": { "npm": "@ai-sdk/openai-compatible", "options": { "baseURL": "https://a.test/v1", "apiKey": "oc-key" } },
    "envvar": { "npm": "@ai-sdk/openai-compatible", "options": { "baseURL": "https://b.test/v1" }, "env": ["MY_KEY"] }
  }
}"#,
    );
    write(
        &root.join("opencode/data/auth.json"),
        r#"{ "login": { "type": "api", "key": "oc-auth" } }"#,
    );
    assert_eq!(
        stored_key("opencode", "langsung").as_deref(),
        Some("oc-key")
    );
    assert_eq!(
        stored_key("opencode", "envvar").as_deref(),
        Some("$MY_KEY"),
        "nama variabel diteruskan untuk di-resolve backend"
    );
    assert_eq!(stored_key("opencode", "login").as_deref(), Some("oc-auth"));

    // --- Hermes: key_env → nilai di .env, atau $VAR bila .env kosong.
    let hermes = root.join("hermes");
    write(
        &hermes.join("config.yaml"),
        r#"providers:
  groq:
    name: Groq
    base_url: https://api.groq.com/openai/v1
    key_env: GROQ_API_KEY
    api_mode: chat_completions
  kosong:
    name: Kosong
    base_url: https://b.test/v1
    key_env: KOSONG_API_KEY
    api_mode: chat_completions
"#,
    );
    write(&hermes.join(".env"), "GROQ_API_KEY=hermes-key\n");
    assert_eq!(stored_key("hermes", "groq").as_deref(), Some("hermes-key"));
    assert_eq!(
        stored_key("hermes", "kosong").as_deref(),
        Some("$KOSONG_API_KEY")
    );
}

#[test]
fn stored_key_matches_snapshot_has_key() {
    let (_lock, root) = sandbox("stored_key_has_key");
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{ "providers": {
  "ada": { "name": "Ada", "baseUrl": "https://a.test/v1", "api": "openai-completions", "apiKey": "kunci", "models": [{ "id": "m" }] },
  "tidak": { "name": "Tidak", "baseUrl": "https://b.test/v1", "api": "openai-completions", "models": [{ "id": "m" }] }
} }"#,
    );
    write(&pi.join("auth.json"), "{}");

    for provider in snapshot("pi").providers {
        assert_eq!(
            provider.has_key,
            stored_key("pi", &provider.id).is_some(),
            "provider {}: hasKey dan stored_key harus sepakat",
            provider.id
        );
    }
}

#[test]
fn request_key_uses_stored_key_when_field_is_empty() {
    let (_lock, root) = sandbox("request_key");
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{ "providers": {
  "dari-file": { "name": "File", "baseUrl": "https://a.test/v1", "api": "openai-completions", "apiKey": "file-key", "models": [{ "id": "m" }] },
  "dari-env": { "name": "Env", "baseUrl": "https://b.test/v1", "api": "openai-completions", "apiKey": "$HAKONFIG_TEST_KEY", "models": [{ "id": "m" }] }
} }"#,
    );
    write(&pi.join("auth.json"), "{}");

    // key yang diketik pengguna selalu menang
    assert_eq!(
        api::request_key("diketik", Some("pi"), Some("dari-file")).unwrap(),
        "diketik"
    );
    // kolom kosong → baca dari config
    assert_eq!(
        api::request_key("   ", Some("pi"), Some("dari-file")).unwrap(),
        "file-key"
    );
    // $VAR di-resolve dari environment
    std::env::set_var("HAKONFIG_TEST_KEY", "env-key");
    assert_eq!(
        api::request_key("", Some("pi"), Some("dari-env")).unwrap(),
        "env-key"
    );
    std::env::remove_var("HAKONFIG_TEST_KEY");
    // tanpa identitas provider / provider tak dikenal → kosong, bukan error
    assert_eq!(api::request_key("", None, None).unwrap(), "");
    assert_eq!(
        api::request_key("", Some("pi"), Some("tidak-ada")).unwrap(),
        ""
    );
}

// ------------------------------------------------- edit tanpa key baru

#[test]
fn edit_without_new_key_keeps_credentials() {
    let (_lock, root) = sandbox("edit_tanpa_key");

    // --- Pi: apiKey inline di models.json harus selamat saat nama diganti.
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{ "providers": {
  "inline": { "name": "Lama", "baseUrl": "https://a.test/v1", "api": "openai-completions", "apiKey": "sk-inline-asli", "models": [{ "id": "m1" }] },
  "envref": { "name": "Env", "baseUrl": "https://b.test/v1", "api": "openai-completions", "apiKey": "$ENVREF_API_KEY", "models": [{ "id": "m1" }] }
} }"#,
    );
    write(&pi.join("auth.json"), "{}");

    let mut renamed = draft("inline", None, KeyMode::Inline);
    renamed.name = "Nama Baru".to_string();
    api::apply_edit("pi".into(), EditOp::Upsert(renamed)).unwrap();

    let models = json(&pi.join("models.json"));
    assert_eq!(
        models["providers"]["inline"]["apiKey"], "sk-inline-asli",
        "apiKey inline tidak boleh hilang saat edit tanpa key baru"
    );
    assert_eq!(models["providers"]["inline"]["name"], "Nama Baru");
    assert_eq!(
        stored_key("pi", "inline").as_deref(),
        Some("sk-inline-asli"),
        "key masih bisa dipakai Test/Sync setelah edit"
    );

    // Referensi environment juga tidak boleh dihapus.
    let mut env_edit = draft("envref", None, KeyMode::Inline);
    env_edit.name = "Env Baru".to_string();
    api::apply_edit("pi".into(), EditOp::Upsert(env_edit)).unwrap();
    let models = json(&pi.join("models.json"));
    assert_eq!(models["providers"]["envref"]["apiKey"], "$ENVREF_API_KEY");

    // --- OpenCode: options.apiKey dan env[] harus selamat.
    let oc = root.join("opencode/config");
    write(
        &oc.join("opencode.json"),
        r#"{ "provider": {
  "langsung": { "npm": "@ai-sdk/openai-compatible", "options": { "baseURL": "https://a.test/v1", "apiKey": "oc-asli" }, "models": { "m1": {} } },
  "envvar": { "npm": "@ai-sdk/openai-compatible", "options": { "baseURL": "https://b.test/v1" }, "env": ["OC_KEY"], "models": { "m1": {} } }
} }"#,
    );
    write(&root.join("opencode/data/auth.json"), "{}");

    let mut oc_edit = draft("langsung", None, KeyMode::Inline);
    oc_edit.name = "Langsung Baru".to_string();
    api::apply_edit("opencode".into(), EditOp::Upsert(oc_edit)).unwrap();

    let config = json(&oc.join("opencode.json"));
    assert_eq!(
        config["provider"]["langsung"]["options"]["apiKey"], "oc-asli",
        "options.apiKey tidak boleh hilang saat edit tanpa key baru"
    );
    assert_eq!(config["provider"]["langsung"]["name"], "Langsung Baru");

    let mut oc_env = draft("envvar", None, KeyMode::Inline);
    oc_env.name = "Envvar Baru".to_string();
    api::apply_edit("opencode".into(), EditOp::Upsert(oc_env)).unwrap();
    let config = json(&oc.join("opencode.json"));
    assert_eq!(
        config["provider"]["envvar"]["env"][0], "OC_KEY",
        "daftar env tidak boleh dihapus saat edit tanpa key baru"
    );

    // --- Hermes: nilai di .env harus selamat.
    let hermes = root.join("hermes");
    write(
        &hermes.join("config.yaml"),
        "providers:\n  tog:\n    name: Tog\n    base_url: https://t.test/v1\n    key_env: TOG_API_KEY\n    api_mode: chat_completions\n",
    );
    write(&hermes.join(".env"), "TOG_API_KEY=tok-asli\n");
    let mut h_edit = draft("tog", None, KeyMode::Inline);
    h_edit.name = "Tog Baru".to_string();
    api::apply_edit("hermes".into(), EditOp::Upsert(h_edit)).unwrap();
    let env = fs::read_to_string(hermes.join(".env")).unwrap();
    assert!(env.contains("TOG_API_KEY=tok-asli"), "env: {env}");
}

#[test]
fn new_key_still_overwrites_old_key() {
    let (_lock, root) = sandbox("edit_key_baru");
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{ "providers": { "groq": { "name": "Groq", "baseUrl": "https://a.test/v1", "api": "openai-completions", "apiKey": "sk-lama", "models": [{ "id": "m1" }] } } }"#,
    );
    write(&pi.join("auth.json"), "{}");

    api::apply_edit(
        "pi".into(),
        upsert("groq", Some("sk-baru"), KeyMode::Inline),
    )
    .unwrap();

    // Inline → key pindah ke auth.json, apiKey lama dibersihkan.
    let auth = json(&pi.join("auth.json"));
    assert_eq!(auth["groq"]["key"], "sk-baru");
    let models = json(&pi.join("models.json"));
    assert!(models["providers"]["groq"].get("apiKey").is_none());
    assert_eq!(stored_key("pi", "groq").as_deref(), Some("sk-baru"));

    // Mode env → apiKey jadi referensi $VAR.
    api::apply_edit("pi".into(), upsert("groq", Some("apa saja"), KeyMode::Env)).unwrap();
    let models = json(&pi.join("models.json"));
    assert_eq!(models["providers"]["groq"]["apiKey"], "$GROQ_API_KEY");
    let auth = json(&pi.join("auth.json"));
    assert!(auth.get("groq").is_none(), "mode env tidak menyimpan key");
}

#[test]
fn catalog_contains_dsh_and_openhands() {
    let entries = crate::engine::catalog::all();
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert!(
        ids.contains(&"dsh"),
        "DeepSeek Harness harus ada di katalog"
    );
    assert!(ids.contains(&"openhands"), "openhands harus ada di katalog");
    // entri web chat yang lama tidak boleh kembali
    assert!(
        !ids.contains(&"deepseek"),
        "web chat bukan harness, jangan didaftarkan"
    );

    let dsh = crate::engine::catalog::get("dsh").unwrap();
    assert_eq!(dsh.name, "DeepSeek Harness");
    assert_eq!(
        dsh.install_command().as_deref(),
        Some("npm install -g @deepseek-ai/dsh")
    );
    assert_eq!(dsh.binary.as_deref(), Some("dsh"));
    assert_eq!(dsh.kind, "planned");

    let hands = crate::engine::catalog::get("openhands").unwrap();
    assert_eq!(
        hands.install_command().as_deref(),
        Some("npm install -g @openhands/agent-canvas")
    );
}

#[test]
fn default_pi_write_also_updates_enabled_models() {
    // Pi memakai model pertama dari `enabledModels` selama scope tidak kosong,
    // jadi menulis defaultProvider/defaultModel saja tidak cukup.
    let (_lock, root) = sandbox("pi_default_scope");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{ "providers": { "deepseek": { "name": "DeepSeek", "baseUrl": "https://api.deepseek.com", "api": "openai-completions", "models": [{ "id": "deepseek-flash" }] } } }"#,
    );
    write(&dir.join("auth.json"), "{}");
    write(
        &dir.join("settings.json"),
        r#"{ "defaultProvider": "commandcode", "defaultModel": "commandcode/claude-sonnet-5-5",
        "enabledModels": ["commandcode/*", "deepseek/*"] }"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetDefault {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-flash".to_string(),
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(settings["defaultProvider"], "deepseek");
    assert_eq!(settings["defaultModel"], "deepseek-flash");

    let scope: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("enabledModels tetap array")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        scope.first().copied(),
        Some("deepseek/deepseek-flash"),
        "model default harus jadi entri pertama scope: {scope:?}"
    );
    assert_eq!(
        scope
            .iter()
            .filter(|item| **item == "deepseek/deepseek-flash")
            .count(),
        1,
        "tidak boleh ada duplikat: {scope:?}"
    );
    // entri glob yang lama tetap ada
    assert!(
        scope.contains(&"commandcode/*"),
        "scope lama hilang: {scope:?}"
    );
}

#[test]
fn default_pi_without_scope_does_not_create_enabled_models() {
    let (_lock, root) = sandbox("pi_default_no_scope");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{ "providers": { "groq": { "name": "Groq", "baseUrl": "https://api.groq.com/openai/v1", "api": "openai-completions", "models": [{ "id": "m1" }] } } }"#,
    );
    write(&dir.join("auth.json"), "{}");
    write(&dir.join("settings.json"), r#"{ "theme": "dark" }"#);

    api::apply_edit(
        "pi".into(),
        EditOp::SetDefault {
            provider_id: "groq".to_string(),
            model_id: "m1".to_string(),
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(settings["defaultModel"], "m1");
    assert!(
        settings.get("enabledModels").is_none(),
        "jangan mengarang scope bila user belum punya"
    );
}

#[test]
fn default_omp_write_also_updates_enabled_models() {
    // oh-my-pi adalah turunan Pi: model pertama scope `enabledModels` menang
    // atas `modelRoles.default`, jadi default harus dipindah ke urutan pertama.
    let (_lock, root) = sandbox("omp_default_scope");
    let dir = root.join("omp/agent");
    write(
        &dir.join("models.yml"),
        "providers:\n  deepseek:\n    baseUrl: https://api.deepseek.com\n    api: openai-completions\n    apiKey: k\n    models:\n      - id: deepseek-flash\n",
    );
    write(
        &dir.join("config.yml"),
        "modelRoles:\n  default: commandcode/claude-sonnet-5-5\nenabledModels:\n  - commandcode/*\n  - deepseek/*\n",
    );

    api::apply_edit(
        "omp".into(),
        EditOp::SetDefault {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-flash".to_string(),
        },
    )
    .unwrap();

    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["modelRoles"]["default"], "deepseek/deepseek-flash");
    let scope: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("enabledModels tetap list")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        scope.first().copied(),
        Some("deepseek/deepseek-flash"),
        "default harus jadi entri pertama scope: {scope:?}"
    );
    assert_eq!(
        scope
            .iter()
            .filter(|item| **item == "deepseek/deepseek-flash")
            .count(),
        1,
        "tidak boleh duplikat: {scope:?}"
    );
    assert!(
        scope.contains(&"commandcode/*"),
        "scope lama hilang: {scope:?}"
    );
}

#[test]
fn default_omp_without_scope_does_not_invent_enabled_models() {
    let (_lock, root) = sandbox("omp_default_no_scope");
    let dir = root.join("omp/agent");
    write(
        &dir.join("models.yml"),
        "providers:\n  groq:\n    baseUrl: https://api.groq.com/openai/v1\n    api: openai-completions\n    apiKey: k\n    models:\n      - id: m1\n",
    );
    write(&dir.join("config.yml"), "theme: dark\n");

    api::apply_edit(
        "omp".into(),
        EditOp::SetDefault {
            provider_id: "groq".to_string(),
            model_id: "m1".to_string(),
        },
    )
    .unwrap();

    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["modelRoles"]["default"], "groq/m1");
    assert!(
        settings.get("enabledModels").is_none(),
        "jangan mengarang scope bila user belum punya"
    );
}

// ------------------------------------------------------------------ notices

#[test]
fn pi_scope_notice_reports_models_hidden_by_enabled_models() {
    let (_lock, root) = sandbox("notices_pi_scope");
    let dir = root.join("pi/agent");
    // `provider/*` tidak melewati garis miring, jadi model bersarang hilang.
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"},{"id":"deepseek/b"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/*"]}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    let scope = notices
        .iter()
        .find(|notice| notice.code == "pi.scope")
        .expect("harus ada notice scope");
    assert_eq!(scope.provider, "gateway");
    assert!(
        scope.message.contains("gateway/deepseek/b"),
        "pesan harus menyebut contoh model yang tersembunyi: {}",
        scope.message
    );
    assert!(
        scope.message.contains("gateway/**"),
        "pesan harus menyebut pola yang benar: {}",
        scope.message
    );
    assert!(scope.fix.is_some(), "harus ada perbaikan otomatis");
}

#[test]
fn pi_scope_fix_widens_the_pattern_and_writes_a_backup() {
    let (_lock, root) = sandbox("notices_pi_fix");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"deepseek/b"}]},"other":{"baseUrl":"https://o.test/v1","api":"openai-completions","models":[{"id":"x"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/*","other/**"],"theme":"dark"}"#,
    );

    let result = api::fix_notice("pi".into(), "pi.scope".into(), "gateway".into()).unwrap();
    assert!(
        result.backup_dir.is_some(),
        "perbaikan harus membuat backup"
    );
    assert!(
        result.notices.is_empty(),
        "setelah diperbaiki tidak boleh ada notice lagi: {:?}",
        result.notices
    );

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert!(patterns.contains(&"gateway/**"), "{patterns:?}");
    assert!(
        !patterns.contains(&"gateway/*"),
        "pola sempit harus diganti: {patterns:?}"
    );
    assert!(
        patterns.contains(&"other/**"),
        "pola lain tidak boleh hilang: {patterns:?}"
    );
    assert_eq!(settings["theme"], "dark");
}

#[test]
fn pi_without_enabled_models_has_no_scope_notice() {
    let (_lock, root) = sandbox("notices_pi_no_scope");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );
    write(&dir.join("settings.json"), "{}");

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    assert!(
        !notices.iter().any(|notice| notice.code == "pi.scope"),
        "tanpa enabledModels semua model terlihat: {notices:?}"
    );
}

#[test]
fn a_deliberately_disabled_provider_is_not_reported_as_a_problem() {
    // Provider yang ada di `disabledProviders` adalah pilihan pengguna (dan
    // persis yang ditulis tombol toggle kita). Melaporkannya sebagai masalah
    // membuat `doctor --fix` menyalakannya kembali tanpa diminta.
    let (_lock, root) = sandbox("notices_omp_disabled");
    let dir = root.join("omp/agent");
    write(
        &dir.join("models.yml"),
        "providers:\n  gateway:\n    baseUrl: https://gw.test/v1\n    api: openai-completions\n    apiKey: k\n    models:\n      - id: m1\n",
    );
    write(
        &dir.join("config.yml"),
        "disabledProviders:\n  - gateway\ntheme: dark\n",
    );

    let notices = api::list_notices(Some("omp".to_string())).unwrap();
    assert!(
        !notices
            .iter()
            .any(|notice| notice.code == "omp.disabled" || notice.provider == "gateway"),
        "provider yang sengaja dimatikan tidak boleh dilaporkan: {notices:?}"
    );

    // Fix yang membatalkan disable juga tidak boleh ditawarkan.
    let error = api::fix_notice("omp".into(), "omp.disabled".into(), "gateway".into()).unwrap_err();
    assert!(
        error.contains("no 'omp.disabled' notice"),
        "fix harus ditolak karena notice-nya tidak ada: {error}"
    );

    // Dan statusnya tetap seperti yang dipilih pengguna.
    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(settings["disabledProviders"][0], "gateway");
    assert_eq!(settings["theme"], "dark");
}

#[test]
fn builtin_id_collision_is_reported_without_a_fix() {
    let (_lock, root) = sandbox("notices_builtin_id");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"openrouter":{"baseUrl":"https://my-gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    let collision = notices
        .iter()
        .find(|notice| notice.code == "harness.builtin-id")
        .expect("harness harus melaporkan bentrok id");
    assert_eq!(collision.provider, "openrouter");
    assert!(
        collision.fix.is_none(),
        "bentrok id tidak bisa diperbaiki otomatis"
    );
    assert!(collision.message.contains("openrouter-gw"));
}

#[test]
fn apply_widens_the_pi_scope_so_the_new_models_are_visible() {
    let (_lock, root) = sandbox("notices_apply");
    let dir = root.join("pi/agent");
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["lain/**","gateway/*"]}"#,
    );

    write(
        &dir.join("models.json"),
        r#"{"providers":{"lain":{"baseUrl":"https://l.test/v1","api":"openai-completions","models":[{"id":"x"}]}}}"#,
    );

    let result =
        api::apply_edit("pi".into(), upsert("gateway", Some("k"), KeyMode::Inline)).unwrap();

    // Perbaikan terjadi saat menulis, bukan sesudahnya: pengguna tidak perlu
    // menjalankan apa pun.
    assert!(
        result.notices.is_empty(),
        "tidak boleh ada notice yang tersisa: {:?}",
        result.notices
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("enabledModels")),
        "perubahan settings harus dilaporkan: {:?}",
        result.warnings
    );

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert!(patterns.contains(&"gateway/**"), "{patterns:?}");
    assert!(!patterns.contains(&"gateway/*"), "{patterns:?}");
    assert!(patterns.contains(&"lain/**"), "{patterns:?}");
}

#[test]
fn apply_does_not_invent_a_scope_when_pi_has_none() {
    let (_lock, root) = sandbox("notices_apply_no_scope");
    let dir = root.join("pi/agent");
    write(&dir.join("settings.json"), r#"{"theme":"dark"}"#);

    api::apply_edit("pi".into(), upsert("gateway", Some("k"), KeyMode::Inline)).unwrap();

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
    assert!(
        settings.get("enabledModels").is_none(),
        "jangan mengarang scope: {settings:?}"
    );
}

#[test]
fn apply_re_enables_a_disabled_provider() {
    let (_lock, root) = sandbox("notices_apply_reenable");
    let dir = root.join("omp/agent");
    write(&dir.join("models.yml"), "providers: {}\n");
    write(
        &dir.join("config.yml"),
        "disabledProviders:\n  - gateway\n  - lain\ntheme: dark\n",
    );

    api::apply_edit("omp".into(), upsert("gateway", Some("k"), KeyMode::Inline)).unwrap();

    let settings = yaml(&dir.join("config.yml"));
    let disabled: Vec<&str> = settings["disabledProviders"]
        .as_array()
        .expect("disabledProviders harus array")
        .iter()
        .map(|item| item.as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        disabled,
        vec!["lain"],
        "hanya provider ini yang diaktifkan lagi"
    );
    assert_eq!(settings["theme"], "dark");
}

#[test]
fn genuine_builtin_provider_is_not_flagged_as_a_collision() {
    let (_lock, root) = sandbox("notices_builtin_real");
    let dir = root.join("pi/agent");
    // Endpoint resmi DeepSeek: bukan gateway yang menumpang id bawaan.
    write(
        &dir.join("models.json"),
        r#"{"providers":{"deepseek":{"baseUrl":"https://api.deepseek.com","api":"openai-completions","models":[{"id":"deepseek-chat"}]}}}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    assert!(
        !notices
            .iter()
            .any(|notice| notice.code == "harness.builtin-id"),
        "provider asli tidak boleh diperingatkan: {notices:?}"
    );
}

// ------------------------------------------------- home user lain (ADE / VPS)

#[test]
fn environment_keys_of_the_runner_do_not_count_for_another_home() {
    // Saat --home menunjuk home user lain, environment yang dibaca adalah
    // environment *runner*: DEEPSEEK_API_KEY milik operator bukan kredensial
    // agent tersebut. Tanpa penjagaan ini, `doctor --home /var/lib/openbot`
    // melaporkan kredensial yang tidak pernah dimiliki agent itu.
    let (_lock, root) = sandbox("ade-env-keys");
    let target = root.join("agent-home");
    std::fs::create_dir_all(target.join(".pi/agent")).unwrap();
    write(&target.join(".pi/agent/models.json"), r#"{"providers":{}}"#);
    std::env::set_var("HAKONFIG_HOME", &target);
    std::env::set_var("DEEPSEEK_API_KEY", "sk-milik-operator");

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    std::env::remove_var("HAKONFIG_HOME");
    std::env::remove_var("DEEPSEEK_API_KEY");

    assert!(
        !notices
            .iter()
            .any(|notice| notice.code == "catalog.credential-without-provider"),
        "env runner tidak boleh dihitung untuk home lain: {notices:?}"
    );
}

#[test]
fn environment_keys_still_count_for_the_own_home() {
    // Kebalikannya juga harus benar: untuk home sendiri, env tetap dibaca.
    let (_lock, root) = sandbox("own-env-keys");
    let dir = root.join("pi/agent");
    write(&dir.join("models.json"), r#"{"providers":{}}"#);
    std::env::set_var("DEEPSEEK_API_KEY", "sk-milik-operator");

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    std::env::remove_var("DEEPSEEK_API_KEY");

    let notice = notices
        .iter()
        .find(|notice| notice.code == "catalog.credential-without-provider")
        .expect("home sendiri tetap membaca env: {notices:?}");
    assert_eq!(notice.provider, "deepseek");
}

#[test]
fn snapshot_and_harness_info_carry_the_target_home() {
    let (_lock, root) = sandbox("ade-identity");
    let target = root.join("agent-home");
    std::fs::create_dir_all(target.join(".omp/agent")).unwrap();
    write(
        &target.join(".omp/agent/models.yml"),
        "providers:\n  groq:\n    baseUrl: https://api.groq.com/openai/v1\n    api: openai-completions\n    models:\n      - id: m1\n",
    );
    std::env::set_var("HAKONFIG_HOME", &target);

    let info = api::list_harnesses();
    let snapshot = api::read_harness("omp".to_string()).unwrap();
    std::env::remove_var("HAKONFIG_HOME");

    let expected = target.display().to_string();
    let omp = info.iter().find(|entry| entry.id == "omp").unwrap();
    assert_eq!(omp.home.as_deref(), Some(expected.as_str()));
    assert_eq!(snapshot.home.as_deref(), Some(expected.as_str()));

    // Tanpa --home, field itu tidak boleh muncul sama sekali.
    let plain = api::list_harnesses();
    assert!(plain.iter().all(|entry| entry.home.is_none()));
}

// ------------------------------------------------------------------ katalog

#[test]
fn credential_without_a_provider_entry_is_reported() {
    let (_lock, root) = sandbox("catalog_credential_only");
    let dir = root.join("pi/agent");
    write(&dir.join("models.json"), r#"{"providers":{}}"#);
    write(
        &dir.join("auth.json"),
        r#"{"xai":{"type":"api_key","key":"xai-fake"}}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "catalog.credential-without-provider")
        .expect("kredensial tanpa provider harus dilaporkan");
    assert_eq!(notice.provider, "xai");
    // Pesan kini menyebut sumber kredensialnya (auth.json / env var) dan
    // menjelaskan bahwa menghapus entri config tidak cukup.
    assert!(notice.message.contains("auth.json"));
    assert!(
        notice.fix.is_some(),
        "kredensial tersimpan bisa dihapus, jadi aksinya harus tersedia"
    );
}

#[test]
fn empty_stored_catalog_is_reported() {
    let (_lock, root) = sandbox("catalog_empty_cache");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"xai":{"baseUrl":"https://api.x.ai/v1","api":"openai-completions","models":[{"id":"grok"}]}}}"#,
    );
    // Bentuk yang ditulis pi ketika permintaan katalog gagal atau ditolak.
    write(
        &dir.join("models-store.json"),
        r#"{"xai":{"models":[],"lastModified":0,"checkedAt":1794000000000}}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "catalog.empty-cache")
        .expect("cache kosong harus dilaporkan");
    assert_eq!(notice.provider, "xai");
}

#[test]
fn healthy_stored_catalog_produces_no_notice() {
    let (_lock, root) = sandbox("catalog_ok_cache");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"xai":{"baseUrl":"https://api.x.ai/v1","api":"openai-completions","models":[{"id":"grok"}]}}}"#,
    );
    write(
        &dir.join("models-store.json"),
        r#"{"xai":{"models":[{"id":"grok"}],"lastModified":1793900000000,"checkedAt":1794000000000}}"#,
    );

    let notices = api::list_notices(Some("pi".to_string())).unwrap();
    assert!(
        !notices
            .iter()
            .any(|notice| notice.code == "catalog.empty-cache"),
        "{notices:?}"
    );
}

#[test]
fn catalog_probe_targets_only_builtin_credentialed_providers() {
    let (_lock, root) = sandbox("catalog_targets");
    let dir = root.join("pi/agent");
    write(&dir.join("models.json"), r#"{"providers":{}}"#);
    write(
        &dir.join("auth.json"),
        r#"{"xai":{"type":"api_key","key":"k"},"gateway-saya":{"type":"api_key","key":"k"}}"#,
    );

    let adapter = adapters::pi::PiAdapter;
    let docs = Docs::load(&adapter.specs()).unwrap();
    let targets = adapter.catalog_targets(&docs);
    let ids: Vec<&str> = targets
        .iter()
        .map(|target| target.provider.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["xai"],
        "gateway custom tidak punya katalog di pi.dev"
    );
}

#[test]
fn disabling_a_provider_removes_its_patterns_from_the_scope() {
    let (_lock, root) = sandbox("scope_disable_prunes");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]},"lain":{"baseUrl":"https://l.test/v1","api":"openai-completions","models":[{"id":"b"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/**","lain/**"]}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "gateway".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = read_json_value(&dir.join("settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("lain masih ada")
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(patterns, vec!["lain/**"], "pola provider mati harus hilang");
}

#[test]
fn disabling_the_last_provider_drops_the_scope_key() {
    let (_lock, root) = sandbox("scope_disable_empty");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/**"]}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "gateway".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = read_json_value(&dir.join("settings.json"));
    assert!(
        settings.get("enabledModels").is_none(),
        "scope kosong harus dibuang supaya semua model terlihat: {settings:?}"
    );
}

#[test]
fn enabling_a_provider_covers_it_again() {
    let (_lock, root) = sandbox("scope_enable_covers");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"lain":{"baseUrl":"https://l.test/v1","api":"openai-completions","models":[{"id":"b"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["lain/**"]}"#,
    );

    // Provider baru dibuat, lalu dimatikan, lalu dihidupkan lagi.
    api::apply_edit("pi".into(), upsert("gateway", Some("k"), KeyMode::Inline)).unwrap();
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "gateway".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "gateway".to_string(),
            enabled: true,
        },
    )
    .unwrap();

    let settings = read_json_value(&dir.join("settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("scope harus ada")
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert!(patterns.contains(&"gateway/**"), "{patterns:?}");
    assert!(patterns.contains(&"lain/**"), "{patterns:?}");
}

#[test]
fn removing_a_provider_prunes_its_explicit_model_ids() {
    let (_lock, root) = sandbox("scope_remove_provider");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]},"lain":{"baseUrl":"https://l.test/v1","api":"openai-completions","models":[{"id":"b"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/a","gateway/a:high","lain/b"]}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::Remove {
            provider_id: "gateway".to_string(),
        },
    )
    .unwrap();

    let settings = read_json_value(&dir.join("settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("scope harus ada")
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(
        patterns,
        vec!["lain/b"],
        "id model milik provider terhapus harus dibuang, termasuk yang pakai sufiks thinking"
    );
}

#[test]
fn dropping_a_model_from_a_provider_prunes_its_scope_entry() {
    let (_lock, root) = sandbox("scope_remove_model");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"lama"},{"id":"baru"}]}}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"enabledModels":["gateway/lama","gateway/baru"]}"#,
    );

    let mut draft = draft("gateway", Some("k"), KeyMode::Inline);
    draft.models = vec![ModelDraft {
        id: "baru".to_string(),
        ..Default::default()
    }];
    api::apply_edit("pi".into(), EditOp::Upsert(draft)).unwrap();

    let settings = read_json_value(&dir.join("settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("scope harus ada")
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert!(
        !patterns.contains(&"gateway/lama"),
        "model yang dihapus tidak boleh tertinggal di scope: {patterns:?}"
    );
}

// ------------------------------------------------------------------ op terpisah

#[test]
fn set_key_changes_only_the_credential() {
    let (_lock, root) = sandbox("op_set_key");
    write(
        &root.join("pi/agent/models.json"),
        r#"{"providers":{"gateway":{"name":"GW","baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetKey {
            provider_id: "gateway".to_string(),
            key: "sk-baru".to_string(),
            key_mode: KeyMode::Inline,
        },
    )
    .unwrap();

    let auth = read_json_value(&root.join("pi/agent/auth.json"));
    assert_eq!(auth["gateway"]["key"], "sk-baru");

    // Model dan endpoint tidak boleh tersentuh.
    let models = read_json_value(&root.join("pi/agent/models.json"));
    assert_eq!(
        models["providers"]["gateway"]["baseUrl"],
        "https://gw.test/v1"
    );
    assert_eq!(models["providers"]["gateway"]["models"][0]["id"], "a");
}

#[test]
fn set_key_env_mode_writes_a_reference_not_the_secret() {
    let (_lock, root) = sandbox("op_set_key_env");
    write(
        &root.join("pi/agent/models.json"),
        r#"{"providers":{"gateway":{"name":"GW","baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );
    write(
        &root.join("pi/agent/auth.json"),
        r#"{"gateway":{"type":"api_key","key":"lama"}}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetKey {
            provider_id: "gateway".to_string(),
            key: "sk-baru".to_string(),
            key_mode: KeyMode::Env,
        },
    )
    .unwrap();

    let models = read_json_value(&root.join("pi/agent/models.json"));
    assert_eq!(models["providers"]["gateway"]["apiKey"], "$GATEWAY_API_KEY");
    let auth = read_json_value(&root.join("pi/agent/auth.json"));
    assert!(
        auth.get("gateway").is_none(),
        "kredensial inline lama harus dibuang: {auth:?}"
    );
}

#[test]
fn set_models_replaces_the_list_and_keeps_the_key() {
    let (_lock, root) = sandbox("op_set_models");
    write(
        &root.join("pi/agent/models.json"),
        r#"{"providers":{"gateway":{"name":"GW","baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"},{"id":"b"}]}}}"#,
    );
    write(
        &root.join("pi/agent/auth.json"),
        r#"{"gateway":{"type":"api_key","key":"sk-ada"}}"#,
    );

    let result = api::apply_edit(
        "pi".into(),
        EditOp::SetModels {
            provider_id: "gateway".to_string(),
            models: vec![ModelDraft {
                id: "b".to_string(),
                ..Default::default()
            }],
        },
    )
    .unwrap();
    assert!(result.backup_dir.is_some());

    let models = read_json_value(&root.join("pi/agent/models.json"));
    let ids: Vec<&str> = models["providers"]["gateway"]["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|model| model["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["b"], "hanya model yang dicentang yang tersisa");
    assert_eq!(
        models["providers"]["gateway"]["baseUrl"], "https://gw.test/v1",
        "endpoint tidak berubah"
    );

    let auth = read_json_value(&root.join("pi/agent/auth.json"));
    assert_eq!(auth["gateway"]["key"], "sk-ada", "key tidak tersentuh");
}

#[test]
fn set_models_prunes_the_scope_of_the_dropped_model() {
    let (_lock, root) = sandbox("op_set_models_scope");
    write(
        &root.join("pi/agent/models.json"),
        r#"{"providers":{"gateway":{"name":"GW","baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"},{"id":"b"}]}}}"#,
    );
    write(
        &root.join("pi/agent/settings.json"),
        r#"{"enabledModels":["gateway/a","gateway/b"]}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetModels {
            provider_id: "gateway".to_string(),
            models: vec![ModelDraft {
                id: "b".to_string(),
                ..Default::default()
            }],
        },
    )
    .unwrap();

    let settings = read_json_value(&root.join("pi/agent/settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(patterns, vec!["gateway/b"], "entry model mati harus hilang");
}

#[test]
fn set_key_on_a_missing_provider_is_rejected() {
    let (_lock, root) = sandbox("op_set_key_missing");
    write(&root.join("pi/agent/models.json"), r#"{"providers":{}}"#);

    let error = api::apply_edit(
        "pi".into(),
        EditOp::SetKey {
            provider_id: "tidak-ada".to_string(),
            key: "sk".to_string(),
            key_mode: KeyMode::Inline,
        },
    )
    .unwrap_err();
    assert!(error.contains("not found"), "{error}");
}

#[test]
fn set_models_is_rejected_for_hermes_with_a_clear_message() {
    let (_lock, root) = sandbox("op_set_models_hermes");
    write(
        &root.join("hermes/config.yaml"),
        "providers:\n  gateway:\n    name: GW\n    base_url: https://gw.test/v1\n    key_env: GATEWAY_API_KEY\n    api_mode: chat_completions\n",
    );

    let error = api::apply_edit(
        "hermes".into(),
        EditOp::SetModels {
            provider_id: "gateway".to_string(),
            models: vec![ModelDraft {
                id: "a".to_string(),
                ..Default::default()
            }],
        },
    )
    .unwrap_err();
    assert!(error.contains("no model list"), "{error}");
}

// ------------------------------------------------- default saat toggle (pi)

/// Dua provider aktif + default di salah satunya.
fn pi_two_providers(root: &Path) {
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "deepseek": {
      "name": "DeepSeek",
      "baseUrl": "https://api.deepseek.com",
      "api": "openai-completions",
      "models": [{ "id": "deepseek-chat", "name": "DeepSeek Chat" }]
    },
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [{ "id": "proxy-a", "name": "Proxy A" }]
    }
  }
}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"deepseek":{"type":"api_key","key":"sk-ds"},"myproxy":{"type":"api_key","key":"sk-px"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"deepseek","defaultModel":"deepseek/deepseek-chat"}"#,
    );
}

#[test]
fn pi_disable_moves_default_to_another_active_provider() {
    let (_lock, root) = sandbox("pi-default-move");
    pi_two_providers(&root);

    let result = api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "deepseek".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = json(&root.join("pi/agent/settings.json"));
    assert_eq!(
        settings["defaultProvider"], "myproxy",
        "default tidak boleh tertinggal di provider yang dinonaktifkan: {settings}"
    );
    assert_eq!(settings["defaultModel"], "proxy-a");
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line.contains("Default moved to 'myproxy/proxy-a'")),
        "perpindahan default harus terlihat: {:?}",
        result.warnings
    );
}

#[test]
fn pi_disable_last_provider_clears_default() {
    let (_lock, root) = sandbox("pi-default-clear");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"deepseek":{"name":"DeepSeek","baseUrl":"https://api.deepseek.com","api":"openai-completions","models":[{"id":"deepseek-chat"}]}}}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"deepseek":{"type":"api_key","key":"sk-ds"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"deepseek","defaultModel":"deepseek/deepseek-chat","theme":"dark"}"#,
    );

    let result = api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "deepseek".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert!(
        settings.get("defaultProvider").is_none(),
        "semua provider nonaktif: default harus dikosongkan, bukan menunjuk provider mati: {settings}"
    );
    assert!(settings.get("defaultModel").is_none());
    // Key lain di settings.json tidak boleh ikut hilang.
    assert_eq!(settings["theme"], "dark");
    let cleared = result
        .warnings
        .iter()
        .find(|line| line.contains("default pointing at 'deepseek' was cleared"))
        .unwrap_or_else(|| panic!("{:?}", result.warnings));
    // "Tidak ada default" bukan "tidak ada model": pi memilih sendiri, dan
    // provider yang masih punya kredensial bisa terpilih. Pesannya harus jujur.
    assert!(
        cleared.contains("credentials"),
        "pesan harus menyebut bahwa pi memilih sendiri: {cleared}"
    );
}

#[test]
fn pi_enable_fills_a_missing_default() {
    let (_lock, root) = sandbox("pi-default-fill");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"myproxy":{"name":"My Proxy","baseUrl":"https://proxy.example.com/v1","api":"openai-completions","models":[{"id":"proxy-a"}]}}}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"sk"}}"#,
    );
    write(&dir.join("settings.json"), r#"{"theme":"dark"}"#);

    // Nonaktifkan dulu supaya provider tercatat sebagai nonaktif.
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "myproxy".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "myproxy".to_string(),
            enabled: true,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(settings["defaultProvider"], "myproxy", "{settings}");
    assert_eq!(settings["defaultModel"], "proxy-a");
}

#[test]
fn pi_default_pointing_at_a_disabled_provider_becomes_a_notice_with_fix() {
    let (_lock, root) = sandbox("pi-default-notice");
    pi_two_providers(&root);

    // Config lama: provider dinonaktifkan tanpa memindahkan default.
    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "deepseek".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    // Kembalikan default ke provider yang sudah mati, seperti file hasil edit manual.
    write(
        &root.join("pi/agent/settings.json"),
        r#"{"defaultProvider":"deepseek","defaultModel":"deepseek/deepseek-chat"}"#,
    );

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "pi.default_disabled")
        .unwrap_or_else(|| panic!("notice harus muncul: {notices:?}"));
    assert_eq!(notice.provider, "deepseek");
    assert!(notice.fix.is_some(), "harus bisa dibereskan otomatis");

    let fixed =
        api::fix_notice("pi".into(), "pi.default_disabled".into(), "deepseek".into()).unwrap();
    assert!(!fixed.files_written.is_empty());

    let settings = json(&root.join("pi/agent/settings.json"));
    assert_eq!(settings["defaultProvider"], "myproxy", "{settings}");
    let notices = api::list_notices(Some("pi".into())).unwrap();
    assert!(!notices.iter().any(|n| n.code == "pi.default_disabled"));
}

// ------------------------------------------------------ kredensial (forget)

#[test]
fn pi_forget_key_removes_the_stored_credential_but_keeps_the_provider() {
    let (_lock, root) = sandbox("pi-forget-key");
    pi_two_providers(&root);

    let result = api::apply_edit(
        "pi".into(),
        EditOp::ForgetKey {
            provider_id: "deepseek".to_string(),
        },
    )
    .unwrap();

    let auth = json(&root.join("pi/agent/auth.json"));
    assert!(
        auth.get("deepseek").is_none(),
        "kredensial harus hilang dari auth.json: {auth}"
    );
    assert_eq!(
        auth["myproxy"]["key"], "sk-px",
        "provider lain tidak disentuh"
    );
    // Provider tetap ada di models.json: yang dihapus hanya kredensialnya.
    let models = read_json_value(&root.join("pi/agent/models.json"));
    assert!(models["providers"].get("deepseek").is_some());
    // File sensitif di-backup, jadi nilainya bisa dipulihkan.
    assert!(result.backup_dir.is_some());
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line.contains("Stored credential for 'deepseek' removed")),
        "{:?}",
        result.warnings
    );
}

#[test]
fn pi_forget_key_says_so_when_the_credential_lives_in_the_environment() {
    let (_lock, root) = sandbox("pi-forget-env");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"deepseek":{"name":"DeepSeek","baseUrl":"https://api.deepseek.com","api":"openai-completions","apiKey":"$DEEPSEEK_API_KEY","models":[{"id":"deepseek-chat"}]}}}"#,
    );
    write(&dir.join("auth.json"), "{}");

    let error = api::apply_edit(
        "pi".into(),
        EditOp::ForgetKey {
            provider_id: "deepseek".to_string(),
        },
    )
    .unwrap_err();
    assert!(
        error.contains("environment variable"),
        "pesan harus menunjuk env var, bukan gagal tanpa penjelasan: {error}"
    );
}

#[test]
fn forget_key_is_rejected_where_the_key_lives_in_the_provider_entry() {
    let (_lock, root) = sandbox("omp-forget-key");
    let dir = root.join("omp");
    write(
        &dir.join("models.yml"),
        "providers:\n  groq:\n    apiKey: gsk-abc\n    baseUrl: https://api.groq.com/openai/v1\n    models:\n      - id: m1\n",
    );

    let error = api::apply_edit(
        "omp".into(),
        EditOp::ForgetKey {
            provider_id: "groq".to_string(),
        },
    )
    .unwrap_err();
    assert!(
        error.to_lowercase().contains("credential"),
        "harness yang menyimpan key di entri provider harus menolak dengan jelas: {error}"
    );
}

// ------------------------------------------- variant thinking pada model (pi)

/// Map thinking yang dijanjikan: none/low/medium/high/max.
fn assert_full_thinking_map(map: &Value, context: &str) {
    assert_eq!(map["off"], "none", "{context}");
    assert_eq!(map["low"], "low", "{context}");
    assert_eq!(map["medium"], "medium", "{context}");
    assert_eq!(map["high"], "high", "{context}");
    assert_eq!(map["max"], "max", "{context}");
    // Level di luar variant sengaja ditandai tidak didukung.
    assert!(map["minimal"].is_null(), "{context}");
    assert!(map["xhigh"].is_null(), "{context}");
}

#[test]
fn pi_upsert_writes_thinking_variants_for_reasoning_models() {
    let (_lock, root) = sandbox("pi-thinking-upsert");
    let dir = root.join("pi/agent");
    write(&dir.join("models.json"), r#"{"providers":{}}"#);
    write(&dir.join("auth.json"), "{}");

    let mut provider = draft("myproxy", Some("sk-1"), KeyMode::Inline);
    provider.models = vec![
        ModelDraft {
            id: "thinker".to_string(),
            name: Some("Thinker".to_string()),
            context_window: Some(128000),
            max_tokens: Some(8192),
            reasoning: true,
        },
        ModelDraft {
            id: "plain".to_string(),
            name: Some("Plain".to_string()),
            context_window: Some(128000),
            max_tokens: Some(8192),
            reasoning: false,
        },
    ];

    api::apply_edit("pi".into(), EditOp::Upsert(provider)).unwrap();

    let models = read_json_value(&dir.join("models.json"));
    let list = models["providers"]["myproxy"]["models"].as_array().unwrap();
    let thinker = list.iter().find(|m| m["id"] == "thinker").unwrap();
    assert_full_thinking_map(&thinker["thinkingLevelMap"], "model reasoning");
    assert_eq!(thinker["reasoning"], true);

    let plain = list.iter().find(|m| m["id"] == "plain").unwrap();
    assert!(
        plain.get("thinkingLevelMap").is_none(),
        "model tanpa reasoning tidak boleh mengaku punya level thinking: {plain}"
    );
}

#[test]
fn pi_upsert_keeps_a_handwritten_thinking_map() {
    let (_lock, root) = sandbox("pi-thinking-keep");
    let dir = root.join("pi/agent");
    // Map buatan pengguna: hanya medium dan high yang dipetakan, sisanya null.
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [
        {
          "id": "thinker",
          "reasoning": true,
          "thinkingLevelMap": { "medium": "medium", "high": "high" }
        }
      ]
    }
  }
}"#,
    );
    write(&dir.join("auth.json"), "{}");

    let mut provider = draft("myproxy", None, KeyMode::Inline);
    provider.models = vec![ModelDraft {
        id: "thinker".to_string(),
        name: Some("Thinker Box".to_string()),
        context_window: Some(64000),
        max_tokens: Some(4096),
        reasoning: true,
    }];
    api::apply_edit("pi".into(), EditOp::Upsert(provider)).unwrap();

    let models = read_json_value(&dir.join("models.json"));
    let thinker = &models["providers"]["myproxy"]["models"][0];
    assert_eq!(
        thinker["thinkingLevelMap"]["medium"], "medium",
        "map pengguna tidak boleh ditimpa: {thinker}"
    );
    assert!(
        thinker["thinkingLevelMap"].get("max").is_none(),
        "map yang sudah ada dipakai apa adanya: {thinker}"
    );
    assert_eq!(thinker["name"], "Thinker Box");
}

#[test]
fn pi_set_models_adds_thinking_variants_and_keeps_unknown_fields() {
    let (_lock, root) = sandbox("pi-thinking-set-models");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [
        { "id": "thinker", "reasoning": true, "cost": { "input": 1.5 } }
      ]
    }
  }
}"#,
    );
    write(&dir.join("auth.json"), "{}");

    api::apply_edit(
        "pi".into(),
        EditOp::SetModels {
            provider_id: "myproxy".to_string(),
            models: vec![
                ModelDraft {
                    id: "thinker".to_string(),
                    name: Some("Thinker".to_string()),
                    context_window: Some(128000),
                    max_tokens: Some(8192),
                    reasoning: true,
                },
                ModelDraft {
                    id: "fresh".to_string(),
                    name: Some("Fresh".to_string()),
                    context_window: None,
                    max_tokens: None,
                    reasoning: true,
                },
            ],
        },
    )
    .unwrap();

    let models = read_json_value(&dir.join("models.json"));
    let list = models["providers"]["myproxy"]["models"].as_array().unwrap();

    let thinker = list.iter().find(|m| m["id"] == "thinker").unwrap();
    assert_full_thinking_map(&thinker["thinkingLevelMap"], "model lama");
    assert_eq!(
        thinker["cost"]["input"], 1.5,
        "field di luar form harus bertahan: {thinker}"
    );

    let fresh = list.iter().find(|m| m["id"] == "fresh").unwrap();
    assert_full_thinking_map(&fresh["thinkingLevelMap"], "model baru");
}

#[test]
fn omp_models_do_not_get_pi_only_thinking_keys() {
    let (_lock, root) = sandbox("omp-no-thinking-map");
    let dir = root.join("omp");
    write(&dir.join("models.yml"), "providers: {}\n");

    let mut provider = draft("myproxy", Some("sk-1"), KeyMode::Inline);
    provider.models = vec![ModelDraft {
        id: "thinker".to_string(),
        name: Some("Thinker".to_string()),
        context_window: Some(128000),
        max_tokens: Some(8192),
        reasoning: true,
    }];
    api::apply_edit("omp".into(), EditOp::Upsert(provider)).unwrap();

    let raw = std::fs::read_to_string(dir.join("models.yml")).unwrap();
    assert!(
        !raw.contains("thinkingLevelMap"),
        "map thinking hanya milik pi: {raw}"
    );
}

#[test]
fn pi_models_without_thinking_variants_are_reported_and_fixed() {
    let (_lock, root) = sandbox("pi-thinking-notice");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [
        { "id": "thinker", "reasoning": true },
        { "id": "plain" }
      ]
    }
  }
}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"sk"}}"#,
    );

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "pi.thinking_map")
        .unwrap_or_else(|| panic!("notice thinking harus muncul: {notices:?}"));
    assert_eq!(notice.provider, "myproxy");
    assert!(
        notice.message.contains("1 reasoning model"),
        "hanya model reasoning yang dihitung: {}",
        notice.message
    );

    api::fix_notice("pi".into(), "pi.thinking_map".into(), "myproxy".into()).unwrap();

    let models = read_json_value(&dir.join("models.json"));
    let list = models["providers"]["myproxy"]["models"].as_array().unwrap();
    let thinker = list.iter().find(|m| m["id"] == "thinker").unwrap();
    assert_full_thinking_map(&thinker["thinkingLevelMap"], "hasil fix");
    let plain = list.iter().find(|m| m["id"] == "plain").unwrap();
    assert!(plain.get("thinkingLevelMap").is_none());

    let notices = api::list_notices(Some("pi".into())).unwrap();
    assert!(!notices.iter().any(|n| n.code == "pi.thinking_map"));
}

#[test]
fn pi_toggle_heals_a_default_that_already_pointed_at_a_parked_provider() {
    let (_lock, root) = sandbox("pi-default-heal");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [{ "id": "proxy-a" }]
    },
    "otherproxy": {
      "name": "Other Proxy",
      "baseUrl": "https://other.example.com/v1",
      "api": "openai-completions",
      "models": [{ "id": "other-a" }]
    }
  }
}
// hakonfig:disabled opencode-go
// { "name": "OpenCode Go", "baseUrl": "https://opencode.ai/inference/go/openai/v1" }
// hakonfig:end
"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"a"},"otherproxy":{"type":"api_key","key":"b"}}"#,
    );
    // Keadaan hasil versi lama: default menunjuk provider yang sudah diparkir.
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"opencode-go","defaultModel":"opencode-go/deepseek-v4.1-flash"}"#,
    );

    // Toggle provider yang **bukan** default: tetap harus membereskan default mati.
    let result = api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "otherproxy".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(
        settings["defaultProvider"], "myproxy",
        "default yang menunjuk provider non-aktif harus dipindah: {settings}"
    );
    assert_eq!(settings["defaultModel"], "proxy-a");
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line.contains("no longer enabled")),
        "alasan perpindahan harus jelas: {:?}",
        result.warnings
    );
}

#[test]
fn pi_enable_heals_a_parked_default_into_the_re_enabled_provider() {
    let (_lock, root) = sandbox("pi-default-heal-enable");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [{ "id": "proxy-a" }]
    }
  }
}
// hakonfig:disabled otherproxy
// { "name": "Other Proxy", "baseUrl": "https://other.example.com/v1", "api": "openai-completions", "models": [ { "id": "other-a" } ] }
// hakonfig:end
"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"a"},"otherproxy":{"type":"api_key","key":"b"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"otherproxy","defaultModel":"otherproxy/other-a"}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "otherproxy".to_string(),
            enabled: true,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(settings["defaultProvider"], "otherproxy");
    assert_eq!(settings["defaultModel"], "other-a");
}

#[test]
fn pi_disabling_everything_clears_a_stale_default_too() {
    let (_lock, root) = sandbox("pi-default-heal-all");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://proxy.example.com/v1",
      "api": "openai-completions",
      "models": [{ "id": "proxy-a" }]
    }
  }
}
// hakonfig:disabled opencode-go
// { "name": "OpenCode Go", "baseUrl": "https://opencode.ai/inference/go/openai/v1" }
// hakonfig:end
"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"a"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"opencode-go","defaultModel":"opencode-go/deepseek-v4.1-flash","theme":"dark"}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "myproxy".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert!(
        settings.get("defaultProvider").is_none(),
        "provider terakhir dimatikan: default mati harus dibuang, bukan dipindah: {settings}"
    );
    assert!(settings.get("defaultModel").is_none());
    assert_eq!(settings["theme"], "dark");
}

// ------------------------------- default mati di harness lain (kelas bug sama)

#[test]
fn omp_disable_heals_a_default_pointing_at_the_disabled_provider() {
    let (_lock, root) = sandbox("omp-default-heal");
    let dir = root.join("omp/agent");
    write(
        &dir.join("models.yml"),
        "providers:\n  groq:\n    apiKey: gsk-a\n    baseUrl: https://api.groq.com/openai/v1\n    models:\n      - id: m1\n  other:\n    apiKey: sk-b\n    baseUrl: https://other.example.com/v1\n    models:\n      - id: n1\n",
    );
    write(
        &dir.join("config.yml"),
        "modelRoles:\n  default: groq/m1\nenabledModels:\n  - groq/m1\n  - other/n1\n",
    );

    let result = api::apply_edit(
        "omp".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = yaml(&dir.join("config.yml"));
    assert_eq!(
        settings["modelRoles"]["default"], "other/n1",
        "default harus pindah ke provider yang masih aktif: {settings}"
    );
    assert_eq!(
        settings["enabledModels"].as_array().unwrap().len(),
        1,
        "referensi scope milik provider mati harus dibuang: {settings}"
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line.contains("Default moved to 'other/n1'")),
        "{:?}",
        result.warnings
    );
}

#[test]
fn omp_default_is_cleared_when_the_last_provider_goes_off() {
    let (_lock, root) = sandbox("omp-default-clear");
    let dir = root.join("omp/agent");
    write(
        &dir.join("models.yml"),
        "providers:\n  groq:\n    apiKey: gsk-a\n    baseUrl: https://api.groq.com/openai/v1\n    models:\n      - id: m1\n",
    );
    write(&dir.join("config.yml"), "modelRoles:\n  default: groq/m1\n");

    api::apply_edit(
        "omp".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = yaml(&dir.join("config.yml"));
    assert!(
        settings["modelRoles"].get("default").is_none(),
        "default harus dikosongkan: {settings}"
    );
}

#[test]
fn opencode_disable_heals_a_default_pointing_at_the_disabled_provider() {
    let (_lock, root) = sandbox("opencode-default-heal");
    let dir = root.join("opencode/config");
    write(
        &dir.join("opencode.json"),
        r#"{
  "provider": {
    "groq": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "https://api.groq.com/openai/v1" },
      "models": { "m1": { "name": "M1" } }
    },
    "other": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "https://other.example.com/v1" },
      "models": { "n1": { "name": "N1" } }
    }
  },
  "model": "groq/m1"
}"#,
    );

    api::apply_edit(
        "opencode".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let config = json(&dir.join("opencode.json"));
    assert_eq!(
        config["model"], "other/n1",
        "default harus pindah ke provider lain yang aktif: {config}"
    );
}

#[test]
fn hermes_disable_clears_a_default_pointing_at_the_disabled_provider() {
    let (_lock, root) = sandbox("hermes-default-clear");
    let dir = root.join("hermes");
    write(
        &dir.join("config.yaml"),
        "providers:\n  groq:\n    base_url: https://api.groq.com/openai/v1\n    key_env: GROQ_API_KEY\nmodel:\n  provider: custom:groq\n  default: m1\n  base_url: https://api.groq.com/openai/v1\n",
    );

    api::apply_edit(
        "hermes".into(),
        EditOp::SetEnabled {
            provider_id: "groq".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let config = yaml(&dir.join("config.yaml"));
    assert!(
        config["model"].get("provider").is_none(),
        "default yang menunjuk provider mati harus dibersihkan: {config}"
    );
    assert!(config["model"].get("default").is_none());
}

#[test]
fn pi_reports_a_disabled_provider_that_still_has_credentials() {
    let (_lock, root) = sandbox("pi-parked-with-credential");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {}
}
// hakonfig:disabled deepseek
// { "name": "DeepSeek", "baseUrl": "https://api.deepseek.com", "api": "openai-completions" }
// hakonfig:end
"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"deepseek":{"type":"api_key","key":"sk-ds"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"theme":"dark","defaultThinkingLevel":"medium"}"#,
    );

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| {
            notice.code == "catalog.credential-without-provider" && notice.provider == "deepseek"
        })
        .unwrap_or_else(|| panic!("notice kredensial harus muncul: {notices:?}"));

    // Pesannya harus menjelaskan kenapa provider mati tetap dipakai pi.
    assert!(
        notice.message.contains("disabled in models.json"),
        "{}",
        notice.message
    );
    assert!(
        notice.message.contains("built-in provider"),
        "{}",
        notice.message
    );
    assert!(notice.message.contains("auth.json"), "{}", notice.message);
    assert!(
        notice.message.contains("remove the credential"),
        "harus ada jalan keluar yang jelas: {}",
        notice.message
    );
}

// ---------------------- format defaultModel pi (penyebab "Hakonfig vs pi beda")

#[test]
fn pi_reports_and_fixes_a_default_model_written_with_the_provider_prefix() {
    let (_lock, root) = sandbox("pi-default-format");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "opencode-go": {
      "name": "OpenCode Go",
      "baseUrl": "https://opencode.ai/inference/go/openai/v1",
      "api": "openai-completions",
      "apiKey": "$OPENCODE_GO_API_KEY",
      "models": [{ "id": "deepseek-v4-pro" }]
    }
  }
}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"deepseek":{"type":"api_key","key":"sk"}}"#,
    );
    // Format lama Hakonfig: provider/model.
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"opencode-go","defaultModel":"opencode-go/deepseek-v4-pro"}"#,
    );

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "pi.default_model_prefixed")
        .unwrap_or_else(|| panic!("notice format default harus muncul: {notices:?}"));
    assert_eq!(notice.provider, "opencode-go");
    assert!(
        notice.message.contains("bare model id"),
        "pesan harus menjelaskan format yang benar: {}",
        notice.message
    );

    api::fix_notice(
        "pi".into(),
        "pi.default_model_prefixed".into(),
        "opencode-go".into(),
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(
        settings["defaultModel"], "deepseek-v4-pro",
        "pi membaca id model saja: {settings}"
    );
    assert_eq!(settings["defaultProvider"], "opencode-go");

    let notices = api::list_notices(Some("pi".into())).unwrap();
    assert!(!notices
        .iter()
        .any(|n| n.code == "pi.default_model_prefixed"));
}

#[test]
fn pi_set_default_writes_the_bare_model_id_that_pi_reads() {
    let (_lock, root) = sandbox("pi-default-bare");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{"providers":{"myproxy":{"name":"My","baseUrl":"https://p.example/v1","api":"openai-completions","models":[{"id":"m1"}]}}}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"myproxy":{"type":"api_key","key":"k"}}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetDefault {
            provider_id: "myproxy".to_string(),
            model_id: "m1".to_string(),
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(settings["defaultProvider"], "myproxy");
    assert_eq!(
        settings["defaultModel"], "m1",
        "harus id model saja, bukan myproxy/m1: {settings}"
    );
}

#[test]
fn disabling_an_unrelated_provider_keeps_a_healthy_default() {
    let (_lock, root) = sandbox("pi-default-untouched");
    let dir = root.join("pi/agent");
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "keepme": {
      "name": "Keep Me",
      "baseUrl": "https://keep.example/v1",
      "api": "openai-completions",
      "models": [{ "id": "k1" }]
    },
    "other": {
      "name": "Other",
      "baseUrl": "https://other.example/v1",
      "api": "openai-completions",
      "models": [{ "id": "o1" }]
    }
  }
}"#,
    );
    write(
        &dir.join("auth.json"),
        r#"{"keepme":{"type":"api_key","key":"a"},"other":{"type":"api_key","key":"b"}}"#,
    );
    write(
        &dir.join("settings.json"),
        r#"{"defaultProvider":"keepme","defaultModel":"k1"}"#,
    );

    api::apply_edit(
        "pi".into(),
        EditOp::SetEnabled {
            provider_id: "other".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let settings = json(&dir.join("settings.json"));
    assert_eq!(
        settings["defaultProvider"], "keepme",
        "default yang sehat tidak boleh disentuh: {settings}"
    );
    assert_eq!(settings["defaultModel"], "k1");
}

// ------------------------------------------- reasoning massal + aksi notice

#[test]
fn a_credential_notice_offers_the_action_that_can_actually_be_done() {
    let (_lock, root) = sandbox("pi-notice-credential-action");
    let dir = root.join("pi/agent");
    write(&dir.join("models.json"), r#"{"providers":{}}"#);
    write(
        &dir.join("auth.json"),
        r#"{"xai":{"type":"api_key","key":"xai-key"},"soulparking":{"type":"api_key","key":"s"}}"#,
    );
    write(&dir.join("settings.json"), "{}");

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let xai = notices
        .iter()
        .find(|notice| notice.provider == "xai")
        .unwrap_or_else(|| panic!("notice xai harus muncul: {notices:?}"));
    assert!(
        xai.fix.is_some(),
        "kredensial tersimpan bisa dihapus, jadi harus ada aksi"
    );

    api::fix_notice("pi".into(), xai.code.clone(), "xai".into()).unwrap();

    let auth = json(&dir.join("auth.json"));
    assert!(
        auth.get("xai").is_none(),
        "kredensial xai harus hilang: {auth}"
    );
    assert!(
        auth.get("soulparking").is_some(),
        "provider lain tidak disentuh"
    );
}

#[test]
fn models_that_the_catalog_says_support_thinking_are_reported_and_fixed() {
    let (_lock, root) = sandbox("pi-reasoning-massal");
    let dir = root.join("pi/agent");
    // Katalog pi (models-store.json) adalah sumber kebenaran di mesin ini.
    write(
        &dir.join("models-store.json"),
        r#"{
  "myproxy": {
    "models": [
      { "id": "thinker", "reasoning": true },
      { "id": "plain", "reasoning": false }
    ]
  }
}"#,
    );
    write(
        &dir.join("models.json"),
        r#"{
  "providers": {
    "myproxy": {
      "name": "My Proxy",
      "baseUrl": "https://p.example/v1",
      "api": "openai-completions",
      "apiKey": "k",
      "models": [
        { "id": "thinker" },
        { "id": "plain" }
      ]
    }
  }
}"#,
    );
    write(&dir.join("auth.json"), "{}");
    write(&dir.join("settings.json"), "{}");

    let notices = api::list_notices(Some("pi".into())).unwrap();
    let notice = notices
        .iter()
        .find(|notice| notice.code == "pi.reasoning_missing")
        .unwrap_or_else(|| panic!("notice reasoning harus muncul: {notices:?}"));
    assert_eq!(notice.provider, "myproxy");
    assert!(
        notice.message.contains("1 model"),
        "hanya model yang katalognya reasoning: {}",
        notice.message
    );

    api::fix_notice("pi".into(), "pi.reasoning_missing".into(), "myproxy".into()).unwrap();

    let models = read_json_value(&dir.join("models.json"));
    let list = models["providers"]["myproxy"]["models"].as_array().unwrap();
    let thinker = list.iter().find(|m| m["id"] == "thinker").unwrap();
    assert_eq!(thinker["reasoning"], true);
    assert_full_thinking_map(&thinker["thinkingLevelMap"], "model yang diperbaiki");
    let plain = list.iter().find(|m| m["id"] == "plain").unwrap();
    assert!(
        plain.get("reasoning").is_none(),
        "model yang katalognya bukan reasoning tidak boleh ikut ditandai: {plain}"
    );
}
