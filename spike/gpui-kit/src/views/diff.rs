//! Layar Diff: pratinjau perubahan read-only sebelum ditulis.
//!
//! Menggunakan `hakonfig_core::api::preview_edit` (tidak pernah `apply_edit`)
//! dengan draft demo `spike-demo`, lalu mem-porting algoritma diff baris
//! dari `web/diff.js`: potong kepala/ekor yang sama, LCS dengan batas sel,
//! fallback "blok diganti", konteks 3 baris, masking kredensial.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::api::preview_edit;
use hakonfig_core::engine::model::{EditOp, FileDiff, ModelDraft, PreviewResult, ProviderDraft};

use crate::palette::{self, Palette};

/// Batas sel matriks LCS; di atas ini diff dianggap penggantian blok penuh.
const MAX_LCS_CELLS: usize = 250_000;
/// Jumlah baris konteks di atas/bawah blok yang berubah.
const CONTEXT_LINES: usize = 3;
/// Batas keras baris diff yang dirender per file.
const MAX_DIFF_LINES: usize = 4_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Same,
    Add,
    Del,
}

struct DiffLine {
    tag: Tag,
    old_no: Option<u32>,
    new_no: Option<u32>,
    text: String,
}

struct FileTab {
    path: String,
    adds: usize,
    dels: usize,
    lines: Vec<DiffLine>,
}

pub struct DiffView {
    pub harness: String,
    files: Vec<FileDiff>,
    warnings: Vec<String>,
    will_create: Vec<String>,
    will_modify: Vec<String>,
    tabs: Vec<FileTab>,
    active_tab: usize,
    loading: bool,
    error: Option<String>,
    _task: Option<Task<()>>,
}

impl DiffView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            harness: String::new(),
            files: Vec::new(),
            warnings: Vec::new(),
            will_create: Vec::new(),
            will_modify: Vec::new(),
            tabs: Vec::new(),
            active_tab: 0,
            loading: false,
            error: None,
            _task: None,
        }
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness != harness {
            self.harness = harness;
            self.active_tab = 0;
            cx.notify();
            self.reload(cx);
        }
    }

    /// Muat ulang pratinjau untuk harness aktif (read-only: hanya preview_edit).
    fn reload(&mut self, cx: &mut Context<Self>) {
        let harness = self.harness.clone();
        self.loading = true;
        self.error = None;
        self.tabs.clear();
        self.files.clear();
        self.warnings.clear();
        self.will_create.clear();
        self.will_modify.clear();
        cx.notify();

        self._task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { preview_edit(harness, demo_op()) })
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(res) => this.absorb(res),
                    Err(err) => this.error = Some(err),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn absorb(&mut self, res: PreviewResult) {
        self.warnings = res.warnings;
        self.will_create = res.will_create;
        self.will_modify = res.will_modify;
        self.tabs.clear();
        for file in res.files.iter().filter(|f| f.changed) {
            let before = file.before.as_deref().unwrap_or("");
            let after = file.after.as_deref().unwrap_or("");
            let (lines, adds, dels) = diff_file(before, after);
            if lines.len() > MAX_DIFF_LINES {
                self.warnings.push(format!(
                    "Diff {} dipotong pada {MAX_DIFF_LINES} baris dari total {} baris.",
                    file.path,
                    lines.len()
                ));
                self.tabs.push(FileTab {
                    path: file.path.clone(),
                    adds,
                    dels,
                    lines: lines.into_iter().take(MAX_DIFF_LINES).collect(),
                });
            } else {
                self.tabs.push(FileTab {
                    path: file.path.clone(),
                    adds,
                    dels,
                    lines,
                });
            }
        }
        self.files = res.files;
        if self.active_tab >= self.tabs.len() {
            self.active_tab = 0;
        }
    }
}

/// Draft demo `spike-demo` — hanya untuk preview, tidak pernah ditulis.
fn demo_op() -> EditOp {
    EditOp::Upsert(ProviderDraft {
        id: "spike-demo".into(),
        name: "Spike Demo".into(),
        base_url: "https://example.invalid/v1".into(),
        api: "openai-completions".into(),
        key: None,
        key_mode: None,
        models: vec![ModelDraft {
            id: "spike-demo-1".into(),
            name: Some("Spike Demo 1".into()),
            context_window: Some(128_000),
            max_tokens: Some(4_096),
            reasoning: false,
        }],
    })
}

// ---------------------------------------------------------------- diff engine

/// String kosong berarti "tidak ada baris", bukan "satu baris kosong".
fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').map(str::to_string).collect()
    }
}

/// Diff berbasis longest-common-subsequence atas dua potongan baris.
fn lcs_diff(before: &[String], after: &[String]) -> Vec<(Tag, String)> {
    let n = before.len();
    let m = after.len();
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if before[i] == after[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let mut ops: Vec<(Tag, String)> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if before[i] == after[j] {
            ops.push((Tag::Same, before[i].clone()));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            ops.push((Tag::Del, before[i].clone()));
            i += 1;
        } else {
            ops.push((Tag::Add, after[j].clone()));
            j += 1;
        }
    }
    while i < n {
        ops.push((Tag::Del, before[i].clone()));
        i += 1;
    }
    while j < m {
        ops.push((Tag::Add, after[j].clone()));
        j += 1;
    }
    ops
}

/// Porting `diffLines` web/diff.js: potong kepala/ekor sama, konteks 3,
/// fallback blok-diganti bila matriks LCS melebihi batas sel.
fn diff_file(before_text: &str, after_text: &str) -> (Vec<DiffLine>, usize, usize) {
    let before = split_lines(before_text);
    let after = split_lines(after_text);

    let mut start = 0usize;
    while start < before.len() && start < after.len() && before[start] == after[start] {
        start += 1;
    }
    let mut end_before = before.len() as isize - 1;
    let mut end_after = after.len() as isize - 1;
    while end_before >= start as isize
        && end_after >= start as isize
        && before[end_before as usize] == after[end_after as usize]
    {
        end_before -= 1;
        end_after -= 1;
    }
    // JS memakai rentang inklusif `slice(start, endBefore + 1)`;
    // konversi ke eksklusif: end = indeks_akhir + 1 (bisa == start bila tak ada perubahan).
    let end_before = (end_before + 1) as usize;
    let end_after = (end_after + 1) as usize;

    let head_start = start.saturating_sub(CONTEXT_LINES);
    let mut lines: Vec<DiffLine> = Vec::new();
    for k in head_start..start {
        lines.push(DiffLine {
            tag: Tag::Same,
            old_no: Some((k + 1) as u32),
            new_no: Some((k + 1) as u32),
            text: mask_line(&before[k]),
        });
    }

    let mid_before = &before[start..end_before];
    let mid_after = &after[start..end_after];

    let middle: Vec<(Tag, String)> = if mid_before.is_empty() && mid_after.is_empty() {
        Vec::new()
    } else if mid_before.len() * mid_after.len() > MAX_LCS_CELLS {
        let mut ops: Vec<(Tag, String)> = mid_before
            .iter()
            .map(|t| (Tag::Del, t.clone()))
            .collect();
        ops.extend(mid_after.iter().map(|t| (Tag::Add, t.clone())));
        ops
    } else {
        lcs_diff(mid_before, mid_after)
    };

    let (mut old_cur, mut new_cur) = (start, start);
    let (mut adds, mut dels) = (0usize, 0usize);
    for (tag, text) in middle {
        let (old_no, new_no) = match tag {
            Tag::Same => {
                old_cur += 1;
                new_cur += 1;
                (Some(old_cur as u32), Some(new_cur as u32))
            }
            Tag::Del => {
                dels += 1;
                old_cur += 1;
                (Some(old_cur as u32), None)
            }
            Tag::Add => {
                adds += 1;
                new_cur += 1;
                (None, Some(new_cur as u32))
            }
        };
        lines.push(DiffLine {
            tag,
            old_no,
            new_no,
            text: mask_line(&text),
        });
    }

    let tail_end = (end_before + CONTEXT_LINES).min(before.len());
    for k in end_before..tail_end {
        // Baris ekor sama: nomor lama lanjut dari `before`, nomor baru
        // lanjut dari posisi padanannya di `after`.
        let new_k = end_after + (k - end_before);
        lines.push(DiffLine {
            tag: Tag::Same,
            old_no: Some((k + 1) as u32),
            new_no: Some((new_k + 1) as u32),
            text: mask_line(&before[k]),
        });
    }

    (lines, adds, dels)
}

// ---------------------------------------------------------------- masking

const MASK: &str = "••••";

/// Ganti nilai kredensial dengan `••••`. Baris yang memuat penanda
/// kredensial, atau token panjang tanpa spasi (≥ 20 karakter) yang bukan
/// URL, tidak boleh dirender mentah.
fn mask_line(line: &str) -> String {
    let lower = line.to_lowercase();
    const KEYWORDS: [&str; 6] = ["apikey", "api_key", "\"key\"", "token", "password", "secret"];
    let marker = KEYWORDS.iter().find(|k| lower.contains(*k));

    if let Some(marker) = marker {
        let pos = lower.find(marker).unwrap();
        if let Some(sep_off) = line[pos..].find([':', '=']) {
            let sep_at = pos + sep_off;
            let prefix = &line[..=sep_at];
            let rest = &line[sep_at + 1..];
            let lead: usize = rest.chars().take_while(|c| c.is_whitespace()).count();
            let value_part = &rest[lead..];
            let open_quote = value_part.starts_with('"');
            let inner = if open_quote { &value_part[1..] } else { value_part };
            let (value, tail) = match (open_quote, inner.rfind('"')) {
                (true, Some(_)) => {
                    let close = inner.find('"').unwrap();
                    (&inner[..close], &inner[close..])
                }
                _ => (
                    inner.trim_end_matches(
                        |c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace(),
                    ),
                    "",
                ),
            };
            let v = value.trim();
            let placeholder = v.is_empty()
                || v == MASK
                || v.eq_ignore_ascii_case("null")
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("false")
                || v.chars().all(|c| c.is_ascii_digit());
            if !placeholder {
                return format!("{prefix} {MASK}{tail}");
            }
        }
    }

    // Deretan mirip kredensial: token panjang tanpa spasi yang bukan URL.
    let mut out = String::with_capacity(line.len());
    let mut changed = false;
    for token in line.split_whitespace() {
        let core =
            token.trim_matches(|c: char| c == '"' || c == ',' || c == ':' || c == '{' || c == '}');
        if core.chars().count() >= 20 && !core.contains("://") && !core.contains('/') {
            changed = true;
            out.push_str(&token.replace(core, MASK));
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    if changed {
        out.trim_end().to_string()
    } else {
        line.to_string()
    }
}

// ---------------------------------------------------------------- render

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p: Palette = crate::palette::palette(cx);
        let mut col = v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(20.))
            .gap(px(12.));

        // Header: judul + harness + tombol muat ulang.
        col = col.child(
            h_flex()
                .gap(px(10.))
                .items_center()
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .child("Pratinjau perubahan"),
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
                        .child(if self.harness.is_empty() {
                            "—".to_string()
                        } else {
                            self.harness.clone()
                        }),
                )
                .child(div().flex_1())
                .child(
                    palette::soft_button(&p, "diff-reload", "Muat ulang")
                        .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                ),
        );

        if self.loading {
            col = col.child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.ink_secondary)
                    .child("Menghitung pratinjau…"),
            );
        }

        if let Some(err) = self.error.clone() {
            col = col.child(
                v_flex()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.warn_bg)
                    .text_color(p.warn_text)
                    .gap(px(4.))
                    .text_size(px(12.))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Pratinjau gagal"))
                    .child(div().text_size(px(11.5)).child(err)),
            );
        }

        // Chip ringkasan: peringatan, file yang akan dibuat/diubah.
        let chips = summary_chips(self, &p);
        if chips.is_some() {
            col = col.child(chips.unwrap());
        }

        // Tab per file yang berubah.
        if !self.tabs.is_empty() {
            let mut tabs = h_flex().flex_wrap().gap(px(6.)).items_center();
            let active = self.active_tab.min(self.tabs.len() - 1);
            for (ix, tab) in self.tabs.iter().enumerate() {
                let is_active = ix == active;
                tabs = tabs.child(
                    div()
                        .id(format!("diff-tab-{ix}"))
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(if is_active { p.brand } else { p.line })
                        .bg(if is_active { p.brand_soft } else { p.surface })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.active_tab = ix;
                            cx.notify();
                        }))
                        .child(
                            h_flex()
                                .gap(px(7.))
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_family(palette::MONO)
                                        .text_color(p.ink)
                                        .child(tab.path.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.ok_text)
                                        .child(format!("+{}", tab.adds)),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.danger)
                                        .child(format!("−{}", tab.dels)),
                                ),
                        ),
                );
            }
            col = col.child(tabs);
        }

        // Badan diff (scroll vertikal) atau empty state.
        if self.tabs.is_empty() {
            col = col.child(empty_state(&p, self.loading, self.error.is_some()));
        } else {
            let active = self.active_tab.min(self.tabs.len() - 1);
            let tab = &self.tabs[active];
            let mut rows = v_flex()
                .rounded(px(10.))
                .border_1()
                .border_color(p.line)
                .bg(p.surface)
                .py(px(4.));
            for line in tab.lines.iter() {
                rows = rows.child(diff_row(&p, line));
            }
            col = col.child(
                v_flex()
                    .id("diff-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(rows),
            );
        }

        // Footer: Apply nonaktif (spike read-only).
        col.child(
            h_flex()
                .flex_shrink_0()
                .gap(px(8.))
                .items_center()
                .child(
                    div()
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(8.))
                        .bg(p.neutral_bg)
                        .text_color(p.ink_muted)
                        .text_size(px(11.))
                        .child("Apply (nonaktif — spike read-only)"),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_muted)
                        .child("Tidak ada file yang ditulis; hanya preview_edit."),
                ),
        )
    }
}

fn summary_chips(this: &DiffView, p: &Palette) -> Option<impl IntoElement> {
    if this.warnings.is_empty() && this.will_create.is_empty() && this.will_modify.is_empty() {
        return None;
    }
    let mut row = h_flex().flex_wrap().gap(px(6.)).items_center();
    for warning in this.warnings.iter() {
        row = row.child(chip(p.warn_bg, p.warn_text, format!("peringatan: {warning}")));
    }
    for path in this.will_create.iter() {
        row = row.child(chip(p.ok_bg, p.ok_text, format!("buat: {path}")));
    }
    for path in this.will_modify.iter() {
        row = row.child(chip(p.neutral_bg, p.neutral_text, format!("ubah: {path}")));
    }
    Some(row)
}

fn chip(bg: Hsla, fg: Hsla, text: String) -> impl IntoElement {
    div()
        .px(px(7.))
        .py(px(2.))
        .rounded_full()
        .bg(bg)
        .text_color(fg)
        .text_size(px(10.5))
        .font_family(palette::MONO)
        .truncate()
        .child(text)
}

fn empty_state(p: &Palette, loading: bool, errored: bool) -> impl IntoElement {
    let (title, sub) = if loading {
        ("Menghitung pratinjau…", "preview_edit berjalan di background.".to_string())
    } else if errored {
        (
            "Pratinjau tidak tersedia",
            "Perbaiki galat di atas, lalu coba \"Muat ulang\".".to_string(),
        )
    } else {
        (
            "Tidak ada perubahan",
            "Draft demo tidak mengubah file apa pun di harness ini.".to_string(),
        )
    };
    v_flex()
        .flex_1()
        .min_h_0()
        .py(px(60.))
        .items_center()
        .gap(px(5.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink_secondary)
                .child(title),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(p.ink_muted)
                .child(sub),
        )
}

fn diff_row(p: &Palette, line: &DiffLine) -> impl IntoElement {
    let (bg, fg) = match line.tag {
        Tag::Add => (p.ok_bg, p.ok_text),
        Tag::Del => (p.danger.opacity(0.10), p.danger),
        Tag::Same => (p.surface, p.ink_secondary),
    };
    let (mark, mark_fg) = match line.tag {
        Tag::Add => ("+", p.ok_text),
        Tag::Del => ("−", p.danger),
        Tag::Same => (" ", p.ink_muted),
    };
    let text = if line.text.is_empty() {
        " ".to_string()
    } else {
        line.text.clone()
    };
    h_flex()
        .w_full()
        .items_center()
        .bg(bg)
        .child(
            div()
                .w(px(46.))
                .flex_shrink_0()
                .text_size(px(9.5))
                .text_color(p.ink_muted)
                .font_family(palette::MONO)
                .child(line.old_no.map(|n| n.to_string()).unwrap_or_default()),
        )
        .child(
            div()
                .w(px(46.))
                .flex_shrink_0()
                .text_size(px(9.5))
                .text_color(p.ink_muted)
                .font_family(palette::MONO)
                .child(line.new_no.map(|n| n.to_string()).unwrap_or_default()),
        )
        .child(
            div()
                .w(px(14.))
                .flex_shrink_0()
                .text_size(px(11.))
                .text_color(mark_fg)
                .font_family(palette::MONO)
                .child(mark),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(11.))
                .text_color(fg)
                .font_family(palette::MONO)
                .child(text),
        )
}

