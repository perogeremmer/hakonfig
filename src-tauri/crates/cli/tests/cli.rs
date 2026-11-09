//! Automation test: menjalankan binary `hakonfig-cli` seperti pengguna sungguhan.
//!
//! Setiap test memakai `HAKONFIG_ROOT` sendiri (lewat `--root`), jadi tidak
//! pernah menyentuh config asli dan aman berjalan paralel.
//!
//! Yang diuji di sini adalah kontrak CLI: exit code, pesan error, dan isi file
//! yang benar-benar ditulis. Logika keputusan murni diuji sebagai unit test di
//! `src/draft.rs` dan `src/output.rs`.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_hakonfig-cli");
const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
const GROQ_PRESET_MODELS: usize = 6;

fn sandbox(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hakonfig-cli-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("gagal membuat sandbox");
    dir
}

/// Command dasar dengan environment yang sudah dibersihkan.
///
/// `doctor` membaca kredensial dari environment (`*_API_KEY`) karena harness
/// me-refresh katalog provider yang punya key di env; test tidak boleh
/// bergantung pada environment mesin yang menjalankannya.
fn base_command(root: &Path) -> Command {
    let mut command = Command::new(BIN);
    command.arg("--root").arg(root);
    for (name, _) in std::env::vars() {
        if name.ends_with("_API_KEY") {
            command.env_remove(&name);
        }
    }
    command
}

fn run(root: &Path, args: &[&str]) -> Output {
    base_command(root)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("gagal menjalankan binary")
}

/// Jalankan binary dengan environment tambahan (untuk mengarahkan endpoint tes).
fn run_env(root: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = base_command(root);
    command.args(args).stdin(Stdio::null());
    for (name, value) in envs {
        command.env(name, value);
    }
    command.output().expect("gagal menjalankan binary")
}

fn run_stdin(root: &Path, args: &[&str], input: &str) -> Output {
    let mut child = base_command(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("gagal menjalankan binary");

    {
        let mut stdin = child.stdin.take().expect("stdin tidak tersedia");
        stdin
            .write_all(input.as_bytes())
            .expect("gagal menulis ke stdin");
    }

    child.wait_with_output().expect("gagal menunggu proses")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn models_path(root: &Path) -> PathBuf {
    root.join("pi/agent/models.json")
}

fn auth_path(root: &Path) -> PathBuf {
    root.join("pi/agent/auth.json")
}

fn read_json(path: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("gagal membaca {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} bukan JSON valid: {error}\n{text}", path.display()))
}

// ------------------------------------------------------------------ tulis

#[test]
fn add_writes_provider_key_and_selected_models() {
    let root = sandbox("add-basic");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--models",
            "llama-3.3-70b-versatile,llama-3.1-8b-instant",
            "--no-test",
        ],
        "gsk-rahasia-123\n",
    );

    assert!(output.status.success(), "harus sukses: {}", stderr(&output));

    let models = read_json(&models_path(&root));
    let groq = &models["providers"]["groq"];
    assert_eq!(groq["baseUrl"], GROQ_BASE_URL, "base URL harus ikut preset");
    assert_eq!(
        groq["models"].as_array().map(Vec::len),
        Some(2),
        "hanya model yang dipilih yang ditulis"
    );
    assert_eq!(groq["models"][0]["id"], "llama-3.3-70b-versatile");

    let auth = read_json(&auth_path(&root));
    assert_eq!(
        auth["groq"]["key"], "gsk-rahasia-123",
        "key inline disimpan di auth.json"
    );
    assert!(
        !std::fs::read_to_string(models_path(&root))
            .unwrap()
            .contains("gsk-rahasia-123"),
        "models.json tidak boleh memuat key mentah"
    );
}

#[test]
fn add_without_models_uses_all_preset_models() {
    let root = sandbox("add-default-models");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--no-test",
        ],
        "gsk-x\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let models = read_json(&models_path(&root));
    assert_eq!(
        models["providers"]["groq"]["models"]
            .as_array()
            .map(Vec::len),
        Some(GROQ_PRESET_MODELS),
        "default non-interaktif = semua model preset"
    );
}

#[test]
fn add_twice_without_change_writes_nothing() {
    let root = sandbox("add-idempotent");
    let args = [
        "add",
        "--harness",
        "pi",
        "--provider",
        "groq",
        "--key-stdin",
        "--models",
        "a,b",
        "--no-test",
    ];
    let first = run_stdin(&root, &args, "gsk-sama\n");
    assert!(first.status.success(), "{}", stderr(&first));

    let before = std::fs::read_to_string(models_path(&root)).unwrap();
    let second = run_stdin(&root, &args, "gsk-sama\n");

    assert!(second.status.success(), "{}", stderr(&second));
    assert!(
        stdout(&second).contains("nothing to write"),
        "harus melaporkan tidak ada perubahan: {}",
        stdout(&second)
    );
    assert_eq!(
        before,
        std::fs::read_to_string(models_path(&root)).unwrap(),
        "isi file tidak boleh berubah"
    );
}

#[test]
fn add_dry_run_writes_no_file() {
    let root = sandbox("add-dry-run");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--dry-run",
        ],
        "gsk-x\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !models_path(&root).exists(),
        "--dry-run tidak boleh membuat models.json"
    );
    assert!(stdout(&output).contains("dry-run"), "{}", stdout(&output));
}

#[test]
fn add_env_mode_writes_a_reference_not_the_key() {
    let root = sandbox("add-key-env");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--key-env",
            "--models",
            "a",
            "--no-test",
        ],
        "gsk-jangan-ditulis\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));

    let models = read_json(&models_path(&root));
    assert_eq!(
        models["providers"]["groq"]["apiKey"], "$GROQ_API_KEY",
        "mode env menulis referensi variabel"
    );
    assert!(
        !std::fs::read_to_string(auth_path(&root))
            .unwrap_or_default()
            .contains("gsk-jangan-ditulis"),
        "mode env tidak menyimpan key mentah di auth.json"
    );
    assert!(
        stdout(&output).contains("GROQ_API_KEY"),
        "harus memberi tahu nama variabel yang perlu diisi: {}",
        stdout(&output)
    );
}

#[test]
fn add_custom_provider_uses_its_own_base_url() {
    let root = sandbox("add-custom");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "toko-saya",
            "--name",
            "Toko",
            "--base-url",
            "https://api.toko.test/v1",
            "--key-stdin",
            "--models",
            "m1",
            "--no-test",
        ],
        "sk-toko\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let models = read_json(&models_path(&root));
    assert_eq!(
        models["providers"]["toko-saya"]["baseUrl"],
        "https://api.toko.test/v1"
    );
    assert_eq!(models["providers"]["toko-saya"]["name"], "Toko");
    assert_eq!(
        models["providers"]["toko-saya"]["api"], "openai-completions",
        "tipe API custom memakai bawaan OpenAI-compatible"
    );
}

#[test]
fn add_creates_a_backup_when_overwriting_an_existing_file() {
    let root = sandbox("add-backup");
    let first = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--models",
            "a",
            "--no-test",
        ],
        "gsk-satu\n",
    );
    assert!(first.status.success(), "{}", stderr(&first));

    let second = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--models",
            "a,b",
            "--no-test",
        ],
        "gsk-dua\n",
    );
    assert!(second.status.success(), "{}", stderr(&second));
    assert!(
        stdout(&second).contains("backup:"),
        "harus melaporkan backup: {}",
        stdout(&second)
    );

    let backups = root.join("pi/agent/.hakonfig/backups");
    let count = std::fs::read_dir(&backups)
        .expect("direktori backup harus ada")
        .count();
    assert!(count >= 1, "minimal satu backup dibuat, dapat {count}");
}

// ------------------------------------------------------------------ error

#[test]
fn add_without_provider_fails_with_a_flag_hint() {
    let root = sandbox("err-no-provider");
    let output = run(&root, &["add", "--harness", "pi", "--no-test"]);

    assert!(!output.status.success(), "harus gagal tanpa --provider");
    let message = stderr(&output);
    assert!(
        message.contains("--provider"),
        "pesan harus menyebut flag: {message}"
    );
    assert!(
        message.contains("hakonfig-cli providers"),
        "pesan harus menunjukkan cara melihat daftar: {message}"
    );
}

#[test]
fn add_provider_typo_suggests_a_candidate() {
    let root = sandbox("err-typo");
    let output = run(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "openroutr",
            "--no-test",
        ],
    );

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("openrouter"),
        "harus menyarankan kandidat terdekat: {}",
        stderr(&output)
    );
}

#[test]
fn add_preset_without_models_requires_explicit_models() {
    let root = sandbox("err-no-models");
    let output = run(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "anthropic",
            "--no-test",
        ],
    );

    assert!(
        !output.status.success(),
        "preset tanpa model harus menolak tanpa --models"
    );
    assert!(
        stderr(&output).contains("--models"),
        "pesan harus menjelaskan jalan keluarnya: {}",
        stderr(&output)
    );
}

#[test]
fn add_custom_without_base_url_is_rejected() {
    let root = sandbox("err-custom");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "toko",
            "--name",
            "Toko",
            "--key-stdin",
            "--no-test",
        ],
        "sk\n",
    );

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("--base-url"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn add_unknown_harness_is_rejected() {
    let root = sandbox("err-harness");
    let output = run(
        &root,
        &[
            "add",
            "--harness",
            "tidak-ada",
            "--provider",
            "groq",
            "--no-test",
        ],
    );

    assert!(!output.status.success());
    assert!(stderr(&output).contains("tidak-ada"), "{}", stderr(&output));
}

// ------------------------------------------------------------------ baca

#[test]
fn show_lists_providers_without_leaking_the_key() {
    let root = sandbox("show-redaction");
    let add = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--models",
            "a",
            "--no-test",
        ],
        "gsk-sangat-rahasia\n",
    );
    assert!(add.status.success(), "{}", stderr(&add));

    let output = run(&root, &["show", "pi"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(text.contains("groq"), "harus menampilkan provider: {text}");
    assert!(
        !text.contains("gsk-sangat-rahasia"),
        "key tidak boleh muncul di output: {text}"
    );
}

#[test]
fn list_json_is_parseable_by_scripts() {
    let root = sandbox("list-json");
    let output = run(&root, &["list", "--json"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("output --json harus JSON valid");
    assert!(
        parsed
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "harus berisi daftar harness"
    );
}

#[test]
fn providers_can_be_filtered() {
    let root = sandbox("providers-filter");
    let output = run(&root, &["providers", "--search", "groq"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("groq"));
    assert!(
        text.contains(GROQ_BASE_URL) || text.contains("GROQ_API_KEY"),
        "{text}"
    );
}

#[test]
fn paths_shows_config_locations() {
    let root = sandbox("paths");
    let output = run(&root, &["paths", "pi"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains(&root.display().to_string()),
        "path harus berada di dalam sandbox: {}",
        stdout(&output)
    );
}

#[test]
fn add_explicit_test_failure_aborts_without_writing() {
    let root = sandbox("test-explicit-fail");
    // Port 1 di localhost selalu menolak koneksi: deterministik, tanpa jaringan.
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "lokal",
            "--name",
            "Lokal",
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--key-stdin",
            "--models",
            "m1",
            "--test",
        ],
        "sk-lokal\n",
    );

    assert!(
        !output.status.success(),
        "--test eksplisit yang gagal harus menggagalkan perintah: {}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("connection test failed"),
        "harus menjelaskan penyebabnya: {}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("--no-test"),
        "pesan harus menunjukkan jalan keluar: {}",
        stderr(&output)
    );
    assert!(
        !models_path(&root).exists(),
        "tidak boleh ada yang ditulis ketika gerbang --test gagal"
    );
}

#[test]
fn add_without_a_test_flag_skips_the_test_and_still_writes() {
    // Tanpa --test/--no-test, mode non-interaktif tidak mencoba endpoint,
    // jadi endpoint yang mati pun tidak menghalangi penulisan.
    let root = sandbox("test-implicit-skip");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "lokal",
            "--name",
            "Lokal",
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--key-stdin",
            "--models",
            "m1",
        ],
        "sk-lokal\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !stderr(&output).contains("connection test failed"),
        "tanpa --test tidak boleh ada percobaan koneksi: {}",
        stderr(&output)
    );
    let models = read_json(&models_path(&root));
    assert_eq!(
        models["providers"]["lokal"]["baseUrl"],
        "http://127.0.0.1:1/v1"
    );
}

// ------------------------------------------------------------------ doctor

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("tanpa parent")).expect("gagal membuat folder");
    std::fs::write(&path, contents).expect("gagal menulis file");
}

#[test]
fn doctor_reports_scope_that_hides_written_models() {
    let root = sandbox("doctor-scope");
    write_file(
        &root,
        "pi/agent/models.json",
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"},{"id":"deepseek/b"}]}}}"#,
    );
    write_file(
        &root,
        "pi/agent/settings.json",
        r#"{"enabledModels":["gateway/*"]}"#,
    );

    let output = run(&root, &["doctor"]);
    let text = stdout(&output);

    assert!(
        !output.status.success(),
        "config yang tersembunyi harus menghasilkan exit non-zero: {text}"
    );
    assert!(text.contains("enabledModels"), "{text}");
    assert!(
        text.contains("gateway/**"),
        "harus menyebut polanya: {text}"
    );
    assert!(
        stderr(&output).contains("doctor --fix"),
        "harus menunjukkan jalan keluar: {}",
        stderr(&output)
    );
}

#[test]
fn doctor_fix_widens_the_scope_and_clears_the_report() {
    let root = sandbox("doctor-fix");
    write_file(
        &root,
        "pi/agent/models.json",
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"deepseek/b"}]}}}"#,
    );
    write_file(
        &root,
        "pi/agent/settings.json",
        r#"{"enabledModels":["gateway/*"]}"#,
    );

    let output = run(&root, &["doctor", "--fix"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let settings = read_json(&root.join("pi/agent/settings.json"));
    let patterns: Vec<&str> = settings["enabledModels"]
        .as_array()
        .expect("enabledModels harus array")
        .iter()
        .map(|item| item.as_str().unwrap_or_default())
        .collect();
    assert!(patterns.contains(&"gateway/**"), "{patterns:?}");
    assert!(!patterns.contains(&"gateway/*"), "{patterns:?}");

    let check = run(&root, &["doctor"]);
    assert!(check.status.success(), "{}", stderr(&check));
    assert!(
        stdout(&check).contains("no hidden config"),
        "{}",
        stdout(&check)
    );
}

#[test]
fn doctor_json_lists_notices_with_their_fix() {
    let root = sandbox("doctor-json");
    write_file(
        &root,
        "pi/agent/models.json",
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );
    write_file(
        &root,
        "pi/agent/settings.json",
        r#"{"enabledModels":["lain/**"]}"#,
    );

    let output = run(&root, &["doctor", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON valid");
    let items = parsed.as_array().expect("array notice");
    assert_eq!(items.len(), 1, "{parsed:#}");
    assert_eq!(items[0]["code"], "pi.scope");
    assert_eq!(items[0]["harness"], "pi");
    assert_eq!(items[0]["provider"], "gateway");
    assert!(items[0]["fix"]
        .as_str()
        .unwrap_or_default()
        .contains("gateway/**"));
}

#[test]
fn doctor_does_not_touch_files_without_fix() {
    let root = sandbox("doctor-readonly");
    write_file(
        &root,
        "pi/agent/models.json",
        r#"{"providers":{"gateway":{"baseUrl":"https://gw.test/v1","api":"openai-completions","models":[{"id":"a"}]}}}"#,
    );
    let before = r#"{"enabledModels":["lain/**"]}"#;
    write_file(&root, "pi/agent/settings.json", before);

    let _ = run(&root, &["doctor"]);

    assert_eq!(
        std::fs::read_to_string(root.join("pi/agent/settings.json")).unwrap(),
        before,
        "doctor tanpa --fix tidak boleh menulis apa pun"
    );
}

// ---------------------------------------------------------------- katalog

#[test]
fn doctor_reports_a_credential_that_refreshes_without_a_provider() {
    let root = sandbox("doctor-catalog-credential");
    write_file(&root, "pi/agent/models.json", r#"{"providers":{}}"#);
    write_file(
        &root,
        "pi/agent/auth.json",
        r#"{"xai":{"type":"api_key","key":"xai-fake"}}"#,
    );

    let output = run(&root, &["doctor", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON valid");
    let notice = parsed
        .as_array()
        .expect("array")
        .iter()
        .find(|notice| notice["code"] == "catalog.credential-without-provider")
        .expect("harus melaporkan kredensial tanpa provider");

    assert_eq!(notice["provider"], "xai");
    assert!(notice["fix"].is_null(), "tidak bisa diperbaiki otomatis");
    assert!(
        output.status.success(),
        "notice manual tidak menggagalkan doctor: {}",
        stderr(&output)
    );
}

#[test]
fn doctor_online_reports_a_failing_catalog_with_the_real_error() {
    let root = sandbox("doctor-catalog-online");
    write_file(&root, "pi/agent/models.json", r#"{"providers":{}}"#);
    write_file(
        &root,
        "pi/agent/auth.json",
        r#"{"xai":{"type":"api_key","key":"xai-fake"}}"#,
    );

    // Port 1 selalu menolak koneksi: deterministik dan tanpa jaringan.
    let output = run_env(
        &root,
        &["doctor", "--online", "--harness", "pi"],
        &[("HAKONFIG_PI_CATALOG_BASE", "http://127.0.0.1:1")],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("catalog request for 'xai' failed"),
        "harus menyebut provider dan kegagalannya: {}",
        stdout(&output)
    );
}

#[test]
fn doctor_without_online_never_touches_the_network() {
    let root = sandbox("doctor-offline");
    write_file(&root, "pi/agent/models.json", r#"{"providers":{}}"#);
    write_file(
        &root,
        "pi/agent/auth.json",
        r#"{"xai":{"type":"api_key","key":"xai-fake"}}"#,
    );

    // Endpoint yang pasti mati: kalau doctor mencoba, pesannya akan muncul.
    let output = run_env(
        &root,
        &["doctor", "--harness", "pi"],
        &[("HAKONFIG_PI_CATALOG_BASE", "http://127.0.0.1:1")],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !stdout(&output).contains("catalog request"),
        "tanpa --online tidak boleh ada permintaan katalog: {}",
        stdout(&output)
    );
}

// ------------------------------------------------------------------ sync

/// Server HTTP tiruan satu tembakan: cukup untuk `GET {base}/models`.
///
/// Mengembalikan port-nya. Thread-nya berhenti sendiri setelah melayani satu
/// permintaan, jadi test tetap offline dan deterministik.
fn mock_models_server(body: &'static str) -> u16 {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("gagal membuka port");
    let port = listener.local_addr().expect("tanpa alamat").port();
    std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().expect("gagal klon stream"));
            // Baca header permintaan sampai baris kosong, lalu jawab.
            let mut line = String::new();
            while reader.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }
            let mut stream = stream;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    port
}

#[test]
fn add_sync_uses_the_endpoint_model_list() {
    // Preset "lokal" tidak punya daftar model, tapi yang diuji di sini adalah
    // jalur sync: id yang ditulis harus datang dari endpoint, bukan dari flag.
    let root = sandbox("sync-endpoint");
    let port = mock_models_server(r#"{"data":[{"id":"live-a"},{"id":"live-b"}]}"#);
    let base_url = format!("http://127.0.0.1:{}/v1", port);
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "lokal",
            "--name",
            "Lokal",
            "--base-url",
            &base_url,
            "--key-stdin",
            "--sync",
            "--no-test",
        ],
        "sk-lokal\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let models = read_json(&models_path(&root));
    let ids: Vec<&str> = models["providers"]["lokal"]["models"]
        .as_array()
        .expect("models harus array")
        .iter()
        .map(|model| model["id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        ids,
        vec!["live-a", "live-b"],
        "--sync harus memakai daftar dari endpoint: {}",
        stdout(&output)
    );
}

#[test]
fn add_sync_with_an_unreachable_endpoint_aborts_without_writing() {
    let root = sandbox("sync-unreachable");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "lokal",
            "--name",
            "Lokal",
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--key-stdin",
            "--sync",
            "--no-test",
        ],
        "sk-lokal\n",
    );

    assert!(
        !output.status.success(),
        "--sync yang gagal harus menggagalkan perintah: {}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains("model sync failed"),
        "error harus menjelaskan kegagalan sync: {}",
        stderr(&output)
    );
    assert!(
        !models_path(&root).exists(),
        "tidak boleh ada yang ditulis ketika sync gagal"
    );
}

#[test]
fn add_models_flag_still_wins_over_sync() {
    // --models eksplisit tidak boleh memicu permintaan jaringan sama sekali.
    let root = sandbox("sync-models-flag");
    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "lokal",
            "--name",
            "Lokal",
            "--base-url",
            "http://127.0.0.1:1/v1",
            "--key-stdin",
            "--models",
            "m1,m2",
            "--sync",
            "--no-test",
        ],
        "sk-lokal\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !stderr(&output).contains("sync"),
        "--models eksplisit tidak boleh menyentuh jaringan: {}",
        stderr(&output)
    );
    let models = read_json(&models_path(&root));
    assert_eq!(
        models["providers"]["lokal"]["models"]
            .as_array()
            .expect("models harus array")
            .len(),
        2
    );
}

// ------------------------------------------------------------------ bahasa

#[test]
fn default_language_is_english() {
    let root = sandbox("lang-default");
    let output = run(&root, &["list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("harnesses have a config"),
        "default harus English: {}",
        stdout(&output)
    );
}

#[test]
fn lang_id_switches_the_output_language() {
    let root = sandbox("lang-id");
    let output = run(&root, &["--lang", "id", "list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("harness punya config"),
        "output harus Bahasa Indonesia: {}",
        stdout(&output)
    );
}

#[test]
fn unknown_language_is_rejected() {
    let root = sandbox("lang-unknown");
    let output = run(&root, &["--lang", "jawa", "list"]);

    assert!(!output.status.success(), "bahasa asing harus ditolak");
    assert!(stderr(&output).contains("jawa"), "{}", stderr(&output));
}

// ------------------------------------------------------------------ kerapian

#[test]
fn the_same_warning_is_not_printed_twice() {
    let root = sandbox("warn-dedupe");
    // Tanpa key: engine memberi peringatan "provider dibuat tanpa API key".
    // Peringatan itu muncul di pratinjau; laporan akhir tidak boleh mengulang.
    let output = run(
        &root,
        &["add", "--harness", "pi", "--provider", "groq", "--no-test"],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let combined = format!("{}{}", stdout(&output), stderr(&output));
    let occurrences = combined.matches("without an API key").count();
    assert_eq!(
        occurrences, 1,
        "peringatan harus muncul sekali saja:\n{combined}"
    );
}

#[test]
fn a_large_diff_is_truncated_and_reported() {
    let root = sandbox("diff-truncate");
    let models = (0..100)
        .map(|index| format!("model-{index}"))
        .collect::<Vec<_>>()
        .join(",");

    let output = run_stdin(
        &root,
        &[
            "add",
            "--harness",
            "pi",
            "--provider",
            "groq",
            "--key-stdin",
            "--models",
            &models,
            "--dry-run",
        ],
        "gsk-x\n",
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    // Batas dihitung per file, jadi yang diuji: model terakhir tidak ikut
    // dicetak dan sisa barisnya dilaporkan.
    assert!(
        !text.contains("model-99"),
        "diff harus dipotong, bukan dicetak semua:\n{text}"
    );
    assert!(
        text.contains("more lines"),
        "harus menyebut sisa baris:\n{text}"
    );
}

#[test]
fn key_stdin_rejects_an_interactive_terminal() {
    // `script` membuat pty supaya stdin benar-benar terlihat seperti terminal.
    if Command::new("script").arg("--version").output().is_err() {
        eprintln!("dilewati: `script` tidak tersedia");
        return;
    }

    let root = sandbox("key-stdin-tty");
    let command = format!(
        "{} --root {} add --harness pi --provider groq --key-stdin --no-test",
        BIN,
        root.display()
    );
    let output = Command::new("script")
        .args(["-qec", &command, "/dev/null"])
        .output()
        .expect("gagal menjalankan script");

    let text = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        text.contains("--key-stdin needs piped input"),
        "harus menolak terminal dengan jelas, bukan menggantung:\n{text}"
    );
    assert!(!models_path(&root).exists(), "tidak boleh ada yang ditulis");
}
