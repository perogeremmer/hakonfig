//! Layar "Pengaturan": tema dan tentang.
//!
//! Read-only: tidak ada file konfigurasi yang ditulis. Skala tampilan, bahasa
//! lokal, dan diagnostik sengaja tidak diporting ke spike — UI ini native,
//! bukan webview, jadi zoom CSS dan i18n parsial hanya membingungkan.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::palette::{self, Palette};

pub struct SettingsView;

impl SettingsView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: Palette = palette::palette(cx);
        let dark = palette::dark(cx);

        // --- Tampilan -------------------------------------------------------
        let appearance = card(&p)
            .child(card_title("TAMPILAN", &p))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(theme_pill("set-theme-light", "Terang", !dark, false, &p))
                    .child(theme_pill("set-theme-dark", "Gelap", dark, true, &p)),
            );

        // --- Tentang --------------------------------------------------------
        let about = card(&p)
            .child(card_title("TENTANG", &p))
            .child(
                h_flex()
                    .gap(px(11.))
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(34.))
                            .flex_shrink_0()
                            .rounded(px(10.))
                            .bg(p.brand_soft)
                            .text_color(p.brand_text)
                            .text_size(px(16.))
                            .font_weight(FontWeight::BOLD)
                            .child("H"),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Hakonfig"),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .font_family(palette::MONO)
                                    .text_color(p.ink_muted)
                                    .child("spike GPUI Kit · read-only"),
                            ),
                    )
                    .child(chip_text(
                        &p,
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        p.neutral_bg,
                        p.neutral_text,
                    ))
                    .child(chip_text(&p, "MIT", p.ok_bg, p.ok_text)),
            )
            .child(div().h(px(1.)).w_full().bg(p.line))
            .child(row(
                "Kode sumber",
                "github.com/perogeremmer/hakonfig".to_string(),
                &p,
            ))
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(
                        "Spike ini tidak menulis config harness; semua aksi menulis \
                         tetap lewat aplikasi Hakonfig.",
                    ),
            );

        v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(
                v_flex()
                    .id("set-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(20.))
                    .gap(px(12.))
                    .child(
                        v_flex()
                            .gap(px(3.))
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Pengaturan"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.ink_secondary)
                                    .child("Tema dan informasi aplikasi."),
                            ),
                    )
                    .child(appearance)
                    .child(about),
            )
    }
}

/// Kartu bagian: bg surface, border line, rounded 12, padding 14.
fn card(p: &Palette) -> Div {
    div()
        .w_full()
        .flex_shrink_0()
        .p(px(14.))
        .gap(px(10.))
        .rounded(px(12.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .shadow_2xs()
}

fn card_title(text: &str, p: &Palette) -> Div {
    div()
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink_muted)
        .child(text.to_uppercase())
}

fn row_label<'a>(label: &'a str, p: &Palette) -> Div {
    div()
        .text_size(px(12.))
        .text_color(p.ink_secondary)
        .child(label.to_string())
}

/// Baris label kiri, nilai mono kanan.
fn row(label: &str, value: String, p: &Palette) -> Div {
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .gap(px(12.))
        .child(row_label(label, p))
        .child(
            div()
                .truncate()
                .text_size(px(11.5))
                .font_family(palette::MONO)
                .text_color(p.ink)
                .child(value),
        )
}

fn chip_text(_p: &Palette, text: impl Into<SharedString>, bg: Hsla, fg: Hsla) -> Div {
    let text: SharedString = text.into();
    div()
        .px(px(7.))
        .py(px(1.))
        .rounded_full()
        .bg(bg)
        .text_color(fg)
        .text_size(px(10.))
        .font_family(palette::MONO)
        .child(text)
}

/// Pil tema. Sengaja memakai closure mentah (bukan `cx.listener`):
/// `palette::set_dark` butuh `&mut App`, dan handler gpui-kit menerima
/// `&mut App` — `Theme::update` me-refresh semua window.
fn theme_pill(
    id: &'static str,
    label: &'static str,
    active: bool,
    want_dark: bool,
    p: &Palette,
) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.))
        .py(px(6.))
        .rounded_full()
        .cursor_pointer()
        .text_size(px(12.))
        .when(active, |s| {
            s.bg(p.brand_soft)
                .text_color(p.brand_text)
                .font_weight(FontWeight::SEMIBOLD)
        })
        .when(!active, |s| {
            s.bg(p.canvas)
                .border_1()
                .border_color(p.line)
                .text_color(p.ink_secondary)
        })
        .child(label)
        .on_click(move |_, _, cx| palette::set_dark(want_dark, cx))
}
