//! Token warna + tema aplikasi.
//!
//! Nilai di sini adalah salinan `web/styles.src.css` (tema terang & gelap), jadi
//! UI native memakai bahasa visual yang sama dengan GUI web. Modul ini juga
//! menyimpan preferensi tema sebagai global GPUI supaya view mana pun bisa
//! mengubahnya tanpa harus menembus state `App`.

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

pub const SANS: &str = "Plus Jakarta Sans";
pub const MONO: &str = "JetBrains Mono";

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Hsla,
    pub surface: Hsla,
    pub surface_muted: Hsla,
    pub line: Hsla,
    pub line_strong: Hsla,
    pub ink: Hsla,
    pub ink_secondary: Hsla,
    pub ink_muted: Hsla,
    pub brand: Hsla,
    pub brand_hover: Hsla,
    pub brand_text: Hsla,
    pub brand_soft: Hsla,
    pub on_brand: Hsla,
    pub ok_bg: Hsla,
    pub ok_text: Hsla,
    pub warn_bg: Hsla,
    pub warn_text: Hsla,
    pub neutral_bg: Hsla,
    pub neutral_text: Hsla,
    pub danger: Hsla,
}

pub fn h(hex: u32) -> Hsla {
    rgb(hex).into()
}

impl Palette {
    pub fn light() -> Self {
        Self {
            canvas: h(0xfaf7f2),
            surface: h(0xffffff),
            surface_muted: h(0xf5f1ea),
            line: h(0xe8e1d6),
            line_strong: h(0xd9d0c2),
            ink: h(0x221f1b),
            ink_secondary: h(0x6b6459),
            ink_muted: h(0x756d62),
            brand: h(0x8c3345),
            brand_hover: h(0x7a2a3b),
            brand_text: h(0x7a2a3b),
            brand_soft: h(0xf7e5e8),
            on_brand: h(0xffffff),
            ok_bg: h(0xe7f6ee),
            ok_text: h(0x0b5c3b),
            warn_bg: h(0xfff1e0),
            warn_text: h(0x8a4a08),
            neutral_bg: h(0xf1ede5),
            neutral_text: h(0x6b6459),
            danger: h(0xb42318),
        }
    }

    pub fn dark() -> Self {
        Self {
            canvas: h(0x16110f),
            surface: h(0x211b18),
            surface_muted: h(0x2a2320),
            line: h(0x352d28),
            line_strong: h(0x483d36),
            ink: h(0xf6f1eb),
            ink_secondary: h(0xb8aea4),
            ink_muted: h(0x8a8076),
            brand: h(0xa34758),
            brand_hover: h(0xb85c6c),
            brand_text: h(0xdca3ac),
            brand_soft: h(0x2e1b20),
            on_brand: h(0xffffff),
            ok_bg: h(0x16352a),
            ok_text: h(0x8ae8b8),
            warn_bg: h(0x3a2a14),
            warn_text: h(0xf0b563),
            neutral_bg: h(0x2c2622),
            neutral_text: h(0xb8aea4),
            danger: h(0xf09186),
        }
    }

    pub fn of(dark: bool) -> Self {
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

/// Preferensi tampilan. Satu sumber kebenaran untuk tema, dipakai App maupun
/// view lain (mis. Pengaturan).
#[derive(Clone, Copy, Default)]
pub struct UiPrefs {
    pub dark: bool,
}

impl Global for UiPrefs {}

pub fn dark(cx: &impl AppContext) -> bool {
    cx.read_global(|p: &UiPrefs, _| p.dark)
}

pub fn palette(cx: &impl AppContext) -> Palette {
    Palette::of(dark(cx))
}

/// Pasang palet Hakonfig ke token gpui-kit, supaya Input/Button/Switch ikut.
pub fn apply_theme(p: Palette, dark_mode: bool, cx: &mut App) {
    Theme::change(
        if dark_mode {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, move |t| {
        t.font_family = SANS.into();
        t.mono_font_family = MONO.into();
        t.font_size = px(13.);
        t.mono_font_size = px(12.);
        t.radius = px(9.);
        t.radius_lg = px(14.);

        t.background = p.canvas;
        t.foreground = p.ink;
        t.border = p.line;
        t.input = p.line;
        t.primary = p.brand;
        t.primary_hover = p.brand_hover;
        t.primary_active = p.brand_hover;
        t.primary_foreground = p.on_brand;
        t.secondary = p.surface_muted;
        t.secondary_foreground = p.ink;
        t.secondary_hover = p.line;
        t.muted = p.surface_muted;
        t.muted_foreground = p.ink_secondary;
        t.accent = p.brand_soft;
        t.accent_foreground = p.brand_text;
        t.success = p.ok_text;
        t.warning = p.warn_text;
        t.danger = p.danger;
        t.ring = p.brand;
        t.selection = p.brand_soft;
        t.popover = p.surface;
        t.popover_foreground = p.ink;

        t.sidebar = p.surface_muted;
        t.sidebar_foreground = p.ink;
        t.sidebar_accent = p.brand_soft;
        t.sidebar_accent_foreground = p.brand_text;
        t.sidebar_border = p.line;
        t.sidebar_primary = p.brand;
        t.sidebar_primary_foreground = p.on_brand;
    });
}

pub fn set_dark(want: bool, cx: &mut App) {
    if cx.try_global::<UiPrefs>().map(|p| p.dark) == Some(want) {
        return;
    }
    cx.set_global(UiPrefs { dark: want });
    apply_theme(Palette::of(want), want, cx);
}

#[allow(dead_code)] // seam: dipakai kalau toggle tema pindah ke top bar lagi
pub fn toggle_dark(cx: &mut App) {
    let next = !dark(cx);
    set_dark(next, cx);
}

/// Tombol aksi utama bergaya brand.
///
/// gpui-kit `Button::primary()` mengambil warna dari token `button_primary`
/// yang dibangun dari tema bawaan, bukan dari `ThemeColor` yang kita timpa —
/// hasilnya tombol tampil gelap, bukan maroon. Elemen ini memakai token palet
/// langsung supaya warnanya pasti sama dengan sisa UI.
pub fn brand_button(p: &Palette, id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(11.))
        .py(px(5.))
        .rounded(px(8.))
        .bg(p.brand)
        .text_color(p.on_brand)
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|s| s.bg(p.brand_hover))
        .child(label.to_string())
}

/// Tombol sekunder (garis tipis) dengan token palet yang sama.
#[allow(dead_code)] // seam untuk view lain
pub fn soft_button(p: &Palette, id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(11.))
        .py(px(5.))
        .rounded(px(8.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .text_color(p.ink_secondary)
        .text_size(px(12.))
        .cursor_pointer()
        .hover(|s| s.bg(p.surface_muted).border_color(p.line_strong))
        .child(label.to_string())
}

/// Kotak teks dengan bentuk yang sama di semua layar.
///
/// Default gpui-kit memakai tinggi 24 px (`.small()`) dan latar
/// `theme.background` — di tema terang itu krem, jadi kotak menyatu dengan
/// kanvas dan tidak terbaca sebagai input. Di sini latarnya putih kartu,
/// garisnya `p.line`, tingginya 36 px, dan ikon kiri memberi petunjuk isi.
pub fn text_input(p: &Palette, state: &Entity<InputState>, icon: IconName) -> Input {
    Input::new(state)
        .h(px(36.))
        .text_size(px(12.5))
        .rounded(px(9.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .prefix(
            div()
                .pr(px(2.))
                .text_size(px(13.))
                .text_color(p.ink_muted)
                .child(icon),
        )
}
