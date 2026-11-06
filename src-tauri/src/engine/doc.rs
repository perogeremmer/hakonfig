//! Layer dokumen: representasi satu file konfigurasi yang bisa dibaca,
//! dimutasi di memori, lalu dirender kembali ke format aslinya.
//!
//! Prinsip keamanan:
//! * file yang tidak ada tidak pernah dibuat kecuali benar-benar diubah;
//! * field yang tidak dikenal selalu dipertahankan (mutasi dilakukan pada
//!   pohon data asli, bukan pada struct hasil normalisasi).
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    /// JSON dengan komentar & trailing comma (mis. `models.json` Pi).
    Jsonc,
    Yaml,
    /// File `.env` — diedit langsung secara baris demi baris agar komentar awet.
    DotEnv,
}

pub struct FileSpec {
    pub role: &'static str,
    pub path: PathBuf,
    pub format: Format,
    pub sensitive: bool,
}

enum Payload {
    Tree(Value),
    Text(String),
}

const NULL_VALUE: Value = Value::Null;

pub struct Doc {
    pub path: PathBuf,
    pub role: String,
    pub format: Format,
    pub sensitive: bool,
    pub exists: bool,
    /// Isi file saat pertama dibaca — dasar perbandingan `changed`.
    pub before: String,
    /// true bila file memuat komentar yang akan hilang saat ditulis ulang.
    pub has_comments: bool,
    pub dirty: bool,
    payload: Payload,
}

impl Doc {
    pub fn load(spec: &FileSpec) -> Result<Doc> {
        let exists = spec.path.exists();
        let before = if exists {
            std::fs::read_to_string(&spec.path)
                .with_context(|| format!("gagal membaca {}", spec.path.display()))?
        } else {
            String::new()
        };

        let payload = match spec.format {
            Format::Json | Format::Jsonc => Payload::Tree(parse_json_like(&before)?),
            Format::Yaml => Payload::Tree(parse_yaml(&before)?),
            Format::DotEnv => Payload::Text(before.clone()),
        };

        let has_comments = match spec.format {
            Format::Json => false,
            Format::Jsonc => has_json_comments(&before),
            Format::Yaml => has_yaml_comments(&before),
            Format::DotEnv => false,
        };

        Ok(Doc {
            path: spec.path.clone(),
            role: spec.role.to_string(),
            format: spec.format,
            sensitive: spec.sensitive,
            exists,
            before,
            has_comments,
            dirty: false,
            payload,
        })
    }

    /// Apakah dokumen ini punya isi yang berbeda dari file di disk.
    pub fn is_changed(&self) -> bool {
        self.dirty && self.render() != self.before
    }

    pub fn render(&self) -> String {
        match &self.payload {
            Payload::Text(text) => text.clone(),
            Payload::Tree(value) => render_value(value, self.format),
        }
    }

    /// Render untuk pratinjau: seluruh nilai rahasia disamarkan.
    pub fn render_redacted(&self) -> String {
        match &self.payload {
            Payload::Text(text) => redact_env_text(text),
            Payload::Tree(value) => render_value(&redact_secrets(value), self.format),
        }
    }

    /// Isi awal file (untuk diff) dengan nilai rahasia disamarkan.
    pub fn before_redacted(&self) -> String {
        if !self.exists {
            return String::new();
        }
        match self.format {
            Format::DotEnv => redact_env_text(&self.before),
            Format::Json | Format::Jsonc => parse_json_like(&self.before)
                .map(|value| render_value(&redact_secrets(&value), self.format))
                .unwrap_or_else(|_| self.before.clone()),
            Format::Yaml => parse_yaml(&self.before)
                .map(|value| render_value(&redact_secrets(&value), self.format))
                .unwrap_or_else(|_| self.before.clone()),
        }
    }

    pub fn data(&self) -> &Value {
        match &self.payload {
            Payload::Tree(v) => v,
            Payload::Text(_) => &NULL_VALUE,
        }
    }

    /// Akses mutable ke pohon data. Tidak membuat apa pun; dipakai untuk
    /// operasi hapus supaya file yang belum ada tidak ikut terbuat.
    pub fn tree_mut(&mut self) -> &mut Value {
        self.dirty = true;
        match &mut self.payload {
            Payload::Tree(v) => v,
            Payload::Text(_) => panic!("tree_mut tidak tersedia untuk format teks"),
        }
    }

    /// Root sebagai object. Dibuat bila kosong.
    pub fn root_object(&mut self) -> &mut Map<String, Value> {
        self.dirty = true;
        if !self.payload_value().is_object() {
            *self.payload_value() = Value::Object(Map::new());
        }
        self.payload_value()
            .as_object_mut()
            .expect("baru saja dipastikan object")
    }

    /// Sub-object pada `key` di root. Dibuat bila belum ada.
    pub fn object_at(&mut self, key: &str) -> &mut Map<String, Value> {
        self.dirty = true;
        let root = self.root_object();
        if !root.get(key).map(Value::is_object).unwrap_or(false) {
            root.insert(key.to_string(), Value::Object(Map::new()));
        }
        root.get_mut(key)
            .and_then(Value::as_object_mut)
            .expect("baru saja dipastikan object")
    }

    fn payload_value(&mut self) -> &mut Value {
        match &mut self.payload {
            Payload::Tree(v) => v,
            Payload::Text(_) => panic!("payload_value tidak tersedia untuk format teks"),
        }
    }

    // -------------------------------------------------------------- DotEnv

    pub fn env_set(&mut self, key: &str, value: &str) {
        let Payload::Text(text) = &mut self.payload else {
            return;
        };
        self.dirty = true;
        let prefix = format!("{}=", key);
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        let mut replaced = false;

        for line in lines.iter_mut() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            let candidate = trimmed
                .split_once('=')
                .map(|(k, _)| k.trim())
                .unwrap_or_default();
            if candidate == key || trimmed.starts_with(&prefix) {
                *line = format!("{}={}", key, quote_env(value));
                replaced = true;
                break;
            }
        }
        if !replaced {
            lines.push(format!("{}={}", key, quote_env(value)));
        }
        *text = join_lines(lines);
    }

    pub fn env_remove(&mut self, key: &str) {
        let Payload::Text(text) = &mut self.payload else {
            return;
        };
        self.dirty = true;
        let lines: Vec<String> = text
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    return true;
                }
                let candidate = trimmed
                    .split_once('=')
                    .map(|(k, _)| k.trim())
                    .unwrap_or_default();
                candidate != key
            })
            .map(str::to_string)
            .collect();
        *text = join_lines(lines);
    }

    pub fn env_get(&self, key: &str) -> Option<String> {
        let Payload::Text(text) = &self.payload else {
            return None;
        };
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim() == key {
                    let raw = v.trim().trim_matches('"').trim_matches('\'');
                    return Some(raw.to_string());
                }
            }
        }
        None
    }
}

const MASK: &str = "••••••••";

/// Nama field yang nilainya tidak boleh muncul di pratinjau.
fn is_secret_field(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "key"
            | "apikey"
            | "api_key"
            | "token"
            | "access"
            | "refresh"
            | "access_token"
            | "refresh_token"
            | "secret"
            | "client_secret"
            | "password"
            | "authorization"
            | "bearer"
    )
}

pub fn redact_secrets(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, child) in map {
                if is_secret_field(key) && !child.is_object() && !child.is_array() {
                    out.insert(key.clone(), Value::String(MASK.to_string()));
                } else {
                    out.insert(key.clone(), redact_secrets(child));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_secrets).collect()),
        other => other.clone(),
    }
}

fn redact_env_text(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || !trimmed.contains('=') {
            lines.push(line.to_string());
            continue;
        }
        match line.split_once('=') {
            Some((name, _)) => lines.push(format!("{}={}", name, MASK)),
            None => lines.push(line.to_string()),
        }
    }
    join_lines(lines)
}

fn render_value(value: &Value, format: Format) -> String {
    match format {
        Format::Yaml => serde_yaml::to_string(value).unwrap_or_default(),
        _ => {
            let mut out = serde_json::to_string_pretty(value).unwrap_or_default();
            out.push('\n');
            out
        }
    }
}

fn join_lines(mut lines: Vec<String>) -> String {
    // buang baris kosong berlebih di ujung
    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.pop();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn quote_env(value: &str) -> String {
    if value.contains(' ') || value.contains('#') || value.contains('"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

// ---------------------------------------------------------------- parsing

pub fn parse_json_like(text: &str) -> Result<Value> {
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let options = jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: true,
        allow_trailing_commas: true,
    };
    match jsonc_parser::parse_to_serde_value(text, &options) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Ok(Value::Object(Map::new())),
        Err(err) => Err(anyhow!("JSON tidak valid: {}", err)),
    }
}

pub fn parse_yaml(text: &str) -> Result<Value> {
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let value: Value = serde_yaml::from_str(text).context("YAML tidak valid")?;
    if value.is_null() {
        return Ok(Value::Object(Map::new()));
    }
    Ok(value)
}

/// Deteksi komentar JSON di luar string.
fn has_json_comments(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == b'/' && i + 1 < bytes.len() && (bytes[i + 1] == b'/' || bytes[i + 1] == b'*')
        {
            return true;
        }
        i += 1;
    }
    false
}

/// Deteksi komentar YAML (heuristik: `#` di awal baris / setelah spasi, di luar quote).
fn has_yaml_comments(text: &str) -> bool {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            return true;
        }
        let mut in_single = false;
        let mut in_double = false;
        let chars: Vec<char> = line.chars().collect();
        for (idx, ch) in chars.iter().enumerate() {
            match ch {
                '\'' if !in_double => in_single = !in_single,
                '"' if !in_single => in_double = !in_double,
                '#' if !in_single && !in_double => {
                    if idx == 0 || chars[idx - 1].is_whitespace() {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

// ---------------------------------------------------------------- collection

pub struct Docs {
    items: Vec<Doc>,
}

impl Docs {
    pub fn load(specs: &[FileSpec]) -> Result<Docs> {
        let mut items = Vec::with_capacity(specs.len());
        for spec in specs {
            items.push(Doc::load(spec)?);
        }
        Ok(Docs { items })
    }

    pub fn get(&self, role: &str) -> Option<&Doc> {
        self.items.iter().find(|d| d.role == role)
    }

    pub fn get_mut(&mut self, role: &str) -> Option<&mut Doc> {
        self.items.iter_mut().find(|d| d.role == role)
    }

    pub fn require_mut(&mut self, role: &str) -> Result<&mut Doc> {
        self.items
            .iter_mut()
            .find(|d| d.role == role)
            .ok_or_else(|| anyhow!("dokumen '{}' tidak tersedia untuk harness ini", role))
    }

    pub fn require(&self, role: &str) -> Result<&Doc> {
        self.get(role)
            .ok_or_else(|| anyhow!("dokumen '{}' tidak tersedia untuk harness ini", role))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Doc> {
        self.items.iter()
    }

    pub fn changed(&self) -> Vec<&Doc> {
        self.items.iter().filter(|d| d.is_changed()).collect()
    }

    pub fn comment_files(&self) -> Vec<&Doc> {
        self.items
            .iter()
            .filter(|d| d.has_comments && d.is_changed())
            .collect()
    }
}

// ---------------------------------------------------------------- helpers

pub fn get_str(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Tampilkan key secara aman: tidak pernah mengembalikan nilai penuh.
pub fn mask_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.starts_with('$') {
        return trimmed.to_string();
    }
    if trimmed.starts_with("${") {
        return trimmed.to_string();
    }
    let visible: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if trimmed.chars().count() <= 8 {
        "••••".to_string()
    } else {
        format!("•••• {}", visible)
    }
}

pub fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .next()
            .map(|c| c.is_ascii_alphanumeric())
            .unwrap_or(false)
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub fn env_name_from(id: &str) -> String {
    let upper: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("{}_API_KEY", upper.trim_matches('_'))
}
