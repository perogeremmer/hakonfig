// Hakonfig - kelola konfigurasi provider AI coding harness.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod ui_scale;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_harnesses,
            commands::list_targets,
            commands::list_presets,
            commands::read_harness,
            commands::preview_edit,
            commands::apply_edit,
            commands::fix_notice,
            commands::list_backups,
            commands::restore_backup,
            commands::test_connection,
            commands::open_in_file_manager,
            commands::list_harness_catalog,
            commands::harness_version,
            commands::install_harness,
            commands::uninstall_harness,
            commands::fetch_models,
            commands::open_external,
            ui_scale::ui_scale_info,
            ui_scale::set_ui_scale,
        ])
        .setup(|app| {
            // Sebelum jendela dipakai: pastikan muat di layar, lalu terapkan
            // zoom terakhir supaya tidak ada satu frame UI kecil saat dibuka.
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = ui_scale::fit_to_monitor(&window) {
                    eprintln!("hakonfig: gagal menyesuaikan jendela: {error}");
                }
                ui_scale::restore_zoom(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start Hakonfig");
}
