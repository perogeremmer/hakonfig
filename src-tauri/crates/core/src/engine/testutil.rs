//! Utilitas bersama untuk test.
//!
//! `HAKONFIG_ROOT` bersifat process-global, jadi semua test yang memakainya
//! wajib memegang mutex ini agar tidak saling menimpa.

use std::path::PathBuf;

pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn sandbox(name: &str) -> (std::sync::MutexGuard<'static, ()>, PathBuf) {
    let guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("hakonfig-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("HAKONFIG_ROOT", &dir);
    (guard, dir)
}

/// Direktori sementara unik tanpa menyentuh `HAKONFIG_ROOT`.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hakonfig-unit-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
