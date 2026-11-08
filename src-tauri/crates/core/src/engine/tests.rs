//! Test integrasi engine pada sandbox (`HAKONFIG_ROOT`), sehingga config asli
//! di mesin tidak pernah tersentuh.
#![cfg(test)]

use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::api;
use crate::engine::adapters;
use crate::engine::doc::Docs;
use crate::engine::model::{EditOp, KeyMode, ModelDraft, ProviderDraft};
use crate::engine::testutil::sandbox;

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

/// Key mentah yang dibaca backend untuk `test_connection` / `fetch_models`.
fn stored_key(harness: &str, provider_id: &str) -> Option<String> {
    let adapter = adapters::get(harness).unwrap();
    let docs = Docs::load(&adapter.specs()).unwrap();
    adapter.stored_key(&docs, provider_id)
}

#[test]
fn ganti_default_model_per_harness() {
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
    assert_eq!(settings["defaultModel"], "groq/llama-3.3-70b-versatile");
    assert_eq!(settings["theme"], "dark", "setelan lain tidak boleh hilang");
    assert_eq!(snapshot("pi").default_provider.as_deref(), Some("groq"));

    // --- oh-my-pi: config.yml modelRoles.default
    write(&omp_dir.join("config.yml"), "setupVersion: 2\nmodelRoles:\n  advisor: a/b\n  default: lama/m0\n");
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
    write(&oc_dir.join("opencode.json"), r#"{"model":"lama/m0","theme":"x"}"#);
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
    assert!(preview.warnings.iter().any(|w| w.contains("disamarkan")));

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
fn omp_toggle_nonaktif_reversible() {
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
    assert!(models["providers"].get("groq").is_some(), "entry tidak boleh hilang");
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
fn pi_nonaktifkan_mengomentari_bukan_menghapus() {
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
    write(&dir.join("auth.json"), r#"{"groq":{"type":"api_key","key":"gsk-abc"}}"#);

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
    assert_eq!(groq.models.len(), 1, "model ikut terbaca dari blok komentar");
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
fn hapus_provider_nonaktif_membuang_blok_komentar() {
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
    assert!(!raw.contains("hakonfig:disabled"), "blok parkir harus hilang: {raw}");
    assert!(!raw.contains("Groq"), "nama provider harus hilang: {raw}");
    assert!(snapshot("pi").providers.is_empty());
}

#[test]
fn hermes_nonaktifkan_mengomentari_di_yaml() {
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
    assert!(raw.contains("# hakonfig:disabled groq"), "yaml parkir: {raw}");
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
fn validasi_input_ditolak() {
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
fn stored_key_dibaca_per_adapter() {
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
    assert_eq!(stored_key("opencode", "langsung").as_deref(), Some("oc-key"));
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
fn stored_key_konsisten_dengan_has_key_snapshot() {
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
fn request_key_memakai_key_tersimpan_bila_kolom_kosong() {
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
fn edit_tanpa_key_baru_tidak_membuang_kredensial() {
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
fn key_baru_tetap_menimpa_key_lama() {
    let (_lock, root) = sandbox("edit_key_baru");
    let pi = root.join("pi/agent");
    write(
        &pi.join("models.json"),
        r#"{ "providers": { "groq": { "name": "Groq", "baseUrl": "https://a.test/v1", "api": "openai-completions", "apiKey": "sk-lama", "models": [{ "id": "m1" }] } } }"#,
    );
    write(&pi.join("auth.json"), "{}");

    api::apply_edit("pi".into(), upsert("groq", Some("sk-baru"), KeyMode::Inline)).unwrap();

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
fn katalog_memuat_dsh_dan_openhands() {
    let entries = crate::engine::catalog::all();
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&"dsh"), "DeepSeek Harness harus ada di katalog");
    assert!(ids.contains(&"openhands"), "openhands harus ada di katalog");
    // entri web chat yang lama tidak boleh kembali
    assert!(!ids.contains(&"deepseek"), "web chat bukan harness, jangan didaftarkan");

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
fn default_pi_ikut_masuk_scope_enabled_models() {
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
    assert_eq!(settings["defaultModel"], "deepseek/deepseek-flash");

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
        scope.iter().filter(|item| **item == "deepseek/deepseek-flash").count(),
        1,
        "tidak boleh ada duplikat: {scope:?}"
    );
    // entri glob yang lama tetap ada
    assert!(scope.contains(&"commandcode/*"), "scope lama hilang: {scope:?}");
}

#[test]
fn default_pi_tanpa_scope_tidak_membuat_enabled_models() {
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
    assert_eq!(settings["defaultModel"], "groq/m1");
    assert!(
        settings.get("enabledModels").is_none(),
        "jangan mengarang scope bila user belum punya"
    );
}

#[test]
fn default_omp_ikut_masuk_scope_enabled_models() {
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
    assert_eq!(
        settings["modelRoles"]["default"],
        "deepseek/deepseek-flash"
    );
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
        scope.iter().filter(|item| **item == "deepseek/deepseek-flash").count(),
        1,
        "tidak boleh duplikat: {scope:?}"
    );
    assert!(scope.contains(&"commandcode/*"), "scope lama hilang: {scope:?}");
}

#[test]
fn default_omp_tanpa_scope_tidak_mengarang_enabled_models() {
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
