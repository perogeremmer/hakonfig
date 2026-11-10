//! Skala UI untuk jendela desktop: zoom webview + jendela yang muat di layar.
//!
//! Dua masalah berbeda yang gejalanya sama ("semuanya kecil"):
//!
//! 1. **Skala render.** Di Linux (WebKitGTK) `devicePixelRatio` bisa tetap 1
//!    padahal monitor 2x, sehingga UI dirender separuh. Keputusan zoom-nya ada
//!    di `hakonfig_core::engine::ui_scale` supaya bisa diuji tanpa jendela; file
//!    ini hanya menerapkannya ke `WebviewWindow`.
//! 2. **Ukuran jendela.** `1200x820` dengan minimum `940x620` (logical px)
//!    tidak muat di laptop 1366x768 @150%: ruang CSS-nya hanya 911x512, jadi
//!    window manager memaksa ukuran lain dan sebagian UI terpotong.
//!
//! Zoom juga di-cache ke berkas kecil di direktori konfigurasi aplikasi supaya
//! bisa diterapkan saat startup sebelum halaman pertama dilukis. Tanpa cache,
//! pengguna zoom 1.5 melihat satu frame UI kecil berkedip setiap kali membuka
//! aplikasi. Preferensi tetap hidup di `localStorage` frontend (sumber
//! kebenaran untuk UI); cache ini hanya nilai yang sudah dihitung.

use std::path::PathBuf;

use hakonfig_core::engine::ui_scale::{self, ScaleDecision, ScaleReason};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewWindow};

/// Batas minimum jendela yang masih layak dipakai (CSS px).
const ABSOLUTE_MIN_WIDTH: f64 = 720.0;
const ABSOLUTE_MIN_HEIGHT: f64 = 480.0;

/// Porsi work area yang boleh dipakai jendela saat pertama dibuka.
const FIT_FRACTION: f64 = 0.9;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiScaleInfo {
    /// Zoom yang sedang berlaku.
    pub zoom: f64,
    pub reason: ScaleReason,
    /// Skala monitor menurut OS.
    pub monitor_scale: f64,
    /// Skala yang dipakai jendela/webview.
    pub window_scale_factor: f64,
    /// Ukuran jendela dalam CSS px menurut sisi Rust.
    pub window_width: f64,
    pub window_height: f64,
    /// `false` kalau platform tidak melaporkan skala monitor.
    pub monitor_known: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedZoom {
    zoom: f64,
}

fn cache_path(window: &WebviewWindow) -> Option<PathBuf> {
    let dir = window.path().app_config_dir().ok()?;
    Some(dir.join("ui-scale.json"))
}

fn read_cached_zoom(window: &WebviewWindow) -> Option<f64> {
    let path = cache_path(window)?;
    let raw = std::fs::read_to_string(path).ok()?;
    let cached: CachedZoom = serde_json::from_str(&raw).ok()?;
    if cached.zoom.is_finite() && cached.zoom > 0.0 {
        Some(ui_scale::clamp_zoom(cached.zoom))
    } else {
        None
    }
}

fn write_cached_zoom(window: &WebviewWindow, zoom: f64) {
    let Some(path) = cache_path(window) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let payload = CachedZoom { zoom };
    if let Ok(text) = serde_json::to_string(&payload) {
        // Cache bersifat optmisasi: gagal menulis tidak boleh menggagalkan UI.
        let _ = std::fs::write(path, text);
    }
}

/// Skala monitor untuk jendela ini, kalau platform melaporkannya.
fn monitor_scale(window: &WebviewWindow) -> Option<f64> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    (scale.is_finite() && scale > 0.0).then_some(scale)
}

/// Ukuran jendela saat ini dalam CSS px (physical / skala jendela).
fn window_css_size(window: &WebviewWindow) -> (f64, f64, f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size().unwrap_or_default();
    let divisor = if scale > 0.0 { scale } else { 1.0 };
    (
        size.width as f64 / divisor,
        size.height as f64 / divisor,
        if scale > 0.0 { scale } else { 1.0 },
    )
}

#[tauri::command]
pub fn ui_scale_info(window: WebviewWindow) -> Result<UiScaleInfo, String> {
    let (width, height, window_scale) = window_css_size(&window);
    let scale = monitor_scale(&window);
    // Tauri tidak punya getter zoom, jadi yang dilaporkan adalah zoom yang
    // terakhir kita terapkan (cache). Hotkey zoom bawaan webview tidak dipakai,
    // jadi angka ini memang satu-satunya sumber.
    let zoom = read_cached_zoom(&window).unwrap_or(1.0);
    Ok(UiScaleInfo {
        zoom,
        // Alasan dihitung ulang di frontend (yang tahu devicePixelRatio); di
        // sini cukup laporkan apa yang sedang berlaku.
        reason: if (zoom - 1.0).abs() < 0.001 {
            ScaleReason::Matches
        } else {
            ScaleReason::Manual
        },
        monitor_scale: scale.unwrap_or(1.0),
        window_scale_factor: window_scale,
        window_width: width,
        window_height: height,
        monitor_known: scale.is_some(),
    })
}

/// Terapkan pilihan skala UI.
///
/// `preference` = `None` untuk mode auto. `device_pixel_ratio` dikirim frontend
/// karena hanya webview yang tahu nilainya — dan justru selisih antara angka
/// itu dan skala monitor yang menentukan perlu tidaknya menambal.
#[tauri::command]
pub fn set_ui_scale(
    window: WebviewWindow,
    preference: Option<f64>,
    device_pixel_ratio: f64,
) -> Result<UiScaleInfo, String> {
    let monitor_scale = monitor_scale(&window).unwrap_or(device_pixel_ratio);
    let decision = ui_scale::resolve(preference, monitor_scale, device_pixel_ratio);
    apply(&window, decision).map_err(|error| error.to_string())?;
    write_cached_zoom(&window, decision.zoom);
    ui_scale_info(window)
}

fn apply(window: &WebviewWindow, decision: ScaleDecision) -> tauri::Result<()> {
    window.set_zoom(decision.zoom)?;
    Ok(())
}

/// Terapkan zoom terakhir sebelum halaman pertama dilukis.
///
/// Hanya nilai hasil hitungan yang di-cache, bukan preferensinya: mode auto
/// butuh `devicePixelRatio` yang belum ada di tahap ini, sedangkan angka hasil
/// hitungan sudah final.
pub fn restore_zoom(window: &WebviewWindow) {
    let Some(zoom) = read_cached_zoom(window) else {
        return;
    };
    let _ = window.set_zoom(zoom);
}

/// Pastikan jendela pertama kali terbuka utuh di layar yang sedang dipakai.
///
/// Bukan kosmetik: `minWidth 940` di config lebih besar dari ruang CSS laptop
/// 1366x768 @150% (911 px), jadi tanpa ini pengguna tidak bisa mengecilkan
/// jendela sampai muat.
pub fn fit_to_monitor(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    if !(scale.is_finite() && scale > 0.0) {
        return Ok(());
    }

    let work = monitor.work_area();
    let available_width = work.size.width as f64 / scale;
    let available_height = work.size.height as f64 / scale;
    if available_width <= 0.0 || available_height <= 0.0 {
        return Ok(());
    }

    // Minimum tidak boleh lebih besar dari ruang yang ada, kalau tidak jendela
    // akan terus dipaksa keluar layar.
    let configured = configured_window(window);
    let min_width = configured
        .min_width
        .unwrap_or(ABSOLUTE_MIN_WIDTH)
        .min(available_width)
        .max(1.0);
    let min_height = configured
        .min_height
        .unwrap_or(ABSOLUTE_MIN_HEIGHT)
        .min(available_height)
        .max(1.0);

    let target_width = configured
        .width
        .min(available_width * FIT_FRACTION)
        .max(min_width);
    let target_height = configured
        .height
        .min(available_height * FIT_FRACTION)
        .max(min_height);

    window.set_min_size(Some(tauri::LogicalSize::new(min_width, min_height)))?;

    let (current_width, current_height, _) = window_css_size(window);
    let too_big = current_width > available_width + 0.5 || current_height > available_height + 0.5;
    let needs_resize =
        too_big || current_width > target_width + 0.5 || current_height > target_height + 0.5;
    if needs_resize {
        window.set_size(tauri::LogicalSize::new(target_width, target_height))?;
        // Setelah mengecil, posisi lama bisa berada di luar layar.
        let _ = window.center();
    }
    Ok(())
}

/// Ukuran jendela dari `tauri.conf.json` (logical px).
struct ConfiguredWindow {
    width: f64,
    height: f64,
    min_width: Option<f64>,
    min_height: Option<f64>,
}

fn configured_window(window: &WebviewWindow) -> ConfiguredWindow {
    let label = window.label().to_string();
    let config = window.config();
    let entry = config
        .app
        .windows
        .iter()
        .find(|entry| entry.label == label)
        .or_else(|| config.app.windows.first());

    let fallback = ConfiguredWindow {
        width: 1200.0,
        height: 820.0,
        min_width: Some(ABSOLUTE_MIN_WIDTH),
        min_height: Some(ABSOLUTE_MIN_HEIGHT),
    };
    match entry {
        Some(entry) => ConfiguredWindow {
            width: entry.width,
            height: entry.height,
            min_width: entry.min_width,
            min_height: entry.min_height,
        },
        None => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_computes_a_window_that_fits_a_small_screen() {
        // 1366x768 @150% => ruang CSS 911x512, minimum absolut 720x480.
        let available = (911.0, 512.0);
        let width = 1200.0_f64.min(available.0 * FIT_FRACTION).max(720.0);
        let height = 820.0_f64.min(available.1 * FIT_FRACTION).max(480.0);
        assert!(
            width <= available.0,
            "{width} harus muat di {}",
            available.0
        );
        assert!(
            height <= available.1,
            "{height} harus muat di {}",
            available.1
        );
        assert_eq!(width, 819.9);
        assert_eq!(height, 480.0);
    }

    #[test]
    fn minimum_never_exceeds_available_space() {
        // Layar sangat sempit: minimum harus ikut mengecil, bukan memaksa.
        let available_width = 600.0_f64;
        let min_width = 720.0_f64.min(available_width).max(1.0);
        assert_eq!(min_width, 600.0);
    }

    #[test]
    fn cached_zoom_rejects_nonsense() {
        assert_eq!(ui_scale::clamp_zoom(f64::NAN), 1.0);
        assert_eq!(ui_scale::clamp_zoom(9.0), ui_scale::MAX_ZOOM);
    }
}
