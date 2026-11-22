//! Placeholder view "Tambah provider" (3 langkah: preset → key → model).
//!
//! Status: BELUM DIPORTING. Lihat `FEATURES.md` butir F1–F6 dan
//! `PROGRESS.md` untuk siapa yang mengerjakan.

use crate::palette::Palette;
use gpui_kit::component::v_flex;
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct AddView {
    pub harness: String,
}

impl AddView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            harness: String::new(),
        }
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness != harness {
            self.harness = harness;
            cx.notify();
        }
    }
}

impl Render for AddView {
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
                    .child("TAMBAH PROVIDER"),
            )
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .child("Form tiga langkah"),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .text_color(p.ink_secondary)
                    .child(format!(
                        "Target: {}. Pilih preset, tempel key, lalu pilih model. \
                         Belum diimplementasikan di spike ini.",
                        if self.harness.is_empty() {
                            "belum ada harness"
                        } else {
                            self.harness.as_str()
                        }
                    )),
            )
    }
}
