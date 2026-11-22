//! Placeholder view "Pengaturan" (tema, bahasa, skala, diagnostik).
//!
//! Status: BELUM DIPORTING. Lihat `FEATURES.md` butir F11–F15.

use crate::palette::Palette;
use gpui_kit::component::v_flex;
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct SettingsView {
    pub harness: String,
    /// Angka hasil load terakhir; diisi `App::sync_views`.
    pub metrics: crate::data::Metrics,
}

impl SettingsView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            harness: String::new(),
            metrics: crate::data::Metrics::default(),
        }
    }

    pub fn set_metrics(&mut self, metrics: crate::data::Metrics, cx: &mut Context<Self>) {
        self.metrics = metrics;
        cx.notify();
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness != harness {
            self.harness = harness;
            cx.notify();
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: Palette = crate::palette::palette(cx);
        v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(20.))
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.ink_muted)
                    .child("PENGATURAN"),
            )
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .child("Tema, bahasa, skala"),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(p.ink_secondary)
                    .child("Belum diimplementasikan di spike ini."),
            )
    }
}
