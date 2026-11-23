//! State aplikasi: daftar harness, harness terpilih, navigasi, dan shell UI
//! (top bar + sidebar). Isi tiap layar ada di `views/`.

use std::sync::OnceLock;
use std::time::Instant;

use gpui_kit::component::{
    h_flex,
    input::{InputEvent, InputState},
    notification::Notification,
    v_flex,
    WindowExt as _,
};
use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use hakonfig_core::engine::model::{Notice, ProviderSummary};

use crate::data::{load_all, load_notices, monogram, probe_versions, HarnessEntry, Metrics};
use crate::palette::{self, Palette};
use crate::views::{
    add::AddView,
    diff::DiffView,
    history::HistoryView,
    providers::{self, Filter},
    settings::SettingsView,
};

/// Diisi `main()`, dibaca render pertama: angka "sampai frame pertama terlihat".
pub static PROCESS_START: OnceLock<Instant> = OnceLock::new();

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Providers,
    Add,
    Diff,
    History,
    Settings,
}

impl Nav {
    /// Label tab. Layar dan harness sengaja dipisah: tab di atas, harness di
    /// sidebar kiri — dua daftar di kolom yang sama bikin bingung.
    pub fn label(self) -> &'static str {
        match self {
            Nav::Providers => "Providers",
            Nav::Add => "Tambah provider",
            Nav::Diff => "Diff",
            Nav::History => "Riwayat",
            Nav::Settings => "Pengaturan",
        }
    }

    #[allow(dead_code)] // dipakai view lain untuk id elemen
    pub fn slug(self) -> &'static str {
        match self {
            Nav::Providers => "providers",
            Nav::Add => "add",
            Nav::Diff => "diff",
            Nav::History => "history",
            Nav::Settings => "settings",
        }
    }
}

pub struct App {
    pub entries: Vec<HarnessEntry>,
    pub selected: usize,
    pub nav: Nav,
    pub query: Entity<InputState>,
    pub query_text: String,
    pub filter: Filter,
    /// Notice milik harness aktif saja (dibaca di background).
    pub notices: Vec<Notice>,
    /// Berapa status provider yang diubah lokal (spike tidak menulis file).
    pub dirty: usize,
    pub load_ms: u128,
    pub data_ready_ms: Option<u128>,
    /// Pesan yang menunggu ditampilkan sebagai toast di render berikutnya.
    toast: Option<String>,
    first_frame: bool,
    pub add: Entity<AddView>,
    pub diff: Entity<DiffView>,
    pub history: Entity<HistoryView>,
    pub settings: Entity<SettingsView>,
    _subs: Vec<Subscription>,
    _load: Option<Task<()>>,
    _load_notices: Option<Task<()>>,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Dua tahap, dan pembagiannya disengaja:
        //
        // 1. Baca config (diukur 34 ms untuk 4 harness) DIBLOKIR di sini, supaya
        //    frame pertama sudah berisi data, bukan layar "Memuat…".
        // 2. Probe versi tool (`<bin> --version`, ±1.000 ms) jalan di background.
        //
        // Sebelumnya semuanya di background: data baru muncul di 884 ms, jadi
        // pengguna melihat layar kosong lebih lama daripada menunggu 34 ms.
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

        let (entries, load_ms) = load_all();
        let data_ready_ms = PROCESS_START.get().map(|s| s.elapsed().as_millis());
        eprintln!(
            "[spike] config siap setelah {:?} ms (baca {} ms, {} harness, {} model)",
            data_ready_ms.unwrap_or_default(),
            load_ms,
            entries.len(),
            entries.iter().map(HarnessEntry::model_count).sum::<usize>()
        );

        // Ketiga view menerima `&mut Window` seperti `App::new`: tanpa itu view
        // tidak bisa membuat `InputState` (kotak teks) karena konstruktornya
        // butuh window. Kontrak: `XView::new(window, cx)`.
        let add = cx.new(|cx| AddView::new(window, cx));
        let diff = cx.new(|cx| DiffView::new(window, cx));
        let history = cx.new(|cx| HistoryView::new(window, cx));
        let settings = cx.new(|cx| SettingsView::new(window, cx));

        let ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
        let _load = Some(cx.spawn(async move |this, cx| {
            // Tahap 2: probe versi tool, tanpa menahan UI.
            let versions = cx
                .background_spawn(async move { probe_versions(ids) })
                .await;
            this.update(cx, |this, cx| {
                for (id, version, ms) in versions {
                    if let Some(entry) = this.entries.iter_mut().find(|e| e.id == id) {
                        entry.version = version;
                        entry.timings.1 = ms;
                    }
                }
                if let Some(start) = PROCESS_START.get() {
                    eprintln!(
                        "[spike] versi tool siap setelah {} ms",
                        start.elapsed().as_millis()
                    );
                }
                this.sync_views(cx);
                cx.notify();
            })
            .ok();
        }));

        Self {
            entries,
            selected: 0,
            nav: Nav::Providers,
            query,
            query_text: String::new(),
            filter: Filter::All,
            notices: Vec::new(),
            dirty: 0,
            load_ms,
            data_ready_ms,
            toast: None,
            first_frame: false,
            add,
            diff,
            history,
            settings,
            _subs,
            _load,
            _load_notices: None,
        }
    }

    pub fn active(&self) -> Option<&HarnessEntry> {
        self.entries.get(self.selected)
    }

    pub fn active_id(&self) -> String {
        self.active().map(|e| e.id.clone()).unwrap_or_default()
    }

    /// (indeks asli, provider) — indeks asli dipakai untuk mengubah state lokal.
    pub fn visible_providers(&self) -> Vec<(usize, ProviderSummary)> {
        let Some(snap) = self.active().and_then(|e| e.snapshot.as_ref()) else {
            return Vec::new();
        };
        let needle = self.query_text.trim().to_lowercase();
        let mut list: Vec<(usize, ProviderSummary)> = snap
            .providers
            .iter()
            .enumerate()
            .filter(|(_, p)| self.filter.matches(p))
            .filter(|(_, p)| {
                needle.is_empty()
                    || p.id.to_lowercase().contains(&needle)
                    || p.name.to_lowercase().contains(&needle)
            })
            .map(|(ix, p)| (ix, p.clone()))
            .collect();
        list.sort_by(|a, b| {
            b.1.models
                .len()
                .cmp(&a.1.models.len())
                .then(a.1.id.cmp(&b.1.id))
        });
        list
    }

    pub fn total_models(&self) -> usize {
        self.entries.iter().map(HarnessEntry::model_count).sum()
    }

    fn sync_views(&mut self, cx: &mut Context<Self>) {
        let harness = self.active_id();
        if !harness.is_empty() {
            let task = cx.spawn(async move |this, cx| {
                let notices = cx
                    .background_spawn(async move { load_notices(harness) })
                    .await;
                this.update(cx, |this, cx| {
                    this.notices = notices;
                    cx.notify();
                })
                .ok();
            });
            self._load_notices = Some(task);
        }
        let metrics = Metrics::of(&self.entries, self.load_ms, self.data_ready_ms);
        self.settings
            .update(cx, |v, cx| v.set_metrics(metrics, cx));
        let id = self.active_id();
        self.add.update(cx, |v, cx| v.set_harness(id.clone(), cx));
        self.diff.update(cx, |v, cx| v.set_harness(id.clone(), cx));
        self.history
            .update(cx, |v, cx| v.set_harness(id.clone(), cx));
        self.settings.update(cx, |v, cx| v.set_harness(id, cx));
    }

    pub fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix == self.selected {
            return;
        }
        self.selected = ix;
        self.query_text.clear();
        self.sync_views(cx);
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.dirty = 0;
        self.data_ready_ms = None;
        let task = cx.spawn(async move |this, cx| {
            let (entries, ms) = cx.background_spawn(async { load_all() }).await;
            this.update(cx, |this, cx| {
                this.entries = entries;
                this.load_ms = ms;
                this.data_ready_ms = PROCESS_START.get().map(|s| s.elapsed().as_millis());
                this.selected = this.selected.min(this.entries.len().saturating_sub(1));
                this.toast = Some(format!("Config dibaca ulang · {} ms", this.load_ms));
                this.sync_views(cx);
                cx.notify();
            })
            .ok();
        });
        self._load = Some(task);
        cx.notify();
    }

    pub fn set_nav(&mut self, nav: Nav, cx: &mut Context<Self>) {
        self.nav = nav;
        cx.notify();
    }
}

// ---------------------------------------------------------------- shell

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.first_frame {
            self.first_frame = true;
            if let Some(start) = PROCESS_START.get() {
                eprintln!("[spike] frame pertama setelah {} ms", start.elapsed().as_millis());
            }
        }
        // Toast diantre dari event handler (yang tidak punya &mut App), lalu
        // dipush di sini karena `push_notification` butuh window + App.
        if let Some(msg) = self.toast.take() {
            window.push_notification(Notification::success(msg), cx);
        }
        let p = palette::palette(cx);
        let body = match self.nav {
            Nav::Providers => providers::render(self, &p, cx).into_any_element(),
            Nav::Add => self.add.clone().into_any_element(),
            Nav::Diff => self.diff.clone().into_any_element(),
            Nav::History => self.history.clone().into_any_element(),
            Nav::Settings => self.settings.clone().into_any_element(),
        };

        v_flex()
            .size_full()
            .bg(p.canvas)
            .text_color(p.ink)
            .child(self.topbar(&p, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(self.sidebar(&p, cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .when(self.nav != Nav::Providers, |s| {
                                s.child(self.back_bar(&p, cx))
                            })
                            .child(body),
                    ),
            )
    }
}

impl App {
    /// Layar selain Providers dibuka dari kotak aksi di layar Providers atau
    /// dari ikon cog (Pengaturan), jadi jalan pulangnya harus selalu ada.
    fn back_bar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_shrink_0()
            .px(px(20.))
            .pt(px(12.))
            .gap(px(8.))
            .items_center()
            .child(
                h_flex()
                    .id("back-providers")
                    .gap(px(5.))
                    .px(px(9.))
                    .py(px(4.))
                    .rounded(px(7.))
                    .cursor_pointer()
                    .text_size(px(11.5))
                    .text_color(p.ink_secondary)
                    .hover(|s| s.bg(p.surface_muted).text_color(p.ink))
                    .child(IconName::ArrowLeft)
                    .child("Providers")
                    .on_click(cx.listener(|this, _, _, cx| this.set_nav(Nav::Providers, cx))),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(p.ink_muted)
                    .child(format!("· {}", self.nav.label())),
            )
            .child(div().flex_1())
            .child(
                div()
                    .pr(px(4.))
                    .text_size(px(10.5))
                    .text_color(p.ink_muted)
                    .child(active_hint(self.nav)),
            )
    }

    fn topbar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_shrink_0()
            .px(px(18.))
            .py(px(11.))
            .gap(px(10.))
            .items_center()
            .bg(p.surface)
            .border_b_1()
            .border_color(p.line)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(26.))
                    .rounded(px(8.))
                    .bg(p.brand)
                    .text_color(p.on_brand)
                    .text_size(px(13.))
                    .font_weight(FontWeight::BOLD)
                    .child("H"),
            )
            .child(
                h_flex()
                    .gap(px(7.))
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::BOLD)
                            .child("Hakonfig"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.ink_muted)
                            .child("spike · gpui-kit"),
                    ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(p.ink_muted)
                    .child(format!(
                        "{} tool · {} provider · {} model",
                        self.entries.len(),
                        self.entries.iter().map(|e| e.provider_count).sum::<usize>(),
                        self.total_models()
                    )),
            )
            .child(
                div()
                    .id("open-settings")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(30.))
                    .rounded(px(8.))
                    .cursor_pointer()
                    .text_size(px(16.))
                    .text_color(if self.nav == Nav::Settings {
                        p.brand_text
                    } else {
                        p.ink_secondary
                    })
                    .when(self.nav == Nav::Settings, |s| s.bg(p.brand_soft))
                    .when(self.nav != Nav::Settings, |s| s.hover(|s| s.bg(p.canvas)))
                    .child(IconName::Settings)
                    .on_click(cx.listener(|this, _, _, cx| this.set_nav(Nav::Settings, cx))),
            )
    }

    fn sidebar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut items = v_flex().gap(px(2.)).px(px(10.)).pb(px(10.));
        for (ix, entry) in self.entries.iter().enumerate() {
            let selected = ix == self.selected;
            items = items.child(
                h_flex()
                    .id(("harness", ix))
                    .px(px(10.))
                    .py(px(8.))
                    .gap(px(9.))
                    .items_center()
                    .rounded(px(9.))
                    .when(selected, |s| s.bg(p.surface).shadow_2xs())
                    .when(!selected, |s| s.hover(|s| s.bg(p.canvas)))
                    .on_click(cx.listener(move |this, _, _, cx| this.select(ix, cx)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(22.))
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .bg(if selected { p.brand_soft } else { p.neutral_bg })
                            .text_color(if selected { p.brand_text } else { p.ink_muted })
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .child(monogram(&entry.name, &entry.id)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().text_size(px(12.5)).child(entry.name.clone()))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(p.ink_muted)
                                    .truncate()
                                    .child(if entry.installed {
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
                            .text_size(px(10.5))
                            .text_color(if selected { p.brand_text } else { p.ink_muted })
                            .font_weight(FontWeight::MEDIUM)
                            .child(entry.provider_count.to_string()),
                    ),
            );
        }


        v_flex()
            .w(px(238.))
            .flex_shrink_0()
            .bg(p.surface_muted)
            .border_r_1()
            .border_color(p.line)
            .child(
                h_flex()
                    .px(px(20.))
                    .pt(px(14.))
                    .pb(px(8.))
                    .gap(px(6.))
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.ink_muted)
                            .child("HARNESS"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(p.ink_muted)
                            .child(format!("· {}", self.entries.len())),
                    ),
            )
            .child(items)
            .child(div().flex_1())
            .child(
                v_flex()
                    .px(px(20.))
                    .py(px(12.))
                    .gap(px(2.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_family(palette::MONO)
                            .text_color(p.ink_muted)
                            .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(p.ink_muted)
                            .child("read-only spike"),
                    ),
            )
    }
}

/// Petunjuk singkat di kanan tab: apa yang bisa dilakukan di layar itu.
fn active_hint(nav: Nav) -> &'static str {
    match nav {
        Nav::Providers => "klik … pada kartu untuk aksi",
        Nav::Add => "3 langkah · berakhir di pratinjau",
        Nav::Diff => "read-only · Apply nonaktif",
        Nav::History => "dialog restore tidak menulis apa pun",
        Nav::Settings => "tema & bahasa tersimpan di sesi",
    }
}
