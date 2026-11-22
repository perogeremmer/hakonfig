//! Layar "Tambah provider": form tiga langkah (preset → key → model).
//!
//! READ-ONLY: hanya memanggil `api::list_presets` dan `api::preview_edit`.
//! Tidak pernah memanggil `apply_edit` / `restore_backup` / `fix_notice`.

use std::collections::HashSet;

use gpui_kit::component::{
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
    Sizable,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::api;
use hakonfig_core::engine::model::{EditOp, KeyMode, ModelDraft, ProviderDraft, TestResult};
use hakonfig_core::engine::model::ProviderPreset;

use crate::palette::{self, Palette};

/// Aturan core: `<PROVIDER_ID>_API_KEY`, huruf besar, `-` → `_`.
fn env_var_name(id: &str) -> String {
    format!("{}_API_KEY", id.to_uppercase().replace('-', "_"))
}

fn mask_key(key: &str) -> String {
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("••••{tail}")
}

fn fmt_window(n: u64) -> String {
    if n >= 1_000_000 {
        let m = n as f64 / 1_000_000.0;
        if (m - m.round()).abs() < 0.05 {
            format!("{}M ctx", m.round() as u64)
        } else {
            format!("{:.1}M ctx", m)
        }
    } else if n >= 1_000 {
        format!("{}k ctx", n / 1_000)
    } else {
        format!("{} ctx", n)
    }
}

fn chip(_p: &Palette, text: String, bg: Hsla, fg: Hsla) -> impl IntoElement {
    div()
        .px(px(6.))
        .py(px(1.))
        .rounded(px(5.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(9.5))
        .child(text)
}

fn field(p: &Palette, label: &str, input: Input) -> impl IntoElement {
    v_flex()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.ink_muted)
                .child(label.to_string()),
        )
        .child(input)
}

pub struct AddView {
    pub harness: String,
    step: u8,
    presets: Vec<ProviderPreset>,
    picked: Option<usize>,
    custom: bool,

    search: Entity<InputState>,
    _search_sub: Subscription,
    search_text: String,

    key_input: Entity<InputState>,
    _key_sub: Subscription,
    key_text: String,
    key_mode_env: bool,

    model_query_input: Entity<InputState>,
    _model_sub: Subscription,
    model_query: String,

    custom_id: Entity<InputState>,
    custom_name: Entity<InputState>,
    custom_base: Entity<InputState>,
    custom_api: Entity<InputState>,

    test: Option<Result<TestResult, String>>,
    testing: bool,
    _test_task: Option<Task<()>>,

    selected_models: HashSet<String>,
    preview: Option<Result<(usize, usize, Vec<String>), String>>,
    _preview_task: Option<Task<()>>,
}

impl AddView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Cari provider…"));
        let search_sub = {
            let st = search.clone();
            cx.subscribe_in(&search, window, move |this, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.search_text = st.read(cx).value().to_string();
                    cx.notify();
                }
            })
        };
        let key_input = cx.new(|cx| InputState::new(window, cx).placeholder("Tempel API key…"));
        let key_sub = {
            let st = key_input.clone();
            cx.subscribe_in(&key_input, window, move |this, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.key_text = st.read(cx).value().to_string();
                    cx.notify();
                }
            })
        };
        let model_query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Cari model…"));
        let model_sub = {
            let st = model_query_input.clone();
            cx.subscribe_in(&model_query_input, window, move |this, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.model_query = st.read(cx).value().to_string();
                    cx.notify();
                }
            })
        };
        let custom_id = cx.new(|cx| InputState::new(window, cx).placeholder("mis. my-provider"));
        let custom_name = cx.new(|cx| InputState::new(window, cx).placeholder("Nama tampilan"));
        let custom_base =
            cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com/v1"));
        let custom_api =
            cx.new(|cx| InputState::new(window, cx).placeholder("mis. openai-completions"));

        Self {
            harness: String::new(),
            step: 1,
            presets: api::list_presets(),
            picked: None,
            custom: false,
            search,
            _search_sub: search_sub,
            search_text: String::new(),
            key_input,
            _key_sub: key_sub,
            key_text: String::new(),
            key_mode_env: false,
            model_query_input,
            _model_sub: model_sub,
            model_query: String::new(),
            custom_id,
            custom_name,
            custom_base,
            custom_api,
            test: None,
            testing: false,
            _test_task: None,
            selected_models: HashSet::new(),
            preview: None,
            _preview_task: None,
        }
    }

    pub fn set_harness(&mut self, harness: String, cx: &mut Context<Self>) {
        if self.harness != harness {
            self.harness = harness;
            self.test = None;
            self.preview = None;
            cx.notify();
        }
    }

    /// (id, name, base_url, api) — dari preset yang dipilih atau field Custom.
    fn provider_meta(&self, cx: &App) -> Option<(String, String, String, String)> {
        if self.custom {
            let id = self.custom_id.read(cx).value().trim().to_string();
            let name = self.custom_name.read(cx).value().trim().to_string();
            let base_url = self.custom_base.read(cx).value().trim().to_string();
            let api = self.custom_api.read(cx).value().trim().to_string();
            if id.is_empty() || base_url.is_empty() || api.is_empty() {
                return None;
            }
            let name = if name.is_empty() { id.clone() } else { name };
            Some((id, name, base_url, api))
        } else {
            let pr = self.presets.get(self.picked?)?;
            Some((pr.id.clone(), pr.name.clone(), pr.base_url.clone(), pr.api.clone()))
        }
    }

    /// Daftar model untuk langkah 3: preset dulu, fallback ke hasil tes koneksi.
    fn model_rows(&self) -> Vec<ModelDraft> {
        if !self.custom {
            if let Some(pr) = self.picked.and_then(|ix| self.presets.get(ix)) {
                if !pr.models.is_empty() {
                    return pr
                        .models
                        .iter()
                        .map(|m| ModelDraft {
                            id: m.id.clone(),
                            name: m.name.clone(),
                            context_window: m.context_window,
                            max_tokens: m.max_tokens,
                            reasoning: m.reasoning,
                        })
                        .collect();
                }
            }
        }
        if let Some(Ok(tr)) = &self.test {
            return tr
                .models_sample
                .iter()
                .map(|id| ModelDraft {
                    id: id.clone(),
                    name: None,
                    context_window: None,
                    max_tokens: None,
                    reasoning: false,
                })
                .collect();
        }
        Vec::new()
    }

    fn pick_preset(&mut self, ix: usize) {
        self.picked = Some(ix);
        self.custom = false;
        self.reset_flow();
        self.step = 2;
    }

    fn pick_custom(&mut self) {
        self.picked = None;
        self.custom = true;
        self.reset_flow();
        self.step = 2;
    }

    fn reset_flow(&mut self) {
        self.test = None;
        self.testing = false;
        self.preview = None;
        self.key_mode_env = false;
        self.selected_models.clear();
    }

    fn goto_step3(&mut self) {
        self.selected_models = self.model_rows().iter().map(|m| m.id.clone()).collect();
        self.preview = None;
        self.step = 3;
    }

    fn run_test(&mut self, cx: &mut Context<Self>) {
        if self.testing {
            return;
        }
        let Some((id, _name, base_url, api_kind)) = self.provider_meta(cx) else {
            return;
        };
        let key = if self.key_mode_env {
            std::env::var(env_var_name(&id)).unwrap_or_default()
        } else {
            self.key_text.trim().to_string()
        };
        let harness = (!self.harness.is_empty()).then(|| self.harness.clone());
        self.testing = true;
        self.test = None;
        self._test_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    crate::net::test_connection(base_url, api_kind, key, harness, Some(id))
                })
                .await;
            this.update(cx, |this, cx| {
                this.test = Some(result);
                this.testing = false;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn run_preview(&mut self, cx: &mut Context<Self>) {
        let Some((id, name, base_url, api_kind)) = self.provider_meta(cx) else {
            self.preview = Some(Err("Lengkapi dulu identitas provider.".to_string()));
            cx.notify();
            return;
        };
        if self.harness.is_empty() {
            self.preview = Some(Err("Pilih harness aktif terlebih dahulu.".to_string()));
            cx.notify();
            return;
        }
        let models: Vec<ModelDraft> = self
            .model_rows()
            .into_iter()
            .filter(|m| self.selected_models.contains(&m.id))
            .collect();
        if models.is_empty() {
            self.preview = Some(Err("Pilih minimal satu model.".to_string()));
            cx.notify();
            return;
        }
        let draft = ProviderDraft {
            id,
            name,
            base_url,
            api: api_kind,
            key: if self.key_mode_env {
                None
            } else {
                Some(self.key_text.trim().to_string())
            },
            key_mode: Some(if self.key_mode_env { KeyMode::Env } else { KeyMode::Inline }),
            models,
        };
        let harness = self.harness.clone();
        self.preview = None;
        self._preview_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    api::preview_edit(harness, EditOp::Upsert(draft))
                        .map(|pr| (pr.will_create.len(), pr.will_modify.len(), pr.warnings))
                })
                .await;
            this.update(cx, |this, cx| {
                this.preview = Some(result);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn stepper(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = h_flex().gap(px(8.)).items_center();
        for (n, label) in [(1u8, "Pilih provider"), (2, "Key"), (3, "Model")] {
            let active = self.step == n;
            let done = self.step > n;
            row = row
                .child(
                    div()
                        .id(format!("step-{n}"))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded_full()
                        .text_size(px(11.5))
                        .when(active, |s| {
                            s.bg(p.brand_soft)
                                .text_color(p.brand_text)
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(!active, |s| {
                            s.bg(p.surface)
                                .border_1()
                                .border_color(p.line)
                                .text_color(if done { p.ink_secondary } else { p.ink_muted })
                        })
                        .child(format!("{n}. {label}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if n < this.step {
                                this.step = n;
                                cx.notify();
                            }
                        })),
                )
                .when(n < 3, |s| s.child(div().w(px(18.)).h(px(1.)).bg(p.line)));
        }
        row
    }

    fn step1(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let q = self.search_text.trim().to_lowercase();
        let mut grid = h_flex().flex_wrap().gap(px(10.)).items_start();
        let mut shown = 0usize;
        for (ix, pr) in self.presets.iter().enumerate() {
            if !q.is_empty() {
                let hay = format!("{} {}", pr.name, pr.id).to_lowercase();
                if !hay.contains(&q) {
                    continue;
                }
            }
            shown += 1;
            grid = grid.child(self.preset_card(p, ix, pr, cx));
        }
        grid = grid.child(
            v_flex()
                .id("preset-custom")
                .w(px(230.))
                .gap(px(6.))
                .p(px(12.))
                .rounded(px(12.))
                .bg(p.surface_muted)
                .border_1()
                .border_color(if self.custom { p.brand } else { p.line })
                .shadow_2xs()
                .hover(|s| s.border_color(p.line_strong))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Custom…"),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_muted)
                        .child("Isi sendiri id, base URL, dan jenis API."),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.pick_custom();
                    cx.notify();
                })),
        );

        v_flex()
            .gap(px(12.))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(div().w(px(300.)).child(Input::new(&self.search).small()))
                    .child(chip(
                        p,
                        format!("{shown} preset"),
                        p.neutral_bg,
                        p.neutral_text,
                    ))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_muted)
                            .child("Klik kartu untuk lanjut ke langkah 2."),
                    ),
            )
            .when(shown == 0 && !q.is_empty(), |s| {
                s.child(
                    div()
                        .py(px(20.))
                        .text_size(px(12.))
                        .text_color(p.ink_muted)
                        .child("Tidak ada preset yang cocok. Gunakan kartu Custom."),
                )
            })
            .child(grid)
    }

    fn preset_card(
        &self,
        p: &Palette,
        ix: usize,
        pr: &ProviderPreset,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = !self.custom && self.picked == Some(ix);
        let (badge_bg, badge_fg) = match pr.badge.as_deref() {
            Some("free") => (p.ok_bg, p.ok_text),
            Some("local") => (p.brand_soft, p.brand_text),
            _ => (p.neutral_bg, p.neutral_text),
        };
        v_flex()
            .id(format!("preset-{}", pr.id))
            .w(px(230.))
            .gap(px(6.))
            .p(px(12.))
            .rounded(px(12.))
            .bg(p.surface)
            .border_1()
            .border_color(if selected { p.brand } else { p.line })
            .shadow_2xs()
            .hover(|s| s.border_color(p.line_strong))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .truncate()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(pr.name.clone()),
                    )
                    .when_some(pr.badge.clone(), |s, b| {
                        s.child(chip(p, b, badge_bg, badge_fg))
                    }),
            )
            .child(
                div()
                    .truncate()
                    .text_size(px(10.))
                    .font_family(palette::MONO)
                    .text_color(p.ink_muted)
                    .child(pr.id.clone()),
            )
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_secondary)
                            .child(format!("{} model", pr.models.len())),
                    )
                    .when(!pr.key_required, |s| {
                        s.child(chip(p, "tanpa key".to_string(), p.ok_bg, p.ok_text))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.pick_preset(ix);
                cx.notify();
            }))
    }

    fn step2(
        &self,
        p: &Palette,
        meta: Option<&(String, String, String, String)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut col = v_flex().gap(px(12.)).max_w(px(560.));

        if self.custom {
            col = col
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::BOLD)
                        .child("Provider custom"),
                )
                .child(field(p, "ID provider", Input::new(&self.custom_id).small()))
                .child(field(p, "Nama", Input::new(&self.custom_name).small()))
                .child(field(p, "Base URL", Input::new(&self.custom_base).small()))
                .child(field(p, "Jenis API", Input::new(&self.custom_api).small()));
        } else if let Some(pr) = self.picked.and_then(|ix| self.presets.get(ix)) {
            col = col.child(
                v_flex()
                    .gap(px(6.))
                    .p(px(13.))
                    .rounded(px(12.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_2xs()
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child(pr.name.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .font_family(palette::MONO)
                                    .text_color(p.ink_muted)
                                    .child(pr.id.clone()),
                            ),
                    )
                    .child(
                        div()
                            .truncate()
                            .w_full()
                            .text_size(px(11.))
                            .font_family(palette::MONO)
                            .text_color(p.ink_secondary)
                            .child(pr.base_url.clone()),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.))
                            .items_center()
                            .child(chip(p, format!("api: {}", pr.api), p.neutral_bg, p.neutral_text))
                            .when_some(pr.docs_url.clone(), |s, u| {
                                s.child(chip(p, u, p.neutral_bg, p.neutral_text))
                            }),
                    )
                    .when(!pr.notes.is_empty(), |s| {
                        s.child(
                            div()
                                .text_size(px(10.5))
                                .text_color(p.ink_muted)
                                .child(pr.notes.clone()),
                        )
                    }),
            );
        }

        let env_name = meta
            .map(|m| env_var_name(&m.0))
            .unwrap_or_else(|| "—".to_string());
        let env_present = meta
            .map(|m| !std::env::var(env_var_name(&m.0)).unwrap_or_default().is_empty())
            .unwrap_or(false);
        col = col.child(
            v_flex()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink_muted)
                        .child("MODE API KEY"),
                )
                .child(
                    h_flex()
                        .gap(px(6.))
                        .items_center()
                        .child(
                            div()
                                .id("keymode-inline")
                                .px(px(10.))
                                .py(px(4.))
                                .rounded_full()
                                .text_size(px(11.5))
                                .when(!self.key_mode_env, |s| {
                                    s.bg(p.brand_soft)
                                        .text_color(p.brand_text)
                                        .font_weight(FontWeight::SEMIBOLD)
                                })
                                .when(self.key_mode_env, |s| {
                                    s.bg(p.surface)
                                        .border_1()
                                        .border_color(p.line)
                                        .text_color(p.ink_secondary)
                                })
                                .child("Simpan key")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.key_mode_env = false;
                                    this.test = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .id("keymode-env")
                                .px(px(10.))
                                .py(px(4.))
                                .rounded_full()
                                .text_size(px(11.5))
                                .when(self.key_mode_env, |s| {
                                    s.bg(p.brand_soft)
                                        .text_color(p.brand_text)
                                        .font_weight(FontWeight::SEMIBOLD)
                                })
                                .when(!self.key_mode_env, |s| {
                                    s.bg(p.surface)
                                        .border_1()
                                        .border_color(p.line)
                                        .text_color(p.ink_secondary)
                                })
                                .child("Pakai environment")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.key_mode_env = true;
                                    this.test = None;
                                    cx.notify();
                                })),
                        )
                        .when(self.key_mode_env, |s| {
                            s.child(chip(
                                p,
                                env_name.clone(),
                                p.neutral_bg,
                                p.neutral_text,
                            ))
                            .child(if env_present {
                                chip(p, "terisi di env".to_string(), p.ok_bg, p.ok_text)
                            } else {
                                chip(p, "belum ada di env".to_string(), p.warn_bg, p.warn_text)
                            })
                        }),
                ),
        );

        if self.key_mode_env {
            col = col.child(
                div()
                    .text_size(px(11.))
                    .text_color(p.ink_muted)
                    .child(format!(
                        "Key dibaca dari environment `{env_name}` saat tes koneksi; \
                         config hanya menyimpan nama variabel."
                    )),
            );
        } else {
            col = col.child(field(p, "API key", Input::new(&self.key_input).small()));
            if !self.key_text.trim().is_empty() {
                col = col.child(
                    div()
                        .text_size(px(10.5))
                        .font_family(palette::MONO)
                        .text_color(p.ink_muted)
                        .child(format!("tersimpan sebagai {}", mask_key(self.key_text.trim()))),
                );
            }
        }

        col = col.child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .child(
                    crate::palette::brand_button(
                        p,
                        "test-conn",
                        if self.testing { "Menguji…" } else { "Test & find models" },
                    )
                    .when(self.testing, |s| s.opacity(0.5))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.testing {
                            this.run_test(cx);
                        }
                    })),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_muted)
                        .child("Berjalan di background; tidak menyentuh file."),
                ),
        );

        if let Some(res) = &self.test {
            col = col.child(self.test_panel(p, res));
        }
        col
    }

    fn test_panel(&self, p: &Palette, res: &Result<TestResult, String>) -> impl IntoElement {
        let (bg, fg, title) = match res {
            Ok(tr) if tr.ok => (
                p.ok_bg,
                p.ok_text,
                format!(
                    "✓ Terhubung{}",
                    match (tr.status, tr.latency_ms) {
                        (Some(st), Some(ms)) => format!(" · HTTP {st} · {ms} ms"),
                        (Some(st), None) => format!(" · HTTP {st}"),
                        (None, Some(ms)) => format!(" · {ms} ms"),
                        _ => String::new(),
                    }
                ),
            ),
            Ok(tr) => (p.warn_bg, p.warn_text, format!("✗ Gagal · {}", tr.message)),
            Err(e) => (p.warn_bg, p.warn_text, format!("✗ Error · {e}")),
        };
        let mut panel = v_flex()
            .gap(px(6.))
            .w_full()
            .p(px(12.))
            .rounded(px(12.))
            .bg(bg)
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child(title),
            );
        if let Ok(tr) = res {
            if !tr.message.is_empty() && tr.ok {
                panel = panel.child(
                    div()
                        .text_size(px(10.5))
                        .text_color(fg)
                        .child(tr.message.clone()),
                );
            }
            if !tr.models_sample.is_empty() {
                let sample: Vec<String> = tr.models_sample.iter().take(5).cloned().collect();
                panel = panel.child(
                    v_flex()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .child(format!(
                                    "CONTOH MODEL ({} dari {})",
                                    sample.len(),
                                    tr.models_sample.len()
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_family(palette::MONO)
                                .text_color(fg)
                                .child(sample.join("\n")),
                        ),
                );
            }
        }
        panel
    }

    fn step3(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.model_rows();
        let q = self.model_query.trim().to_lowercase();
        let visible: Vec<ModelDraft> = rows
            .iter()
            .filter(|m| {
                q.is_empty()
                    || m.id.to_lowercase().contains(&q)
                    || m.name.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .cloned()
            .collect();

        let mut col = v_flex().gap(px(10.));
        col = col.child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .child(
                    crate::palette::soft_button(p, "sel-all", "Pilih semua").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.selected_models =
                                this.model_rows().iter().map(|m| m.id.clone()).collect();
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    crate::palette::soft_button(p, "sel-none", "Kosongkan").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.selected_models.clear();
                            cx.notify();
                        }),
                    ),
                )
                .child(chip(
                    p,
                    format!("{} / {} dipilih", self.selected_models.len(), rows.len()),
                    p.neutral_bg,
                    p.neutral_text,
                ))
                .child(div().flex_1())
                .child(div().w(px(220.)).child(Input::new(&self.model_query_input).small())),
        );

        if rows.is_empty() {
            col = col.child(
                v_flex()
                    .py(px(24.))
                    .gap(px(4.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Belum ada daftar model."),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.ink_muted)
                            .child(
                                "Jalankan “Test & find models” di langkah 2 untuk mengambil \
                                 contoh model dari endpoint.",
                            ),
                    ),
            );
        } else if visible.is_empty() {
            col = col.child(
                div()
                    .py(px(18.))
                    .text_size(px(12.))
                    .text_color(p.ink_muted)
                    .child("Tidak ada model yang cocok dengan pencarian."),
            );
        }
        for m in &visible {
            col = col.child(self.model_row(p, m, cx));
        }

        col = col
            .child(div().h(px(1.)).w_full().bg(p.line).my(px(4.)))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(
                        crate::palette::brand_button(p, "run-preview", "Pratinjau diff")
                            .on_click(cx.listener(|this, _, _, cx| this.run_preview(cx))),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_muted)
                            .child("Spike read-only — tidak ada file yang ditulis."),
                    ),
            );
        if let Some(res) = &self.preview {
            col = col.child(self.preview_panel(p, res));
        }
        col
    }

    fn model_row(&self, p: &Palette, m: &ModelDraft, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_models.contains(&m.id);
        let mid = m.id.clone();
        h_flex()
            .id(format!("model-{}", m.id))
            .w_full()
            .gap(px(9.))
            .items_center()
            .px(px(11.))
            .py(px(8.))
            .rounded(px(10.))
            .bg(p.surface)
            .border_1()
            .border_color(if selected { p.brand } else { p.line })
            .hover(|s| s.border_color(p.line_strong))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(17.))
                    .flex_shrink_0()
                    .rounded(px(5.))
                    .text_size(px(10.5))
                    .when(selected, |s| {
                        s.bg(p.brand).text_color(p.on_brand).child("✓")
                    })
                    .when(!selected, |s| {
                        s.border_1().border_color(p.line_strong).text_color(p.ink_muted)
                    }),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(1.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .font_family(palette::MONO)
                            .child(m.name.clone().unwrap_or_else(|| m.id.clone())),
                    )
                    .when(m.name.is_some(), |s| {
                        s.child(
                            div()
                                .truncate()
                                .text_size(px(10.))
                                .font_family(palette::MONO)
                                .text_color(p.ink_muted)
                                .child(m.id.clone()),
                        )
                    }),
            )
            .when_some(m.context_window, |s, n| {
                s.child(chip(p, fmt_window(n), p.neutral_bg, p.neutral_text))
            })
            .when(m.reasoning, |s| {
                s.child(chip(p, "reasoning".to_string(), p.warn_bg, p.warn_text))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.selected_models.remove(&mid) {
                    this.selected_models.insert(mid.clone());
                }
                cx.notify();
            }))
    }

    fn preview_panel(
        &self,
        p: &Palette,
        res: &Result<(usize, usize, Vec<String>), String>,
    ) -> impl IntoElement {
        match res {
            Ok((create, modify, warnings)) => v_flex()
                .gap(px(6.))
                .w_full()
                .p(px(12.))
                .rounded(px(12.))
                .bg(p.surface)
                .border_1()
                .border_color(p.line)
                .shadow_2xs()
                .child(
                    h_flex()
                        .gap(px(6.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Pratinjau perubahan"),
                        )
                        .child(chip(
                            p,
                            format!("{create} file baru"),
                            p.ok_bg,
                            p.ok_text,
                        ))
                        .child(chip(
                            p,
                            format!("{modify} file diubah"),
                            p.warn_bg,
                            p.warn_text,
                        )),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_muted)
                        .child("Read-only: apply_edit tidak dipanggil dari spike ini."),
                )
                .when(!warnings.is_empty(), |s| {
                    s.child(
                        v_flex()
                            .gap(px(3.))
                            .children(warnings.iter().map(|w| {
                                div()
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .bg(p.warn_bg)
                                    .text_color(p.warn_text)
                                    .text_size(px(10.5))
                                    .child(w.clone())
                            })),
                    )
                }),
            Err(e) => div()
                .w_full()
                .p(px(12.))
                .rounded(px(12.))
                .bg(p.warn_bg)
                .text_color(p.warn_text)
                .text_size(px(11.5))
                .child(e.clone()),
        }
    }

    fn nav(&self, p: &Palette, meta_ok: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.step;
        h_flex()
            .flex_shrink_0()
            .gap(px(8.))
            .items_center()
            .when(step > 1, |s| {
                s.child(
                    crate::palette::soft_button(p, "add-back", "Kembali").on_click(
                        cx.listener(|this, _, _, cx| {
                            if this.step > 1 {
                                this.step -= 1;
                                this.preview = None;
                                cx.notify();
                            }
                        }),
                    ),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(if self.harness.is_empty() {
                        "Harness: belum dipilih".to_string()
                    } else {
                        format!("Harness: {}", self.harness)
                    }),
            )
            .when(step == 2, |s| {
                s.child(
                    crate::palette::brand_button(p, "add-next", "Lanjut")
                        .when(!meta_ok, |s| s.opacity(0.5))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.provider_meta(cx).is_some() {
                                this.goto_step3();
                                cx.notify();
                            }
                        })),
                )
            })
            .when(step == 3, |s| {
                s.child(
                    crate::palette::brand_button(p, "add-preview", "Pratinjau diff").on_click(
                        cx.listener(|this, _, _, cx| this.run_preview(cx)),
                    ),
                )
            })
    }
}

impl Render for AddView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = crate::palette::palette(cx);
        let meta = self.provider_meta(cx);

        let body = match self.step {
            1 => self.step1(&p, cx).into_any_element(),
            2 => self.step2(&p, meta.as_ref(), cx).into_any_element(),
            _ => self.step3(&p, cx).into_any_element(),
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p(px(20.))
            .gap(px(12.))
            .child(
                h_flex()
                    .gap(px(10.))
                    .items_center()
                    .child(
                        v_flex()
                            .gap(px(2.))
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
                            ),
                    )
                    .child(div().flex_1())
                    .child(chip(
                        &p,
                        if self.harness.is_empty() {
                            "tanpa harness".to_string()
                        } else {
                            format!("→ {}", self.harness)
                        },
                        p.brand_soft,
                        p.brand_text,
                    )),
            )
            .child(self.stepper(&p, cx))
            .child(
                v_flex()
                    .id("add-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(self.nav(&p, meta.is_some(), cx))
    }
}
