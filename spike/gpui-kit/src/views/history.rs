//! Riwayat & backup: daftar backup yang dibuat setiap kali Hakonfig menulis,
//! plus dialog konfirmasi restore (eksekusinya sengaja tidak diaktifkan di spike).

use gpui_kit::component::{h_flex, v_flex, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::api;
use hakonfig_core::engine::model::BackupInfo;

use crate::palette::{self, Palette};

pub struct HistoryView {
    pub harness: String,
    backups: Vec<BackupInfo>,
    loading: bool,
    error: Option<String>,
    _task: Option<Task<()>>,
}

impl HistoryView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            harness: String::new(),
            backups: Vec::new(),
            loading: false,
            error: None,
            _task: None,
        }
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness == harness {
            return;
        }
        self.harness = harness;
        self.reload(cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.harness.is_empty() {
            return;
        }
        let harness = self.harness.clone();
        self.loading = true;
        self.error = None;
        let task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { api::list_backups(harness) })
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(mut list) => {
                        // Terbaru di atas: id backup memakai timestamp.
                        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                        this.backups = list;
                        this.error = None;
                    }
                    Err(err) => {
                        this.backups.clear();
                        this.error = Some(err);
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self._task = Some(task);
        cx.notify();
    }
}

impl Render for HistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::palette(cx);
        let count = self.backups.len();

        let mut list = v_flex().gap(px(8.));
        if let Some(err) = &self.error {
            list = list.child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.warn_bg)
                    .text_color(p.warn_text)
                    .text_size(px(12.))
                    .child(err.clone()),
            );
        }
        if self.loading && count == 0 {
            list = list.child(
                div()
                    .p(px(12.))
                    .text_size(px(12.))
                    .text_color(p.ink_muted)
                    .child("Membaca backup…"),
            );
        }
        if !self.loading && count == 0 && self.error.is_none() {
            list = list.child(
                v_flex()
                    .py(px(30.))
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Belum ada backup"),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.ink_muted)
                            .child("Backup dibuat otomatis setiap kali Hakonfig menulis config."),
                    ),
            );
        }

        for backup in &self.backups {
            list = list.child(self.row(&p, backup, window, cx));
        }

        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p(px(20.))
            .gap(px(12.))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink_muted)
                            .child("RIWAYAT"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("history-reload")
                            .px(px(11.))
                            .py(px(5.))
                            .rounded(px(8.))
                            .bg(p.surface)
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(12.))
                            .text_color(p.ink_secondary)
                            .cursor_pointer()
                            .hover(|s| s.bg(p.surface_muted))
                            .child("Muat ulang")
                            .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::BOLD)
                            .child("Backup"),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.ink_muted)
                            .child(format!("{count} backup tersimpan")),
                    ),
            )
            .child(
                v_flex()
                    .id("history-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(list),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child("Spike read-only — tombol Restore tidak menulis apa pun."),
            )
    }
}

impl HistoryView {
    fn row(
        &self,
        p: &Palette,
        backup: &BackupInfo,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = backup.id.clone();
        let dir = backup.dir.clone();
        // `created_at` adalah RFC3339 dengan nanodetik; cukup "YYYY-MM-DD HH:MM".
        let when = backup
            .created_at
            .replace('T', " ")
            .chars()
            .take(16)
            .collect::<String>();
        let files = backup
            .files
            .iter()
            .filter(|f| f.exists)
            .map(|f| f.role.clone())
            .collect::<Vec<_>>()
            .join(" · ");
        let missing = backup.files.iter().filter(|f| !f.exists).count();
        let p2 = *p;
        let files_count = backup.files.len();

        h_flex()
            .gap(px(10.))
            .items_center()
            .p(px(12.))
            .rounded(px(12.))
            .bg(p.surface)
            .border_1()
            .border_color(p.line)
            .shadow_2xs()
            .hover(|s| s.border_color(p.line_strong))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .font_family(palette::MONO)
                                    .child(id.split('-').next().unwrap_or(&id).to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(p.ink_muted)
                                    .child(format!("{} berkas", files_count)),
                            )
                            .when(missing > 0, |s| {
                                s.child(
                                    div()
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.warn_bg)
                                        .text_color(p.warn_text)
                                        .text_size(px(9.5))
                                        .child(format!("{} hilang", missing)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_secondary)
                            .truncate()
                            .child(if files.is_empty() {
                                when.clone()
                            } else {
                                format!("{} · {}", when, files)
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(9.5))
                            .font_family(palette::MONO)
                            .text_color(p.ink_muted)
                            .truncate()
                            .child(dir),
                    ),
            )
            .child(
                div()
                    .id(ElementId::Name(format!("restore-{}", backup.id).into()))
                    .px(px(10.))
                    .py(px(4.))
                    .rounded(px(8.))
                    .bg(p.surface_muted)
                    .border_1()
                    .border_color(p.line)
                    .text_size(px(11.5))
                    .text_color(p.ink_muted)
                    .cursor_pointer()
                    .child("Restore")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.confirm_restore(window, cx, &id, p2);
                    })),
            )
    }

    fn confirm_restore(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        id: &str,
        p: Palette,
    ) {
        let id = id.to_string();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog.title("Restore backup (belum aktif)").child(
                v_flex()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(p.ink_secondary)
                            .child(format!(
                                "Backup {} akan menyalin ulang isi config. Di spike ini \
                                 tindakan itu tidak dijalankan: tidak ada file yang ditulis.",
                                id
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.ink_muted)
                            .child("Implementasi nyata: api::restore_backup(harness, backup_id)."),
                    ),
            )
        });
    }
}
