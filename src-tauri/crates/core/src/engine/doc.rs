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
    /// File `.env` - diedit langsung secara baris demi baris agar komentar awet.
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
    /// Isi file saat pertama dibaca - dasar perbandingan `changed`.
    pub before: String,
    /// true bila file memuat komentar yang akan hilang saat ditulis ulang.
    pub has_comments: bool,
    /// Blok provider yang diparkir sebagai komentar (lihat `park`).
    pub trailing: String,
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

        let (body, trailing) = split_parked(&before, spec.format);

        let payload = match spec.format {
            Format::Json | Format::Jsonc => Payload::Tree(parse_json_like(&body)?),
            Format::Yaml => Payload::Tree(parse_yaml(&body)?),
            Format::DotEnv => Payload::Text(before.clone()),
        };

        // Deteksi komentar pengguna pada isi file *tanpa* blok parkir Hakonfig,
        // supaya penanda `hakonfig:disabled` tidak memicu peringatan.
        let has_comments = match spec.format {
            Format::Json => false,
            Format::Jsonc => has_json_comments(&body),
            Format::Yaml => has_yaml_comments(&body),
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
            trailing,
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
            Payload::Tree(value) => {
                let mut out = render_value(value, self.format);
                if !self.trailing.trim().is_empty() {
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str(&self.trailing);
                }
                out
            }
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
    /// Array pada `key` di root. Dibuat bila belum ada.
    pub fn array_at(&mut self, key: &str) -> &mut Vec<Value> {
        self.dirty = true;
        let root = self.root_object();
        if !root.get(key).map(Value::is_array).unwrap_or(false) {
            root.insert(key.to_string(), Value::Array(Vec::new()));
        }
        root.get_mut(key)
            .and_then(Value::as_array_mut)
            .expect("baru saja dipastikan array")
    }

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

    // ------------------------------------------------------- parkir provider
    //
    // Harness tanpa daftar "disabled" (Pi, Hermes) bisa menonaktifkan provider
    // dengan menaruhnya sebagai komentar di file yang sama. Kedua format
    // mendukung komentar (Pi: JSONC, Hermes: YAML), jadi data tidak hilang dan
    // bisa diaktifkan lagi tanpa menyentuh backup.

    fn parked_blocks(&self) -> Vec<(String, usize, usize)> {
        let prefix = comment_prefix(self.format);
        let lines: Vec<&str> = self.trailing.lines().collect();
        let mut out = Vec::new();
        let mut start: Option<(String, usize)> = None;

        for (index, line) in lines.iter().enumerate() {
            let content = marker_content(line, prefix).unwrap_or_default();
            if let Some(id) = content.strip_prefix(PARK_BEGIN) {
                start = Some((id.trim().to_string(), index));
                continue;
            }
            if content == PARK_END {
                if let Some((id, begin)) = start.take() {
                    out.push((id, begin, index));
                }
            }
        }
        out
    }

    pub fn parked_ids(&self) -> Vec<String> {
        self.parked_blocks().into_iter().map(|(id, _, _)| id).collect()
    }

    pub fn parked_value(&self, id: &str) -> Option<Value> {
        let (_, begin, end) = self
            .parked_blocks()
            .into_iter()
            .find(|(candidate, _, _)| candidate == id)?;
        let prefix = comment_prefix(self.format);
        let payload: Vec<String> = self
            .trailing
            .lines()
            .skip(begin + 1)
            .take(end.saturating_sub(begin + 1))
            .map(|line| strip_comment(line, prefix))
            .collect();
        let text = payload.join("\n");
        match self.format {
            Format::Yaml => parse_yaml(&text).ok(),
            _ => parse_json_like(&text).ok(),
        }
    }

    /// Buang blok parkir untuk id tertentu (dipakai saat provider dihapus).
    pub fn drop_parked(&mut self, id: &str) {
        let Some((_, begin, end)) = self
            .parked_blocks()
            .into_iter()
            .find(|(candidate, _, _)| candidate == id)
        else {
            return;
        };
        let mut lines: Vec<String> = self.trailing.lines().map(str::to_string).collect();
        lines.drain(begin..=end);
        self.trailing = normalize_trailing(&lines.join("\n"));
        self.dirty = true;
    }

    /// Parkir satu entri provider sebagai komentar di akhir file.
    pub fn park(&mut self, id: &str, value: &Value) {
        let prefix = comment_prefix(self.format);
        let payload = match self.format {
            Format::Yaml => serde_yaml::to_string(value).unwrap_or_default(),
            _ => {
                let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
                text.push('\n');
                text
            }
        };

        let mut block = String::new();
        block.push_str(&format!("{} {} {}\n", prefix, PARK_BEGIN, id));
        for line in payload.trim_end().lines() {
            if line.trim().is_empty() {
                block.push_str(&format!("{}\n", prefix));
            } else {
                block.push_str(&format!("{} {}\n", prefix, line));
            }
        }
        block.push_str(&format!("{} {}\n", prefix, PARK_END));

        let mut lines: Vec<String> = self.trailing.lines().map(str::to_string).collect();
        while lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
            lines.remove(0);
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(block.trim_end().lines().map(str::to_string));
        self.trailing = normalize_trailing(&lines.join("\n"));
        self.dirty = true;
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

const PARK_BEGIN: &str = "hakonfig:disabled";
const PARK_END: &str = "hakonfig:end";

fn comment_prefix(format: Format) -> &'static str {
    match format {
        Format::Yaml => "#",
        _ => "//",
    }
}

/// Isi baris komentar tanpa prefix (mis. `// hakonfig:end` -> `hakonfig:end`).
fn marker_content(line: &str, prefix: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix(prefix)?;
    let rest = rest.trim_start_matches(['/', '#']).trim();
    Some(rest.to_string())
}

/// Buang prefix komentar + satu spasi, sisakan isi aslinya (indentasi utuh).
fn strip_comment(line: &str, prefix: &str) -> String {
    let trimmed = line.trim_start();
    match trimmed.strip_prefix(prefix) {
        Some(rest) => rest.strip_prefix(' ').unwrap_or(rest).to_string(),
        None => trimmed.to_string(),
    }
}

fn normalize_trailing(text: &str) -> String {
    let body = text.trim_matches('\n');
    if body.trim().is_empty() {
        String::new()
    } else {
        format!("\n{}\n", body)
    }
}

/// Pisahkan blok parkir dari isi file sebelum di-parse.
fn split_parked(text: &str, format: Format) -> (String, String) {
    if !matches!(format, Format::Json | Format::Jsonc | Format::Yaml) {
        return (text.to_string(), String::new());
    }
    let prefix = comment_prefix(format);
    let mut body: Vec<String> = Vec::new();
    let mut parked: Vec<String> = Vec::new();
    let mut inside = false;

    for line in text.lines() {
        let content = marker_content(line, prefix).unwrap_or_default();
        let is_begin = content.starts_with(PARK_BEGIN);
        let is_end = content == PARK_END;

        if is_begin {
            inside = true;
            parked.push(line.trim_end().to_string());
            continue;
        }
        if inside {
            parked.push(line.trim_end().to_string());
            if is_end {
                inside = false;
            }
            continue;
        }
        body.push(line.to_string());
    }

    let mut body_text = body.join("\n");
    if !body_text.is_empty() {
        body_text.push('\n');
    }
    (body_text, normalize_trailing(&parked.join("\n")))
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

/// Validasi ID provider.
///
/// Sengaja **permisif terhadap huruf besar**: config yang sudah ada di mesin
/// pengguna bisa saja punya ID seperti `Groq`, dan menolaknya akan membuat
/// provider tersebut tidak bisa diubah sama sekali. Huruf kecil untuk provider
/// *baru* ditegakkan di UI (`^[a-z0-9][a-z0-9._-]*$`).
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

// ============================================================================
// Unit test untuk fungsi murni di modul ini.
// ============================================================================
#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::engine::testutil::temp_dir;

    fn spec(path: PathBuf, format: Format) -> FileSpec {
        FileSpec {
            role: "config",
            path,
            format,
            sensitive: false,
        }
    }

    // ------------------------------------------------------------- parsing

    #[test]
    fn parse_json_like_menerima_komentar_dan_trailing_comma() {
        let text = r#"{
            // komentar baris
            /* komentar blok */
            "a": 1,
            "b": [1, 2, 3,],
        }"#;
        let value = parse_json_like(text).unwrap();
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"][2], 3);
    }

    #[test]
    fn parse_json_like_menangani_kosong_dan_rusak() {
        assert!(parse_json_like("").unwrap().is_object());
        assert!(parse_json_like("   \n\t ").unwrap().is_object());
        assert!(parse_json_like("{ ini bukan json }").is_err());
    }

    #[test]
    fn parse_yaml_menangani_kosong_null_dan_isi() {
        assert!(parse_yaml("").unwrap().is_object());
        assert!(parse_yaml("null").unwrap().is_object());
        assert!(parse_yaml("~\n").unwrap().is_object());
        let value = parse_yaml("providers:\n  a:\n    baseUrl: https://x/v1\n").unwrap();
        assert_eq!(value["providers"]["a"]["baseUrl"], "https://x/v1");
        assert!(parse_yaml("a: [1, 2").is_err());
    }

    // -------------------------------------------------- deteksi komentar

    #[test]
    fn deteksi_komentar_json() {
        assert!(has_json_comments("{ // x\n }"));
        assert!(has_json_comments("{ /* x */ }"));
        assert!(!has_json_comments(r#"{"url": "https://api.example.com/v1"}"#));
        assert!(!has_json_comments(r#"{"a": "teks // bukan komentar"}"#));
        assert!(!has_json_comments(r#"{"a": 1}"#));
    }

    #[test]
    fn deteksi_komentar_yaml() {
        assert!(has_yaml_comments("# komentar\n"));
        assert!(has_yaml_comments("  # komentar menjorok\n"));
        assert!(has_yaml_comments("color: #fff\n"));
        assert!(!has_yaml_comments("color: \"#fff\"\n"));
        assert!(!has_yaml_comments("url: https://x/v1#anchor\n"));
        assert!(!has_yaml_comments("key: value\n"));
    }

    // ------------------------------------------------------------ masking

    #[test]
    fn mask_key_tidak_pernah_membocorkan_utuh() {
        assert_eq!(mask_key("$GROQ_API_KEY"), "$GROQ_API_KEY");
        assert_eq!(mask_key("${GROQ_API_KEY}"), "${GROQ_API_KEY}");
        assert_eq!(mask_key("sk-abcdefghijklmnop"), "•••• mnop");
        assert_eq!(mask_key("pendek"), "••••");
        assert_eq!(mask_key(""), "••••");
        assert!(!mask_key("sk-abcdefghijklmnop").contains("sk-abc"));
    }

    #[test]
    fn slug_validation() {
        for ok in ["groq", "opencode-go", "a.b_c", "x1", "1a", "Groq"] {
            assert!(is_slug(ok), "{ok} seharusnya valid");
        }
        for bad in ["", "-groq", ".groq", "groq ok", "groq/ok", "é", "groq#1"] {
            assert!(!is_slug(bad), "{bad} seharusnya ditolak");
        }
    }

    #[test]
    fn env_name_dari_id() {
        assert_eq!(env_name_from("groq"), "GROQ_API_KEY");
        assert_eq!(env_name_from("opencode-go"), "OPENCODE_GO_API_KEY");
        assert_eq!(env_name_from("a.b_c"), "A_B_C_API_KEY");
        assert_eq!(env_name_from("-x-"), "X_API_KEY");
    }

    // ------------------------------------------------------------ redaksi

    #[test]
    fn redaksi_menyamarkan_field_rahasia() {
        let input = serde_json::json!({
            "apiKey": "VALUE-apiKey",
            "key": "VALUE-key",
            "token": "VALUE-token",
            "refresh": "VALUE-refresh",
            "password": "VALUE-password",
            "name": "boleh terlihat",
            "baseUrl": "https://x/v1",
            "nested": { "client_secret": "VALUE-client-secret", "api": "openai-completions" },
            "list": [{ "key": "VALUE-dalam-array" }],
            "angka": 42
        });
        let out = redact_secrets(&input);
        let teks = serde_json::to_string(&out).unwrap();

        for bocor in [
            "VALUE-apiKey",
            "VALUE-key",
            "VALUE-token",
            "VALUE-refresh",
            "VALUE-password",
            "VALUE-client-secret",
            "VALUE-dalam-array",
        ] {
            assert!(!teks.contains(bocor), "{bocor} bocor di pratinjau");
        }
        assert_eq!(out["name"], "boleh terlihat");
        assert_eq!(out["baseUrl"], "https://x/v1");
        assert_eq!(out["nested"]["api"], "openai-completions");
        assert_eq!(out["angka"], 42);
        assert_eq!(out["apiKey"], MASK);
        assert_eq!(out["list"][0]["key"], MASK);
    }

    #[test]
    fn redaksi_env_menjaga_komentar_dan_nama() {
        let text = "# komentar\nA=rahasia\nB = \"dua kata\"\nTANPA_SAMA_DENGAN\n";
        let out = redact_env_text(text);
        assert!(out.contains("# komentar"));
        assert!(out.contains(&format!("A={MASK}")));
        assert!(!out.contains("rahasia"));
        assert!(!out.contains("dua kata"));
        assert!(out.contains("TANPA_SAMA_DENGAN"));
    }

    // ------------------------------------------------------- dokumen .env

    #[test]
    fn env_set_update_tidak_menduplikasi_dan_komentar_awet() {
        let dir = temp_dir("dotenv");
        let path = dir.join(".env");
        std::fs::write(&path, "# kredensial\nA=lama\nB=biarkan\n").unwrap();

        let mut doc = Doc::load(&spec(path.clone(), Format::DotEnv)).unwrap();
        assert!(!doc.is_changed());

        doc.env_set("A", "baru");
        let rendered = doc.render();
        assert_eq!(rendered.matches("A=").count(), 1, "{rendered}");
        assert!(rendered.contains("A=baru"));
        assert!(rendered.contains("# kredensial"));
        assert!(rendered.contains("B=biarkan"));
        assert!(doc.is_changed());
    }

    #[test]
    fn env_set_menambah_key_baru_di_akhir() {
        let dir = temp_dir("dotenv-add");
        let path = dir.join(".env");
        std::fs::write(&path, "A=1\n").unwrap();

        let mut doc = Doc::load(&spec(path, Format::DotEnv)).unwrap();
        doc.env_set("Z", "9");
        let rendered = doc.render();
        assert!(rendered.contains("A=1"));
        assert!(rendered.trim_end().ends_with("Z=9"), "{rendered}");
    }

    #[test]
    fn env_set_mengutip_nilai_ber_spasi_dan_menghapus() {
        let dir = temp_dir("dotenv-quote");
        let path = dir.join(".env");
        std::fs::write(&path, "A=1\nB=2\n").unwrap();

        let mut doc = Doc::load(&spec(path, Format::DotEnv)).unwrap();
        doc.env_set("A", "dua kata");
        assert_eq!(doc.env_get("A").as_deref(), Some("dua kata"));
        assert!(doc.render().contains("A=\"dua kata\""));

        doc.env_remove("B");
        let rendered = doc.render();
        assert!(!rendered.contains("B=2"), "{rendered}");
        assert_eq!(doc.env_get("B"), None);
    }

    #[test]
    fn env_file_tidak_ada_dianggap_kosong() {
        let dir = temp_dir("dotenv-missing");
        let doc = Doc::load(&spec(dir.join(".env"), Format::DotEnv)).unwrap();
        assert!(!doc.exists);
        assert!(!doc.is_changed());
        assert_eq!(doc.render(), "");
        assert_eq!(doc.env_get("APA"), None);
    }

    // -------------------------------------------------- dokumen JSON/YAML

    #[test]
    fn dokumen_json_render_pretty_dan_is_changed_akurat() {
        let dir = temp_dir("json-doc");
        let path = dir.join("config.json");
        std::fs::write(&path, "{\"a\":1}").unwrap();

        let mut doc = Doc::load(&spec(path, Format::Json)).unwrap();
        let rendered = doc.render();
        assert!(rendered.ends_with('\n'));
        assert!(rendered.contains("\n  \"a\": 1"));

        // render identik dengan file → belum dianggap berubah
        std::fs::write(&dir.join("config.json"), rendered.clone()).unwrap();
        let mut reloaded = Doc::load(&spec(dir.join("config.json"), Format::Json)).unwrap();
        assert!(!reloaded.is_changed());
        reloaded.object_at("x").insert("y".into(), 1.into());
        assert!(reloaded.is_changed());

        // mutasi basi tanpa perubahan isi tidak boleh dianggap berubah
        let mut same = Doc::load(&spec(dir.join("config.json"), Format::Json)).unwrap();
        same.dirty = true;
        assert!(!same.is_changed());

        doc.tree_mut(); // memastikan akses tree tidak panik
    }

    #[test]
    fn dokumen_missing_tidak_pernah_dianggap_berubah_tanpa_mutasi() {
        let dir = temp_dir("json-missing");
        let doc = Doc::load(&spec(dir.join("x.json"), Format::Json)).unwrap();
        assert!(!doc.exists);
        assert!(!doc.is_changed());
        assert_eq!(doc.render(), "{}\n");
        assert!(!doc.is_changed(), "render saja tidak boleh menandai dirty");
    }

    #[test]
    fn object_at_membuat_subobject_dan_mempertahankan_field_lain() {
        let dir = temp_dir("json-object-at");
        let path = dir.join("c.json");
        std::fs::write(&path, r#"{"providers":{"a":{"custom":1}}}"#).unwrap();

        let mut doc = Doc::load(&spec(path, Format::Json)).unwrap();
        doc.object_at("providers").insert("b".into(), serde_json::json!({}));
        let value: serde_json::Value = serde_json::from_str(&doc.render()).unwrap();
        assert_eq!(value["providers"]["a"]["custom"], 1);
        assert!(value["providers"]["b"].is_object());
    }

    #[test]
    fn before_redacted_aman_untuk_file_rusak() {
        let dir = temp_dir("before-redacted");
        let path = dir.join("a.json");
        std::fs::write(&path, r#"{"key":"sk-rahasia"}"#).unwrap();
        let doc = Doc::load(&spec(path, Format::Json)).unwrap();
        let before = doc.before_redacted();
        assert!(!before.contains("sk-rahasia"));
        assert!(before.contains(MASK));

    }

    #[test]
    fn file_rusak_gagal_dimuat_bukan_ditimpa() {
        // Keputusan desain: file yang tidak bisa diparse tidak boleh ditulis
        // ulang secara diam-diam - lebih baik gagal keras dan biarkan pengguna
        // memperbaikinya.
        let dir = temp_dir("json-rusak");
        let path = dir.join("rusak.json");
        std::fs::write(&path, "bukan json {").unwrap();
        assert!(Doc::load(&spec(path, Format::Json)).is_err());
    }
}
