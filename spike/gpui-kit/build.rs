// Di mesin ini `libxkbcommon-x11-dev` tidak terpasang, padahal gpui_linux
// menautkan `-lxkbcommon-x11` untuk backend X11. Library runtime-nya ada
// (`libxkbcommon-x11.so.0`), jadi cukup sediakan nama taut yang dicari linker
// di OUT_DIR lalu arahkan `cargo:rustc-link-search` ke sana.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let link = out.join("libxkbcommon-x11.so");
    if !link.exists() {
        let _ = std::fs::remove_file(&link);
        let _ = std::os::unix::fs::symlink(
            "/usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0",
            &link,
        );
    }
    println!("cargo:rustc-link-search=native={}", out.display());
}
