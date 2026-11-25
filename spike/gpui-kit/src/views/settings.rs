//! Layar "Pengaturan": tema, bahasa, dan tentang.
//!
//! Read-only: tidak ada file konfigurasi yang ditulis. Skala tampilan dan
//! diagnostik sengaja tidak diporting ke spike — UI ini native, bukan webview.
//!
//! Catatan bahasa: i18n global belum diporting; switch di sini hanya mengubah
//! label pada halaman Pengaturan.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::palette::{self, Palette};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Id,
    En,
}

/// Label halaman Pengaturan per bahasa.
struct L {
    title: &'static str,
    subtitle: &'static str,
    appearance: &'static str,
    light: &'static str,
    dark: &'static str,
    language: &'static str,
    about: &'static str,
    about_source: &'static str,
    about_note: &'static str,
}

impl Lang {
    fn t(self) -> L {
        match self {
            Lang::Id => L {
                title: "Pengaturan",
                subtitle: "Tema, bahasa, dan informasi aplikasi.",
                appearance: "Tampilan",
                light: "Terang",
                dark: "Gelap",
                language: "Bahasa",
                about: "Tentang",
                about_source: "Kode sumber",
                about_note: "Spike ini tidak menulis config harness; semua aksi menulis \
                             tetap lewat aplikasi Hakonfig.",
            },
            Lang::En => L {
                title: "Settings",
                subtitle: "Theme, language, and app information.",
                appearance: "Appearance",
                light: "Light",
                dark: "Dark",
                language: "Language",
                about: "About",
                about_source: "Source code",
                about_note: "This spike never writes harness config; all write actions stay \
                             in the Hakonfig app.",
            },
        }
    }
}

pub struct SettingsView {
    /// Bahasa label halaman ini (lokal, bukan i18n global).
    pub lang: Lang,
}

impl SettingsView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            lang: Lang::default(),
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: Palette = palette::palette(cx);
        let dark = palette::dark(cx);
        let l = self.lang.t();

        // --- Tampilan -------------------------------------------------------
        let appearance = card(&p)
            .child(card_title(l.appearance, &p))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(theme_pill("set-theme-light", l.light, !dark, false, &p))
                    .child(theme_pill("set-theme-dark", l.dark, dark, true, &p)),
            );

        // --- Bahasa ---------------------------------------------------------
        let language = card(&p)
            .child(card_title(l.language, &p))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(lang_pill(
                        "set-lang-id",
                        "Indonesia",
                        self.lang == Lang::Id,
                        Lang::Id,
                        &p,
                        cx,
                    ))
                    .child(lang_pill(
                        "set-lang-en",
                        "English",
                        self.lang == Lang::En,
                        Lang::En,
                        &p,
                        cx,
                    )),
            );

        // --- Tentang --------------------------------------------------------
        let about = card(&p)
            .child(card_title(l.about, &p))
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
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        p.neutral_bg,
                        p.neutral_text,
                    ))
                    .child(chip_text("MIT", p.ok_bg, p.ok_text)),
            )
            .child(div().h(px(1.)).w_full().bg(p.line))
            .child(row(
                l.about_source,
                "github.com/perogeremmer/hakonfig".to_string(),
                &p,
            ))
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(l.about_note),
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
                                    .child(l.title),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.ink_secondary)
                                    .child(l.subtitle),
                            ),
                    )
                    .child(appearance)
                    .child(language)
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

fn chip_text(text: impl Into<SharedString>, bg: Hsla, fg: Hsla) -> Div {
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

fn lang_pill(
    id: &'static str,
    label: &'static str,
    active: bool,
    lang: Lang,
    p: &Palette,
    cx: &mut Context<SettingsView>,
) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(11.))
        .py(px(5.))
        .rounded(px(8.))
        .text_size(px(12.))
        .cursor_pointer()
        .when(active, |s| {
            s.bg(p.brand_soft)
                .text_color(p.brand_text)
                .font_weight(FontWeight::SEMIBOLD)
        })
        .when(!active, |s| {
            s.bg(p.surface)
                .border_1()
                .border_color(p.line)
                .text_color(p.ink_secondary)
                .hover(|s| s.border_color(p.line_strong))
        })
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| {
            if this.lang != lang {
                this.lang = lang;
                cx.notify();
            }
        }))
}
