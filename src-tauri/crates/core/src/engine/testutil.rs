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
    // `--home` menyatakan niat berbeda; test yang memakainya wajib melepasnya
    // sendiri, dan sandbox memastikan sisa test sebelumnya tidak terbawa.
    std::env::remove_var("HAKONFIG_HOME");
    // `doctor` juga membaca kredensial dari environment (`*_API_KEY`), karena
    // harness me-refresh katalog provider yang punya key di env. Test harus
    // tidak bergantung pada environment mesin yang menjalankannya.
    let inherited: Vec<String> = std::env::vars()
        .map(|(name, _)| name)
        .filter(|name| name.ends_with("_API_KEY"))
        .collect();
    for name in inherited {
        std::env::remove_var(name);
    }
    (guard, dir)
}

/// Direktori sementara unik tanpa menyentuh `HAKONFIG_ROOT`.
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hakonfig-unit-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
