//! Layar Providers: header harness, kartu default model, grid kartu provider,
//! pencarian, dan footer status. Semua data dari `App` (yang membacanya lewat
//! `hakonfig_core::api`).

use gpui_kit::component::{
    Sizable,
    button::Button,
    h_flex,
    switch::Switch,
    v_flex,
    WindowExt as _,
};
use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::engine::model::ProviderSummary;

use crate::data::{monogram, HarnessEntry};
use crate::app::{App, Nav};
use crate::palette::{self, Palette};

/// Filter cepat di atas grid provider.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    WithKey,
    NoKey,
    Disabled,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "Semua",
            Filter::WithKey => "Punya key",
            Filter::NoKey => "Tanpa key",
            Filter::Disabled => "Nonaktif",
        }
    }

    pub fn matches(self, provider: &ProviderSummary) -> bool {
        match self {
            Filter::All => true,
            Filter::WithKey => provider.has_key,
            Filter::NoKey => !provider.has_key,
            Filter::Disabled => !provider.enabled,
        }
    }
}

pub fn render(app: &App, p: &Palette, cx: &mut Context<App>) -> impl IntoElement {
    let mut column = v_flex().w_full().gap(px(12.));

    if app.entries.is_empty() {
        return v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p(px(20.))
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .w(px(420.))
                    .gap(px(8.))
                    .items_center()
                    .p(px(26.))
                    .rounded(px(16.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(44.))
                            .rounded(px(12.))
                            .bg(p.brand_soft)
                            .text_color(p.brand_text)
                            .text_size(px(20.))
                            .child(IconName::SquareTerminal),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::BOLD)
                            .child("Belum ada harness"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.ink_secondary)
                            .text_center()
                            .child(
                                "Hakonfig mengatur provider untuk tool seperti Pi, OMP, \
                                 OpenCode, dan Hermes. Tambahkan satu untuk mulai.",
                            ),
                    )
                    .child(
                        div()
                            .pt(px(4.))
                            .child(
                                palette::brand_button(
                                    p,
                                    "empty-add-harness",
                                    "Tambah harness baru",
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.open_picker(cx)
                                })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_muted)
                            .child("Tool yang sudah punya config terdeteksi otomatis."),
                    ),
            );
    }

    let Some(entry) = app.active() else {
        return v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p(px(20.))
            .child(
                column
                    .child(div().text_size(px(10.)).text_color(p.ink_muted).child("MEMUAT"))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Membaca konfigurasi harness…"),
                    ),
            );
    };

    column = column.child(harness_header(app, p, entry, cx));

    if let Some(err) = &entry.error {
        column = column.child(
            div()
                .p(px(12.))
                .rounded(px(10.))
                .bg(p.warn_bg)
                .text_color(p.warn_text)
                .text_size(px(12.))
                .child(err.clone()),
        );
    }

    column = column.child(config_files_card(entry, p, cx));
    column = column.child(actions_card(app, p, cx));

    if !app.notices.is_empty() {
        let mut list = v_flex().gap(px(4.));
        for notice in &app.notices {
            list = list.child(
                h_flex()
                    .gap(px(8.))
                    .items_start()
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .mt(px(5.))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(if notice.destructive {
                                p.danger
                            } else {
                                p.warn_text
                            }),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.warn_text)
                                    .child(notice.message.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .font_family(palette::MONO)
                                    .text_color(p.ink_muted)
                                    .child(format!(
                                        "{} · {}",
                                        notice.provider,
                                        notice.fix.clone().unwrap_or_else(|| {
                                            "perlu dibereskan manual".into()
                                        })
                                    )),
                            ),
                    ),
            );
        }
        column = column.child(
            v_flex()
                .gap(px(4.))
                .p(px(10.))
                .rounded(px(12.))
                .bg(p.warn_bg)
                .border_1()
                .border_color(p.warn_text.opacity(0.25))
                .child(
                    h_flex()
                        .gap(px(6.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(9.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.warn_text)
                                .child(format!("{} TEMUAN", app.notices.len())),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(px(9.5))
                                .text_color(p.warn_text)
                                .child("aksi perbaikan belum aktif di spike"),
                        ),
                )
                .child(list),
        );
    }

    if let Some(provider) = entry.snapshot.as_ref().and_then(|s| s.default_provider.clone()) {
        if let Some(model) = entry.snapshot.as_ref().and_then(|s| s.default_model.clone()) {
            column = column.child(default_model_card(p, &provider, &model));
        }
    }

    let providers = app.visible_providers();
    column = column.child(
        h_flex()
            .gap(px(8.))
            .items_center()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::BOLD)
                    .child("Providers"),
            )
            .child(
                div()
                    .px(px(7.))
                    .py(px(1.))
                    .rounded_full()
                    .bg(p.neutral_bg)
                    .text_color(p.neutral_text)
                    .text_size(px(10.5))
                    .child(entry.provider_count.to_string()),
            )
            .child(div().flex_1())
            .child(
                div().w(px(280.)).child(palette::text_input(
                    p,
                    &app.query,
                    IconName::Search,
                )),
            ),
    );

    let mut chips = h_flex().gap(px(6.)).items_center().pb(px(2.));
    for filter in [Filter::All, Filter::WithKey, Filter::NoKey, Filter::Disabled] {
        let active = app.filter == filter;
        chips = chips.child(
            div()
                .id(ElementId::Name(
                    format!("filter-{}", filter.label()).into(),
                ))
                .px(px(9.))
                .py(px(3.))
                .rounded_full()
                .text_size(px(11.))
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
                .child(filter.label())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.filter = filter;
                    cx.notify();
                })),
        );
    }
    column = column.child(chips);

    let mut grid = h_flex().flex_wrap().gap(px(12.)).items_start();
    if providers.is_empty() && entry.error.is_none() {
        grid = grid.child(
            v_flex()
                .w_full()
                .py(px(30.))
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if app.query_text.trim().is_empty() {
                            "Belum ada provider di harness ini."
                        } else {
                            "Tidak ada provider yang cocok."
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(p.ink_muted)
                        .child(if app.query_text.trim().is_empty() {
                            "Tambahkan provider lewat form tiga langkah."
                        } else {
                            "Coba kata kunci lain, misalnya nama provider."
                        }),
                ),
        );
    }
    for (ix, provider) in providers {
        grid = grid.child(provider_card(app, p, ix, provider, cx));
    }
    column = column.child(grid);

    column = column.child(
        h_flex()
            .flex_shrink_0()
            .pt(px(4.))
            .gap(px(6.))
            .items_center()
            .text_size(px(10.5))
            .text_color(p.ink_muted)
            .child("Spike read-only — tidak ada file yang ditulis.")
            .when(app.dirty > 0, |s| {
                s.child(
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded_full()
                        .bg(p.warn_bg)
                        .text_color(p.warn_text)
                        .child(format!("{} perubahan lokal", app.dirty)),
                )
            }),
    );

    // Seluruh halaman yang menggulir. Sebelumnya hanya grid provider yang
    // punya `overflow_y_scroll`, sehingga pada harness besar (pi: 5 provider,
    // 322 model) kartu di atas memakan seluruh tinggi dan grid-nya menyusut
    // sampai tak bisa digulir sama sekali.
    v_flex()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .p(px(20.))
        .child(
            v_flex()
                .id("providers-scroll")
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_y_scroll()
                .child(column),
        )
}

/// Kotak aksi: tiga layar lain dibuka dari sini, bukan dari sidebar atau tab
/// di atas. Sidebar tetap khusus memilih harness.
fn actions_card(_app: &App, p: &Palette, cx: &mut Context<App>) -> impl IntoElement {
    let rows: [(Nav, IconName, &'static str, &'static str); 3] = [
        (
            Nav::Add,
            IconName::Plus,
            "Tambah provider",
            "preset → key → model → pratinjau diff",
        ),
        (
            Nav::Diff,
            IconName::FileText,
            "Diff",
            "pratinjau perubahan per berkas, read-only",
        ),
        (
            Nav::History,
            IconName::Undo,
            "Riwayat",
            "daftar backup config beserta waktunya",
        ),
    ];

    let mut card = v_flex()
        .gap(px(2.))
        .p(px(8.))
        .rounded(px(12.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .child(
            div()
                .px(px(8.))
                .pt(px(4.))
                .pb(px(6.))
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.ink_muted)
                .child("AKSI"),
        );

    for (nav, icon, label, hint) in rows {
        card = card.child(
            h_flex()
                .id(ElementId::Name(format!("act-{}", nav.label()).into()))
                .gap(px(9.))
                .px(px(8.))
                .py(px(7.))
                .rounded(px(9.))
                .items_center()
                .cursor_pointer()
                .hover(|s| s.bg(p.canvas))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.))
                        .flex_shrink_0()
                        .rounded(px(7.))
                        .bg(p.brand_soft)
                        .text_color(p.brand_text)
                        .text_size(px(13.))
                        .child(icon),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_size(px(12.5)).child(label))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(10.5))
                                .text_color(p.ink_muted)
                                .child(hint),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.ink_muted)
                        .child("›"),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_nav(nav, cx))),
        );
    }

    card
}

fn harness_header(
    app: &App,
    p: &Palette,
    entry: &HarnessEntry,
    cx: &mut Context<App>,
) -> impl IntoElement {
    v_flex()
        .gap(px(3.))
        .child(
            h_flex()
                .gap(px(12.))
                .items_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(36.))
                        .rounded(px(10.))
                        .bg(p.brand_soft)
                        .text_color(p.brand_text)
                        .text_size(px(13.))
                        .font_weight(FontWeight::BOLD)
                        .child(monogram(&entry.name, &entry.id)),
                )
                .child(
                    h_flex()
                        .gap(px(8.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(FontWeight::BOLD)
                                .child(entry.name.clone()),
                        )
                        .when_some(entry.version.clone(), |s, v| {
                            s.child(
                                div()
                                    .px(px(7.))
                                    .py(px(1.))
                                    .rounded_full()
                                    .bg(p.neutral_bg)
                                    .text_color(p.neutral_text)
                                    .text_size(px(10.5))
                                    .font_family(palette::MONO)
                                    .child(v),
                            )
                        })
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.ink_muted)
                                .child(format!(
                                    "{} provider · {} model",
                                    entry.provider_count,
                                    entry.model_count()
                                )),
                        )
                        .when(entry.experimental, |s| {
                            s.child(
                                div()
                                    .px(px(7.))
                                    .py(px(1.))
                                    .rounded_full()
                                    .bg(p.warn_bg)
                                    .text_color(p.warn_text)
                                    .text_size(px(10.5))
                                    .child("eksperimental"),
                            )
                        }),
                )
                .child(div().flex_1())
                .child(
                    Button::new("refresh")
                        .small()
                        .label("Refresh")
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                ),
        )
        .child(
            div()
                .pl(px(48.))
                .text_size(px(11.))
                .font_family(palette::MONO)
                .text_color(p.ink_muted)
                .truncate()
                .child(entry.config_path.clone()),
        )
        .when(app.load_ms > 0, |s| {
            s.child(
                div()
                    .pl(px(48.))
                    .text_size(px(10.))
                    .text_color(p.ink_muted)
                    .child(format!(
                        "{} ms baca config · versi {} ms",
                        entry.timings.0, entry.timings.1
                    )),
            )
        })
}

/// Berkas config yang dikelola harness ini, dengan tombol buka folder.
///
/// Membuka folder memakai perintah OS (`xdg-open`/`open`/`explorer`) — sama
/// seperti tombol "open folder" di GUI web, tanpa menambah dependency.
fn config_files_card(
    entry: &HarnessEntry,
    p: &Palette,
    _cx: &mut Context<App>,
) -> impl IntoElement {
    let files = entry
        .snapshot
        .as_ref()
        .map(|s| s.files.clone())
        .unwrap_or_default();

    let mut rows = v_flex().gap(px(2.));
    for file in files {
        let dir = std::path::Path::new(&file.path)
            .parent()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| file.path.clone());
        rows = rows.child(
            h_flex()
                .id(ElementId::Name(format!("open-{}", file.path).into()))
                .gap(px(8.))
                .items_center()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(7.))
                .cursor_pointer()
                .hover(|s| s.bg(p.surface_muted))
                .on_click(move |_, _, _| open_folder(&dir))
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .text_size(px(11.))
                        .text_color(p.ink_secondary)
                        .child(file.role.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(10.))
                        .font_family(palette::MONO)
                        .text_color(p.ink_muted)
                        .child(file.path.clone()),
                )
                .when(file.sensitive, |s| {
                    s.child(
                        div()
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.warn_bg)
                            .text_color(p.warn_text)
                            .text_size(px(9.))
                            .child("kredensial"),
                    )
                })
                .child(
                    div()
                        .px(px(5.))
                        .rounded(px(4.))
                        .bg(if file.exists { p.ok_bg } else { p.neutral_bg })
                        .text_color(if file.exists { p.ok_text } else { p.neutral_text })
                        .text_size(px(9.))
                        .child(if file.exists { "ada" } else { "belum ada" }),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(p.ink_muted)
                        .child("buka folder"),
                ),
        );
    }

    v_flex()
        .gap(px(4.))
        .p(px(10.))
        .rounded(px(12.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .shadow_2xs()
        .child(
            h_flex()
                .px(px(8.))
                .gap(px(6.))
                .items_center()
                .child(
                    div()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink_muted)
                        .child("BERKAS CONFIG"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(p.ink_muted)
                        .child("klik untuk membuka folder"),
                ),
        )
        .child(rows)
}

impl App {
    /// Daftar aksi per provider. Semua baris sengaja tidak menulis apa pun:
    /// spike ini read-only, tapi nama aksi + panggilan core-nya dicantumkan
    /// supaya jelas apa yang harus diimplementasikan nanti.
    fn show_actions(&self, window: &mut Window, cx: &mut Context<Self>, provider: &ProviderSummary) {
        let p = palette::palette(cx);
        let name = provider.name.clone();
        let id = provider.id.clone();
        let models = provider.models.len();
        let has_stored_key = provider.key_stored;
        let enabled = provider.enabled;

        let actions: Vec<(String, String)> = vec![
            (
                "Edit provider".into(),
                format!("api::preview_edit → EditOp::Upsert ({} model)", models),
            ),
            (
                "Kelola model".into(),
                "api::preview_edit → EditOp::SetModels".into(),
            ),
            (
                "Jadikan default".into(),
                "api::preview_edit → EditOp::SetDefault".into(),
            ),
            (
                if enabled {
                    "Nonaktifkan".to_string()
                } else {
                    "Aktifkan".to_string()
                },
                "api::preview_edit → EditOp::SetEnabled".into(),
            ),
            (
                "Lupakan kredensial".into(),
                if has_stored_key {
                    "api::preview_edit → EditOp::ForgetKey".into()
                } else {
                    "tidak ada kredensial tersimpan".to_string()
                },
            ),
            (
                "Hapus provider".into(),
                "api::preview_edit → EditOp::Remove".into(),
            ),
        ];
        window.open_dialog(cx, move |dialog, _, _| {
            // Closure dialog bisa dipanggil lebih dari sekali, jadi elemen
            // dibangun ulang di dalam — jangan pindahkan `Div` keluar.
            let mut rows = v_flex().gap(px(2.));
            for (label, hint) in &actions {
                rows = rows.child(
                    v_flex()
                        .px(px(8.))
                        .py(px(5.))
                        .rounded(px(7.))
                        .hover(|s| s.bg(p.surface_muted))
                        .child(div().text_size(px(12.5)).child(label.clone()))
                        .child(
                            div()
                                .text_size(px(9.5))
                                .font_family(palette::MONO)
                                .text_color(p.ink_muted)
                                .child(hint.clone()),
                        ),
                );
            }

            dialog.title(format!("{name} · {id}")).child(
                v_flex()
                    .gap(px(6.))
                    .w(px(420.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.ink_muted)
                            .child(
                                "Aksi di bawah belum aktif: spike read-only, tidak ada file yang ditulis. \
                                 Baris kecil menunjukkan panggilan core yang akan dipakai.",
                            ),
                    )
                    .child(rows),
            )
        });
    }
}

fn open_folder(dir: &str) {
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}

fn default_model_card(p: &Palette, provider: &str, model: &str) -> impl IntoElement {
    v_flex()
        .gap(px(3.))
        .p(px(14.))
        .rounded(px(12.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .shadow_2xs()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.ink_muted)
                .child("DEFAULT MODEL"),
        )
        .child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.brand_text)
                        .child(provider.to_string()),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_family(palette::MONO)
                        .text_color(p.ink_secondary)
                        .truncate()
                        .child(model.to_string()),
                ),
        )
}

fn provider_card(
    app: &App,
    p: &Palette,
    ix: usize,
    provider: ProviderSummary,
    cx: &mut Context<App>,
) -> impl IntoElement {
    let _ = app;
    let key_mode_env = provider.key_mode == "env";
    let enabled = provider.enabled;

    let (key_label, key_bg, key_fg) = if key_mode_env {
        (
            format!(
                "env · {}",
                provider.key_env.clone().unwrap_or_else(|| "—".into())
            ),
            p.neutral_bg,
            p.neutral_text,
        )
    } else if provider.has_key {
        ("✓ key tersimpan".to_string(), p.ok_bg, p.ok_text)
    } else {
        ("tanpa key".to_string(), p.neutral_bg, p.neutral_text)
    };

    v_flex()
        .id(format!("provider-{}", provider.id))
        .w(px(276.))
        .gap(px(10.))
        .p(px(13.))
        .rounded(px(14.))
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .shadow_2xs()
        .hover(|s| s.border_color(p.line_strong))
        .when(!enabled, |s| s.opacity(0.62))
        .child(
            h_flex()
                .gap(px(9.))
                .items_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.))
                        .flex_shrink_0()
                        .rounded(px(8.))
                        .bg(p.brand_soft)
                        .text_color(p.brand_text)
                        .text_size(px(10.))
                        .font_weight(FontWeight::BOLD)
                        .child(monogram(&provider.name, &provider.id)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            h_flex()
                                .gap(px(5.))
                                .items_center()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(provider.name.clone()),
                                )
                                .when(!enabled, |s| {
                                    s.child(
                                        div()
                                            .px(px(5.))
                                            .rounded(px(4.))
                                            .bg(p.neutral_bg)
                                            .text_color(p.neutral_text)
                                            .text_size(px(9.5))
                                            .child("nonaktif"),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(10.))
                                .font_family(palette::MONO)
                                .text_color(p.ink_muted)
                                .child(provider.id.clone()),
                        ),
                )
                .child(
                    div()
                        .id(ElementId::Name(format!("menu-{}", provider.id).into()))
                        .px(px(5.))
                        .py(px(0.))
                        .rounded(px(6.))
                        .text_size(px(13.))
                        .text_color(p.ink_muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(p.surface_muted).text_color(p.ink))
                        .child("…")
                        .on_click(cx.listener({
                            let provider = provider.clone();
                            move |this, _, window, cx| {
                                this.show_actions(window, cx, &provider);
                            }
                        })),
                )
                .child(
                    Switch::new(("sw", ix))
                        .small()
                        .checked(enabled)
                        .on_change(cx.listener(move |this, checked: &bool, _, cx| {
                            if let Some(snap) = this
                                .entries
                                .get_mut(this.selected)
                                .and_then(|e| e.snapshot.as_mut())
                            {
                                if let Some(target) = snap.providers.get_mut(ix) {
                                    target.enabled = *checked;
                                    this.dirty += 1;
                                }
                            }
                            cx.notify();
                        })),
                ),
        )
        .child(div().h(px(1.)).w_full().bg(p.line))
        .child(
            h_flex()
                .gap(px(6.))
                .items_center()
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_secondary)
                        .child(format!("{} model", provider.models.len())),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .px(px(6.))
                        .py(px(2.))
                        .rounded(px(5.))
                        .bg(key_bg)
                        .text_color(key_fg)
                        .text_size(px(9.5))
                        .child(key_label),
                ),
        )
        .when(!provider.models.is_empty(), |s| {
            s.child(
                h_flex()
                    .gap(px(4.))
                    .items_center()
                    .child(
                        div()
                            .truncate()
                            .flex_1()
                            .text_size(px(9.5))
                            .font_family(palette::MONO)
                            .text_color(p.ink_muted)
                            .child(
                                provider
                                    .models
                                    .iter()
                                    .take(2)
                                    .map(|m| {
                                        m.name.clone().unwrap_or_else(|| m.id.clone())
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                            ),
                    )
                    .when(provider.models.iter().any(|m| m.reasoning), |s| {
                        s.child(
                            div()
                                .px(px(5.))
                                .py(px(1.))
                                .rounded(px(4.))
                                .bg(p.warn_bg)
                                .text_color(p.warn_text)
                                .text_size(px(9.))
                                .child("reasoning"),
                        )
                    }),
            )
        })
        .child(
            div()
                .truncate()
                .text_size(px(10.))
                .font_family(palette::MONO)
                .text_color(p.ink_muted)
                .child(provider.base_url.clone().unwrap_or_else(|| "—".into())),
        )
}
