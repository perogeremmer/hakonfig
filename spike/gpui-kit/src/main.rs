//! Spike: layar Providers Hakonfig di atas GPUI Kit.
//!
//! Tujuannya bukan menggantikan GUI Tauri, tapi menjawab satu pertanyaan:
//! apakah `hakonfig_core` bisa dipakai apa adanya oleh UI native Rust, dan
//! seberapa cepat hasilnya. Semua data di sini datang dari crate core yang sama
//! dengan yang dipakai `hakonfig-cli` dan GUI Tauri; tidak ada satu pun logika
//! konfigurasi yang ditulis ulang di spike ini.

use std::sync::OnceLock;
use std::time::Instant;

use gpui_kit::component::{
    ActiveTheme, Sizable,
    button::Button,
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::api;
use hakonfig_core::engine::model::{HarnessSnapshot, ProviderSummary};

/// Diisi `main()`, dibaca render pertama: angka "sampai frame pertama terlihat".
static PROCESS_START: OnceLock<Instant> = OnceLock::new();

// ---------------------------------------------------------------- data

struct HarnessEntry {
    id: String,
    name: String,
    installed: bool,
    experimental: bool,
    provider_count: usize,
    connected_count: usize,
    version: Option<String>,
    config_path: String,
    snapshot: Option<HarnessSnapshot>,
    error: Option<String>,
    /// (baca config, deteksi versi tool) dalam milidetik.
    timings: (u128, u128),
}

impl HarnessEntry {
    fn load(id: String) -> Self {
        let info = api::list_harnesses()
            .into_iter()
            .find(|h| h.id == id)
            .expect("harness dari list_harnesses selalu bisa dibaca ulang");

        // `read_harness` mengembalikan Result<String>, sama seperti yang
        // diterima frontend Tauri lewat `#[tauri::command]`.
        let t_read = Instant::now();
        let (snapshot, error) = match api::read_harness(id.clone()) {
            Ok(snap) => (Some(snap), None),
            Err(e) => (None, Some(e)),
        };
        let read_ms = t_read.elapsed().as_millis();
        // Versi tool datang dari `<bin> --version`, jadi butuh spawn proses.
        let t_ver = Instant::now();
        let version = api::harness_version(&id);
        let ver_ms = t_ver.elapsed().as_millis();

        Self {
            timings: (read_ms, ver_ms),
            version,
            config_path: info
                .files
                .first()
                .map(|f| f.path.clone())
                .unwrap_or_default(),
            id,
            name: info.name,
            installed: info.installed,
            experimental: info.experimental,
            provider_count: info.provider_count,
            connected_count: info.connected_count,
            snapshot,
            error,
        }
    }

    fn model_count(&self) -> usize {
        self.snapshot
            .as_ref()
            .map(|s| s.providers.iter().map(|p| p.models.len()).sum())
            .unwrap_or(0)
    }
}

fn load_all() -> (Vec<HarnessEntry>, u128) {
    let started = Instant::now();
    let entries: Vec<HarnessEntry> = api::list_harnesses()
        .into_iter()
        .map(|h| {
            let t = Instant::now();
            let entry = HarnessEntry::load(h.id);
            eprintln!(
                "[spike]   {}: {} ms total (config {} ms + versi {} ms) — {} provider, {} model",
                entry.id,
                t.elapsed().as_millis(),
                entry.timings.0,
                entry.timings.1,
                entry.provider_count,
                entry.model_count()
            );
            entry
        })
        .collect();
    (entries, started.elapsed().as_millis())
}

// ---------------------------------------------------------------- app

struct Spike {
    entries: Vec<HarnessEntry>,
    selected: usize,
    query: Entity<InputState>,
    query_text: String,
    load_ms: u128,
    data_ready_ms: Option<u128>,
    first_frame: bool,
    _subs: Vec<Subscription>,
    _load: Option<Task<()>>,
}

impl Spike {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Jendela dibuka dengan state kosong; pembacaan config + spawn
        // `<bin> --version` jalan di background executor. Ini beda arsitektur
        // yang penting: konstruktor GPUI sinkron, jadi kerja berat di sini
        // akan menahan frame pertama kalau tidak dipindah.
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Cari provider…"));
        let _subs = vec![cx.subscribe_in(&query, window, {
            let query = query.clone();
            move |this, _, ev: &InputEvent, _, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.query_text = query.read(cx).value().to_string();
                    cx.notify();
                }
            }
        })];

        let _load = Some(cx.spawn(async move |this, cx| {
            let (entries, load_ms) = cx.background_spawn(async { load_all() }).await;
            this.update(cx, |this, cx| {
                this.entries = entries;
                this.load_ms = load_ms;
                this.data_ready_ms = PROCESS_START
                    .get()
                    .map(|start| start.elapsed().as_millis());
                this.selected = this.selected.min(this.entries.len().saturating_sub(1));
                if let Some(ms) = this.data_ready_ms {
                    eprintln!(
                        "[spike] data siap setelah {} ms (baca config {} ms, {} harness, {} model)",
                        ms,
                        this.load_ms,
                        this.entries.len(),
                        this.total_models()
                    );
                }
                cx.notify();
            })
            .ok();
        }));

        Self {
            entries: Vec::new(),
            selected: 0,
            query,
            query_text: String::new(),
            load_ms: 0,
            data_ready_ms: None,
            first_frame: false,
            _subs,
            _load,
        }
    }

    fn active(&self) -> Option<&HarnessEntry> {
        self.entries.get(self.selected)
    }

    fn visible_providers(&self) -> Vec<ProviderSummary> {
        let Some(snap) = self.active().and_then(|e| e.snapshot.as_ref()) else {
            return Vec::new();
        };
        let needle = self.query_text.trim().to_lowercase();
        let mut list: Vec<ProviderSummary> = snap
            .providers
            .iter()
            .filter(|p| {
                needle.is_empty()
                    || p.id.to_lowercase().contains(&needle)
                    || p.name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        list.sort_by(|a, b| b.models.len().cmp(&a.models.len()).then(a.id.cmp(&b.id)));
        list
    }

    fn total_models(&self) -> usize {
        self.entries.iter().map(HarnessEntry::model_count).sum()
    }
}

// ---------------------------------------------------------------- view

impl Render for Spike {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.first_frame {
            self.first_frame = true;
            if let Some(start) = PROCESS_START.get() {
                eprintln!(
                    "[spike] frame pertama setelah {} ms",
                    start.elapsed().as_millis(),
                );
            }
        }
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.header(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(self.sidebar(cx))
                    .child(self.content(cx)),
            )
    }
}

impl Spike {
    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .px_4()
            .py_3()
            .gap_3()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("Hakonfig"))
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("spike · gpui-kit 0.10"),
            )
            .child(div().flex_1())
            .child(
                div().text_xs().text_color(muted).child(format!(
                    "{} tool · {} provider · {} model · load {} ms{}",
                    self.entries.len(),
                    self.entries.iter().map(|e| e.provider_count).sum::<usize>(),
                    self.total_models(),
                    self.load_ms,
                    match self.data_ready_ms {
                        Some(ms) => format!(" · siap {ms} ms"),
                        None => " · memuat…".to_string(),
                    },
                )),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;

        let mut items = v_flex().gap_1().p_2();
        for (ix, entry) in self.entries.iter().enumerate() {
            let selected = ix == self.selected;
            let installed = entry.installed;
            items = items.child(
                h_flex()
                    .id(("harness", ix))
                    .px_3()
                    .py_2()
                    .gap_2()
                    .items_center()
                    .rounded_md()
                    .when(selected, |s| s.bg(cx.theme().muted))
                    .when(!selected, |s| s.hover(|s| s.bg(cx.theme().muted)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = ix;
                        this.query_text.clear();
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(div().truncate().child(entry.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .truncate()
                                    .child(if installed {
                                        format!(
                                            "{} provider · {} key",
                                            entry.provider_count, entry.connected_count
                                        )
                                    } else {
                                        "belum terpasang".to_string()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_full()
                            .bg(if selected {
                                cx.theme().primary
                            } else {
                                cx.theme().secondary
                            })
                            .text_color(if selected {
                                cx.theme().primary_foreground
                            } else {
                                cx.theme().secondary_foreground
                            })
                            .text_xs()
                            .child(entry.provider_count.to_string()),
                    ),
            );
        }

        v_flex()
            .w(px(232.))
            .flex_shrink_0()
            .border_r_1()
            .border_color(border)
            .child(
                div()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .text_xs()
                    .text_color(muted)
                    .child("HARNESS"),
            )
            .child(items)
    }

    fn content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let Some(entry) = self.active() else {
            return v_flex()
                .flex_1()
                .p_4()
                .gap_2()
                .child(div().text_xs().text_color(muted).child("MEMUAT"))
                .child(div().child("Membaca konfigurasi harness…"));
        };
        let providers = self.visible_providers();

        let mut list = v_flex().gap_2();
        if let Some(err) = &entry.error {
            list = list.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(cx.theme().danger.opacity(0.12))
                    .text_color(cx.theme().danger)
                    .child(err.clone()),
            );
        }
        if providers.is_empty() && entry.error.is_none() {
            list = list.child(
                div()
                    .p_6()
                    .text_color(muted)
                    .child("Belum ada provider di harness ini."),
            );
        }
        let shown = providers.len();
        for provider in providers {
            list = list.child(self.provider_row(provider, cx));
        }

        v_flex()
            .flex_1()
            .min_w_0()
            .p_4()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().text_2xl().font_weight(FontWeight::BOLD).child(entry.name.clone()))
                            .when_some(entry.version.clone(), |s, v| {
                                s.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(cx.theme().secondary)
                                        .text_color(cx.theme().secondary_foreground)
                                        .text_xs()
                                        .child(v),
                                )
                            })
                            .when(entry.experimental, |s| {
                                s.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(cx.theme().warning.opacity(0.18))
                                        .text_color(cx.theme().warning)
                                        .text_xs()
                                        .child("eksperimental"),
                                )
                            }),
                    )
                    .child(div().text_xs().text_color(muted).child(entry.config_path.clone())),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(Input::new(&self.query).small()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(format!("{} tampil", shown)),
                    ),
            )
            .child(
                v_flex()
                    .id("providers")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(list),
            )
            .when_some(
                entry
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.default_model.clone()),
                |s, model| {
                    s.child(
                        h_flex()
                            .pt_2()
                            .gap_2()
                            .items_center()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(div().text_xs().text_color(muted).child("default model"))
                            .child(div().text_xs().child(model))
                            .child(div().flex_1())
                            .child(Button::new("refresh").small().label("Refresh").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.data_ready_ms = None;
                                    let task = cx.spawn(async move |this, cx| {
                                        let (entries, ms) =
                                            cx.background_spawn(async { load_all() }).await;
                                        this.update(cx, |this, cx| {
                                            this.entries = entries;
                                            this.load_ms = ms;
                                            this.data_ready_ms = PROCESS_START
                                                .get()
                                                .map(|start| start.elapsed().as_millis());
                                            this.selected = this
                                                .selected
                                                .min(this.entries.len().saturating_sub(1));
                                            cx.notify();
                                        })
                                        .ok();
                                    });
                                    this._load = Some(task);
                                    cx.notify();
                                }),
                            )),
                    )
                },
            )
    }

    fn provider_row(&self, provider: ProviderSummary, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;

        let dot = if provider.has_key {
            cx.theme().success
        } else {
            cx.theme().border
        };

        h_flex()
            .id(format!("provider-{}", provider.id))
            .px_3()
            .py_2()
            .gap_3()
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(border)
            .when(!provider.enabled, |s| s.opacity(0.55))
            .child(div().size_2().rounded_full().bg(dot))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(provider.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(provider.id.clone()),
                            )
                            .when(!provider.enabled, |s| {
                                s.child(
                                    div()
                                        .px_1p5()
                                        .rounded_sm()
                                        .bg(cx.theme().muted)
                                        .text_xs()
                                        .text_color(muted)
                                        .child("nonaktif"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .truncate()
                            .child(provider.base_url.clone().unwrap_or_else(|| "—".into())),
                    ),
            )
            .child(
                v_flex()
                    .items_end()
                    .gap_0p5()
                    .child(
                        div()
                            .text_xs()
                            .child(format!("{} model", provider.models.len())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if provider.has_key {
                                cx.theme().success
                            } else {
                                muted
                            })
                            .child(
                                provider
                                    .key_display
                                    .clone()
                                    .or_else(|| provider.key_env.clone())
                                    .unwrap_or_else(|| "tanpa key".into()),
                            ),
                    ),
            )
    }
}

fn main() {
    PROCESS_START.get_or_init(Instant::now);
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        let bounds = WindowBounds::centered(size(px(1200.), px(820.)), cx);
        let options = WindowOptions {
            window_bounds: Some(bounds),
            titlebar: Some(TitlebarOptions {
                title: Some("Hakonfig — spike gpui-kit".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Spike::new(window, cx))
        })
        .expect("gagal membuka jendela");
    });
}
