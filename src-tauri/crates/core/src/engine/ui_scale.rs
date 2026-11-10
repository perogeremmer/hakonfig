//! Skala UI: satu angka yang membuat seluruh antarmuka terbaca di layar apa pun.
//!
//! Kenapa zoom, bukan sekadar ukuran font: frontend memakai ratusan nilai px
//! hardcoded (`text-[13px]`, `h-[148px]`, `max-w-[1008px]`). Mengubah ukuran
//! font saja tidak ikut membesarkan tombol, padding, atau ikon. Zoom webview
//! menskalakan semuanya sekaligus, jadi satu keputusan di sini memperbaiki
//! seluruh tampilan.
//!
//! Perhitungannya murni (tanpa Tauri) supaya bisa diuji tanpa membuka jendela.

use serde::Serialize;

/// Batas bawah/batas atas zoom. Di bawah 0.8 teks 11 px menjadi tidak terbaca;
/// di atas 2.0 tata letak 4 kolom tidak lagi muat di layar mana pun.
pub const MIN_ZOOM: f64 = 0.8;
pub const MAX_ZOOM: f64 = 2.0;

/// Rasio skala monitor terhadap `devicePixelRatio` yang dianggap cocok.
///
/// Di bawah ini tidak ada yang perlu ditambal: 1.0 berarti webview sudah
/// dirender sebesar skala monitor. Ambang 1.1 memberi ruang untuk pembulatan
/// skala pecahan (1.25/1.5) yang sering dilaporkan sebagai 1.2499.
const COMPATIBLE_RATIO: f64 = 1.1;

/// Alasan satu keputusan zoom diambil — dipakai UI untuk menjelaskan angka,
/// bukan sekadar menampilkannya.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleReason {
    /// Skala monitor dan webview sepakat: tidak ada yang perlu ditambal.
    Matches,
    /// Webview melaporkan `devicePixelRatio` lebih kecil dari skala monitor.
    /// Khas WebKitGTK di Linux: UI dirender 1x di layar 2x.
    PatchesWebkit,
    /// Pengguna memilih sendiri.
    Manual,
    /// Pilihan pengguna di luar batas, dipotong ke [`MIN_ZOOM`]/[`MAX_ZOOM`].
    Clamped,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaleDecision {
    pub zoom: f64,
    pub reason: ScaleReason,
}

impl ScaleDecision {
    /// Apakah keputusan ini mengubah tampilan dari 100%.
    pub fn is_scaled(&self) -> bool {
        (self.zoom - 1.0).abs() > 0.001
    }
}

/// Tentukan zoom dari preferensi pengguna dan dua skala yang dilaporkan sistem.
///
/// `preference` adalah `None` untuk mode auto. `monitor_scale` berasal dari OS
/// (`Window::current_monitor().scale_factor()`), `device_pixel_ratio` dari
/// webview.
pub fn resolve(
    preference: Option<f64>,
    monitor_scale: f64,
    device_pixel_ratio: f64,
) -> ScaleDecision {
    // NaN/inf dari driver tidak boleh membuat seluruh UI tak terukur.
    if let Some(wanted) = preference.filter(|value| value.is_finite() && *value > 0.0) {
        let clamped = clamp_zoom(wanted);
        let reason = if (clamped - wanted).abs() < 1e-9 {
            ScaleReason::Manual
        } else {
            ScaleReason::Clamped
        };
        return ScaleDecision {
            zoom: round2(clamped),
            reason,
        };
    }

    let ratio = scale_ratio(monitor_scale, device_pixel_ratio);
    if ratio >= COMPATIBLE_RATIO {
        ScaleDecision {
            zoom: round2(clamp_zoom(ratio)),
            reason: ScaleReason::PatchesWebkit,
        }
    } else {
        ScaleDecision {
            zoom: 1.0,
            reason: ScaleReason::Matches,
        }
    }
}

/// Berapa kali UI harus diperbesar agar sama dengan skala monitor.
///
/// Nilai tak masuk akal (0, negatif, NaN) dianggap 1.0: lebih baik tidak
/// menambal apa pun daripada menebak.
fn scale_ratio(monitor_scale: f64, device_pixel_ratio: f64) -> f64 {
    let sane = |value: f64| value.is_finite() && value > 0.0;
    if !sane(monitor_scale) || !sane(device_pixel_ratio) {
        return 1.0;
    }
    monitor_scale / device_pixel_ratio
}

pub fn clamp_zoom(zoom: f64) -> f64 {
    if !zoom.is_finite() {
        return 1.0;
    }
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}

/// Bulatkan ke dua desimal: `1.4999999` di UI hanya membingungkan.
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_scaling_needs_no_patch() {
        // macOS/Windows/Wayland normal: monitor 2x, webview 2x.
        let decision = resolve(None, 2.0, 2.0);
        assert_eq!(decision.zoom, 1.0);
        assert_eq!(decision.reason, ScaleReason::Matches);
        assert!(!decision.is_scaled());

        // Skala pecahan 1.25 juga tidak boleh ditambal.
        assert_eq!(resolve(None, 1.25, 1.25).zoom, 1.0);
        assert_eq!(resolve(None, 1.0, 1.0).reason, ScaleReason::Matches);
    }

    #[test]
    fn webkit_reporting_1x_on_a_2x_screen_gets_patched() {
        // Kasus WebKitGTK: UI dirender separuh dari seharusnya.
        let decision = resolve(None, 2.0, 1.0);
        assert_eq!(decision.zoom, 2.0);
        assert_eq!(decision.reason, ScaleReason::PatchesWebkit);
        assert!(decision.is_scaled());

        assert_eq!(resolve(None, 1.5, 1.0).zoom, 1.5);
        assert_eq!(resolve(None, 1.25, 1.25).zoom, 1.0);
    }

    #[test]
    fn manual_choice_always_wins() {
        let decision = resolve(Some(1.25), 2.0, 2.0);
        assert_eq!(decision.zoom, 1.25);
        assert_eq!(decision.reason, ScaleReason::Manual);

        // Pengguna boleh mengecilkan walau monitor 1x.
        assert_eq!(resolve(Some(0.9), 1.0, 1.0).zoom, 0.9);
    }

    #[test]
    fn manual_choice_is_clamped_and_says_so() {
        let too_big = resolve(Some(4.0), 1.0, 1.0);
        assert_eq!(too_big.zoom, MAX_ZOOM);
        assert_eq!(too_big.reason, ScaleReason::Clamped);

        let too_small = resolve(Some(0.2), 1.0, 1.0);
        assert_eq!(too_small.zoom, MIN_ZOOM);
        assert_eq!(too_small.reason, ScaleReason::Clamped);
    }

    #[test]
    fn nonsense_input_falls_back_to_100_percent() {
        for (monitor, dpr) in [
            (0.0, 1.0),
            (2.0, 0.0),
            (f64::NAN, 1.0),
            (2.0, f64::INFINITY),
        ] {
            let decision = resolve(None, monitor, dpr);
            assert_eq!(decision.zoom, 1.0, "monitor={monitor} dpr={dpr}");
            assert_eq!(decision.reason, ScaleReason::Matches);
        }
        // Preferensi NaN diperlakukan sebagai auto, bukan 0.8.
        assert_eq!(resolve(Some(f64::NAN), 2.0, 2.0).zoom, 1.0);
        assert_eq!(resolve(Some(0.0), 2.0, 2.0).zoom, 1.0);
    }

    #[test]
    fn zoom_is_rounded_for_display() {
        // Rasio yang muncul dari skala pecahan tidak boleh jadi 1.2499999999.
        let decision = resolve(None, 2.4999, 1.0);
        assert_eq!(decision.zoom, 2.0); // dipotong batas atas
        assert_eq!(resolve(Some(1.333_333), 1.0, 1.0).zoom, 1.33);
    }

    #[test]
    fn patched_ratio_respects_upper_bound() {
        // Monitor 4x dengan webview 1x: tetap 2.0, bukan 4.0.
        let decision = resolve(None, 4.0, 1.0);
        assert_eq!(decision.zoom, MAX_ZOOM);
        assert_eq!(decision.reason, ScaleReason::PatchesWebkit);
    }
}
