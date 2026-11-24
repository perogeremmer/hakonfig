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
use hakonfig_core::api;
use hakonfig_core::engine::model::{EditOp, Notice, ProviderSummary};

use crate::data::{catalog, load_all, load_notices, probe_versions, HarnessEntry, Metrics};
use crate::logos;
use crate::palette::{self, Palette};
use crate::state;
use crate::views::{
    add::{AddEvent, AddView},
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
            Nav::History => "Riwayat",
            Nav::Settings => "Pengaturan",
        }
    }

    #[allow(dead_code)] // dipakai view lain untuk id elemen
    pub fn slug(self) -> &'static str {
        match self {
            Nav::Providers => "providers",
            Nav::Add => "add",
            Nav::History => "history",
            Nav::Settings => "settings",
        }
    }
}

/// Dialog konfirmasi berbasis overlay milik `App`. Dialog bawaan gpui-kit
/// butuh root `Root`, sementara spike membuka `App` langsung sebagai root.
pub struct ConfirmState {
    pub title: String,
    pub body: String,
    pub ok_text: &'static str,
    pub success: &'static str,
    pub op: EditOp,
}

pub struct App {
    pub entries: Vec<HarnessEntry>,
    pub selected: usize,
    pub nav: Nav,
    /// Harness yang dipilih lewat picker meski belum punya config; dipersist
    /// oleh `state` supaya daftar kiri tidak kosong lagi setelah restart.
    pub added: Vec<String>,
    /// Picker "Tambah harness baru" sedang terbuka.
    pub picker: bool,
    /// Menu aksi provider: (id, nama).
    pub actions: Option<(String, String)>,
    pub confirm: Option<ConfirmState>,
    pub query: Entity<InputState>,
    pub query_text: String,
    pub filter: Filter,
    /// Notice milik harness aktif saja (dibaca di background).
    pub notices: Vec<Notice>,
    pub load_ms: u128,
    pub data_ready_ms: Option<u128>,
    /// Picker model default sedang terbuka; `model_provider` membatasi ke satu
    /// provider (dipakai dari menu kartu provider).
    pub model_picker: bool,
    pub model_provider: Option<String>,
    pub model_query: Entity<InputState>,
    pub model_query_text: String,
    /// Pesan yang menunggu ditampilkan sebagai toast di render berikutnya.
    toast: Option<String>,
    first_frame: bool,
    pub add: Entity<AddView>,
    pub history: Entity<HistoryView>,
    pub settings: Entity<SettingsView>,
    _subs: Vec<Subscription>,
    _load: Option<Task<()>>,
    _load_notices: Option<Task<()>>,
    _apply: Option<Task<()>>,
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
        let model_query = cx.new(|cx| InputState::new(window, cx).placeholder("Cari model…"));
        let mut _subs = vec![
            cx.subscribe_in(&query, window, {
                let query = query.clone();
                move |this, _, ev: &InputEvent, _, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.query_text = query.read(cx).value().to_string();
                        cx.notify();
                    }
                }
            }),
            cx.subscribe_in(&model_query, window, {
                let model_query = model_query.clone();
                move |this, _, ev: &InputEvent, _, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.model_query_text = model_query.read(cx).value().to_string();
                        cx.notify();
                    }
                }
            }),
        ];

        let added = state::added();
        let (entries, load_ms) = load_all(&added);
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
        let history = cx.new(|cx| HistoryView::new(window, cx));
        let settings = cx.new(|cx| SettingsView::new(window, cx));
        _subs.push(cx.subscribe(&add, |this, _, event: &AddEvent, cx| {
            let AddEvent::Applied { message } = event;
            this.toast = Some(message.clone());
            this.nav = Nav::Providers;
            this.reload(cx);
            cx.notify();
        }));

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
            added,
            picker: false,
            actions: None,
            confirm: None,
            query,
            query_text: String::new(),
            filter: Filter::All,
            notices: Vec::new(),
            load_ms,
            data_ready_ms,
            model_picker: false,
            model_provider: None,
            model_query,
            model_query_text: String::new(),
            toast: None,
            first_frame: false,
            add,
            history,
            settings,
            _subs,
            _load,
            _load_notices: None,
            _apply: None,
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

    pub fn open_picker(&mut self, cx: &mut Context<Self>) {
        self.picker = true;
        self.model_picker = false;
        cx.notify();
    }

    pub fn close_picker(&mut self, cx: &mut Context<Self>) {
        self.picker = false;
        cx.notify();
    }

    /// Pilih satu harness dari picker. Harness yang sudah punya config hanya
    /// dipilih; yang belum ada ditambahkan ke daftar dan ke state.
    pub fn add_harness(&mut self, id: String, cx: &mut Context<Self>) {
        self.picker = false;
        let known = self.entries.iter().any(|e| e.id == id);
        let name = catalog()
            .into_iter()
            .find(|h| h.id == id)
            .map(|h| h.name)
            .unwrap_or_else(|| id.clone());

        if !known {
            state::add(&id);
            if !self.added.iter().any(|added| added == &id) {
                self.added.push(id.clone());
            }
            let added = self.added.clone();
            let (entries, ms) = load_all(&added);
            self.entries = entries;
            self.load_ms = ms;
        }

        if let Some(ix) = self.entries.iter().position(|e| e.id == id) {
            self.selected = ix;
        }
        self.nav = Nav::Providers;
        self.sync_views(cx);
        if !known {
            self.toast = Some(format!("{name} ditambahkan"));
        }
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.data_ready_ms = None;
        let added = self.added.clone();
        let task = cx.spawn(async move |this, cx| {
            let (entries, ms) = cx.background_spawn(async move { load_all(&added) }).await;
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

    /// Baca ulang config setelah menulis, tanpa kehilangan harness terpilih.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let keep = self.active_id();
        let added = self.added.clone();
        let versions: Vec<(String, Option<String>, u128)> = self
            .entries
            .iter()
            .map(|entry| (entry.id.clone(), entry.version.clone(), entry.timings.1))
            .collect();
        let (mut entries, ms) = load_all(&added);
        for entry in &mut entries {
            if let Some((_, version, probe_ms)) =
                versions.iter().find(|(id, _, _)| id == &entry.id)
            {
                entry.version = version.clone();
                entry.timings.1 = *probe_ms;
            }
        }
        self.entries = entries;
        self.load_ms = ms;
        self.data_ready_ms = PROCESS_START.get().map(|s| s.elapsed().as_millis());
        self.selected = self
            .entries
            .iter()
            .position(|entry| entry.id == keep)
            .unwrap_or(0);
        self.sync_views(cx);
        cx.notify();
    }

    /// Tulis satu `EditOp` di background. Core membuat backup sebelum menulis,
    /// jadi setiap aksi di sini tetap bisa di-restore lewat layar Riwayat.
    pub fn apply_op(&mut self, op: EditOp, success: &'static str, cx: &mut Context<Self>) {
        let harness = self.active_id();
        if harness.is_empty() {
            return;
        }
        self._apply = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { api::apply_edit(harness, op) })
                .await;
            this.update(cx, |this, cx| match result {
                Ok(applied) => {
                    let mut message =
                        format!("{success} · {} file ditulis", applied.files_written.len());
                    if !applied.warnings.is_empty() {
                        message.push_str(&format!(" · {} peringatan", applied.warnings.len()));
                    }
                    this.toast = Some(message);
                    this.reload(cx);
                }
                Err(error) => {
                    this.toast = Some(format!("Gagal: {error}"));
                    cx.notify();
                }
            })
            .ok();
        }));
        cx.notify();
    }

    /// Tampilkan konfirmasi overlay untuk aksi yang menghapus data.
    pub fn confirm_op(
        &mut self,
        title: String,
        body: String,
        ok_text: &'static str,
        op: EditOp,
        success: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.actions = None;
        self.confirm = Some(ConfirmState {
            title,
            body,
            ok_text,
            success,
            op,
        });
        cx.notify();
    }

    /// Buka picker model default; `provider` membatasi daftar ke satu provider.
    pub fn open_model_picker(
        &mut self,
        provider: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker = false;
        self.model_picker = true;
        self.model_provider = provider;
        self.model_query_text.clear();
        self.model_query
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    pub fn close_model_picker(&mut self, cx: &mut Context<Self>) {
        self.model_picker = false;
        cx.notify();
    }

    pub fn choose_default_model(
        &mut self,
        provider_id: String,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        self.model_picker = false;
        self.apply_op(
            EditOp::SetDefault {
                provider_id,
                model_id,
            },
            "Model default diperbarui",
            cx,
        );
    }

    /// Masuk ke form Tambah provider dengan data provider yang sudah ada.
    pub fn edit_provider(
        &mut self,
        provider: &ProviderSummary,
        step: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add
            .update(cx, |view, cx| view.begin_edit(provider, step, window, cx));
        self.nav = Nav::Add;
        cx.notify();
    }

    /// Buka menu aksi untuk satu provider (overlay shell, bukan dialog gpui).
    pub fn open_provider_actions(&mut self, provider: &ProviderSummary, cx: &mut Context<Self>) {
        self.actions = Some((provider.id.clone(), provider.name.clone()));
        cx.notify();
    }

    /// Dispatcher untuk baris menu kartu provider di layar Providers.
    pub fn provider_action(
        &mut self,
        provider_id: String,
        code: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self
            .active()
            .and_then(|entry| entry.snapshot.as_ref())
            .and_then(|snapshot| {
                snapshot
                    .providers
                    .iter()
                    .find(|provider| provider.id == provider_id)
            })
            .cloned()
        else {
            return;
        };
        self.actions = None;
        match code {
            0 => self.edit_provider(&provider, 2, window, cx),
            1 => self.edit_provider(&provider, 3, window, cx),
            2 => self.open_model_picker(Some(provider.id.clone()), window, cx),
            3 if provider.can_toggle => {
                let success = if provider.enabled {
                    "Provider dinonaktifkan"
                } else {
                    "Provider diaktifkan"
                };
                self.apply_op(
                    EditOp::SetEnabled {
                        provider_id: provider.id.clone(),
                        enabled: !provider.enabled,
                    },
                    success,
                    cx,
                );
            }
            4 => {
                let title = format!("Lupakan kredensial {}", provider.name);
                let body = format!(
                    "Kredensial tersimpan untuk `{}` akan dihapus. Harness berhenti \
                     menawarkan provider ini bila tidak ada sumber key lain. File \
                     config tetap di-backup.",
                    provider.id
                );
                self.confirm_op(
                    title,
                    body,
                    "Lupakan",
                    EditOp::ForgetKey {
                        provider_id: provider.id.clone(),
                    },
                    "Kredensial dilupakan",
                    cx,
                );
            }
            5 => {
                let title = format!("Hapus {}", provider.name);
                let body = format!(
                    "Entry provider `{}` beserta modelnya akan dihapus dari config. \
                     Backup dibuat sebelum penulisan.",
                    provider.id
                );
                self.confirm_op(
                    title,
                    body,
                    "Hapus",
                    EditOp::Remove {
                        provider_id: provider.id.clone(),
                    },
                    "Provider dihapus",
                    cx,
                );
            }
            _ => {}
        }
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
            Nav::History => self.history.clone().into_any_element(),
            Nav::Settings => self.settings.clone().into_any_element(),
        };

        let mut root = v_flex()
            .relative()
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
            );
        if self.picker {
            root = root.child(self.picker_overlay(&p, cx));
        }
        if self.model_picker {
            root = root.child(self.model_picker_overlay(&p, cx));
        }
        if let Some((provider_id, provider_name)) = self.actions.clone() {
            root = root.child(self.actions_overlay(provider_id, provider_name, &p, cx));
        }
        if self.confirm.is_some() {
            root = root.child(self.confirm_overlay(&p, cx));
        }
        root
    }
}

impl App {
    /// Layar selain Providers dibuka dari kotak aksi di layar Providers atau
    /// dari ikon cog (Pengaturan), jadi jalan pulangnya harus selalu ada.
    fn back_bar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let editing = self.nav == Nav::Add && self.add.read(cx).is_editing();
        let label = if editing {
            "Ubah provider"
        } else {
            self.nav.label()
        };
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
                    .child(format!("· {label}")),
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
        let dark = palette::dark(cx);
        let mut items = v_flex().gap(px(2.)).px(px(10.)).pb(px(4.));
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
                            .child(logos::logo(cx, dark, &entry.id, 15.)),
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

        // Instalasi baru: tidak ada config dan belum ada pilihan, jadi kolom
        // kiri benar-benar kosong dan satu-satunya jalan masuk adalah tombol.
        let empty = self.entries.is_empty();
        let body = if empty {
            v_flex()
                .flex_1()
                .justify_center()
                .px(px(18.))
                .gap(px(6.))
                .items_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(34.))
                        .rounded(px(10.))
                        .bg(p.neutral_bg)
                        .text_color(p.ink_muted)
                        .text_size(px(16.))
                        .child(IconName::SquareTerminal),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Belum ada harness"),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(p.ink_muted)
                        .text_center()
                        .child("Pilih tool yang mau diatur lewat tombol di bawah."),
                )
                .into_any_element()
        } else {
            items.into_any_element()
        };

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
            .child(body)
            .when(!empty, |s| s.child(div().flex_1()))
            .child(
                h_flex()
                    .id("add-harness")
                    .mx(px(10.))
                    .mb(px(10.))
                    .when(!empty, |s| s.mt(px(4.)))
                    .when(empty, |s| s.mb(px(16.)))
                    .px(px(10.))
                    .py(px(8.))
                    .gap(px(7.))
                    .items_center()
                    .rounded(px(9.))
                    .border_1()
                    .border_dashed()
                    .border_color(p.line_strong)
                    .text_size(px(11.5))
                    .text_color(if empty { p.brand_text } else { p.ink_secondary })
                    .cursor_pointer()
                    .hover(|s| s.bg(p.surface).border_color(p.brand))
                    .child(IconName::Plus)
                    .child("Tambah harness baru")
                    .on_click(cx.listener(|this, _, _, cx| this.open_picker(cx))),
            )
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
                            .child("config manager · backup otomatis"),
                    ),
            )
    }

    /// Modal pemilih harness. Menampilkan seluruh harness yang dikenal core;
    /// yang sudah terpasang ditandai, yang sudah ada di sidebar tidak
    /// diduplikasi saat diklik.
    fn picker_overlay(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = palette::dark(cx);
        let mut options = v_flex().gap(px(4.));
        for info in catalog() {
            let id = info.id.clone();
            let already = self.entries.iter().any(|e| e.id == info.id);
            let selected = self
                .active()
                .map(|e| e.id == info.id)
                .unwrap_or(false);
            let chip = if already {
                ("sudah di daftar", p.brand_soft, p.brand_text)
            } else if info.installed {
                ("terpasang", p.ok_bg, p.ok_text)
            } else {
                ("belum terpasang", p.neutral_bg, p.neutral_text)
            };

            options = options.child(
                h_flex()
                    .id(ElementId::Name(format!("pick-{id}").into()))
                    .px(px(10.))
                    .py(px(9.))
                    .gap(px(10.))
                    .items_center()
                    .rounded(px(9.))
                    .border_1()
                    .border_color(if selected { p.brand } else { p.line })
                    .cursor_pointer()
                    .hover(|s| s.bg(p.canvas))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(26.))
                            .flex_shrink_0()
                            .rounded(px(7.))
                            .bg(if selected { p.brand_soft } else { p.neutral_bg })
                            .child(logos::logo(cx, dark, &info.id, 18.)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(1.))
                            .child(
                                h_flex()
                                    .gap(px(6.))
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(info.name.clone()),
                                    )
                                    .when(info.experimental, |s| {
                                        s.child(
                                            div()
                                                .px(px(5.))
                                                .py(px(1.))
                                                .rounded_full()
                                                .bg(p.warn_bg)
                                                .text_color(p.warn_text)
                                                .text_size(px(9.5))
                                                .child("eksperimental"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .text_color(p.ink_muted)
                                    .child(info.notes.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .px(px(6.))
                            .py(px(2.))
                            .rounded_full()
                            .bg(chip.1)
                            .text_color(chip.2)
                            .text_size(px(9.5))
                            .child(chip.0),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.add_harness(id.clone(), cx)
                    })),
            );
        }

        div()
            .id("picker-backdrop")
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(black().opacity(0.4))
            .on_click(cx.listener(|this, _, _, cx| this.close_picker(cx)))
            .child(
                v_flex()
                    .id("picker-card")
                    .w(px(430.))
                    .gap(px(10.))
                    .p(px(14.))
                    .rounded(px(14.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Tambah harness baru"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(p.ink_muted)
                                            .child(
                                                "Pilih tool yang mau diatur. Yang sudah punya \
                                                 config langsung terdeteksi.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("picker-close")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(24.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .text_color(p.ink_muted)
                                    .hover(|s| s.bg(p.canvas).text_color(p.ink))
                                    .child(IconName::Close)
                                    .on_click(cx.listener(|this, _, _, cx| this.close_picker(cx))),
                            ),
                    )
                    .child(options),
            )
    }

    /// Overlay pemilih model default. Daftar diambil dari snapshot harness
    /// aktif, jadi yang bisa dipilih hanya model yang benar-benar ada di config.
    fn model_picker_overlay(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.model_query_text.trim().to_lowercase();
        let snapshot = self.active().and_then(|entry| entry.snapshot.as_ref());
        let current = snapshot.and_then(|s| s.default_provider.clone().zip(s.default_model.clone()));
        let filter_name = self.model_provider.as_ref().and_then(|id| {
            snapshot.and_then(|s| {
                s.providers
                    .iter()
                    .find(|provider| &provider.id == id)
                    .map(|provider| provider.name.clone())
            })
        });

        let mut rows = v_flex().gap(px(1.));
        let mut shown = 0usize;
        if let Some(snapshot) = snapshot {
            for provider in &snapshot.providers {
                if let Some(filter) = &self.model_provider {
                    if &provider.id != filter {
                        continue;
                    }
                }
                for model in &provider.models {
                    let hay = format!(
                        "{} {} {} {}",
                        provider.id,
                        provider.name,
                        model.id,
                        model.name.clone().unwrap_or_default()
                    )
                    .to_lowercase();
                    if !query.is_empty() && !hay.contains(&query) {
                        continue;
                    }
                    shown += 1;
                    let is_current = current
                        .as_ref()
                        .is_some_and(|(default_provider, default_model)| {
                            default_provider == &provider.id && default_model == &model.id
                        });
                    let provider_id = provider.id.clone();
                    let model_id = model.id.clone();
                    rows = rows.child(
                        h_flex()
                            .id(ElementId::Name(
                                format!("pick-model-{}-{}", provider.id, model.id).into(),
                            ))
                            .px(px(10.))
                            .py(px(6.))
                            .gap(px(8.))
                            .items_center()
                            .rounded(px(8.))
                            .cursor_pointer()
                            .when(is_current, |s| s.bg(p.brand_soft))
                            .when(!is_current, |s| s.hover(|s| s.bg(p.canvas)))
                            .child(
                                div()
                                    .w(px(118.))
                                    .flex_shrink_0()
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(p.ink_muted)
                                    .child(provider.name.clone()),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.))
                                            .font_family(palette::MONO)
                                            .child(model.id.clone()),
                                    )
                                    .when_some(model.name.clone(), |s, name| {
                                        s.child(
                                            div()
                                                .truncate()
                                                .text_size(px(9.5))
                                                .text_color(p.ink_muted)
                                                .child(name),
                                        )
                                    }),
                            )
                            .when(is_current, |s| {
                                s.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.brand_text)
                                        .child(IconName::Check),
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.choose_default_model(
                                    provider_id.clone(),
                                    model_id.clone(),
                                    cx,
                                )
                            })),
                    );
                }
            }
        }

        if shown == 0 {
            rows = rows.child(
                v_flex()
                    .py(px(24.))
                    .items_center()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Tidak ada model yang cocok."),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_muted)
                            .child("Kosongkan pencarian atau pilih provider lain."),
                    ),
            );
        }

        let title = match &filter_name {
            Some(name) => format!("Model default · {name}"),
            None => "Pilih model default".to_string(),
        };

        div()
            .id("model-picker-backdrop")
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(black().opacity(0.4))
            .on_click(cx.listener(|this, _, _, cx| this.close_model_picker(cx)))
            .child(
                v_flex()
                    .id("model-picker-card")
                    .w(px(520.))
                    .gap(px(10.))
                    .p(px(14.))
                    .rounded(px(14.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_start()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::BOLD)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(p.ink_muted)
                                            .child(
                                                "Model ini yang dipakai harness saat tidak \
                                                 ada model yang ditentukan per sesi.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("model-picker-close")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(24.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .text_color(p.ink_muted)
                                    .hover(|s| s.bg(p.canvas).text_color(p.ink))
                                    .child(IconName::Close)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_model_picker(cx)),
                                    ),
                            ),
                    )
                    .child(palette::text_input(
                        p,
                        &self.model_query,
                        IconName::Search,
                    ))
                    .child(
                        v_flex()
                            .id("model-picker-scroll")
                            .max_h(px(420.))
                            .overflow_y_scroll()
                            .child(rows),
                    ),
            )
    }

    /// Overlay menu aksi provider. Barisnya sama dengan `provider_actions`,
    /// dipakai bersama dispatcher agar tidak ada aksi yang hanya tampil.
    fn actions_overlay(
        &self,
        provider_id: String,
        provider_name: String,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let provider = self
            .active()
            .and_then(|entry| entry.snapshot.as_ref())
            .and_then(|snapshot| {
                snapshot
                    .providers
                    .iter()
                    .find(|provider| provider.id == provider_id)
            })
            .cloned();

        let mut rows = v_flex().gap(px(2.));
        if let Some(provider) = &provider {
            for (code, label, hint) in provider_actions(provider) {
                let id = provider_id.clone();
                rows = rows.child(
                    v_flex()
                        .id(ElementId::Name(format!("action-{code}-{id}").into()))
                        .px(px(9.))
                        .py(px(6.))
                        .rounded(px(8.))
                        .cursor_pointer()
                        .hover(|s| s.bg(p.canvas))
                        .child(div().text_size(px(12.5)).child(label))
                        .child(
                            div()
                                .text_size(px(9.5))
                                .font_family(palette::MONO)
                                .text_color(p.ink_muted)
                                .child(hint),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.provider_action(id.clone(), code, window, cx)
                        })),
                );
            }
        }

        div()
            .id("actions-backdrop")
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(black().opacity(0.4))
            .on_click(cx.listener(|this, _, _, cx| {
                this.actions = None;
                cx.notify();
            }))
            .child(
                v_flex()
                    .id("actions-card")
                    .w(px(430.))
                    .gap(px(6.))
                    .p(px(14.))
                    .rounded(px(14.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{provider_name} · {provider_id}")),
                            )
                            .child(
                                div()
                                    .id("actions-close")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(24.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .text_color(p.ink_muted)
                                    .hover(|s| s.bg(p.canvas).text_color(p.ink))
                                    .child(IconName::Close)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.actions = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.ink_muted)
                            .child(
                                "Aksi langsung menulis config; backup dibuat \
                                 otomatis sebelum penulisan.",
                            ),
                    )
                    .child(rows),
            )
    }

    /// Overlay konfirmasi untuk aksi destruktif.
    fn confirm_overlay(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(state) = &self.confirm else {
            return div().into_any_element();
        };
        let title = state.title.clone();
        let body = state.body.clone();
        let ok_text = state.ok_text;
        let success = state.success;
        let op = state.op.clone();

        div()
            .id("confirm-backdrop")
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(black().opacity(0.4))
            .on_click(cx.listener(|this, _, _, cx| {
                this.confirm = None;
                cx.notify();
            }))
            .child(
                v_flex()
                    .id("confirm-card")
                    .w(px(440.))
                    .gap(px(10.))
                    .p(px(14.))
                    .rounded(px(14.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.ink_secondary)
                            .child(body),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .justify_end()
                            .child(
                                palette::soft_button(p, "confirm-cancel", "Batal").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.confirm = None;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(
                                palette::brand_button(p, "confirm-ok", ok_text).on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.confirm = None;
                                        this.apply_op(op.clone(), success, cx);
                                    }),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// Baris menu yang tersedia untuk satu provider. Dipakai overlay aksi supaya
/// aksi yang tidak relevan (mis. key env tidak bisa dilupakan) tidak muncul.
fn provider_actions(provider: &ProviderSummary) -> Vec<(u8, String, String)> {
    let mut actions: Vec<(u8, String, String)> = vec![
        (
            0,
            "Edit provider".into(),
            "nama, endpoint, key, dan model".into(),
        ),
        (
            1,
            "Kelola model".into(),
            format!("{} model terpasang", provider.models.len()),
        ),
    ];
    if !provider.models.is_empty() {
        actions.push((
            2,
            "Jadikan model default".into(),
            "pilih model default harness".into(),
        ));
    }
    if provider.can_toggle {
        actions.push((
            3,
            if provider.enabled {
                "Nonaktifkan".to_string()
            } else {
                "Aktifkan".to_string()
            },
            "EditOp::SetEnabled · reversible".into(),
        ));
    }
    if provider.key_stored {
        actions.push((
            4,
            "Lupakan kredensial".into(),
            "hapus key tersimpan · backup dulu".into(),
        ));
    }
    actions.push((
        5,
        "Hapus provider".into(),
        "hapus entry dari config · backup dulu".into(),
    ));
    actions
}

/// Petunjuk singkat di kanan tab: apa yang bisa dilakukan di layar itu.
fn active_hint(nav: Nav) -> &'static str {
    match nav {
        Nav::Providers => "klik … pada kartu untuk aksi",
        Nav::Add => "3 langkah · pratinjau lalu tulis",
        Nav::History => "restore belum aktif",
        Nav::Settings => "tema & bahasa tersimpan di sesi",
    }
}
