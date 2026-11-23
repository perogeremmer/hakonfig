//! Spike Hakonfig di atas GPUI Kit.
//!
//! Struktur:
//! - `app`      : state + shell UI (top bar, sidebar, navigasi)
//! - `data`     : pembacaan harness lewat `hakonfig_core::api`
//! - `palette`  : token warna/tema (salinan `web/styles.src.css`)
//! - `views/*`  : satu modul per layar
//!
//! Spike ini read-only: tidak ada file konfigurasi yang ditulis.

mod app;
mod data;
mod logos;
mod net;
mod palette;
mod state;
mod views;

use std::borrow::Cow;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;

// Font dibundel, bukan diambil dari sistem: aplikasi desktop harus tampil sama
// di mesin yang belum pernah memasang Plus Jakarta Sans.
const FONT_SANS: &[u8] = include_bytes!("../assets/fonts/PlusJakartaSans-Variable.ttf");
const FONT_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const FONT_MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

fn main() {
    app::PROCESS_START.get_or_init(Instant::now);

    let app = gpui_kit::application().with_assets(logos::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(FONT_SANS as &[u8]),
                Cow::Borrowed(FONT_MONO as &[u8]),
                Cow::Borrowed(FONT_MONO_BOLD as &[u8]),
            ])
            .ok();
        // SPIKE_DARK=1 memulai aplikasi dalam tema gelap: dipakai skrip
        // screenshot, tidak mengubah perilaku normal.
        let dark = std::env::var_os("SPIKE_DARK").is_some();
        cx.set_global(palette::UiPrefs { dark });
        palette::apply_theme(palette::Palette::of(dark), dark, cx);

        let bounds = WindowBounds::centered(size(px(1200.), px(820.)), cx);
        let options = WindowOptions {
            window_bounds: Some(bounds),
            titlebar: Some(TitlebarOptions {
                title: Some("Hakonfig — spike gpui-kit".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| app::App::new(window, cx)))
            .expect("gagal membuka jendela");
    });
}
