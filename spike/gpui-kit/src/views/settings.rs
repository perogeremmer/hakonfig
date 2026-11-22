//! Layar "Pengaturan": tampilan (terang/gelap + skala OS), bahasa (lokal untuk
//! halaman ini), diagnostik (waktu baca config vs probe versi), dan tentang.
//!
//! Read-only: tidak ada file konfigurasi yang ditulis. Butir FEATURES.md:
//! F29 (tema), F30 (bahasa, subset), F31 (skala), F32 (diagnostik), F33
//! (tentang). i18n global (F38) belum diporting — switch bahasa hanya
//! mengubah label di halaman ini.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::palette::{self, Palette};

/// Bahasa label halaman ini. Sumber istilah: `web/i18n/id.js` 268–292 dan
/// `web/i18n/en.js` 281–305 (title, language, appearance, light, dark,
/// scale, about, version, repo, close).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Id,
    En,
}

/// Kumpulan label per bahasa.
struct L {
    title: &'static str,
    subtitle: &'static str,
    appearance: &'static str,
    light: &'static str,
    dark: &'static str,
    scale: &'static str,
    scale_note: &'static str,
    language: &'static str,
    lang_note: &'static str,
    diagnostics: &'static str,
    diag_config: &'static str,
    diag_ready: &'static str,
    diag_counts: &'static str,
    diag_version: &'static str,
    diag_explain: &'static str,
    loading: &'static str,
    about: &'static str,
    about_version: &'static str,
    about_spike_note: &'static str,
    about_license: &'static str,
    about_repo: &'static str,
    about_readonly: &'static str,
}

impl Lang {
    fn t(self) -> L {
        match self {
            Lang::Id => L {
                title: "Pengaturan",
                subtitle: "Tema, bahasa, dan diagnostik spike GPUI Kit.",
                appearance: "Tampilan",
                light: "Terang",
                dark: "Gelap",
                scale: "Skala tampilan",
                scale_note: "zoom CSS tidak relevan di UI native",
                language: "Bahasa",
                lang_note: "i18n global (F38) belum diporting — switch ini hanya mengubah label di halaman ini.",
                diagnostics: "Diagnostik",
                diag_config: "Baca config",
                diag_ready: "Data siap",
                diag_counts: "Jumlah",
                diag_version: "Versi tool harness aktif",
                diag_explain: "baca {n} config = ±2 ms, probe versi tool = ±1.000 ms (omp 604 ms), karena itu versi dimuat di background",
                loading: "memuat…",
                about: "Tentang",
                about_version: "Versi",
                about_spike_note: "Ini versi spike GPUI Kit; produk Hakonfig yang sebenarnya 0.7.3.",
                about_license: "Lisensi MIT",
                about_repo: "Kode sumber",
                about_readonly: "read-only: tidak ada file konfigurasi yang ditulis",
            },
            Lang::En => L {
                title: "Settings",
                subtitle: "Theme, language, and GPUI Kit spike diagnostics.",
                appearance: "Appearance",
                light: "Light",
                dark: "Dark",
                scale: "Display scale",
                scale_note: "CSS zoom is irrelevant in a native UI",
                language: "Language",
                lang_note: "Global i18n (F38) is not ported — this switch only changes labels on this page.",
                diagnostics: "Diagnostics",
                diag_config: "Config read",
                diag_ready: "Data ready",
                diag_counts: "Counts",
                diag_version: "Active harness tool version",
                diag_explain: "reading {n} configs = ±2 ms, tool version probe = ±1,000 ms (omp 604 ms), so versions load in the background",
                loading: "loading…",
                about: "About",
                about_version: "Version",
                about_spike_note: "This is the GPUI Kit spike version; the actual Hakonfig product is 0.7.3.",
                about_license: "MIT license",
                about_repo: "Source code",
                about_readonly: "read-only: no configuration files are written",
            },
        }
    }
}

pub struct SettingsView {
    pub harness: String,
    /// Angka hasil load terakhir; diisi `App::sync_views`.
    pub metrics: crate::data::Metrics,
    /// Bahasa label halaman ini (lokal, bukan i18n global).
    pub lang: Lang,
    /// Versi tool harness aktif; `None` = sedang dimuat di background.
    pub version: Option<String>,
}

impl SettingsView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            harness: String::new(),
            metrics: crate::data::Metrics::default(),
            lang: Lang::default(),
            version: None,
        }
    }

    pub fn set_metrics(&mut self, metrics: crate::data::Metrics, cx: &mut Context<Self>) {
        self.metrics = metrics;
        cx.notify();
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness != harness {
            self.harness = harness.clone();
            // Probe versi tool itu lambat (ratusan ms) — jangan pernah di
            // render; jalankan di background thread dan simpan hasilnya.
            self.version = None;
            cx.spawn(async move |this, cx| {
                let v = cx
                    .background_spawn(async move {
                        hakonfig_core::api::harness_version(&harness)
                    })
                    .await;
                this.update(cx, |this, cx| {
                    this.version = Some(v.unwrap_or_else(|| "—".to_string()));
                    cx.notify();
                })
                .ok();
            })
            .detach();
            cx.notify();
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: Palette = palette::palette(cx);
        let dark = palette::dark(cx);
        let l = self.lang.t();
        let scale = window.scale_factor();

        // --- Tampilan -------------------------------------------------------
        let appearance = card(&p)
            .child(card_title(l.appearance, &p))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(theme_pill("set-theme-light", l.light, !dark, false, &p))
                    .child(theme_pill("set-theme-dark", l.dark, dark, true, &p)),
            )
            .child(div().h(px(1.)).w_full().bg(p.line))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(row_label(l.scale, &p))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_family(palette::MONO)
                            .text_color(p.ink_secondary)
                            .child(format!(
                                "{}: {:.2}× · {}",
                                l.scale, scale, l.scale_note
                            )),
                    ),
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
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(l.lang_note),
            );

        // --- Diagnostik -----------------------------------------------------
        let m = self.metrics;
        let ready = match m.data_ready_ms {
            Some(ms) => format!("{} ms", ms),
            None => l.loading.to_string(),
        };
        let version_value = match &self.version {
            Some(v) => v.clone(),
            None => l.loading.to_string(),
        };
        let diagnostics = card(&p)
            .child(card_title(l.diagnostics, &p))
            .child(row(
                l.diag_config,
                format!("{} ms", m.load_ms),
                &p,
            ))
            .child(row(l.diag_ready, ready, &p))
            .child(row(
                l.diag_counts,
                format!(
                    "{} harness · {} provider · {} model",
                    m.harnesses, m.providers, m.models
                ),
                &p,
            ))
            .child(row(
                l.diag_version,
                if self.harness.is_empty() {
                    "—".to_string()
                } else {
                    format!("{} {}", self.harness, version_value)
                },
                &p,
            ))
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(l.diag_explain.replace("{n}", &m.harnesses.to_string())),
            );

        // --- Tentang --------------------------------------------------------
        let about = card(&p)
            .child(card_title(l.about, &p))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::BOLD)
                            .child("Hakonfig — spike GPUI Kit"),
                    )
                    .child(
                        div()
                            .px(px(7.))
                            .py(px(1.))
                            .rounded_full()
                            .bg(p.neutral_bg)
                            .text_color(p.neutral_text)
                            .text_size(px(10.5))
                            .font_family(palette::MONO)
                            .child(format!(
                                "{} {}",
                                l.about_version,
                                env!("CARGO_PKG_VERSION")
                            )),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(p.ink_secondary)
                    .child(l.about_spike_note),
            )
            .child(div().h(px(1.)).w_full().bg(p.line))
            .child(row(
                l.about_license,
                "MIT".to_string(),
                &p,
            ))
            .child(row(
                l.about_repo,
                "https://github.com/perogeremmer/hakonfig".to_string(),
                &p,
            ))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(4.))
                            .bg(p.ok_bg)
                            .text_color(p.ok_text)
                            .text_size(px(9.5))
                            .child(l.about_readonly),
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
                    .child(diagnostics)
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

/// Baris diagnostik: label kiri, nilai mono kanan.
fn row(label: &str, value: String, p: &Palette) -> Div {
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .gap(px(12.))
        .child(row_label(label, p))
        .child(
            div()
                .text_size(px(11.5))
                .font_family(palette::MONO)
                .text_color(p.ink)
                .truncate()
                .child(value),
        )
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
        .on_click(move |_ev, _window, cx: &mut App| {
            palette::set_dark(want_dark, cx);
        })
}

/// Pil bahasa. State ada di view, jadi `cx.listener` cukup.
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
