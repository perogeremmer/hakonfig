// =============================================================================
// Rekonfig — frontend (vanilla JS, tanpa framework)
// Kontrak dengan backend: docs/UI-CONTRACT.md
// =============================================================================

import { esc, renderDiff } from "./diff.js";

const invoke = window.__TAURI__?.core?.invoke;

// ----------------------------------------------------------------- konstanta

const ROLE_LABEL = {
  models: "Katalog model",
  auth: "Kredensial",
  settings: "Preferensi",
  config: "Konfigurasi",
  env: "Environment",
};

const API_OPTIONS = [
  ["openai-completions", "openai-completions (paling umum)"],
  ["openai-responses", "openai-responses"],
  ["anthropic-messages", "anthropic-messages"],
  ["google-generative-ai", "google-generative-ai"],
];

// ------------------------------------------------------------------- helper

const $ = (selector, root = document) => root.querySelector(selector);

// Warna avatar provider — deterministik dari id, supaya stabil antar render.
const AVATAR_TINTS = [
  ["bg-tint-ok", "text-emerald-700"],
  ["bg-tint-info", "text-blue-700"],
  ["bg-amber-100", "text-amber-700"],
  ["bg-peach", "text-orange-700"],
  ["bg-violet-100", "text-violet-700"],
  ["bg-rose-100", "text-rose-700"],
];

function avatarTint(id) {
  let hash = 0;
  for (const char of String(id)) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return AVATAR_TINTS[hash % AVATAR_TINTS.length];
}

function avatarInitials(id) {
  const cleaned = String(id).replace(/[^a-zA-Z0-9]/g, "");
  return (cleaned.slice(0, 2) || "?").toUpperCase();
}

function fmtTokens(value) {
  if (!value) return "—";
  if (value >= 1048576) return `${(value / 1048576).toFixed(1).replace(".0", "")}M`;
  if (value >= 1000) return `${Math.round(value / 1000)}K`;
  return String(value);
}

function fmtDate(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString("id-ID", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function busy(isBusy) {
  state.busy = isBusy;
  document
    .querySelectorAll("[data-action]")
    .forEach((element) => {
      if (element.dataset.alwaysOn !== "1") element.disabled = isBusy;
    });
}

// ---------------------------------------------------------------------- API

const api = {
  harnesses: () => invoke("list_harnesses"),
  presets: () => invoke("list_presets"),
  read: (harnessId) => invoke("read_harness", { harnessId }),
  preview: (harnessId, op) => invoke("preview_edit", { harnessId, op }),
  apply: (harnessId, op) => invoke("apply_edit", { harnessId, op }),
  backups: (harnessId) => invoke("list_backups", { harnessId }),
  restore: (harnessId, backupId) => invoke("restore_backup", { harnessId, backupId }),
  test: (baseUrl, apiName, key) => invoke("test_connection", { baseUrl, api: apiName, key }),
  reveal: (path) => invoke("open_in_file_manager", { path }),
};

// -------------------------------------------------------------------- state

const state = {
  view: "home", // "home" | "harness"
  harnesses: [],
  activeId: null,
  snapshot: null,
  presets: [],
  backups: [],
  busy: false,
  expanded: new Set(),
  loadError: null,
};

// -------------------------------------------------------------------- toast

function toast(message, variant = "ok", detail = "") {
  const root = $("#toast-root");
  const palette = {
    ok: "border-emerald-300 bg-tint-ok text-emerald-800",
    err: "border-rose-300 bg-rose-50 text-rose-800",
    info: "border-stone-300 bg-white text-ink",
  };
  const element = document.createElement("div");
  element.className = `pointer-events-auto selectable rounded-xl border px-3 py-2 text-xs shadow-lg backdrop-blur ${
    palette[variant] ?? palette.info
  }`;
  element.innerHTML = `<p class="font-medium">${esc(message)}</p>${
    detail ? `<p class="mt-1 font-mono text-[11px] break-all opacity-80">${esc(detail)}</p>` : ""
  }`;
  root.append(element);
  setTimeout(() => element.remove(), variant === "err" ? 9000 : 6000);
}

// ------------------------------------------------------------------- modal

function openModal({ title, subtitle = "", body, footer, wide = false }) {
  const root = $("#modal-root");
  const overlay = document.createElement("div");
  overlay.className =
    "fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-stone-900/25 p-6 backdrop-blur-sm";

  overlay.innerHTML = `
    <div class="my-6 w-full ${
      wide ? "max-w-4xl" : "max-w-2xl"
    } rounded-2xl border border-line bg-white shadow-xl">
      <div class="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
        <div class="min-w-0">
          <h3 class="text-sm font-semibold">${esc(title)}</h3>
          ${subtitle ? `<p class="mt-0.5 text-xs text-sub">${esc(subtitle)}</p>` : ""}
        </div>
        <button data-action class="rounded px-2 py-1 text-sub hover:bg-sand hover:text-ink" data-close>✕</button>
      </div>
      <div class="max-h-[64vh] overflow-y-auto px-5 py-4">${body}</div>
      <div class="flex items-center justify-end gap-2 border-t border-line px-5 py-3">${footer}</div>
    </div>`;

  root.append(overlay);

  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey);
  };
  const onKey = (event) => {
    if (event.key === "Escape") close();
  };
  document.addEventListener("keydown", onKey);
  overlay.addEventListener("click", (event) => {
    if (event.target === overlay) close();
  });
  overlay.querySelectorAll("[data-close]").forEach((button) => {
    button.addEventListener("click", close);
  });

  const firstInput = overlay.querySelector("input, select, textarea");
  if (firstInput) firstInput.focus();

  return { overlay, close, root: overlay.firstElementChild };
}

function confirmModal({ title, message, confirmLabel = "Lanjutkan", danger = false }) {
  return new Promise((resolve) => {
    const { close, overlay } = openModal({
      title,
      body: `<p class="selectable text-sm leading-relaxed text-stone-700">${esc(message)}</p>`,
      footer: `
        <button data-action class="rounded-xl border border-stone-300 px-3 py-1.5 text-xs text-stone-700 hover:bg-sand" data-cancel>Batal</button>
        <button data-action class="rounded-xl px-3 py-1.5 text-xs font-medium text-white ${
          danger ? "bg-rose-600 hover:bg-rose-500" : "bg-brand hover:bg-brand/90"
        }" data-confirm>${esc(confirmLabel)}</button>`,
    });
    overlay.querySelector("[data-cancel]").addEventListener("click", () => {
      close();
      resolve(false);
    });
    overlay.querySelector("[data-confirm]").addEventListener("click", () => {
      close();
      resolve(true);
    });
  });
}

// ============================================================================
// RENDER
// ============================================================================

const LOGO_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="h-5 w-5"><path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h9M17 18h3"/><circle cx="15" cy="6" r="2" fill="currentColor" stroke="none"/><circle cx="9" cy="12" r="2" fill="currentColor" stroke="none"/><circle cx="15" cy="18" r="2" fill="currentColor" stroke="none"/></svg>`;
const ARROW_LEFT_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4"><path d="M19 12H5m0 0l6-6m-6 6l6 6"/></svg>`;

function renderHomeHeader() {
  $("#topbar").innerHTML = `
    <div class="mx-auto flex max-w-5xl items-center gap-3 px-6 py-3">
      <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-brand text-white shadow-sm">${LOGO_SVG}</div>
      <div class="min-w-0">
        <h1 class="text-base font-semibold tracking-tight text-ink">Rekonfig</h1>
        <p class="text-xs text-sub">Kelola provider AI-mu</p>
      </div>
    </div>`;
}

function renderHarnessHeader() {
  const harness = state.harnesses.find((item) => item.id === state.activeId);
  $("#topbar").innerHTML = `
    <div class="mx-auto flex max-w-5xl items-center justify-between gap-4 px-6 py-3">
      <div class="flex min-w-0 items-center gap-2.5">
        <button data-action data-home title="Kembali ke daftar harness"
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border border-line bg-white text-sub transition hover:bg-sand hover:text-ink">${ARROW_LEFT_SVG}</button>
        <div class="min-w-0">
          <h2 class="truncate text-base font-semibold text-ink">${esc(harness?.name ?? "…")}</h2>
          <p class="truncate text-xs text-sub">${esc(harness?.notes ?? "")}</p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <button data-action data-refresh class="rounded-xl border border-stone-300 px-2.5 py-1.5 text-xs text-stone-700 hover:bg-sand">Muat ulang</button>
        <button data-action data-backups class="rounded-xl border border-stone-300 px-2.5 py-1.5 text-xs text-stone-700 hover:bg-sand">Backup</button>
        <button data-action data-add class="rounded-xl bg-brand px-3 py-1.5 text-xs font-medium text-white shadow-sm hover:bg-brand/90">+ Tambah Provider</button>
      </div>
    </div>`;

  $("[data-home]", $("#topbar")).addEventListener("click", goHome);
  $("[data-refresh]", $("#topbar")).addEventListener("click", () => loadSnapshot(state.activeId));
  $("[data-backups]", $("#topbar")).addEventListener("click", () => openBackups());
  $("[data-add]", $("#topbar")).addEventListener("click", () => openProviderForm("add"));
}

function harnessCard(harness) {
  const [tint, tintText] = avatarTint(harness.id);
  const configured = harness.connectedCount > 0;

  // Badge: hijau bila sudah dikonfigurasi, abu bila belum.
  const badge = configured
    ? `<span class="inline-flex items-center gap-1 rounded-full bg-tint-ok px-2 py-0.5 text-[10px] font-medium text-emerald-700">
         <span class="h-1.5 w-1.5 rounded-full bg-brand"></span>terkonfigurasi
       </span>`
    : `<span class="inline-flex items-center gap-1 rounded-full bg-sand px-2 py-0.5 text-[10px] font-medium text-sub">
         <span class="h-1.5 w-1.5 rounded-full bg-mute"></span>belum disetel
       </span>`;

  const experimental = harness.experimental
    ? `<span class="rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-medium text-amber-700">eksperimental</span>`
    : "";

  // Keterangan singkat di bawah nama.
  let caption;
  if (harness.readError) {
    caption = `<span class="text-rose-600">config tidak terbaca</span>`;
  } else if (!harness.installed) {
    caption = `Belum ada file config`;
  } else if (harness.connectedCount > 0) {
    caption = `${harness.connectedCount} provider terhubung`;
  } else if (harness.providerCount > 0) {
    caption = `${harness.providerCount} provider, belum ada API key`;
  } else {
    caption = `Belum ada provider`;
  }

  return `
    <div data-harness="${esc(harness.id)}" role="button" tabindex="0"
      class="group flex cursor-pointer items-center gap-3 rounded-2xl border border-line bg-white p-4 shadow-sm transition hover:border-brand/40 hover:shadow-md">
      <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl ${tint} text-sm font-semibold ${tintText}">
        ${esc(avatarInitials(harness.id))}
      </div>
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium text-ink">${esc(harness.name)}</p>
        <p class="truncate text-[11px] text-sub">${caption}</p>
      </div>
      <div class="flex shrink-0 flex-col items-end gap-1">
        ${badge}
        ${experimental}
      </div>
      <span class="shrink-0 text-mute transition group-hover:translate-x-0.5 group-hover:text-brand">→</span>
    </div>`;
}

function renderHome() {
  const container = $("#content");

  if (state.loadError) {
    container.innerHTML = `<div class="mx-auto max-w-5xl rounded-2xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700">${esc(
      state.loadError,
    )}</div>`;
    return;
  }

  if (state.harnesses.length === 0) {
    container.innerHTML = `
      <div class="mx-auto max-w-lg rounded-2xl border border-line bg-white p-6 text-center">
        <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-peach text-xl">🔍</div>
        <p class="text-sm font-medium text-ink">Tidak ada harness yang dikenali</p>
        <p class="mx-auto mt-1 max-w-sm text-xs leading-relaxed text-sub">
          Rekonfig belum menemukan konfigurasi Pi, oh-my-pi, OpenCode, atau Hermes.
          Jalankan salah satunya sekali, lalu muat ulang.
        </p>
      </div>`;
    return;
  }

  container.innerHTML = `
    <div class="mx-auto max-w-5xl">
      <div class="mb-4">
        <h2 class="text-lg font-semibold tracking-tight text-ink">Pilih harness</h2>
        <p class="text-xs text-sub">Klik salah satu untuk melihat provider yang terpasang.</p>
      </div>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        ${state.harnesses.map(harnessCard).join("")}
      </div>

      <div class="mt-5 rounded-2xl border border-line bg-white p-4 shadow-sm">
        <p class="text-xs leading-relaxed text-sub">
          Harness yang belum terpasang tetap bisa dipilih untuk menyiapkan config baru.
          Setiap penulisan selalu membuat backup otomatis.
        </p>
      </div>
    </div>`;

  container.querySelectorAll("[data-harness]").forEach((card) => {
    card.addEventListener("click", () => selectHarness(card.dataset.harness));
    card.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        selectHarness(card.dataset.harness);
      }
    });
  });
  container.querySelectorAll("[data-reveal]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      try {
        await api.reveal(button.dataset.reveal);
      } catch (error) {
        toast("Gagal membuka folder", "err", String(error));
      }
    });
  });
}

function providerCard(provider) {
  const expanded = state.expanded.has(provider.id);
  const [tint, tintText] = avatarTint(provider.id);

  const status = provider.hasKey
    ? `<span class="inline-flex items-center gap-1 rounded-full bg-tint-ok px-2 py-0.5 text-[10px] font-medium text-emerald-700">
         <span class="h-1.5 w-1.5 rounded-full bg-brand"></span>terhubung
       </span>`
    : `<span class="inline-flex items-center gap-1 rounded-full bg-sand px-2 py-0.5 text-[10px] font-medium text-sub">
         belum ada key
       </span>`;

  const models = expanded
    ? `<div class="mt-3 rounded-xl border border-line bg-sand p-2.5">
         ${
           provider.models.length
             ? `<div class="max-h-56 space-y-1 overflow-y-auto">
                  ${provider.models
                    .map(
                      (model) => `<div class="flex items-baseline gap-2 rounded-lg bg-white px-2 py-1">
                        <span class="selectable truncate font-mono text-[11px] text-ink">${esc(model.id)}</span>
                        <span class="ml-auto shrink-0 font-mono text-[10px] text-mute">${fmtTokens(
                          model.contextWindow,
                        )}${model.reasoning ? " · 🧠" : ""}</span>
                      </div>`,
                    )
                    .join("")}
                </div>`
             : `<p class="px-1 py-1 text-[11px] text-sub">Tidak ada model di config ini.</p>`
         }
       </div>`
    : "";

  return `
    <div class="flex flex-col rounded-2xl border border-line bg-white p-4 shadow-sm transition hover:border-stone-300 hover:shadow-md">
      <div class="flex items-start gap-3">
        <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl ${tint} text-xs font-semibold ${tintText}">
          ${esc(avatarInitials(provider.id))}
        </div>
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-ink">${esc(provider.name)}</p>
          <p class="selectable truncate font-mono text-[10px] text-mute">${esc(provider.id)}</p>
        </div>
        ${status}
      </div>

      <p class="selectable mt-3 truncate font-mono text-[11px] text-sub" title="${esc(
        provider.baseUrl ?? "",
      )}">${esc(provider.baseUrl ?? "—")}</p>
      <p class="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-[10px] text-mute">
        ${provider.api ? `<span class="font-mono">${esc(provider.api)}</span>` : ""}
        ${provider.keyDisplay ? `<span>·</span><span class="font-mono">${esc(provider.keyDisplay)}</span>` : ""}
      </p>

      <div class="mt-3 flex items-center justify-between gap-2 border-t border-line pt-2.5">
        <button data-action data-expand="${esc(provider.id)}"
          class="rounded-lg px-2 py-1 text-[11px] text-sub hover:bg-peach hover:text-ink">
          ${provider.models.length} model ${expanded ? "▴" : "▾"}
        </button>
        <div class="flex items-center gap-1">
          <button data-action data-edit="${esc(provider.id)}"
            class="rounded-lg px-2 py-1 text-[11px] text-sub hover:bg-sand hover:text-ink">Ubah</button>
          <button data-action data-remove="${esc(provider.id)}"
            class="rounded-lg px-2 py-1 text-[11px] text-rose-600 hover:bg-rose-50">Hapus</button>
        </div>
      </div>

      ${models}
    </div>`;
}

function renderContent() {
  const container = $("#content");
  const snapshot = state.snapshot;

  if (state.loadError) {
    container.innerHTML = `<div class="mx-auto max-w-5xl rounded-xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700">${esc(
      state.loadError,
    )}</div>`;
    return;
  }
  if (!snapshot) {
    container.innerHTML = `<p class="text-sm text-sub">Memuat konfigurasi…</p>`;
    return;
  }

  const warnings = snapshot.warnings.length
    ? `<div class="mb-4 space-y-1 rounded-xl border border-amber-300 bg-amber-50 p-3">
         ${snapshot.warnings
           .map((warning) => `<p class="text-xs leading-relaxed text-amber-800">• ${esc(warning)}</p>`)
           .join("")}
       </div>`
    : "";

  const defaults =
    snapshot.defaultProvider || snapshot.defaultModel
      ? `<div class="mb-4 flex flex-wrap items-center gap-x-6 gap-y-1 rounded-2xl border border-line bg-white px-4 py-2.5 text-xs shadow-sm">
           <span class="text-sub">Default aktif:</span>
           <span class="selectable font-mono text-stone-700">${esc(snapshot.defaultProvider ?? "—")}</span>
           <span class="selectable font-mono text-stone-700">${esc(snapshot.defaultModel ?? "—")}</span>
         </div>`
      : "";

  const providersHeader = `
    <div class="mb-3 flex items-baseline gap-2">
      <h3 class="text-[13px] font-semibold text-ink">Provider</h3>
      <span class="rounded-full bg-sand px-2 py-0.5 text-[10px] font-medium text-sub">${snapshot.providers.length}</span>
    </div>`;

  const table =
    snapshot.providers.length === 0
      ? `<div class="rounded-2xl border border-dashed border-stone-300 bg-white px-6 py-14 text-center">
           <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-peach text-xl">🧩</div>
           <p class="text-sm font-medium text-ink">Belum ada provider di harness ini</p>
           <p class="mx-auto mt-1 max-w-sm text-xs leading-relaxed text-sub">
             Tambahkan provider gratis seperti Groq, Gemini, atau Cerebras —
             cukup tempel API key, Rekonfig menulis file konfigurasinya.
           </p>
           <button data-action data-add-empty class="mt-4 rounded-xl bg-brand px-3 py-1.5 text-xs font-medium text-white shadow-sm hover:bg-brand/90">
             + Tambah Provider
           </button>
         </div>`
      : `<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
           ${snapshot.providers.map(providerCard).join("")}
         </div>`;

  const fileList = snapshot.files
    .map(
      (file) => `
      <div class="flex items-center gap-2 border-b border-line px-3 py-1.5 last:border-0">
        <span class="w-32 shrink-0 text-[11px] text-sub">${esc(
          ROLE_LABEL[file.role] ?? file.role,
        )}</span>
        <span class="min-w-0 flex-1 truncate font-mono text-[11px] text-sub">${esc(file.path)}</span>
        <span class="shrink-0 text-[10px] ${file.exists ? "text-emerald-600" : "text-mute"}">${
          file.exists ? "ada" : "belum ada"
        }</span>
        <button data-action data-reveal="${esc(file.path)}" class="shrink-0 rounded p-1 text-sub hover:bg-sand hover:text-ink" title="Buka folder">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-3.5 w-3.5">
            <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>
        </button>
      </div>`,
    )
    .join("");

  container.innerHTML = `
    <div class="mx-auto max-w-5xl">
      ${warnings}
      ${defaults}
      ${providersHeader}
      ${table}
      <div class="mt-5 overflow-hidden rounded-2xl border border-line bg-white shadow-sm">
        <div class="border-b border-line bg-sand px-4 py-2 text-[11px] font-semibold tracking-wide text-sub uppercase">
          File konfigurasi
        </div>
        ${fileList}
      </div>
    </div>`;

  container.querySelectorAll("[data-expand]").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.dataset.expand;
      if (state.expanded.has(id)) state.expanded.delete(id);
      else state.expanded.add(id);
      renderContent();
    });
  });
  container.querySelectorAll("[data-edit]").forEach((button) => {
    button.addEventListener("click", () => {
      const provider = state.snapshot.providers.find((item) => item.id === button.dataset.edit);
      if (provider) openProviderForm("edit", provider);
    });
  });
  container.querySelectorAll("[data-remove]").forEach((button) => {
    button.addEventListener("click", () => removeProvider(button.dataset.remove));
  });
  container.querySelectorAll("[data-reveal]").forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await api.reveal(button.dataset.reveal);
      } catch (error) {
        toast("Gagal membuka folder", "err", String(error));
      }
    });
  });
  const addEmpty = $("[data-add-empty]", container);
  if (addEmpty) addEmpty.addEventListener("click", () => openProviderForm("add"));
}

function render() {
  if (state.view === "home") {
    renderHomeHeader();
    renderHome();
  } else {
    renderHarnessHeader();
    renderContent();
  }
}

function goHome() {
  state.view = "home";
  state.activeId = null;
  state.snapshot = null;
  state.expanded.clear();
  render();
}

// ============================================================================
// DATA LOADING
// ============================================================================

async function loadHarnesses() {
  try {
    state.harnesses = await api.harnesses();
    state.loadError = null;
  } catch (error) {
    state.harnesses = [];
    state.loadError = `Gagal memuat daftar harness: ${String(error)}`;
  }
  render();
}

async function loadSnapshot(harnessId) {
  state.snapshot = null;
  state.expanded.clear();
  renderContent();
  try {
    state.snapshot = await api.read(harnessId);
    state.loadError = null;
  } catch (error) {
    state.snapshot = null;
    state.loadError = String(error);
  }
  renderContent();
}

async function selectHarness(harnessId) {
  state.activeId = harnessId;
  state.view = "harness";
  render();
  await loadSnapshot(harnessId);
}

async function refreshEverything() {
  await loadHarnesses();
  if (state.view === "harness" && state.activeId) await loadSnapshot(state.activeId);
}

// ============================================================================
// FORM PROVIDER
// ============================================================================

function emptyModelRow() {
  return { id: "", name: "", contextWindow: "", maxTokens: "", reasoning: false };
}

function modelRowHtml(index, model) {
  return `
    <div class="grid grid-cols-[1fr_1fr_5rem_5rem_2.5rem_1.75rem] items-center gap-1.5" data-model-row="${index}">
      <input data-field="id" value="${esc(model.id)}" placeholder="model-id"
        class="rounded border border-stone-300 bg-white px-2 py-1 font-mono text-[11px] outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
      <input data-field="name" value="${esc(model.name ?? "")}" placeholder="Nama tampilan"
        class="rounded border border-stone-300 bg-white px-2 py-1 text-[11px] outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
      <input data-field="contextWindow" value="${esc(model.contextWindow ?? "")}" placeholder="ctx"
        inputmode="numeric" class="rounded border border-stone-300 bg-white px-2 py-1 font-mono text-[11px] outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
      <input data-field="maxTokens" value="${esc(model.maxTokens ?? "")}" placeholder="out"
        inputmode="numeric" class="rounded border border-stone-300 bg-white px-2 py-1 font-mono text-[11px] outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
      <label class="flex items-center justify-center gap-1 text-[10px] text-sub">
        <input data-field="reasoning" type="checkbox" ${model.reasoning ? "checked" : ""}
          class="h-3 w-3 rounded border-stone-300 bg-white accent-brand" />R
      </label>
      <button data-action data-model-remove="${index}" title="Hapus baris"
        class="rounded p-1 text-mute hover:bg-rose-50 hover:text-rose-600">✕</button>
    </div>`;
}

function openProviderForm(mode, provider) {
  const isEdit = mode === "edit";
  const draft = {
    id: provider?.id ?? "",
    name: provider?.name ?? "",
    baseUrl: provider?.baseUrl ?? "",
    api: provider?.api ?? "openai-completions",
    key: "",
    keyMode: "inline",
    models: provider?.models?.length
      ? provider.models.map((model) => ({
          id: model.id,
          name: model.name ?? "",
          contextWindow: model.contextWindow ?? "",
          maxTokens: model.maxTokens ?? "",
          reasoning: Boolean(model.reasoning),
        }))
      : [emptyModelRow()],
  };

  const presetOptions = state.presets
    .map((preset) => `<option value="${esc(preset.id)}">${esc(preset.name)}</option>`)
    .join("");

  const body = `
    <div class="space-y-4">
      ${
        isEdit
          ? ""
          : `<div>
              <label class="mb-1 block text-[11px] font-medium text-sub">Preset provider</label>
              <select data-preset class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20">
                <option value="">Custom / lainnya</option>
                ${presetOptions}
              </select>
              <p data-preset-note class="mt-1 text-[11px] leading-relaxed text-sub"></p>
            </div>`
      }

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="mb-1 block text-[11px] font-medium text-sub">ID provider</label>
          <input data-input="id" value="${esc(draft.id)}" placeholder="groq" ${isEdit ? "readonly" : ""}
            class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 font-mono text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20 ${
              isEdit ? "opacity-60" : ""
            }" />
        </div>
        <div>
          <label class="mb-1 block text-[11px] font-medium text-sub">Nama tampilan</label>
          <input data-input="name" value="${esc(draft.name)}" placeholder="Groq"
            class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
        </div>
      </div>

      <div>
        <label class="mb-1 block text-[11px] font-medium text-sub">Base URL</label>
        <input data-input="baseUrl" value="${esc(draft.baseUrl)}" placeholder="https://api.groq.com/openai/v1"
          class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 font-mono text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
        <p class="mt-1 text-[11px] text-sub">Endpoint OpenAI-compatible biasanya berakhiran <span class="font-mono">/v1</span>.</p>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="mb-1 block text-[11px] font-medium text-sub">Jenis API</label>
          <select data-input="api" class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20">
            ${API_OPTIONS.map(
              ([value, label]) =>
                `<option value="${value}" ${draft.api === value ? "selected" : ""}>${label}</option>`,
            ).join("")}
          </select>
        </div>
        <div>
          <label class="mb-1 block text-[11px] font-medium text-sub">Mode penyimpanan key</label>
          <select data-input="keyMode" class="w-full rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20">
            <option value="inline">Simpan key (paling mudah)</option>
            <option value="env">Rujuk variabel environment</option>
          </select>
        </div>
      </div>

      <div>
        <label class="mb-1 block text-[11px] font-medium text-sub">API key</label>
        <div class="flex gap-2">
          <input data-input="key" type="password" value="" autocomplete="off" spellcheck="false"
            placeholder="${esc(
              isEdit ? "biarkan kosong untuk mempertahankan key saat ini" : "tempel API key di sini",
            )}"
            class="min-w-0 flex-1 rounded-xl border border-stone-300 bg-white px-2.5 py-1.5 font-mono text-xs outline-none focus:border-brand focus:ring-2 focus:ring-brand/20" />
          <button data-action data-toggle-key class="rounded-xl border border-stone-300 px-2.5 py-1.5 text-[11px] text-stone-700 hover:bg-sand">Lihat</button>
          <button data-action data-test class="rounded-xl border border-stone-300 px-2.5 py-1.5 text-[11px] text-stone-700 hover:bg-sand">Test koneksi</button>
        </div>
        <div data-test-result class="mt-1.5 hidden rounded-xl border px-2.5 py-2 text-[11px] leading-relaxed"></div>
      </div>

      <div>
        <div class="mb-1.5 flex items-center justify-between">
          <label class="text-[11px] font-medium text-sub">Model</label>
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-mute">R = reasoning</span>
            <button data-action data-add-model class="rounded border border-stone-300 px-2 py-0.5 text-[11px] text-stone-700 hover:bg-sand">+ Baris</button>
          </div>
        </div>
        <div class="grid grid-cols-[1fr_1fr_5rem_5rem_2.5rem_1.75rem] gap-1.5 px-0.5 pb-1 text-[10px] text-mute">
          <span>ID model</span><span>Nama</span><span>Context</span><span>Max out</span><span class="text-center">R</span><span></span>
        </div>
        <div data-models class="space-y-1.5">${draft.models
          .map((model, index) => modelRowHtml(index, model))
          .join("")}</div>
      </div>
    </div>`;

  const footer = `
    <button data-action data-cancel class="rounded-xl border border-stone-300 px-3 py-1.5 text-xs text-stone-700 hover:bg-sand">Batal</button>
    <button data-action data-save class="rounded-xl bg-brand px-3 py-1.5 text-xs font-medium text-white shadow-sm hover:bg-brand/90">
      ${isEdit ? "Simpan perubahan" : "Simpan provider"}
    </button>`;

  const { overlay, close } = openModal({
    title: isEdit ? `Ubah provider ${provider.id}` : "Tambah provider",
    subtitle: isEdit
      ? "Perubahan akan ditampilkan sebagai pratinjau sebelum ditulis."
      : "Pilih preset lalu tempel API key — sisanya sudah terisi.",
    body,
    footer,
    wide: true,
  });

  const modelsContainer = $("[data-models]", overlay);
  const input = (name) => $(`[data-input="${name}"]`, overlay);

  const readModels = () => {
    const rows = [...modelsContainer.querySelectorAll("[data-model-row]")];
    return rows.map((row) => {
      const field = (name) => row.querySelector(`[data-field="${name}"]`);
      const number = (value) => {
        const trimmed = String(value ?? "").trim();
        if (!trimmed) return null;
        const parsed = Number(trimmed);
        return Number.isFinite(parsed) ? parsed : null;
      };
      return {
        id: field("id").value.trim(),
        name: field("name").value.trim() || null,
        contextWindow: number(field("contextWindow").value),
        maxTokens: number(field("maxTokens").value),
        reasoning: field("reasoning").checked,
      };
    });
  };

  const rerenderModels = () => {
    draft.models = readModels();
    modelsContainer.innerHTML = draft.models
      .map((model, index) => modelRowHtml(index, model))
      .join("");
    wireModelRows();
  };

  const wireModelRows = () => {
    modelsContainer.querySelectorAll("[data-model-remove]").forEach((button) => {
      button.addEventListener("click", () => {
        draft.models = readModels();
        draft.models.splice(Number(button.dataset.modelRemove), 1);
        if (draft.models.length === 0) draft.models.push(emptyModelRow());
        modelsContainer.innerHTML = draft.models
          .map((model, index) => modelRowHtml(index, model))
          .join("");
        wireModelRows();
      });
    });
  };
  wireModelRows();

  $("[data-add-model]", overlay).addEventListener("click", () => {
    draft.models = readModels();
    draft.models.push(emptyModelRow());
    modelsContainer.innerHTML = draft.models
      .map((model, index) => modelRowHtml(index, model))
      .join("");
    wireModelRows();
  });

  const presetSelect = $("[data-preset]", overlay);
  if (presetSelect) {
    presetSelect.addEventListener("change", () => {
      const preset = state.presets.find((item) => item.id === presetSelect.value);
      const note = $("[data-preset-note]", overlay);
      if (!preset) {
        note.textContent = "";
        return;
      }
      input("id").value = preset.id;
      input("name").value = preset.name;
      input("baseUrl").value = preset.baseUrl;
      input("api").value = preset.api;
      if (preset.docsUrl) {
        note.innerHTML = `Dapatkan API key di <span class="selectable font-mono text-brand">${esc(
          preset.docsUrl,
        )}</span>${preset.notes ? ` — ${esc(preset.notes)}` : ""}`;
      } else {
        note.textContent = preset.notes ?? "";
      }
      draft.models = preset.models.map((model) => ({
        id: model.id,
        name: model.name ?? "",
        contextWindow: model.contextWindow ?? "",
        maxTokens: model.maxTokens ?? "",
        reasoning: Boolean(model.reasoning),
      }));
      if (draft.models.length === 0) draft.models.push(emptyModelRow());
      modelsContainer.innerHTML = draft.models
        .map((model, index) => modelRowHtml(index, model))
        .join("");
      wireModelRows();
      input("key").focus();
    });
  }

  $("[data-toggle-key]", overlay).addEventListener("click", () => {
    const field = input("key");
    field.type = field.type === "password" ? "text" : "password";
  });

  $("[data-test]", overlay).addEventListener("click", async () => {
    const result = $("[data-test-result]", overlay);
    const key = input("key").value.trim();
    const baseUrl = input("baseUrl").value.trim();
    result.className =
      "mt-1.5 rounded-xl border border-stone-300 bg-white px-2.5 py-2 text-[11px] leading-relaxed text-sub";
    result.textContent = "Menguji koneksi…";
    try {
      const output = await api.test(baseUrl, input("api").value, key);
      const tone = output.ok
        ? "border-emerald-300 bg-tint-ok text-emerald-800"
        : "border-rose-300 bg-rose-50 text-rose-700";
      result.className = `mt-1.5 rounded-xl border px-2.5 py-2 text-[11px] leading-relaxed ${tone}`;
      const latency = output.latencyMs != null ? ` · ${output.latencyMs} ms` : "";
      const sample = output.modelsSample?.length
        ? `<div class="mt-1 selectable font-mono text-[10px] opacity-80">${output.modelsSample
            .map(esc)
            .join(" · ")}</div>`
        : "";
      result.innerHTML = `${esc(output.message)}${latency}${sample}`;
    } catch (error) {
      result.className =
        "mt-1.5 rounded-xl border border-rose-300 bg-rose-50 px-2.5 py-2 text-[11px] text-rose-700";
      result.textContent = String(error);
    }
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  $("[data-save]", overlay).addEventListener("click", async () => {
    const built = {
      kind: "upsert",
      id: input("id").value.trim(),
      name: input("name").value.trim(),
      baseUrl: input("baseUrl").value.trim(),
      api: input("api").value,
      key: input("key").value.trim() || null,
      keyMode: input("keyMode").value,
      models: readModels(),
    };

    if (!/^[a-z0-9][a-z0-9._-]*$/.test(built.id)) {
      toast("ID provider tidak valid", "err", "Gunakan huruf kecil, angka, titik, minus, atau garis bawah.");
      return;
    }
    if (!built.name) return toast("Nama provider wajib diisi", "err");
    if (!/^https?:\/\//.test(built.baseUrl)) {
      return toast("Base URL harus diawali http:// atau https://", "err");
    }
    if (built.models.length === 0 || built.models.some((model) => !model.id)) {
      return toast("Setiap model harus punya ID", "err");
    }

    close();
    await showPreview(built);
  });
}

// ============================================================================
// PRATINJAU & TULIS
// ============================================================================

async function showPreview(op) {
  const harnessId = state.activeId;
  let preview;
  busy(true);
  try {
    preview = await api.preview(harnessId, op);
  } catch (error) {
    busy(false);
    toast("Gagal membuat pratinjau", "err", String(error));
    return;
  }
  busy(false);

  const changed = preview.files.filter((file) => file.changed);
  const isRemove = op.kind === "remove";

  const fileBlocks = changed.length
    ? changed
        .map(
          (file) => `
          <div class="mb-4">
            <div class="mb-1.5 flex items-center gap-2">
              <span class="rounded bg-sand px-1.5 py-0.5 text-[10px] text-sub">${esc(
                ROLE_LABEL[file.role] ?? file.role,
              )}</span>
              <span class="selectable min-w-0 flex-1 truncate font-mono text-[11px] text-stone-700">${esc(
                file.path,
              )}</span>
              ${
                file.sensitive
                  ? `<span class="shrink-0 rounded-full bg-amber-100 px-2 py-0.5 text-[10px] text-amber-700">rahasia disamarkan</span>`
                  : ""
              }
            </div>
            ${renderDiff(file.before ?? "", file.after ?? "")}
          </div>`,
        )
        .join("")
    : `<p class="rounded-xl border border-line bg-white px-3 py-6 text-center text-xs text-sub">
         Tidak ada file yang akan berubah.
       </p>`;

  const warnings = preview.warnings.length
    ? `<div class="mb-4 space-y-1 rounded-xl border border-amber-300 bg-amber-50 p-3">
         ${preview.warnings
           .map((warning) => `<p class="text-xs leading-relaxed text-amber-800">• ${esc(warning)}</p>`)
           .join("")}
       </div>`
    : "";

  const summary = `
    <div class="mb-4 flex flex-wrap gap-x-6 gap-y-1 rounded-xl border border-line bg-white px-3 py-2 text-[11px]">
      ${preview.willCreate.length ? `<span class="text-emerald-600">dibuat: ${preview.willCreate.length}</span>` : ""}
      ${preview.willModify.length ? `<span class="text-amber-600">diubah: ${preview.willModify.length}</span>` : ""}
      <span class="text-sub">backup otomatis dibuat sebelum menulis</span>
    </div>`;

  const { overlay, close } = openModal({
    title: isRemove ? "Konfirmasi hapus provider" : "Pratinjau perubahan",
    subtitle: "Periksa diff di bawah. Belum ada yang ditulis ke disk.",
    body: summary + warnings + fileBlocks,
    footer: `
      <button data-action data-cancel class="rounded-xl border border-stone-300 px-3 py-1.5 text-xs text-stone-700 hover:bg-sand">Batal</button>
      <button data-action data-write ${
        changed.length ? "" : "disabled"
      } class="rounded-xl px-3 py-1.5 text-xs font-medium text-white ${
        isRemove ? "bg-rose-600 hover:bg-rose-500" : "bg-brand hover:bg-brand/90"
      } disabled:opacity-40">Tulis ke disk</button>`,
    wide: true,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);
  $("[data-write]", overlay).addEventListener("click", async () => {
    busy(true);
    try {
      const result = await api.apply(harnessId, op);
      close();
      toast(
        isRemove ? "Provider dihapus" : "Konfigurasi ditulis",
        "ok",
        result.filesWritten.join("\n"),
      );
      if (result.backupDir) toast("Backup dibuat", "info", result.backupDir);
      await refreshEverything();
    } catch (error) {
      toast("Gagal menulis konfigurasi", "err", String(error));
    } finally {
      busy(false);
    }
  });
}

async function removeProvider(providerId) {
  const confirmed = await confirmModal({
    title: "Hapus provider",
    message: `Provider "${providerId}" akan dihapus dari konfigurasi harness ini. Backup otomatis tetap dibuat, jadi perubahan masih bisa dipulihkan.`,
    confirmLabel: "Hapus",
    danger: true,
  });
  if (!confirmed) return;
  await showPreview({ kind: "remove", providerId });
}

// ============================================================================
// BACKUP
// ============================================================================

async function openBackups() {
  const harnessId = state.activeId;
  let backups = [];
  try {
    backups = await api.backups(harnessId);
  } catch (error) {
    toast("Gagal memuat backup", "err", String(error));
    return;
  }

  const list = backups.length
    ? backups
        .map(
          (backup) => `
          <div class="mb-2 rounded-xl border border-line bg-white px-3 py-2">
            <div class="flex items-center justify-between gap-3">
              <div class="min-w-0">
                <p class="selectable text-xs text-ink">${esc(fmtDate(backup.createdAt))}</p>
                <p class="selectable truncate font-mono text-[10px] text-sub">${esc(backup.dir)}</p>
              </div>
              <button data-action data-restore="${esc(backup.id)}"
                class="shrink-0 rounded border border-stone-300 px-2 py-1 text-[11px] text-stone-700 hover:bg-sand">Pulihkan</button>
            </div>
            <div class="mt-1.5 space-y-0.5">
              ${backup.files
                .map(
                  (file) =>
                    `<p class="selectable truncate font-mono text-[10px] text-mute">${esc(file.path)}</p>`,
                )
                .join("")}
            </div>
          </div>`,
        )
        .join("")
    : `<p class="rounded-xl border border-dashed border-stone-300 px-3 py-8 text-center text-xs text-sub">Belum ada backup untuk harness ini.</p>`;

  const { overlay, close } = openModal({
    title: "Backup",
    subtitle: "Rekonfig menyimpan salinan setiap file sebelum menulis.",
    body: list,
    footer: `<button data-action data-cancel class="rounded-xl border border-stone-300 px-3 py-1.5 text-xs text-stone-700 hover:bg-sand">Tutup</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  overlay.querySelectorAll("[data-restore]").forEach((button) => {
    button.addEventListener("click", async () => {
      const confirmed = await confirmModal({
        title: "Pulihkan backup",
        message:
          "File konfigurasi akan dikembalikan ke kondisi pada backup ini. Keadaan sekarang juga akan di-backup lebih dulu.",
        confirmLabel: "Pulihkan",
      });
      if (!confirmed) return;
      busy(true);
      try {
        const result = await api.restore(harnessId, button.dataset.restore);
        close();
        toast("Backup dipulihkan", "ok", result.filesWritten.join("\n"));
        await refreshEverything();
      } catch (error) {
        toast("Gagal memulihkan backup", "err", String(error));
      } finally {
        busy(false);
      }
    });
  });
}

// ============================================================================
// INIT
// ============================================================================

function renderFatal(message) {
  $("#content").innerHTML = `
    <div class="mx-auto max-w-lg rounded-2xl border border-rose-200 bg-rose-50 p-5">
      <p class="text-sm font-medium text-rose-800">${esc(message)}</p>
      <p class="mt-2 text-xs leading-relaxed text-rose-700/80">
        Rekonfig perlu dijalankan sebagai aplikasi desktop (Tauri), bukan dibuka langsung di browser.
      </p>
    </div>`;
}

async function init() {
  if (!invoke) {
    renderFatal("Rekonfig harus dijalankan sebagai aplikasi desktop.");
    renderHomeHeader();
    return;
  }

  try {
    state.presets = await api.presets();
  } catch (error) {
    state.presets = [];
    toast("Preset provider tidak bisa dimuat", "err", String(error));
  }

  // Muat daftar harness dan tampilkan grid pilihan (view "home").
  await loadHarnesses();
}

init();

// tangkap error di luar promise agar tidak hilang diam-diam
window.addEventListener("unhandledrejection", (event) => {
  toast("Terjadi kesalahan tak terduga", "err", String(event.reason));
});
