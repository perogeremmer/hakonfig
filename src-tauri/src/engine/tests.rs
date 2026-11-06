//! Test integrasi engine pada sandbox (`REKONFIG_ROOT`), sehingga config asli
//! di mesin tidak pernah tersentuh.
#![cfg(test)]

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::commands;
use crate::engine::adapters;
use crate::engine::doc::Docs;
use crate::engine::model::{EditOp, KeyMode, ModelDraft, ProviderDraft};

/// `REKONFIG_ROOT` bersifat process-global, jadi test yang memakainya harus
/// berjalan bergantian lewat mutex ini.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn sandbox(name: &str) -> (std::sync::MutexGuard<'static, ()>, PathBuf) {
    let guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("rekonfig-test-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    std::env::set_var("REKONFIG_ROOT", &dir);
    (guard, dir)
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
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

#[test]
fn pi_merge_backup_dan_redaksi() {
    let (_lock, root) = sandbox("pi_merge_backup_dan_redaksi");
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
        snap.warnings.iter().any(|w| w.contains("komentar")),
        "harus ada peringatan komentar: {:?}",
        snap.warnings
    );

    // --- tulis provider baru (key inline)
    let result = commands::apply_edit(
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
    let preview = commands::preview_edit(
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
    assert!(preview.warnings.iter().any(|w| w.contains("disamarkan")));

    // --- endpoint junction: mode env menulis referensi $VAR
    commands::apply_edit(
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
    commands::apply_edit(
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
    let preview = commands::preview_edit(
        "pi".into(),
        EditOp::Remove {
            provider_id: "tidak-ada".to_string(),
        },
    )
    .unwrap();
    assert!(preview.files.iter().all(|file| !file.changed));

    // --- backup bisa dipulihkan
    let backups = commands::list_backups("pi".into()).unwrap();
    assert!(!backups.is_empty(), "harus ada backup");
    let restored = commands::restore_backup("pi".into(), backups[0].id.clone()).unwrap();
    assert!(!restored.files_written.is_empty());
    // backup restore sendiri juga memicu backup baru
    assert!(restored.backup_dir.is_some());
}

#[test]
fn omp_yaml_merge() {
    let (_lock, root) = sandbox("pi_merge_backup_dan_redaksi");
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

    commands::apply_edit(
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
    let preview = commands::preview_edit(
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
fn opencode_provider_dan_auth() {
    let (_lock, root) = sandbox("pi_merge_backup_dan_redaksi");
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

    commands::apply_edit(
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

    // hapus
    commands::apply_edit(
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
fn hermes_config_dan_env() {
    let (_lock, root) = sandbox("pi_merge_backup_dan_redaksi");
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

    commands::apply_edit(
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
    commands::apply_edit(
        "hermes".into(),
        upsert("groq", Some("gsk-hermes-baru"), KeyMode::Inline),
    )
    .unwrap();
    let env = fs::read_to_string(dir.join(".env")).unwrap();
    assert_eq!(env.matches("GROQ_API_KEY=").count(), 1, "env: {env}");
    assert!(env.contains("GROQ_API_KEY=gsk-hermes-baru"));

    // hapus provider ikut membersihkan env-nya
    commands::apply_edit(
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
fn validasi_input_ditolak() {
    let (_lock, _root) = sandbox("validasi");
    let mut bad = draft("Groq Salah!", Some("k"), KeyMode::Inline);
    assert!(commands::preview_edit("pi".into(), EditOp::Upsert(bad.clone())).is_err());

    bad = draft("groq", Some("k"), KeyMode::Inline);
    bad.base_url = "api.example.com".to_string();
    assert!(commands::preview_edit("pi".into(), EditOp::Upsert(bad.clone())).is_err());

    bad = draft("groq", Some("k"), KeyMode::Inline);
    bad.models.clear();
    assert!(commands::preview_edit("pi".into(), EditOp::Upsert(bad)).is_err());
}
