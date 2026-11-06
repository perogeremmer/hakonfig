// Rekonfig — kelola konfigurasi provider AI coding harness.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod engine;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_harnesses,
            commands::list_presets,
            commands::read_harness,
            commands::preview_edit,
            commands::apply_edit,
            commands::list_backups,
            commands::restore_backup,
            commands::test_connection,
            commands::open_in_file_manager,
        ])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan Rekonfig");
}
