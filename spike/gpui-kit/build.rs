// gpui_linux menautkan `-lxkbcommon-x11` untuk backend X11. Kalau paket dev-nya
// terpasang, cukup diam. Kalau tidak, linker hanya butuh *nama taut*
// (`libxkbcommon-x11.so`), sedangkan library runtime-nya (`…so.0`) hampir selalu
// sudah ada — jadi sediakan taut di OUT_DIR dan arahkan link-search ke sana.
//
// Debian/Ubuntu : sudo apt install libxkbcommon-x11-dev
// Fedora        : sudo dnf install libxkbcommon-x11-devel
// Arch          : sudo pacman -S libxkbcommon-x11
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    if pkg_config_has("xkbcommon-x11") {
        return;
    }

    const DIRS: &[&str] = &[
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
        "/usr/lib/x86_64-linux-gnu",
        "/lib/x86_64-linux-gnu",
        "/usr/local/lib",
    ];
    const NAMES: &[&str] = &["libxkbcommon-x11.so.0", "libxkbcommon-x11.so"];

    let found = DIRS
        .iter()
        .flat_map(|d| NAMES.iter().map(move |n| std::path::Path::new(d).join(n)))
        .find(|p| p.exists());

    let Some(found) = found else {
        println!(
            "cargo:warning=libxkbcommon-x11 tidak ditemukan. Pasang paket dev-nya: \
             apt install libxkbcommon-x11-dev / dnf install libxkbcommon-x11-devel / \
             pacman -S libxkbcommon-x11"
        );
        return;
    };

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let link = out.join("libxkbcommon-x11.so");
    let _ = std::fs::remove_file(&link);
    if std::os::unix::fs::symlink(&found, &link).is_ok() {
        println!("cargo:rustc-link-search=native={}", out.display());
        println!(
            "cargo:warning=memakai taut lokal ke {}; pasang paket dev-nya untuk build yang bersih",
            found.display()
        );
    }
}

fn pkg_config_has(lib: &str) -> bool {
    std::process::Command::new("pkg-config")
        .args(["--exists", lib])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
