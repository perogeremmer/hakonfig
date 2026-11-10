// =============================================================================
// Hakonfig - frontend (vanilla JS, tanpa framework)
// Kontrak backend: docs/UI-CONTRACT.md
// =============================================================================

import { esc, renderDiff } from "./diff.js";
import { icon } from "./icons.js";
import { t, setLang, getLang, fmtDate } from "./i18n/index.js";
import { APP_VERSION } from "./version.js";
import { envVarFor, isRegisteredProvider, registeredApi } from "./provider-form.js";
import {
  LOGO_DARK_VARIANT,
  LOGO_EXT_PNG,
  LOGO_IDS,
} from "../assets/providers/logo-index.js";

const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;

// ------------------------------------------------------------------ konstanta

const REPO_URL = "https://github.com/perogeremmer/hakonfig";
const GUIDE_URL = `${REPO_URL}#readme`;

const ROLE_KEY = {
  models: "file.models",
  auth: "file.auth",
  settings: "file.settings",
  config: "file.config",
  env: "file.env",
};

const API_OPTIONS = [
  "openai-completions",
  "openai-responses",
  "anthropic-messages",
  "google-generative-ai",
];

/** Tint logo: maroon untuk brand besar, sisanya beragam agar grid tidak monoton. */
const TINTS = [
  ["bg-brand-soft", "text-brand-strong"],
  ["bg-info-bg", "text-info-text"],
  ["bg-warn-bg", "text-warn-text"],
  ["bg-ok-bg", "text-ok-text"],
  ["bg-neutral-bg", "text-neutral-text"],
  ["bg-danger-bg", "text-danger-text"],
];

const PREFERRED_TINT = {
  groq: 0,
  openrouter: 4,
  gemini: 1,
  deepseek: 0,
  cerebras: 1,
  mistral: 2,
  together: 3,
  nvidia: 3,
  "opencode-go": 2,
  ollama: 4,
  pi: 0,
  omp: 2,
  opencode: 1,
  hermes: 3,
};

// --------------------------------------------------------------------- helper

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

function tint(id) {
  const index = PREFERRED_TINT[id];
  if (index != null) return TINTS[index % TINTS.length];
  let hash = 0;
  for (const char of String(id)) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return TINTS[hash % TINTS.length];
}

function initials(id) {
  const parts = String(id).split(/[-_.]/).filter(Boolean);
  if (parts.length > 1) return parts.slice(0, 2).map((p) => p[0]).join("").toUpperCase();
  const cleaned = String(id).replace(/[^a-zA-Z0-9]/g, "");
  return (cleaned.slice(0, 2) || "?").toUpperCase();
}

/**
 * Logo provider/harness. File ada di web/assets/providers/<id>.<ext>.
 *
 * Daftar id, ekstensi PNG, dan varian gelap dibaca dari `logo-index.js` yang
 * dibuat `scripts/sync-logos.mjs`, jadi menambah logo tidak perlu menyunting
 * main.js. Provider yang id-nya tidak ada di daftar memakai mark
 * OpenAI-compatible (`custom.svg`), bukan monogram.
 */
const FALLBACK_LOGO = "custom";

function logo(id, size = 40, extraClass = "") {
  const [bg, fg] = tint(id);
  const font = size >= 44 ? "text-sm" : "text-xs";
  const known = LOGO_IDS.has(id);
  const assetId = known ? id : FALLBACK_LOGO;
  const base = `./assets/providers/${esc(assetId)}`;
  const ext = LOGO_EXT_PNG.has(assetId) ? "png" : "svg";
  const dual = LOGO_DARK_VARIANT.has(assetId);
  const dark = dual
    ? `<img class="hk-logo-img hk-logo-img-dark" src="${base}-dark.${ext}" alt="" loading="lazy"
      onerror="this.remove()" />`
    : "";
  // Monogram hanya untuk id yang belum punya logo; provider custom memakai mark.
  const monogram = known
    ? `<span class="select-none ${font} font-bold">${esc(initials(id))}</span>`
    : "";
  return `<span class="hk-logo ${bg} ${fg} ${dual ? "hk-logo-dual" : ""} ${extraClass}" style="width:${size}px;height:${size}px">
    ${monogram}
    <img class="hk-logo-img" src="${base}.${ext}" alt="" loading="lazy"
      onerror="this.remove()" />
    ${dark}
  </span>`;
}

function chip(tone, label, iconName) {
  return `<span class="hk-chip hk-chip-${tone}">${iconName ? icon(iconName, "h-3 w-3") : ""}${esc(label)}</span>`;
}

function pluralProviders(count) {
  return count === 1 ? t("status.providersOne") : t("status.providers", { count });
}

function pluralModels(count) {
  return count === 1 ? t("provider.modelsOne") : t("provider.models", { count });
}

// ------------------------------------------------------------------------ api

const api = {
  harnesses: () => invoke("list_harnesses"),
  targets: () => invoke("list_targets"),
  presets: () => invoke("list_presets"),
  read: (harnessId) => invoke("read_harness", { harnessId }),
  preview: (harnessId, op) => invoke("preview_edit", { harnessId, op }),
  apply: (harnessId, op) => invoke("apply_edit", { harnessId, op }),
  backups: (harnessId) => invoke("list_backups", { harnessId }),
  restore: (harnessId, backupId) => invoke("restore_backup", { harnessId, backupId }),
  test: (baseUrl, apiName, key, harnessId = null, providerId = null) =>
    invoke("test_connection", { baseUrl, api: apiName, key, harnessId, providerId }),
  reveal: (path) => invoke("open_in_file_manager", { path }),
  external: (url) => invoke("open_external", { url }),
  catalog: () => invoke("list_harness_catalog"),
  version: (harnessId) => invoke("harness_version", { harnessId }),
  install: (harnessId) => invoke("install_harness", { harnessId }),
  uninstall: (harnessId) => invoke("uninstall_harness", { harnessId }),
  models: (baseUrl, apiName, key, harnessId = null, providerId = null) =>
    invoke("fetch_models", { baseUrl, api: apiName, key, harnessId, providerId }),
  fixNotice: (harnessId, code, provider) => invoke("fix_notice", { harnessId, code, provider }),
  uiScaleInfo: () => invoke("ui_scale_info"),
  setUiScale: (preference, devicePixelRatio) =>
    invoke("set_ui_scale", { preference, devicePixelRatio }),
};

// ------------------------------------------------------------------ preferensi

const PREFS_KEY = "hakonfig.prefs";
const DEFAULT_PREFS = {
  lang: "en",
  theme: "light",
  confirmDelete: true,
  onboarded: false,
  // "auto" membiarkan skala mengikuti monitor; angka = pilihan pengguna.
  uiScale: "auto",
};

/** Pilihan skala yang ditawarkan di Settings (dan dipakai Ctrl +/-). */
const SCALE_STEPS = [0.9, 1, 1.1, 1.25, 1.5];

function loadPrefs() {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    return raw ? { ...DEFAULT_PREFS, ...JSON.parse(raw) } : { ...DEFAULT_PREFS };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

function savePrefs(patch) {
  state.prefs = { ...state.prefs, ...patch };
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(state.prefs));
  } catch {
    // localStorage bisa diblokir; preferensi tetap berlaku untuk sesi ini.
  }
}

function applyTheme() {
  const dark = state.prefs.theme === "dark";
  document.documentElement.classList.toggle("dark", dark);
}

// ------------------------------------------------------------------- skala UI

/**
 * Terapkan skala UI lewat zoom webview.
 *
 * Zoom dipakai (bukan ukuran font) karena seluruh tampilan memakai nilai px
 * hardcoded: hanya zoom yang ikut membesarkan tombol, padding, dan ikon
 * sekaligus. Perhitungannya di Rust (`hakonfig_core::engine::ui_scale`) supaya
 * bisa diuji; di sini kita hanya mengirim preferensi + devicePixelRatio.
 */
async function applyUiScale() {
  if (!invoke) return;
  const pref = state.prefs.uiScale;
  const preference = typeof pref === "number" && Number.isFinite(pref) ? pref : null;
  try {
    state.uiScale = await api.setUiScale(preference, window.devicePixelRatio || 1);
  } catch (error) {
    // Gagal menyesuaikan skala tidak boleh memblokir aplikasi.
    state.uiScale = null;
    console.warn("ui scale:", error);
  }
  syncScalePanel();
  return state.uiScale;
}

/** Satu langkah naik/turun dari posisi sekarang; berhenti di ujung. */
function stepUiScale(direction) {
  const current = state.uiScale?.zoom ?? 1;
  const next =
    direction > 0
      ? SCALE_STEPS.find((step) => step > current + 0.001)
      : [...SCALE_STEPS].reverse().find((step) => step < current - 0.001);
  if (next === undefined) return;
  savePrefs({ uiScale: next });
  applyUiScale().then(() => toast(t("settings.scaleSet", { percent: Math.round(next * 100) })));
}

/** Label singkat untuk status skala, dipakai panel Settings. */
function scaleStatus() {
  const info = state.uiScale;
  if (!info) return "";
  const percent = Math.round(info.zoom * 100);
  const auto = state.prefs.uiScale === "auto";
  const reason = !auto
    ? t("settings.scaleManual")
    : info.reason === "patchesWebkit"
      ? t("settings.scalePatched")
      : info.reason === "clamped"
        ? t("settings.scaleClamped")
        : t("settings.scaleNormal");
  return `${percent}% · ${reason}`;
}

/**
 * Segarkan panel Settings yang sedang terbuka.
 *
 * Tanpa ini, Ctrl +/- mengubah layar tetapi panel masih menampilkan angka lama:
 * dua sumber kebenaran di satu layar adalah cara termurah membuat pengguna ragu.
 */
function syncScalePanel() {
  const root = document.querySelector("[data-scale-root]");
  if (!root) return;
  root.querySelectorAll("[data-scale]").forEach((button) => {
    const value = button.dataset.scale === "auto" ? "auto" : Number(button.dataset.scale);
    const active = state.prefs.uiScale === value;
    button.classList.toggle("hk-btn-active", active);
    button.setAttribute("aria-pressed", String(active));
  });
  const status = root.querySelector("[data-scale-status]");
  if (status) status.textContent = scaleStatus();
  const diag = root.querySelector("[data-scale-diag]");
  if (diag) diag.textContent = scaleDiagnostics();
}

/**
 * Angka mentah untuk diagnosa HiDPI.
 *
 * Tanpa ini, laporan "kecil di laptop saya" tidak bisa dipastikan penyebabnya:
 * skala monitor, devicePixelRatio, dan ukuran jendela harus terlihat bersamaan.
 */
function scaleDiagnostics() {
  const info = state.uiScale;
  const dpr = window.devicePixelRatio || 1;
  const viewport = `${Math.round(window.innerWidth)}x${Math.round(window.innerHeight)}`;
  if (!info) return `webview ${dpr}x · viewport ${viewport} css px`;
  return [
    `monitor ${info.monitorKnown ? info.monitorScale.toFixed(2) : "?"}x`,
    `webview ${dpr}x`,
    `viewport ${viewport} css px`,
    `zoom ${info.zoom.toFixed(2)}`,
  ].join(" · ");
}

// ---------------------------------------------------------------------- state

const state = {
  view: "home", // "onboarding" | "home" | "harness" | "provider"
  harnesses: [],
  catalog: [],
  presets: [],
  targets: [], // home user lain (ADE/VPS) yang punya harness terpasang
  versions: {},
  snapshot: null,
  form: null,
  activeId: null,
  loadError: null,
  prefs: loadPrefs(),
  uiScale: null, // hasil ui_scale_info dari Rust (zoom + alasan)
  busy: false,
  onboarding: null, // { step, harnesses, installed, providers, models }
};

// ---------------------------------------------------------------------- toast

function toast(message, { variant = "ok", detail = "", action = null, timeout } = {}) {
  const root = $("#toast-root");
  const tone = {
    ok: { bg: "bg-ok-bg", fg: "text-ok-text", name: "check" },
    err: { bg: "bg-danger-bg", fg: "text-danger-text", name: "alert" },
    info: { bg: "bg-surface-muted", fg: "text-ink-secondary", name: "info" },
  }[variant] ?? { bg: "bg-surface-muted", fg: "text-ink-secondary", name: "info" };

  const element = document.createElement("div");
  element.className =
    "pointer-events-auto selectable flex items-start gap-2.5 rounded-[14px] border border-line bg-surface px-3.5 py-3 shadow-lg";
  element.innerHTML = `
    <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full ${tone.bg} ${tone.fg}">
      ${icon(tone.name, "h-3.5 w-3.5")}
    </span>
    <div class="min-w-0 flex-1">
      <p class="text-base font-semibold leading-snug text-ink">${esc(message)}</p>
      ${detail ? `<p class="mt-1 line-clamp-2 break-all font-mono text-xs leading-relaxed text-ink-secondary">${esc(detail)}</p>` : ""}
    </div>
    ${
      action
        ? `<button data-action class="hk-btn hk-btn-ghost hk-btn-sm shrink-0 font-semibold underline">${esc(action.label)}</button>`
        : ""
    }`;

  if (action) {
    $("[data-action]", element).addEventListener("click", () => {
      element.remove();
      action.onClick();
    });
  }

  root.append(element);
  setTimeout(() => element.remove(), timeout ?? (variant === "err" ? 8000 : 4500));
}

// ---------------------------------------------------------------------- modal

let lastFocused = null;

function openModal({ title, subtitle = "", body, footer = "", size = "md", onClose }) {
  lastFocused = document.activeElement;

  const root = $("#modal-root");
  const overlay = document.createElement("div");
  overlay.className = "hk-overlay";
  overlay.innerHTML = `
    <div class="hk-modal hk-modal-${size}" role="dialog" aria-modal="true">
      <div class="flex items-start justify-between gap-4 border-b border-line px-6 py-5">
        <div class="min-w-0">
          <h2 class="text-2xl font-bold leading-snug text-ink">${esc(title)}</h2>
          ${subtitle ? `<p class="mt-1 text-base leading-relaxed text-ink-secondary">${esc(subtitle)}</p>` : ""}
        </div>
        <button data-action data-close class="hk-btn hk-btn-icon shrink-0" aria-label="${esc(t("action.close", {}))}">
          ${icon("x")}
        </button>
      </div>
      <div data-body class="px-6 py-5">${body}</div>
      ${footer ? `<div class="flex flex-wrap items-center justify-end gap-2 border-t border-line px-6 py-4">${footer}</div>` : ""}
    </div>`;

  root.append(overlay);

  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey, true);
    if (lastFocused?.isConnected) lastFocused.focus();
    onClose?.();
  };

  // Focus trap: Tab tidak boleh keluar dari modal.
  const onKey = (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "Tab") return;
    const focusables = $$(
      'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"]), summary',
      overlay,
    ).filter((el) => el.offsetParent !== null);
    if (focusables.length === 0) return;
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  document.addEventListener("keydown", onKey, true);
  overlay.addEventListener("click", (event) => {
    if (event.target === overlay) close();
  });
  $$("[data-close]", overlay).forEach((button) => button.addEventListener("click", close));

  const first = $("input:not([readonly]), select, textarea", overlay) ?? $("button", overlay);
  first?.focus();

  return { overlay, close, body: $("[data-body]", overlay) };
}

function confirmDialog({
  title,
  body,
  bullets = [],
  confirmLabel,
  danger = false,
  allowDontAsk = false,
}) {
  return new Promise((resolve) => {
    const { overlay, close } = openModal({
      title,
      size: "sm",
      body: `
        <p class="selectable text-lg leading-relaxed text-ink">${esc(body)}</p>
        ${
          bullets.length
            ? `<ul class="mt-3 space-y-2">
                 ${bullets
                   .map(
                     (line) => `<li class="flex gap-2 text-base leading-relaxed text-ink-secondary">
                        <span class="mt-0.5 shrink-0 text-ok-text">${icon("check", "h-4 w-4")}</span>
                        <span class="selectable">${esc(line)}</span>
                      </li>`,
                   )
                   .join("")}
               </ul>`
            : ""
        }
        ${
          allowDontAsk
            ? `<label class="mt-4 flex cursor-pointer items-center gap-2.5 rounded-lg bg-surface-muted px-3 py-2.5">
                 <input data-dont-ask type="checkbox" class="hk-check" />
                 <span class="text-base text-ink">${esc(t("delete.dontAsk"))}</span>
               </label>`
            : ""
        }`,
      footer: `
        <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
        <button data-action data-confirm class="hk-btn ${
          danger ? "hk-btn-danger" : "hk-btn-primary"
        }">${esc(confirmLabel)}</button>`,
      onClose: () => resolve(false),
    });

    $("[data-cancel]", overlay).addEventListener("click", () => {
      close();
      resolve(false);
    });
    $("[data-confirm]", overlay).addEventListener("click", () => {
      const dontAsk = $("[data-dont-ask]", overlay);
      if (dontAsk?.checked) savePrefs({ confirmDelete: false });
      close();
      resolve(true);
    });
  });
}

/** Menu ⋯ generik, ditempel di dekat tombol pemicu. */
function openMenu(anchor, items) {
  document.querySelectorAll(".hk-menu").forEach((menu) => menu.remove());

  const menu = document.createElement("div");
  menu.className = "hk-menu";
  menu.innerHTML = items
    .map((item, index) => {
      if (item.separator) return `<div class="hk-menu-sep"></div>`;
      return `<button data-action data-menu="${index}"
        class="hk-menu-item ${item.danger ? "hk-menu-item-danger" : ""}">
        ${item.icon ? icon(item.icon) : ""}<span>${esc(item.label)}</span>
      </button>`;
    })
    .join("");

  document.body.append(menu);

  const rect = anchor.getBoundingClientRect();
  const width = menu.offsetWidth;
  const left = Math.min(rect.right - width, window.innerWidth - width - 12);
  menu.style.left = `${Math.max(12, left)}px`;
  menu.style.top = `${rect.bottom + 6}px`;
  menu.style.position = "fixed";

  const close = () => {
    menu.remove();
    document.removeEventListener("click", outside, true);
    document.removeEventListener("keydown", onKey, true);
  };
  const outside = (event) => {
    if (!menu.contains(event.target)) close();
  };
  const onKey = (event) => {
    if (event.key === "Escape") close();
  };
  document.addEventListener("click", outside, true);
  document.addEventListener("keydown", onKey, true);

  $$("[data-menu]", menu).forEach((button) => {
    button.addEventListener("click", () => {
      close();
      items[Number(button.dataset.menu)]?.onSelect?.();
    });
  });
}

// ============================================================================
// KOMPONEN
// ============================================================================

function harnessStatusLine(harness) {
  if (harness.readError) return { label: t("status.readError"), tone: "danger", iconName: "alert" };
  if (!harness.installed) return { label: t("status.noConfig"), tone: "neutral", iconName: null };
  if (harness.connectedCount > 0) {
    return { label: t("status.ready"), tone: "ok", iconName: "check" };
  }
  if (harness.providerCount > 0) return { label: t("status.noKey"), tone: "warn", iconName: "alert" };
  return { label: t("status.needsSetup"), tone: "warn", iconName: null };
}

function harnessTile(harness) {
  const status = harnessStatusLine(harness);
  const version = state.versions[harness.id];
  const detail =
    harness.providerCount > 0
      ? `${pluralProviders(harness.providerCount)}`
      : t("status.noProviders");

  return `
    <div data-harness="${esc(harness.id)}" role="button" tabindex="0"
      class="hk-card group flex min-h-[9.25rem] cursor-pointer flex-col gap-3 p-5 transition hover:-translate-y-0.5 hover:border-brand/40 hover:shadow-md">
      <div class="flex items-start justify-between gap-2">
        ${logo(harness.id, 44)}

      </div>
      <div class="min-w-0">
        <p class="truncate text-lg font-semibold text-ink">${esc(harness.name)}</p>
        <p class="truncate text-base text-ink-secondary">${esc(detail)}</p>
      </div>
      <div class="mt-auto flex flex-wrap items-center gap-x-2 gap-y-1.5">
        ${chip(status.tone, status.label, status.iconName)}
        ${harness.experimental ? chip("neutral", t("status.experimental")) : ""}
        ${version ? `<span class="font-mono text-sm text-ink-muted">v${esc(version)}</span>` : ""}
      </div>
    </div>`;
}

/**
 * Kartu untuk tool berbasis web (mis. DeepSeek). Tidak ada file config dan
 * tidak ada provider yang bisa diatur, jadi Hakonfig hanya membuka alamatnya
 * dan menjelaskan cara memasangnya sebagai aplikasi.
 */
function webTile(entry) {
  const url = entry.launchUrl ?? entry.docsUrl ?? "";

  return `
    <div class="hk-card flex min-h-[9.25rem] flex-col gap-3 p-5">
      <div class="flex items-start justify-between gap-2">
        ${logo(entry.id, 44)}
        ${chip("info", t("web.badge"), "book")}
      </div>
      <div class="min-w-0">
        <p class="truncate text-lg font-semibold text-ink">${esc(entry.name)}</p>
        <p class="mt-0.5 line-clamp-2 text-base leading-relaxed text-ink-secondary">${esc(
          getLang() === "id" ? (entry.descriptionId ?? entry.description ?? "") : (entry.description ?? ""),
        )}</p>
      </div>
      <div class="mt-auto flex flex-col gap-2 pt-1">
        <button data-action data-open-web="${esc(url)}" class="hk-btn hk-btn-primary w-full">
          ${icon("arrowUpRight")}<span>${esc(t("web.open"))}</span>
        </button>
        <button data-action data-pwa-help class="hk-btn hk-btn-ghost hk-btn-sm justify-start">
          ${icon("download", "h-3.5 w-3.5")}${esc(t("web.installHint"))}
        </button>
      </div>
    </div>`;
}

function catalogTile(entry) {
  const version = state.versions[entry.id];
  const hasBinary = Boolean(version);
  // Sudah terpasang berarti tidak ada yang perlu dipasang lagi. Tanpa ini,
  // kartu menampilkan chip "Terpasang" sekaligus tombol Install.
  const hasInstall = Boolean(entry.installCommand) && !hasBinary;

  return `
    <div class="hk-card flex min-h-[9.25rem] flex-col gap-3 p-5">
      <div class="flex items-start justify-between gap-2">
        ${logo(entry.id, 44)}
        ${hasBinary ? chip("ok", t("home.installed"), "check") : ""}
      </div>
      <div class="min-w-0">
        <p class="truncate text-lg font-semibold text-ink">${esc(entry.name)}</p>
        <p class="mt-0.5 line-clamp-2 text-base leading-relaxed text-ink-secondary">${esc(
          getLang() === "id" ? (entry.descriptionId ?? entry.description ?? "") : (entry.description ?? ""),
        )}</p>
      </div>
      <div class="mt-auto flex flex-col gap-2 pt-1">
        ${
          hasInstall
            ? `<button data-action data-install="${esc(entry.id)}" class="hk-btn hk-btn-primary w-full">
                 ${icon("download")}<span>${esc(t("install.run"))}</span>
               </button>`
            : hasBinary
              ? ""
              : `<span class="text-sm leading-relaxed text-ink-muted">${esc(t("install.manualHint"))}</span>`
        }
        <div class="flex items-center justify-between gap-2">
          ${
            entry.docsUrl
              ? `<button data-action data-external="${esc(entry.docsUrl)}" class="hk-btn hk-btn-ghost hk-btn-sm">
                   ${esc(t("docs"))}${icon("arrowUpRight", "h-3.5 w-3.5")}
                 </button>`
              : "<span></span>"
          }
          ${hasBinary && entry.uninstallCommand ? `<button data-action data-uninstall="${esc(entry.id)}" class="hk-btn hk-btn-ghost hk-btn-sm text-danger-text">${esc(t("install.uninstallShort"))}</button>` : ""}
        </div>
      </div>
    </div>`;
}

function providerCard(provider) {
  const on = provider.enabled !== false;
  const keyChip = provider.hasKey
    ? chip("ok", t("provider.keySaved"), "check")
    : chip("warn", t("provider.noKey"));
  const paused = on ? "" : chip("neutral", t("provider.paused"), "pause");

  return `
    <div class="hk-card flex flex-col gap-3 p-4 ${on ? "" : "opacity-70"}">
      <div class="flex items-start justify-between gap-2">
        ${logo(provider.id, 40)}
        <button data-action data-toggle="${esc(provider.id)}" role="switch" aria-checked="${on}"
          class="hk-switch mt-1" aria-label="${esc(provider.name)}"></button>
      </div>
      <div class="min-w-0">
        <p class="truncate text-lg font-semibold text-ink" title="${esc(provider.name)}">${esc(provider.name)}</p>
        <p class="truncate text-base text-ink-secondary">
          ${provider.models.length ? esc(pluralModels(provider.models.length)) : esc(t("status.noModels"))}
        </p>
      </div>
      <div class="mt-auto flex items-center justify-between gap-2 border-t border-line pt-2.5">
        <div class="flex min-w-0 flex-wrap items-center gap-1">${keyChip}${paused}</div>
        <button data-action data-menu-provider="${esc(provider.id)}" class="hk-btn hk-btn-icon shrink-0" aria-label="${esc(t("harness.more"))}">
          ${icon("more")}
        </button>
      </div>
    </div>`;
}

// ============================================================================
// HEADER & VIEW
// ============================================================================

// Mark dari ~/Documents/Hakonfig (H + dua kenop). Inline supaya warnanya ikut
// tema: garis memakai currentColor, kenop memakai --logo-knob.
const LOGO_SVG = `<svg viewBox="191 167 601 690" fill="none" class="h-6 w-auto" aria-hidden="true">
  <g stroke="currentColor" stroke-width="96" stroke-linecap="round">
    <line x1="319.5" y1="295.5" x2="319.5" y2="751"/>
    <line x1="703.5" y1="272" x2="703.5" y2="727.5"/>
    <line x1="319.5" y1="511.5" x2="703.5" y2="511.5"/>
  </g>
  <circle cx="319.5" cy="295.5" r="88.5" fill="currentColor"/>
  <circle cx="703.5" cy="727.5" r="89" fill="var(--logo-knob)"/>
</svg>`;

function renderHomeHeader() {
  $("#topbar").innerHTML = `
    <div class="mx-auto flex w-full max-w-[63rem] items-center justify-between gap-3 px-6 py-3.5">
      <div class="flex min-w-0 items-center gap-3">
        <div class="flex h-10 w-10 items-center justify-center rounded-xl border border-line bg-surface-muted text-ink shadow-xs">${LOGO_SVG}</div>
        <div class="min-w-0">
          <p class="text-lg font-bold leading-tight text-ink">Hakonfig</p>
          <p class="text-sm leading-tight text-ink-secondary">${esc(t("app.tagline"))}</p>
        </div>
      </div>
      <button data-action data-settings class="hk-btn hk-btn-icon" aria-label="${esc(t("harness.settings"))}">
        ${icon("gear")}
      </button>
    </div>`;
  $("[data-settings]", $("#topbar")).addEventListener("click", openSettings);
}

function renderHarnessHeader() {
  const harness = state.harnesses.find((item) => item.id === state.activeId);
  const snapshot = state.snapshot;
  const count = snapshot?.providers.length ?? 0;
  const missingKey = snapshot?.providers.filter((p) => p.hasKey === false).length ?? 0;
  const missingKeyLabel = missingKey === 1 ? t("harness.needsKeyOne") : t("harness.needsKey", { count: missingKey });
  const subtitle = count
    ? `${pluralProviders(count)}${missingKey ? ` · ${missingKeyLabel}` : ""}`
    : t("status.noProviders");

  $("#topbar").innerHTML = `
    <div class="mx-auto flex w-full max-w-[63rem] flex-wrap items-center justify-between gap-3 px-6 py-3">
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <button data-action data-home class="hk-btn hk-btn-icon" aria-label="${esc(t("action.cancel"))}">
          ${icon("arrowLeft")}
        </button>
        ${logo(state.activeId ?? "", 40)}
        <div class="min-w-0">
          <p class="truncate text-lg font-bold leading-tight text-ink">${esc(harness?.name ?? "…")}</p>
          <p class="truncate text-sm leading-tight text-ink-secondary">${esc(subtitle)}</p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <button data-action data-history class="hk-btn hk-btn-secondary hk-btn-sm">
          ${icon("history", "h-4 w-4")}<span class="hidden sm:inline">${esc(t("harness.history"))}</span>
        </button>
        ${harnessMenuItems(state.activeId).length
          ? `<button data-action data-menu-harness-header class="hk-btn hk-btn-icon" aria-label="${esc(t("harness.more"))}">
               ${icon("more")}
             </button>`
          : ""}
        <button data-action data-add class="hk-btn hk-btn-primary hidden sm:inline-flex">
          ${icon("plus", "h-4 w-4")}<span>${esc(t("harness.addProvider"))}</span>
        </button>
        <button data-action data-settings class="hk-btn hk-btn-icon" aria-label="${esc(t("harness.settings"))}">
          ${icon("gear")}
        </button>
      </div>
    </div>`;

  const harnessId = state.activeId;
  $("[data-settings]", $("#topbar")).addEventListener("click", openSettings);
  $("[data-home]", $("#topbar")).addEventListener("click", goHome);
  $("[data-history]", $("#topbar")).addEventListener("click", openHistory);
  $("[data-add]", $("#topbar"))?.addEventListener("click", openProviderPicker);
  $("[data-menu-harness-header]", $("#topbar"))?.addEventListener("click", (event) => {
    openMenu(event.currentTarget, harnessMenuItems(harnessId));
  });
}

function harnessMenuItems(harnessId) {
  const entry = state.catalog.find((item) => item.id === harnessId);
  const items = [];
  const folder = state.snapshot?.files.find((file) => file.exists)?.path;
  if (folder) {
    items.push({
      label: t("harness.showFolder"),
      icon: "folder",
      onSelect: () => api.reveal(folder).catch((error) => toast(t("toast.revealFailed"), { variant: "err", detail: String(error) })),
    });
  }
  if (entry?.uninstallCommand && state.versions[harnessId]) {
    items.push({ separator: true });
    items.push({
      label: t("harness.uninstall"),
      icon: "trash",
      danger: true,
      onSelect: () => runPackage(harnessId, "uninstall"),
    });
  }
  return items;
}

// ------------------------------------------------------------------- home

function renderHome() {
  const container = $("#content");

  if (state.loadError) {
    container.innerHTML = `<div class="mx-auto max-w-[60rem]">
      <div class="rounded-[14px] border border-line bg-danger-bg px-4 py-3 text-base text-danger-text">${esc(state.loadError)}</div>
    </div>`;
    return;
  }

  const installed = state.harnesses.filter((harness) => harness.installed);
  const catalogEntries = state.catalog
    .filter((entry) => !state.harnesses.some((harness) => harness.id === entry.id))
    .filter((entry) => entry.kind === "planned");
  const catalogReady = catalogEntries.filter((entry) => state.versions[entry.id]);
  const catalogOnly = catalogEntries.filter((entry) => !state.versions[entry.id]);
  const webOnly = state.catalog.filter((entry) => entry.kind === "web");
  const notInstalled = state.harnesses.filter((harness) => !harness.installed);

  if (
    installed.length === 0 &&
    notInstalled.length === 0 &&
    catalogOnly.length === 0 &&
    catalogReady.length === 0 &&
    webOnly.length === 0
  ) {
    container.innerHTML = `<div class="mx-auto max-w-[60rem]">
      <div class="hk-card mx-auto max-w-lg p-8 text-center">
        <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-xl bg-surface-muted text-ink-muted">${icon("search", "h-6 w-6")}</div>
        <p class="text-lg font-semibold text-ink">${esc(t("home.empty.title"))}</p>
        <p class="mx-auto mt-1 max-w-sm text-base leading-relaxed text-ink-secondary">${esc(t("home.empty.body"))}</p>
      </div>
    </div>`;
    return;
  }

  const section = (title, hint, items) => `
    <section class="mb-8">
      <div class="mb-1 flex items-baseline gap-2">
        <h2 class="text-xl font-bold text-ink">${esc(title)}</h2>
        <span class="text-base font-semibold text-ink-muted">${items.length}</span>
      </div>
      <p class="mb-3 text-base text-ink-secondary">${esc(hint)}</p>
      ${
        items.length
          ? `<div class="grid grid-cols-2 gap-4 lg:grid-cols-4">${items.join("")}</div>`
          : `<div class="rounded-[14px] border border-dashed border-line-strong px-4 py-6 text-center text-base text-ink-muted">${esc(hint)}</div>`
      }
    </section>`;

  container.innerHTML = `
    <div class="mx-auto max-w-[60rem]">
      <div class="mb-7">
        <h1 class="text-4xl font-extrabold leading-tight tracking-tight text-ink">${esc(t("home.title"))}</h1>
        <p class="mt-1 text-md text-ink-secondary">${esc(t("home.subtitle"))}</p>
      </div>
      ${targetsBanner()}
      ${section(
        t("home.installed"),
        t("home.installedHint"),
        [...installed.map(harnessTile), ...catalogReady.map(catalogTile)],
      )}
      ${section(
        t("home.notInstalled"),
        t("home.notInstalledHint"),
        [...notInstalled.map(harnessTile), ...catalogOnly.map(catalogTile)],
      )}
      ${
        webOnly.length
          ? section(t("home.webTitle"), t("home.webHint"), webOnly.map(webTile))
          : ""
      }
    </div>`;

  wiringHome(container);
}

/**
 * Home user lain (ADE/VPS) yang punya harness terpasang.
 *
 * Aplikasi desktop hanya bisa mengelola home milik user yang menjalankannya,
 * jadi banner ini menyebutkan keberadaannya beserta perintah CLI yang tepat.
 * Tanpa ini, harness milik agent tampak "hilang" padahal hanya ada di home
 * lain.
 */
function targetsBanner() {
  if (!state.targets.length) return "";
  const rows = state.targets
    .map((target) => {
      const where = `${target.user} (${target.home})`;
      return `<li class="mt-1 text-base text-ink-secondary">${esc(
        t("home.targets.item", {
          where,
          harnesses: target.harnesses.join(", "),
        }),
      )}</li>`;
    })
    .join("");
  const command = `hakonfig-cli --target ${state.targets[0].user} providers`;
  return `<div class="mb-7 rounded-[14px] border border-line bg-surface-muted px-4 py-3">
    <p class="text-base font-semibold text-ink">${esc(t("home.targets.title"))}</p>
    <ul class="mt-1 list-none">${rows}</ul>
    <p class="mt-2 text-sm text-ink-muted">${esc(t("home.targets.hint", { command }))}</p>
  </div>`;
}

function wiringHome(container) {
  $$("[data-open-web]", container).forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await api.external(button.dataset.openWeb);
      } catch (error) {
        toast(t("toast.generic"), { variant: "err", detail: String(error) });
      }
    });
  });
  $$("[data-pwa-help]", container).forEach((button) => {
    button.addEventListener("click", () => openPwaHelp());
  });

  $$("[data-harness]", container).forEach((card) => {
    card.addEventListener("click", (event) => {
      if (event.target.closest("[data-menu-harness]")) return;
      selectHarness(card.dataset.harness);
    });
    card.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        selectHarness(card.dataset.harness);
      }
    });
  });
  $$("[data-menu-harness]", container).forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openMenu(button, harnessMenuItems(button.dataset.menuHarness));
    });
  });
  $$("[data-install]", container).forEach((button) => {
    button.addEventListener("click", () => runPackage(button.dataset.install, "install"));
  });
  $$("[data-uninstall]", container).forEach((button) => {
    button.addEventListener("click", () => runPackage(button.dataset.uninstall, "uninstall"));
  });
  wireExternalButtons(container);
}

// ---------------------------------------------------------------- harness page

function renderHarness() {
  const container = $("#content");
  const snapshot = state.snapshot;

  if (state.loadError) {
    container.innerHTML = `<div class="mx-auto max-w-[60rem]">
      <div class="rounded-[14px] border border-line bg-danger-bg px-4 py-3 text-base text-danger-text">${esc(state.loadError)}</div>
    </div>`;
    return;
  }
  if (!snapshot) {
    container.innerHTML = `<div class="mx-auto max-w-[60rem]"><p class="text-base text-ink-secondary">…</p></div>`;
    return;
  }

  const warnings = snapshot.warnings.length
    ? `<div class="mb-4 space-y-1.5 rounded-[14px] border border-line bg-warn-bg px-4 py-3">
         ${snapshot.warnings.map((line) => `<p class="text-base leading-relaxed text-warn-text">${esc(line)}</p>`).join("")}
       </div>`
    : "";

  const defaultCard = `
    <div class="hk-card flex flex-wrap items-center justify-between gap-3 p-4">
      <div class="min-w-0">
        <p class="text-sm font-semibold uppercase tracking-wide text-ink-muted">${esc(t("harness.defaultTitle"))}</p>
        ${
          snapshot.defaultProvider
            ? `<div class="mt-1 flex flex-wrap items-center gap-2">
                 ${logo(snapshot.defaultProvider, 24)}
                 <span class="text-lg font-semibold text-ink">${esc(
                   snapshot.providers.find((p) => p.id === snapshot.defaultProvider)?.name ?? snapshot.defaultProvider,
                 )}</span>
                 ${
                   snapshot.defaultModel
                     ? `<span class="selectable font-mono text-base text-ink-secondary">${esc(snapshot.defaultModel)}</span>`
                     : ""
                 }
               </div>
               <p class="mt-1 text-sm text-ink-secondary">${esc(
                 t("harness.defaultHint", { name: state.harnesses.find((h) => h.id === state.activeId)?.name ?? "" }),
               )}</p>`
            : `<p class="mt-1 text-base text-ink-muted">${esc(t("harness.notSet"))}</p>`
        }
      </div>
      <button data-action data-set-default class="hk-btn ${snapshot.defaultProvider ? "hk-btn-secondary" : "hk-btn-primary"} hk-btn-sm">
        ${snapshot.defaultProvider ? esc(t("harness.changeDefault")) : esc(t("harness.setDefault"))}
      </button>
    </div>`;

  const filesDisclosure = `
    <details class="group hk-card overflow-hidden">
      <summary class="flex cursor-pointer list-none items-center justify-between gap-2 px-4 py-3">
        <span class="flex items-center gap-2 text-base font-semibold text-ink">
          ${icon("chevronRight", "h-4 w-4 text-ink-muted transition group-open:rotate-90")}
          ${esc(t("harness.configFiles"))}
          <span class="text-sm font-medium text-ink-muted">${snapshot.files.length}</span>
        </span>
        <span class="text-sm text-ink-secondary">${esc(t("harness.explain"))}</span>
      </summary>
      <div class="border-t border-line">
        ${snapshot.files
          .map(
            (file) => `<div class="flex items-center gap-3 border-b border-line px-4 py-2.5 last:border-0">
              <span class="w-32 shrink-0 text-base text-ink-secondary">${esc(t(ROLE_KEY[file.role] ?? "file.config"))}</span>
              <span class="selectable min-w-0 flex-1 truncate font-mono text-sm text-ink-muted" title="${esc(file.path)}">${esc(
                file.path.replace(/^.*\/(\.?[^/]+\/[^/]+\/[^/]+)$/, "~/$1"),
              )}</span>
              <span class="shrink-0 text-sm font-semibold ${file.exists ? "text-ok-text" : "text-ink-muted"}">${esc(
                file.exists ? t("file.exists") : t("file.missing"),
              )}</span>
              <button data-action data-reveal="${esc(file.path)}" class="hk-btn hk-btn-icon shrink-0" aria-label="${esc(
                t("harness.showFolder"),
              )}">${icon("folder", "h-3.5 w-3.5")}</button>
            </div>`,
          )
          .join("")}
      </div>
    </details>`;

  const providersBlock = snapshot.providers.length
    ? `<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">${snapshot.providers.map(providerCard).join("")}</div>`
    : `<div class="hk-card px-6 py-12 text-center">
         <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-xl bg-surface-muted text-ink-muted">${icon("plug", "h-6 w-6")}</div>
         <p class="text-lg font-semibold text-ink">${esc(t("provider.empty.title"))}</p>
         <p class="mx-auto mt-1 max-w-sm text-base leading-relaxed text-ink-secondary">${esc(t("provider.empty.body"))}</p>
         <button data-action data-add-empty class="hk-btn hk-btn-primary mt-4">${icon("plus", "h-4 w-4")}<span>${esc(
           t("provider.empty.action"),
         )}</span></button>
       </div>`;

  container.innerHTML = `
    <div class="mx-auto max-w-[60rem]">
      ${warnings}
      <div class="mb-4 space-y-3">
        ${defaultCard}
        ${filesDisclosure}
      </div>
      <div class="mb-3 flex items-baseline gap-2">
        <h2 class="text-xl font-bold text-ink">${esc(t("harness.providers"))}</h2>
        <span class="text-base font-semibold text-ink-muted">${snapshot.providers.length}</span>
      </div>
      ${providersBlock}
    </div>`;

  wiringHarness(container);
}

function wiringHarness(container) {
  $$("[data-toggle]", container).forEach((button) => {
    button.addEventListener("click", () => toggleProvider(button.dataset.toggle));
  });
  $$("[data-menu-provider]", container).forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      const id = button.dataset.menuProvider;
      const provider = state.snapshot?.providers.find((item) => item.id === id);
      if (!provider) return;
      openMenu(button, [
        { label: t("provider.changeKey"), icon: "key", onSelect: () => openKeyModal(provider) },
        ...(provider.keyStored
          ? [
              {
                label: t("provider.forgetKey"),
                icon: "trash",
                danger: true,
                onSelect: () => askForgetKey(provider),
              },
            ]
          : []),
        {
          label: t("provider.changeModels"),
          icon: "list",
          onSelect: () => openModelsModal(provider),
        },
        {
          label: t("provider.changeDefaultModel"),
          icon: "star",
          onSelect: () => openProviderDefaultPicker(provider),
        },
        { separator: true },
        { label: t("provider.edit"), icon: "pencil", onSelect: () => openAdvancedForm("edit", provider) },
        { label: t("provider.delete"), icon: "trash", danger: true, onSelect: () => askDelete(id) },
      ]);
    });
  });
  $$("[data-reveal]", container).forEach((button) => {
    button.addEventListener("click", async () => {
      try {
        await api.reveal(button.dataset.reveal);
      } catch (error) {
        toast(t("toast.revealFailed"), { variant: "err", detail: String(error) });
      }
    });
  });

  const addEmpty = $("[data-add-empty]", container);
  if (addEmpty) addEmpty.addEventListener("click", openProviderPicker);
  const setDefault = $("[data-set-default]", container);
  if (setDefault) setDefault.addEventListener("click", openDefaultForm);
  wireExternalButtons(container);
}

function wireExternalButtons(root) {
  $$("[data-external]", root).forEach((button) => {
    button.addEventListener("click", () => {
      api.external(button.dataset.external).catch(() => {
        window.open(button.dataset.external, "_blank");
      });
    });
  });
}

function render() {
  if (state.view === "onboarding") {
    renderOnboarding();
    return;
  }
  if (state.view === "home") {
    renderHomeHeader();
    renderHome();
  } else if (state.view === "provider") {
    renderProviderForm(state.form.mode, state.form.provider);
  } else {
    renderHarnessHeader();
    renderHarness();
  }
}

function goHome() {
  state.view = "home";
  state.activeId = null;
  state.snapshot = null;
  state.loadError = null;
  render();
}

// ============================================================================
// DATA
// ============================================================================

async function loadHarnesses() {
  try {
    state.harnesses = await api.harnesses();
    state.loadError = null;
  } catch (error) {
    state.harnesses = [];
    state.loadError = `${t("error.loadHarnesses")} ${String(error)}`;
  }
  render();
  loadVersions();
}

/** Deteksi versi paralel - tidak memblokir render. */
async function loadVersions() {
  const ids = [
    ...state.harnesses.filter((harness) => harness.installed).map((harness) => harness.id),
    ...state.catalog.map((entry) => entry.id),
  ];
  const unique = [...new Set(ids)];
  await Promise.all(
    unique.map(async (id) => {
      if (state.versions[id] !== undefined) return;
      try {
        state.versions[id] = await api.version(id);
      } catch {
        state.versions[id] = null;
      }
    }),
  );
  if (state.view === "home") render();
}

async function loadSnapshot(harnessId) {
  state.snapshot = null;
  renderHarness();
  try {
    state.snapshot = await api.read(harnessId);
    state.loadError = null;
  } catch (error) {
    state.snapshot = null;
    state.loadError = String(error);
  }
  if (state.view === "harness" && state.activeId === harnessId) {
    renderHarnessHeader();
    renderHarness();
  }
}

async function selectHarness(harnessId) {
  state.activeId = harnessId;
  state.view = "harness";
  renderHarnessHeader();
  await loadSnapshot(harnessId);
}

async function refreshAll() {
  await loadHarnesses();
  if (state.view === "harness" && state.activeId) await loadSnapshot(state.activeId);
}

// ============================================================================
// TULIS KE DISK (pratinjau → terapkan)
// ============================================================================

function summaryFor(op) {
  const harnessName = state.harnesses.find((item) => item.id === state.activeId)?.name ?? "";
  const provider = state.snapshot?.providers.find((item) => item.id === (op.providerId ?? op.id));
  const name = provider?.name ?? op.id ?? "";

  switch (op.kind) {
    case "upsert":
      return [t("review.addProvider", { name: op.name || name, harness: harnessName })];
    case "remove":
      return [t("review.removeProvider", { name, harness: harnessName })];
    case "set_enabled":
      return [
        op.enabled
          ? t("review.enableProvider", { name })
          : t("review.disableProvider", { name }),
      ];
    case "set_default":
      return [t("review.setDefault", { provider: op.providerId, model: op.modelId })];
    case "forget_key":
      return [t("review.forgetKey", { name })];
    default:
      return [];
  }
}

async function commit(op, targetHarness = null) {
  const harnessId = targetHarness ?? state.activeId;
  let preview;

  try {
    preview = await api.preview(harnessId, op);
  } catch (error) {
    toast(t("toast.previewFailed"), { variant: "err", detail: String(error) });
    return;
  }

  const changed = preview.files.filter((file) => file.changed);
  if (changed.length === 0) {
    toast(t("review.noChanges"), { variant: "info" });
    return;
  }

  const summaries = [...summaryFor(op), t("review.backup")];
  const created = preview.willCreate ?? [];
  const isRemove = op.kind === "remove";

  const fileBlocks = changed
    .map(
      (file) => `
      <div class="mb-4 last:mb-0">
        <div class="mb-1.5 flex items-center gap-2">
          <span class="hk-chip hk-chip-neutral">${esc(t(ROLE_KEY[file.role] ?? "file.config"))}</span>
          <span class="selectable min-w-0 flex-1 truncate font-mono text-sm text-ink-secondary">${esc(file.path)}</span>
        </div>
        ${renderDiff(file.before ?? "", file.after ?? "")}
      </div>`,
    )
    .join("");

  const { overlay, close } = openModal({
    title: isRemove ? t("review.title") : t("review.title"),
    subtitle: t("review.subtitle"),
    size: "xl",
    body: `
      <ul class="mb-4 space-y-2 rounded-[14px] border border-line bg-surface-muted px-4 py-3">
        ${summaries
          .map(
            (line) => `<li class="flex gap-2 text-base leading-relaxed text-ink">
              <span class="mt-0.5 shrink-0 text-ok-text">${icon("check", "h-4 w-4")}</span>${esc(line)}
            </li>`,
          )
          .join("")}
        ${created.map((path) => `<li class="flex gap-2 text-base text-ink">${icon("plus", "h-4 w-4 shrink-0 text-ok-text")}${esc(t("review.newFile", { path }))}</li>`).join("")}
      </ul>
      <p class="mb-3 flex items-center gap-1.5 text-sm text-ink-muted">${icon("lock", "h-3.5 w-3.5")}${esc(t("review.secrets"))}</p>
      <details class="group">
        <summary class="mb-2 flex cursor-pointer list-none items-center gap-1.5 text-base font-semibold text-ink-secondary">
          ${icon("chevronRight", "h-4 w-4 transition group-open:rotate-90")}${esc(t("review.technical"))}
        </summary>
        ${fileBlocks}
      </details>`,
    footer: `
      <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
      <button data-action data-apply class="hk-btn hk-btn-primary">${esc(t("review.apply"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);
  $("[data-apply]", overlay).addEventListener("click", async () => {
    const button = $("[data-apply]", overlay);
    button.disabled = true;
    try {
      const result = await api.apply(harnessId, op);
      close();
      toast(t("toast.saved"), { detail: result.filesWritten.join("\n") });
      if (result.backupDir) toast(t("toast.backupSaved"), { variant: "info", detail: result.backupDir });
      offerNoticeFix(harnessId, result.notices ?? []);
      await refreshAll();
    } catch (error) {
      button.disabled = false;
      toast(t("toast.applyFailed"), { variant: "err", detail: String(error) });
    }
  });
}

/**
 * Config sudah tertulis, tapi harness bisa saja belum menampilkannya: pi
 * menyaring model lewat `enabledModels`, omp/OpenCode punya daftar disable.
 * Notice yang punya perbaikan otomatis ditawarkan sekali klik.
 */
function offerNoticeFix(harnessId, notices) {
  const fixable = notices.filter((notice) => notice.fix);
  if (fixable.length === 0) return;
  const notice = fixable[0];
  const more = fixable.length > 1 ? ` (+${fixable.length - 1})` : "";

  toast(t("toast.noticeHidden", { harness: notice.harness, provider: notice.provider }) + more, {
    variant: "info",
    detail: notice.fix,
    action: {
      label: t("toast.noticeFix"),
      onClick: async () => {
        try {
          const result = await api.fixNotice(harnessId, notice.code, notice.provider);
          toast(t("toast.noticeFixed"), { detail: result.filesWritten.join("\n") });
          offerNoticeFix(harnessId, result.notices ?? []);
          await refreshAll();
        } catch (error) {
          toast(t("toast.noticeFixFailed"), { variant: "err", detail: String(error) });
        }
      },
    },
  });
}

async function toggleProvider(providerId) {
  const provider = state.snapshot?.providers.find((item) => item.id === providerId);
  if (!provider) return;
  if (!provider.canToggle) {
    toast(t("toast.noToggle"), { variant: "info" });
    return;
  }

  const next = !(provider.enabled !== false);
  try {
    const result = await api.apply(state.activeId, {
      kind: "set_enabled",
      providerId,
      enabled: next,
    });
    // Perubahan default dan penjelasan "provider ini juga bawaan harness" hanya
    // ada di warnings; menyembunyikannya membuat tombol ini tampak tidak bekerja.
    const detail = (result.warnings ?? []).join("\n");
    toast(next ? t("toast.providerEnabled", { name: provider.name }) : t("toast.providerDisabled", { name: provider.name }), {
      detail: detail || undefined,
      action: {
        label: t("toast.undo"),
        onClick: async () => {
          try {
            await api.apply(state.activeId, { kind: "set_enabled", providerId, enabled: !next });
            await refreshAll();
          } catch (error) {
            toast(t("toast.toggleFailed"), { variant: "err", detail: String(error) });
          }
        },
      },
    });
    offerNoticeFix(state.activeId, result.notices ?? []);
    await refreshAll();
  } catch (error) {
    toast(t("toast.toggleFailed"), { variant: "err", detail: String(error) });
  }
}

/**
 * Hapus kredensial tersimpan satu provider.
 *
 * Satu-satunya cara membuat harness berhenti menawarkan provider bawaan: selama
 * kredensialnya ada, pi tetap mengenalinya walau entri kita di models.json sudah
 * dinonaktifkan. Destruktif, jadi selalu lewat konfirmasi, dan file di-backup.
 */
async function askForgetKey(provider) {
  const ok = await confirmDialog({
    title: t("forget.title", { name: provider.name || provider.id }),
    body: t("forget.body"),
    bullets: [t("forget.keepsProvider"), t("forget.backup"), t("forget.env")],
    confirmLabel: t("forget.confirm"),
    danger: true,
  });
  if (!ok) return;

  try {
    const result = await api.apply(state.activeId, {
      kind: "forget_key",
      providerId: provider.id,
    });
    const detail = (result.warnings ?? []).join("\n");
    toast(t("forget.done", { name: provider.name || provider.id }), {
      detail: detail || undefined,
    });
    if (result.backupDir) toast(t("toast.backupSaved"), { variant: "info", detail: result.backupDir });
    await refreshAll();
  } catch (error) {
    toast(t("forget.failed"), { variant: "err", detail: String(error) });
  }
}

async function askDelete(providerId) {
  const provider = state.snapshot?.providers.find((item) => item.id === providerId);
  const harnessName = state.harnesses.find((item) => item.id === state.activeId)?.name ?? "";

  if (state.prefs.confirmDelete !== false) {
    const ok = await confirmDialog({
      title: t("delete.title", { name: provider?.name ?? providerId }),
      body: t("delete.body", { name: provider?.name ?? providerId, harness: harnessName }),
      bullets: [t("delete.bulletBackup"), t("delete.bulletOthers")],
      confirmLabel: t("delete.confirm"),
      danger: true,
      allowDontAsk: true,
    });
    if (!ok) return;
  }

  await commit({ kind: "remove", providerId });
}


// ============================================================================
// AKSI PER PROVIDER: ganti key, pilih model, jadikan default
//
// Tiga aksi ini sengaja dipisah supaya mengganti key tidak memaksa memilih
// ulang model (dan sebaliknya). Semuanya memakai EditOp khusus yang hanya
// menyentuh satu hal.
// ============================================================================

function activeHarnessId() {
  return state.activeId;
}

/** Ganti key saja: inline ke file kredensial, atau referensi env. */
function openKeyModal(provider) {
  const harnessId = activeHarnessId();
  const envName = `${provider.id.replace(/[^A-Za-z0-9]+/g, "_").toUpperCase()}_API_KEY`;
  let mode = provider.keyDisplay?.startsWith("$") ? "env" : "inline";

  const { overlay, close } = openModal({
    title: t("provider.changeKeyTitle", { name: provider.name || provider.id }),
    subtitle: t("provider.changeKeySubtitle"),
    size: "md",
    body: `
      <div class="space-y-4">
        <div class="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-secondary">
          ${esc(t("advanced.keyCurrentLabel"))}
          <span class="selectable font-mono font-semibold text-ink">${esc(provider.keyDisplay ?? t("provider.noKey"))}</span>
        </div>
        <div>
          <label class="hk-label">${esc(t("add.keyLabel"))}</label>
          <div class="hk-key-wrap">
            <input data-key type="password" value="" autocomplete="off" spellcheck="false"
              placeholder="${esc(t("advanced.keyReplace"))}" class="hk-input" />
            <button data-action data-toggle-key class="hk-key-toggle">${esc(t("add.show"))}</button>
          </div>
        </div>
        <div class="space-y-2">
          <label class="flex cursor-pointer items-start gap-2 text-base text-ink">
            <input type="radio" name="keyMode" value="inline" ${mode === "inline" ? "checked" : ""}
              class="mt-0.5 accent-[var(--color-brand)]" />
            <span>${esc(t("advanced.keyInline"))}</span>
          </label>
          <label class="flex cursor-pointer items-start gap-2 text-base text-ink">
            <input type="radio" name="keyMode" value="env" ${mode === "env" ? "checked" : ""}
              class="mt-0.5 accent-[var(--color-brand)]" />
            <span>
              ${esc(t("advanced.keyEnv"))}
              <span class="selectable font-mono text-xs text-ink-secondary">${esc(envName)}</span>
            </span>
          </label>
        </div>
      </div>`,
    footer: `
      <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
      <button data-action data-save-key class="hk-btn hk-btn-primary">${esc(t("provider.changeKeySave"))}</button>`,
  });

  const input = $("[data-key]", overlay);
  const toggle = $("[data-toggle-key]", overlay);
  toggle?.addEventListener("click", () => {
    const hidden = input.type === "password";
    input.type = hidden ? "text" : "password";
    toggle.textContent = hidden ? t("add.hide") : t("add.show");
  });
  input.focus();

  $("[data-cancel]", overlay).addEventListener("click", close);
  $("[data-save-key]", overlay).addEventListener("click", async () => {
    const key = input.value.trim();
    if (!key) return toast(t("add.needKey"), { variant: "err" });
    const selected = $("input[name=keyMode]:checked", overlay)?.value ?? "inline";
    close();
    await commit({
      kind: "set_key",
      providerId: provider.id,
      key,
      keyMode: selected,
    }, harnessId);
  });
}

/** Nyalakan/matikan model milik satu provider (tanpa menyentuh key). */
function openModelsModal(provider) {
  const harnessId = activeHarnessId();
  const current = new Map(
    (provider.models ?? []).map((model) => [
      model.id,
      { on: true, name: model.name ?? "", contextWindow: model.contextWindow ?? null },
    ]),
  );

  if (!current.size) {
    toast(t("provider.noModelsToToggle"), { variant: "info" });
    return;
  }

  const { overlay, close } = openModal({
    title: t("provider.changeModelsTitle", { name: provider.name || provider.id }),
    subtitle: t("provider.changeModelsSubtitle"),
    size: "md",
    body: `
      <input data-search type="search" class="hk-input mb-2" autocomplete="off"
        placeholder="${esc(t("provider.searchModels"))}" />
      <div data-list class="max-h-[20rem] space-y-1 overflow-y-auto"></div>
      <p data-count class="mt-2 px-0.5 text-xs font-medium text-ink-muted"></p>`,
    footer: `
      <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
      <button data-action data-save-models class="hk-btn hk-btn-primary">${esc(t("action.save"))}</button>`,
  });

  const list = $("[data-list]", overlay);
  const search = $("[data-search]", overlay);
  const count = $("[data-count]", overlay);

  const render = () => {
    const term = search.value.trim().toLowerCase();
    const rows = [...current.entries()].filter(
      ([id, model]) => !term || id.toLowerCase().includes(term) || model.name.toLowerCase().includes(term),
    );
    list.innerHTML = rows
      .map(
        ([id, model]) => `
        <label class="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-surface-muted">
          <input data-model="${esc(id)}" type="checkbox" class="hk-check" ${model.on ? "checked" : ""} />
          <span class="min-w-0 flex-1 truncate text-base text-ink">${esc(model.name || prettyModel(id))}</span>
          <span class="shrink-0 truncate font-mono text-2xs text-ink-muted" style="max-width:45%">${esc(id)}</span>
        </label>`,
      )
      .join("");
    const active = [...current.values()].filter((model) => model.on).length;
    count.textContent = t("provider.modelsSelected", { active, total: current.size });
  };
  search.addEventListener("input", render);
  list.addEventListener("change", (event) => {
    const id = event.target?.dataset?.model;
    if (!id || !current.has(id)) return;
    current.get(id).on = event.target.checked;
    render();
  });
  render();

  $("[data-cancel]", overlay).addEventListener("click", close);
  $("[data-save-models]", overlay).addEventListener("click", async () => {
    const models = [...current.entries()]
      .filter(([, model]) => model.on)
      .map(([id, model]) => ({
        id,
        name: model.name || null,
        contextWindow: model.contextWindow ?? null,
      }));
    if (!models.length) return toast(t("toast.needOneModel"), { variant: "err" });
    close();
    await commit({ kind: "set_models", providerId: provider.id, models }, harnessId);
  });
}

/** Jadikan salah satu model provider ini sebagai default harness. */
function openProviderDefaultPicker(provider) {
  const models = provider.models ?? [];
  if (!models.length) {
    toast(t("provider.noModelsToToggle"), { variant: "info" });
    return;
  }
  const harnessId = activeHarnessId();
  const snapshot = state.snapshot;
  const currentModel = snapshot?.defaultProvider === provider.id ? snapshot?.defaultModel : null;

  const { overlay, close } = openModal({
    title: t("provider.changeDefaultTitle", { name: provider.name || provider.id }),
    subtitle: t("provider.changeDefaultSubtitle"),
    size: "md",
    body: `
      <input data-search type="search" class="hk-input mb-2" autocomplete="off"
        placeholder="${esc(t("provider.searchModels"))}" />
      <div data-list class="max-h-[20rem] space-y-1 overflow-y-auto"></div>`,
    footer: `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>`,
  });

  const list = $("[data-list]", overlay);
  const search = $("[data-search]", overlay);
  const render = () => {
    const term = search.value.trim().toLowerCase();
    const rows = models.filter(
      (model) =>
        !term || model.id.toLowerCase().includes(term) || (model.name ?? "").toLowerCase().includes(term),
    );
    list.innerHTML = rows
      .map(
        (model) => `
        <button data-action data-pick="${esc(model.id)}"
          class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left hover:bg-brand-soft">
          <span class="shrink-0 text-ink-muted">${model.id === currentModel ? icon("check", "h-4 w-4 text-brand-strong") : icon("star", "h-4 w-4")}</span>
          <span class="min-w-0 flex-1 truncate text-base text-ink">${esc(model.name || prettyModel(model.id))}</span>
          <span class="shrink-0 truncate font-mono text-2xs text-ink-muted" style="max-width:45%">${esc(model.id)}</span>
        </button>`,
      )
      .join("");
  };
  search.addEventListener("input", render);
  render();

  $("[data-cancel]", overlay).addEventListener("click", close);
  $$("[data-pick]", list).forEach((button) => {
    button.addEventListener("click", async () => {
      const modelId = button.dataset.pick;
      close();
      await commit({ kind: "set_default", providerId: provider.id, modelId }, harnessId);
    });
  });
}

// ============================================================================
// TAMBAH PROVIDER - pilih → tempel key → tes & cari model → centang
// ============================================================================

function providerPickCard(preset) {
  const badge =
    preset.badge === "free"
      ? chip("ok", t("preset.free"))
      : preset.badge === "local"
        ? chip("neutral", t("preset.local"))
        : "";
  return `<button data-action data-pick="${esc(preset.id)}"
    class="hk-card flex flex-col items-center gap-2 p-3 text-center transition hover:-translate-y-0.5 hover:border-brand/40 hover:shadow-md">
    ${logo(preset.id, 40)}
    <span class="w-full truncate text-base font-semibold text-ink">${esc(preset.name)}</span>
    ${badge || '<span class="h-[1.375rem]"></span>'}
  </button>`;
}

function openProviderPicker() {
  const { overlay, close } = openModal({
    title: t("provider.add"),
    subtitle: t("provider.addSubtitle"),
    size: "lg",
    body: `
      <div class="mb-3">
        <input data-search type="search" class="hk-input" style="height:40px;font-size:14px"
          placeholder="${esc(t("provider.search"))}" autocomplete="off" />
      </div>
      <div data-grid class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
        ${state.presets.map(providerPickCard).join("")}
      </div>
      <div data-empty class="hidden py-8 text-center text-base text-ink-muted">-</div>
      <div class="mt-4 flex items-center gap-2.5 border-t border-line pt-4">
        ${logo(FALLBACK_LOGO, 32)}
        <button data-action data-custom class="text-left text-base font-semibold text-brand-text hover:underline">
          ${esc(t("provider.custom"))} →
        </button>
      </div>`,
    footer: `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);
  $("[data-custom]", overlay).addEventListener("click", () => {
    close();
    openAdvancedForm("add");
  });

  const search = $("[data-search]", overlay);
  const grid = $("[data-grid]", overlay);
  const empty = $("[data-empty]", overlay);
  search.addEventListener("input", () => {
    const term = search.value.trim().toLowerCase();
    let visible = 0;
    $$("[data-pick]", grid).forEach((card) => {
      const preset = state.presets.find((item) => item.id === card.dataset.pick);
      const hit = !term || `${preset?.name} ${preset?.id}`.toLowerCase().includes(term);
      card.classList.toggle("hidden", !hit);
      if (hit) visible += 1;
    });
    empty.classList.toggle("hidden", visible > 0);
  });

  $$("[data-pick]", grid).forEach((card) => {
    card.addEventListener("click", () => {
      const preset = state.presets.find((item) => item.id === card.dataset.pick);
      if (!preset) return;
      close();
      openAddFlow(preset);
    });
  });
}

/** Pretty-print id model untuk ditampilkan sebagai nama: gpt-oss-120b → GPT OSS 120B */
function prettyModel(id) {
  return String(id)
    .replace(/[._/]/g, "-")
    .split("-")
    .filter(Boolean)
    .map((part) => (/^[a-z]/.test(part) ? part.charAt(0).toUpperCase() + part.slice(1) : part))
    .join(" ");
}

function openAddFlow(preset) {
  const entries = new Map(); // id -> { selected, label }
  for (const model of preset.models ?? []) entries.set(model.id, { selected: true, label: model.name || prettyModel(model.id) });

  const { overlay, close } = openModal({
    title: t("add.title", { name: preset.name }),
    subtitle: t("add.subtitle"),
    size: "lg",
    body: `
      <div class="space-y-4">
        <div>
          <label class="hk-label">${esc(t("add.keyLabel"))}</label>
          <div class="hk-key-wrap">
            <input data-key type="password" class="hk-input" autocomplete="off" spellcheck="false"
              placeholder="${esc(t("add.keyPlaceholder"))}" />
            <button data-action data-toggle-key class="hk-key-toggle">${esc(t("add.show"))}</button>
          </div>
          ${
            preset.docsUrl
              ? `<p class="hk-help">${esc(t("add.getKey"))}
                   <button data-action data-external="${esc(preset.docsUrl)}" class="selectable font-mono font-semibold text-brand-text hover:underline">${esc(
                     preset.docsUrl.replace(/^https?:\/\//, ""),
                   )}</button></p>`
              : ""
          }
        </div>

        <div class="flex flex-wrap gap-2">
          <button data-action data-test class="hk-btn hk-btn-primary hk-btn-sm">
            ${icon("wand", "h-4 w-4")}<span>${esc(t("add.testAndFind"))}</span>
          </button>
        </div>

        <div data-status class="hidden space-y-1.5"></div>

        <div data-models-host class="hidden">
          <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
            <span class="hk-label mb-0">${esc(t("models"))}</span>
            <div class="flex items-center gap-1">
              <button data-action data-all class="hk-btn hk-btn-ghost hk-btn-sm">${esc(t("add.selectAll"))}</button>
              <button data-action data-none class="hk-btn hk-btn-ghost hk-btn-sm">${esc(t("add.clear"))}</button>
            </div>
          </div>
          <input data-filter type="search" class="hk-input mb-2" style="height:36px;font-size:13px"
            placeholder="${esc(t("add.filterModels"))}" autocomplete="off" />
          <div data-models class="max-h-60 space-y-1 overflow-y-auto rounded-[14px] border border-line bg-surface-muted p-2"></div>
          <p data-count class="mt-2 text-sm font-medium text-ink-secondary"></p>
        </div>
      </div>`,
    footer: `
      <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
      <button data-action data-save class="hk-btn hk-btn-primary" disabled data-save>${esc(t("add.saveEmpty"))}</button>`,
  });

  const keyInput = $("[data-key]", overlay);
  const statusBox = $("[data-status]", overlay);
  const host = $("[data-models-host]", overlay);
  const list = $("[data-models]", overlay);
  const countLabel = $("[data-count]", overlay);
  const saveButton = $("[data-save]", overlay);
  const filterInput = $("[data-filter]", overlay);

  const setStatus = (lines) => {
    statusBox.className = "space-y-1.5";
    statusBox.innerHTML = lines
      .map(
        (line) => `<p class="flex items-start gap-2 rounded-lg px-3 py-2 text-base leading-relaxed ${
          line.tone === "ok"
            ? "bg-ok-bg text-ok-text"
            : line.tone === "err"
              ? "bg-danger-bg text-danger-text"
              : "bg-warn-bg text-warn-text"
        }">
          <span class="mt-0.5 shrink-0">${icon(line.tone === "ok" ? "check" : line.tone === "err" ? "x" : "alert", "h-4 w-4")}</span>
          <span>${esc(line.text)}</span>
        </p>`,
      )
      .join("");
  };

  const selectedCount = () => [...entries.values()].filter((entry) => entry.selected).length;

  const syncSaveLabel = () => {
    const count = selectedCount();
    saveButton.textContent = count === 0 ? t("add.saveEmpty") : count === 1 ? t("add.saveOne") : t("add.save", { count });
    saveButton.disabled = count === 0;
  };

  const renderModels = () => {
    const term = filterInput.value.trim().toLowerCase();
    const items = [...entries.entries()].filter(
      ([id, meta]) => !term || id.toLowerCase().includes(term) || meta.label.toLowerCase().includes(term),
    );

    if (entries.size === 0) {
      host.classList.add("hidden");
      return;
    }
    host.classList.remove("hidden");

    list.innerHTML = items.length
      ? items
          .map(([id, meta]) => {
            const index = [...entries.keys()].indexOf(id);
            return `<label class="flex cursor-pointer items-center gap-2.5 rounded-lg bg-surface px-2.5 py-2 hover:bg-surface-muted">
              <input data-check="${index}" type="checkbox" class="hk-check" ${meta.selected ? "checked" : ""} />
              <span class="min-w-0 flex-1 truncate text-base font-medium text-ink">${esc(meta.label)}</span>
              <span class="selectable shrink-0 truncate font-mono text-xs text-ink-muted" style="max-width:45%">${esc(id)}</span>
            </label>`;
          })
          .join("")
      : `<p class="px-2 py-4 text-center text-base text-ink-muted">-</p>`;

    $$("[data-check]", list).forEach((checkbox) => {
      checkbox.addEventListener("change", () => {
        const id = [...entries.keys()][Number(checkbox.dataset.check)];
        const meta = entries.get(id);
        if (meta) meta.selected = checkbox.checked;
        updateCount();
        syncSaveLabel();
      });
    });
    updateCount();
  };

  const updateCount = () => {
    countLabel.textContent = t("add.selected", { selected: selectedCount(), total: entries.size });
  };

  filterInput.addEventListener("input", renderModels);
  $("[data-all]", overlay).addEventListener("click", () => {
    entries.forEach((meta) => (meta.selected = true));
    renderModels();
    syncSaveLabel();
  });
  $("[data-none]", overlay).addEventListener("click", () => {
    entries.forEach((meta) => (meta.selected = false));
    renderModels();
    syncSaveLabel();
  });

  $("[data-toggle-key]", overlay).addEventListener("click", (event) => {
    const shown = keyInput.type === "text";
    keyInput.type = shown ? "password" : "text";
    event.currentTarget.textContent = shown ? t("add.show") : t("add.hide");
  });

  if (entries.size) {
    host.classList.remove("hidden");
    renderModels();
  }
  syncSaveLabel();

  $("[data-test]", overlay).addEventListener("click", async () => {
    const key = keyInput.value.trim();
    if (!key && preset.envVar) {
      setStatus([{ tone: "err", text: t("add.needKey") }]);
      return;
    }
    setStatus([{ tone: "neutral", text: t("add.testing") }]);

    const lines = [];
    try {
      const result = await api.test(preset.baseUrl, preset.api, key);
      if (result.ok) {
        lines.push({
          tone: "ok",
          text:
            result.latencyMs != null
              ? t("add.connected", { ms: result.latencyMs })
              : t("add.connectedNoTime"),
        });
      } else if (String(result.message ?? "").match(/401|403|invalid.*key/i)) {
        lines.push({ tone: "err", text: t("add.invalidKey") });
      } else {
        lines.push({ tone: "err", text: result.message || t("add.unreachable", { name: preset.name }) });
      }
    } catch (error) {
      lines.push({ tone: "err", text: String(error) });
    }

    try {
      const models = await api.models(preset.baseUrl, preset.api, key);
      if (models.length) {
        if (entries.size === 0) {
          for (const model of models) entries.set(model.id, { selected: true, label: prettyModel(model.id) });
        } else {
          for (const model of models) {
            if (!entries.has(model.id)) entries.set(model.id, { selected: true, label: prettyModel(model.id) });
          }
        }
        renderModels();
        syncSaveLabel();
        lines.push({
          tone: "ok",
          text: models.length === 1 ? t("add.modelsFoundOne") : t("add.modelsFound", { count: models.length }),
        });
      } else {
        lines.push({ tone: "warn", text: t("add.modelsFailed") });
      }
    } catch {
      lines.push({ tone: "warn", text: t("add.modelsFailed") });
    }

    if (entries.size) host.classList.remove("hidden");
    setStatus(lines);
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  saveButton.addEventListener("click", async () => {
    const picked = [...entries.entries()].filter(([, meta]) => meta.selected).map(([id]) => id);
    if (picked.length === 0) {
      toast(t("toast.pickModel"), { variant: "err" });
      return;
    }
    const known = new Map((preset.models ?? []).map((model) => [model.id, model]));

    close();
    await commit({
      kind: "upsert",
      id: preset.id,
      name: preset.name,
      baseUrl: preset.baseUrl,
      api: preset.api,
      key: keyInput.value.trim() || null,
      keyMode: "inline",
      models: picked.map((id) => {
        const base = known.get(id);
        return {
          id,
          name: base?.name ?? prettyModel(id),
          contextWindow: base?.contextWindow ?? null,
          maxTokens: base?.maxTokens ?? null,
          reasoning: Boolean(base?.reasoning),
        };
      }),
    });
  });
}

// ============================================================================
// FORM LANJUTAN (custom endpoint / edit provider)
// ============================================================================

/** Penanda kemampuan model - simbol saja, sengaja TIDAK terlihat bisa diklik. */
function capMark(iconName, on, title) {
  return `<span class="inline-flex ${on ? "text-brand-strong" : "text-ink-muted opacity-45"}"
    title="${esc(title)}" aria-label="${esc(title)}" role="img">${icon(iconName, "h-4 w-4")}</span>`;
}

function capMarks(model) {
  return `<span class="flex shrink-0 items-center gap-2 px-1">
    ${capMark("file", true, t("cap.textTitle"))}
    ${capMark(model.vision ? "eye" : "eyeLow", Boolean(model.vision), t("cap.visionTitle"))}
    ${capMark("brain", Boolean(model.reasoning), t("cap.reasoningTitle"))}
  </span>`;
}

function modelRow(index, model) {
  return `<div class="rounded-[14px] border border-line bg-surface-muted p-3" data-row="${index}">
    <div class="flex flex-wrap items-center gap-1.5">
      <input data-field="id" value="${esc(model.id)}" placeholder="model-id" title="${esc(t("advanced.modelId"))}"
        class="hk-input min-w-0 flex-1" style="height:36px;font-family:var(--font-mono);font-size:12px;min-width:120px" />
      <input data-field="name" value="${esc(model.name ?? "")}" placeholder="${esc(t("advanced.modelName"))}"
        class="hk-input min-w-0 flex-1" style="height:36px;font-size:12px;min-width:110px" />
      ${capMarks(model)}
      <button data-action data-test-model="${index}" class="hk-btn hk-btn-icon h-9 w-9" title="${esc(t("advanced.testModel"))}">
        ${icon("wand", "h-4 w-4")}
      </button>
      <button data-action data-default-model="${index}" class="hk-btn hk-btn-icon h-9 w-9" title="${esc(t("harness.setDefault"))}">
        ${icon("sparkle", "h-4 w-4")}
      </button>
      <button data-action data-remove="${index}" class="hk-btn hk-btn-icon h-9 w-9 text-danger-text" title="${esc(t("advanced.removeModel"))}">
        ${icon("trash", "h-4 w-4")}
      </button>
    </div>

    <details data-advanced-row class="mt-3">
      <summary class="hk-btn hk-btn-ghost hk-btn-sm inline-flex w-auto">
        ${icon("gear", "h-3.5 w-3.5")}<span>${esc(t("advanced.more"))}</span>
        ${icon("chevronDown", "h-3.5 w-3.5")}
      </summary>
      <div class="mt-2.5 grid grid-cols-2 gap-2 rounded-lg border border-line bg-surface p-2.5">
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-ink-muted">${esc(t("advanced.modelContext"))}</span>
          <input data-field="contextWindow" value="${esc(model.contextWindow ?? "")}" inputmode="numeric"
            class="hk-input" style="height:34px;font-family:var(--font-mono);font-size:12px" />
        </label>
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-ink-muted">${esc(t("advanced.modelMax"))}</span>
          <input data-field="maxTokens" value="${esc(model.maxTokens ?? "")}" inputmode="numeric"
            class="hk-input" style="height:34px;font-family:var(--font-mono);font-size:12px" />
        </label>
        <div class="col-span-2 flex flex-wrap items-center gap-3 border-t border-line pt-2">
          <label class="flex cursor-pointer items-center gap-1.5 text-sm text-ink">
            <input data-field="vision" type="checkbox" class="hk-check" ${model.vision ? "checked" : ""} />
            ${icon("eye", "h-4 w-4 text-ink-secondary")}${esc(t("cap.vision"))}
          </label>
          <label class="flex cursor-pointer items-center gap-1.5 text-sm text-ink">
            <input data-field="reasoning" type="checkbox" class="hk-check" ${model.reasoning ? "checked" : ""} />
            ${icon("brain", "h-4 w-4 text-ink-secondary")}${esc(t("cap.reasoning"))}
          </label>
        </div>
      </div>
    </details>
  </div>`;
}

/** Kembali dari halaman form ke detail harness. */
function backToHarness() {
  state.view = "harness";
  render();
  loadSnapshot(state.activeId);
}

function openAdvancedForm(mode, provider) {
  state.form = { mode, provider };
  state.view = "provider";
  render();
}

function renderProviderForm(mode, provider) {
  const isEdit = mode === "edit";
  const models = provider?.models?.length
    ? provider.models.map((model) => ({
        id: model.id,
        name: model.name ?? "",
        contextWindow: model.contextWindow ?? "",
        maxTokens: model.maxTokens ?? "",
        reasoning: Boolean(model.reasoning),
        vision: Boolean(model.vision),
      }))
    : [{ id: "", name: "", contextWindow: "", maxTokens: "", reasoning: false }];

  // Provider yang key-nya sudah tersimpan tidak perlu menempel ulang: backend
  // yang membaca key dari file config lewat adapter-nya, UI tidak pernah
  // menerima nilainya. Berlaku untuk key inline maupun `$VAR`.
  const storedKeyAvailable = isEdit && provider?.hasKey === true;

  const keyHint = isEdit
    ? provider?.keyDisplay
      ? t("advanced.keyCurrent", { key: provider.keyDisplay })
      : t("advanced.keyMissing")
    : null;

  // Nama file tempat key inline ditulis, supaya radio "simpan di file config"
  // tidak abstrak. Diambil dari file sensitif milik harness (auth.json / .env /
  // models.yml) yang dikirim backend lewat snapshot.
  const keyFilePath = state.snapshot?.files?.find((file) => file.sensitive)?.path ?? "";
  const keyFile = keyFilePath.split(/[\\/]/).pop() ?? "";

  // Provider dari katalog Hakonfig: endpoint, tipe API, dan nama sudah kita
  // simpan sendiri, jadi jangan tanya lagi. Kalau ternyata isinya sudah beda
  // dari katalog (mis. pernah diubah manual), tampilkan field-nya kembali.
  const preset = isEdit ? state.presets.find((item) => item.id === provider?.id) : null;
  // Nama tampilan tidak ikut dibandingkan: mengganti nama tidak boleh
  // memunculkan kembali field endpoint.
  const managed = isRegisteredProvider(preset, provider);
  const presetValue = preset?.api ?? null;

  const bodyHtml = `
      <div class="space-y-5">
        ${
          isEdit
            ? ""
            : `<div class="rounded-lg bg-surface-muted px-3 py-2 text-sm leading-relaxed text-ink-secondary">
                 ${esc(t("advanced.badge"))} · ${esc(t("advanced.backToPresets"))}
               </div>`
        }

        <section>
          <h3 class="mb-3 text-base font-bold uppercase tracking-wide text-ink-muted">${esc(t("advanced.sectionEndpoint"))}</h3>
          ${
            managed
              ? `<div class="flex items-start gap-3 rounded-[14px] border border-line bg-surface px-3.5 py-3">
                   ${logo(provider.id, 36)}
                   <div class="min-w-0 flex-1">
                     <p class="truncate text-base font-semibold text-ink">${esc(provider.name)}</p>
                     <p class="truncate font-mono text-xs text-ink-muted">${esc(provider.baseUrl ?? "")}</p>
                     <p class="mt-1 text-xs leading-relaxed text-ink-muted">${esc(
                       t("advanced.registeredEndpoint"),
                     )} · ${esc(provider.api ?? "")}</p>
                   </div>
                 </div>`
              : `<div class="space-y-3">
            <div>
              <label class="hk-label">${esc(t("advanced.baseUrl"))} *</label>
              <input data-field="baseUrl" value="${esc(provider?.baseUrl ?? "")}" placeholder="https://api.example.com/v1"
                class="hk-input" style="font-family:var(--font-mono);font-size:13px" />
              <p class="hk-help">${esc(t("advanced.baseUrlHelp"))}</p>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="hk-label">${esc(t("advanced.apiType"))}</label>
                <select data-field="api" class="hk-input">
                  ${API_OPTIONS.map(
                    (value) => `<option value="${value}" ${(presetValue ?? provider?.api ?? "openai-completions") === value ? "selected" : ""}>${value}</option>`,
                  ).join("")}
                </select>
              </div>
              <div>
                <label class="hk-label">${esc(t("advanced.idLabel"))} *</label>
                <input data-field="providerId" value="${esc(provider?.id ?? "")}" placeholder="my-provider" ${isEdit ? "readonly" : ""}
                  class="hk-input" style="font-family:var(--font-mono);font-size:13px;${isEdit ? "opacity:.6" : ""}" />
              </div>
            </div>
            <div>
              <label class="hk-label">${esc(t("advanced.nameLabel"))} *</label>
              <input data-field="name" value="${esc(provider?.name ?? "")}" placeholder="My provider" class="hk-input" />
            </div>
          </div>`
          }
        </section>

        <section>
          <h3 class="mb-3 text-base font-bold uppercase tracking-wide text-ink-muted">
            ${esc(t("advanced.sectionKey"))}
            ${
              isEdit
                ? `<span class="ml-1 font-medium normal-case tracking-normal text-ink-secondary">${esc(
                    t("advanced.keyKeepInline"),
                  )}</span>`
                : ""
            }
          </h3>
          <div class="hk-key-wrap">
            <input data-field="key" type="password" value="" autocomplete="off" spellcheck="false"
              placeholder="${esc(isEdit ? t("advanced.keyReplace") : t("add.keyPlaceholder"))}" class="hk-input" />
            <button data-action data-toggle-key class="hk-key-toggle">${esc(t("add.show"))}</button>
          </div>
          ${
            keyHint
              ? `<p class="mt-3 flex flex-wrap items-center gap-x-1.5 rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-secondary">
                   ${esc(t("advanced.keyCurrentLabel"))} <span class="selectable font-mono font-semibold text-ink">${esc(
                     provider?.keyDisplay ?? "-",
                   )}</span>
                 </p>`
              : ""
          }
          <div class="mt-3 space-y-2">
            <label class="flex cursor-pointer items-start gap-2 text-base text-ink">
              <input type="radio" name="keyMode" value="inline" checked class="mt-0.5 accent-[var(--color-brand)]" />
              <span>
                ${esc(t("advanced.keyInline"))}
                <span data-key-mode-hint="inline" class="mt-0.5 text-sm leading-relaxed text-ink-secondary">${
                  keyFile
                    ? esc(t("advanced.keyInlineWhere", { file: keyFile }))
                    : ""
                }</span>
              </span>
            </label>
            <label class="flex cursor-pointer items-start gap-2 text-base text-ink">
              <input type="radio" name="keyMode" value="env" class="mt-0.5 accent-[var(--color-brand)]" />
              <span>
                ${esc(t("advanced.keyEnv"))}
                <span data-key-mode-hint="env" class="mt-0.5 text-sm leading-relaxed text-ink-secondary">
                  ${esc(t("advanced.keyEnvWhere"))} <span data-env-var class="selectable font-mono font-semibold text-ink">${esc(
                    "\u2039provider-id\u203a_API_KEY",
                  )}</span>
                </span>
              </span>
            </label>
          </div>
        </section>

        <section>
          <div class="mb-3 flex items-center justify-between">
            <h3 class="text-base font-bold uppercase tracking-wide text-ink-muted">${esc(t("advanced.sectionModels"))}</h3>
            <div class="flex items-center gap-1">
              <button data-action data-sync-models class="hk-btn hk-btn-secondary hk-btn-sm">
                ${icon("refresh", "h-3.5 w-3.5")}${esc(t("advanced.syncModels"))}
              </button>
              <button data-action data-add-model class="hk-btn hk-btn-secondary hk-btn-sm">
                ${icon("plus", "h-3.5 w-3.5")}${esc(t("advanced.addModel"))}
              </button>
            </div>
          </div>
          <div data-search-host class="mb-2 hidden rounded-[14px] border border-brand/30 bg-brand-soft/40 p-2.5">
            <input data-search-input type="search" class="hk-input" style="height:38px;font-size:13px"
              placeholder="${esc(t("advanced.searchModelPlaceholder"))}" autocomplete="off" />
            <div data-search-results class="mt-2 max-h-48 space-y-1 overflow-y-auto"></div>
          </div>
          <div data-models class="space-y-2">${models.map((model, index) => modelRow(index, model)).join("")}</div>
        </section>
      </div>`;

  const harnessName = state.harnesses.find((item) => item.id === state.activeId)?.name ?? "";
  // `#content` sudah punya px-6 (24px), jadi wrapper isi cukup max-w-[55rem].
  // Topbar & bar bawah membungkus padding-nya sendiri, jadi lebarnya 880 + 48
  // supaya tombol back/Simpan tepat segaris dengan tepi kolom isi.
  $("#topbar").innerHTML = `
    <div class="mx-auto flex w-full max-w-[58rem] items-center gap-3 px-6 py-3">
      <button data-action data-back class="hk-btn hk-btn-icon" aria-label="${esc(t("action.cancel"))}">
        ${icon("arrowLeft")}
      </button>
      <div class="min-w-0">
        <p class="truncate text-lg font-bold leading-tight text-ink">${esc(
          isEdit ? t("provider.editTitle", { name: provider?.name ?? "" }) : t("advanced.title"),
        )}</p>
        <p class="truncate text-sm leading-tight text-ink-secondary">${esc(harnessName)}</p>
      </div>
    </div>`;
  const container = $("#content");
  container.innerHTML = `
    <div class="mx-auto max-w-[55rem] pb-28">
      ${bodyHtml}
    </div>
    <div class="fixed inset-x-0 bottom-0 z-40 border-t border-line bg-surface">
      <div class="mx-auto flex max-w-[58rem] items-center justify-end gap-2 px-6 py-3">
        <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
        <button data-action data-save class="hk-btn hk-btn-primary">${esc(t("action.save"))}</button>
      </div>
    </div>`;

  const back = $("[data-back]", $("#topbar"));
  if (back) back.addEventListener("click", backToHarness);

  const overlay = container;
  const close = () => backToHarness();

  const field = (name) => $(`[data-field="${name}"]`, overlay);
  const modelsHost = $("[data-models]", overlay);

  // Request ke endpoint: kalau kolom key dibiarkan kosong, backend memakai key
  // tersimpan (identitas harness + provider yang dikirim, bukan key-nya).
  const harnessId = state.activeId;
  const providerId = provider?.id ?? null;
  const typedKey = () => field("key").value.trim();
  const canReachEndpoint = () => typedKey().length > 0 || storedKeyAvailable;

  // Provider terdaftar tidak merender field endpoint, jadi nilainya diambil dari
  // provider itu sendiri (sudah sama dengan katalog karena `managed` dicek).
  const endpointUrl = () => (managed ? provider.baseUrl ?? "" : field("baseUrl").value.trim());
  const endpointApi = () => (managed ? registeredApi(preset, provider) : field("api").value);
  const providerName = () => (managed ? provider.name ?? "" : field("name").value);
  const providerKeyId = () => (managed ? provider.id : field("providerId").value);

  // Tombol Sync: state loading dengan jam pasir berputar selama request jalan.
  const syncButton = $("[data-sync-models]", overlay);
  const setSyncBusy = (busy) => {
    syncButton.disabled = busy;
    syncButton.setAttribute("aria-busy", busy ? "true" : "false");
    syncButton.classList.toggle("opacity-60", busy);
    syncButton.innerHTML = busy
      ? `${icon("hourglass", "h-3.5 w-3.5 hk-spin")}${esc(t("advanced.syncing"))}`
      : `${icon("refresh", "h-3.5 w-3.5")}${esc(t("advanced.syncModels"))}`;
  };

  const readModels = () =>
    $$("[data-row]", modelsHost).map((row) => ({
      id: $('[data-field="id"]', row).value.trim(),
      name: $('[data-field="name"]', row).value.trim() || null,
      contextWindow: Number($('[data-field="contextWindow"]', row).value) || null,
      maxTokens: Number($('[data-field="maxTokens"]', row).value) || null,
      reasoning: $('[data-field="reasoning"]', row)?.checked === true,
      vision: $('[data-field="vision"]', row)?.checked === true,
    }));

  const rerender = (list) => {
    modelsHost.innerHTML = list.map((model, index) => modelRow(index, model)).join("");
    wire();
  };

  const wire = () => {
    const blank = () => ({ id: "", name: "", contextWindow: "", maxTokens: "", reasoning: false, vision: false });

    $$("[data-remove]", modelsHost).forEach((button) => {
      button.addEventListener("click", () => {
        const list = readModels();
        list.splice(Number(button.dataset.remove), 1);
        rerender(list.length ? list : [blank()]);
      });
    });

    // Test satu model: memakai endpoint + key yang sedang diisi di halaman ini.
    $$("[data-test-model]", modelsHost).forEach((button) => {
      button.addEventListener("click", async () => {
        const model = readModels()[Number(button.dataset.testModel)];
        const key = typedKey();
        if (!canReachEndpoint()) {
          return toast(t("advanced.needKeyForTest"), { variant: "info", detail: model?.id ?? "" });
        }
        button.disabled = true;
        try {
          const result = await api.test(endpointUrl(), endpointApi(), key, harnessId, providerId);
          if (result.ok) {
            toast(
              result.latencyMs != null
                ? t("add.connected", { ms: result.latencyMs })
                : t("add.connectedNoTime"),
              { detail: model?.id ?? "" },
            );
          } else {
            toast(t("add.unreachable", { name: model?.id ?? "" }), {
              variant: "err",
              detail: result.message ?? "",
            });
          }
        } catch (error) {
          toast(t("add.unreachable", { name: model?.id ?? "" }), { variant: "err", detail: String(error) });
        } finally {
          button.disabled = false;
        }
      });
    });

    // Jadikan model ini default harness (provider harus sudah tersimpan).
    $$("[data-default-model]", modelsHost).forEach((button) => {
      button.addEventListener("click", async () => {
        const model = readModels()[Number(button.dataset.defaultModel)];
        if (!model?.id) return toast(t("toast.needModelId"), { variant: "err" });
        if (!isEdit) return toast(t("advanced.saveFirst"), { variant: "info" });
        await commit({ kind: "set_default", providerId: providerKeyId(), modelId: model.id });
      });
    });
  };
  wire();

  // Tarik daftar model langsung dari endpoint provider.
  $("[data-sync-models]", overlay).addEventListener("click", async () => {
    const key = typedKey();
    if (!canReachEndpoint()) return toast(t("add.needKey"), { variant: "err" });
    setSyncBusy(true);
    try {
      const found = await api.models(endpointUrl(), endpointApi(), key, harnessId, providerId);
      if (!found.length) return toast(t("add.modelsFailed"), { variant: "info" });
      // Hasil sync juga dipakai kotak pencarian "Add" - tidak perlu request ulang.
      endpointModels = found;
      const list = readModels().filter((model) => model.id);
      const known = new Set(list.map((model) => model.id));
      for (const model of found) {
        if (known.has(model.id)) continue;
        list.push({
          id: model.id,
          name: prettyModel(model.id),
          contextWindow: model.contextWindow ?? "",
          maxTokens: "",
          reasoning: false,
          vision: false,
        });
      }
      rerender(list);
      toast(found.length === 1 ? t("add.modelsFoundOne") : t("add.modelsFound", { count: found.length }));
    } catch (error) {
      toast(t("add.modelsFailed"), { variant: "err", detail: String(error) });
    } finally {
      setSyncBusy(false);
    }
  });

  // "Add": buka kotak pencarian di atas daftar, cari ke endpoint provider.
  const searchHost = $("[data-search-host]", overlay);
  const searchInput = $("[data-search-input]", overlay);
  const resultsHost = $("[data-search-results]", overlay);
  let endpointModels = null;

  const renderResults = () => {
    if (!endpointModels) return;

    // Endpoint tidak mengembalikan model apa pun: bedakan dari "filter tidak cocok".
    if (!endpointModels.length) {
      resultsHost.innerHTML = `<p class="px-2 py-3 text-center text-sm text-ink-muted">${esc(
        t("add.modelsFailed"),
      )}</p>`;
      return;
    }

    const term = searchInput.value.trim().toLowerCase();
    const added = new Set(readModels().map((model) => model.id).filter(Boolean));

    // Tampilkan SEMUA model dari endpoint; filter hanya mempersempit daftar itu.
    // Model yang sudah ada di form tetap tampil (ditandai), tidak disembunyikan.
    const rows = endpointModels.filter((model) => {
      if (!term) return true;
      return (
        model.id.toLowerCase().includes(term) ||
        prettyModel(model.id).toLowerCase().includes(term)
      );
    });

    resultsHost.innerHTML = rows.length
      ? rows
          .map((model) =>
            added.has(model.id)
              ? `<div class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 opacity-55">
              <span class="min-w-0 flex-1 truncate text-sm font-medium text-ink">${esc(prettyModel(model.id))}</span>
              <span class="shrink-0 truncate font-mono text-2xs text-ink-muted" style="max-width:40%">${esc(model.id)}</span>
              <span class="shrink-0 text-2xs font-semibold uppercase tracking-wide text-ink-muted">${esc(
                t("advanced.alreadyAdded"),
              )}</span>
            </div>`
              : `<button data-action data-pick="${esc(model.id)}"
              class="flex w-full items-center gap-2 rounded-lg bg-surface px-2.5 py-1.5 text-left hover:bg-brand-soft">
              <span class="min-w-0 flex-1 truncate text-sm font-medium text-ink">${esc(prettyModel(model.id))}</span>
              <span class="shrink-0 truncate font-mono text-2xs text-ink-muted" style="max-width:40%">${esc(model.id)}</span>
              <span class="shrink-0 text-brand-strong">${icon("plus", "h-3.5 w-3.5")}</span>
            </button>`,
          )
          .join("")
      : `<p class="px-2 py-3 text-center text-sm text-ink-muted">${esc(t("advanced.noModelFound"))}</p>`;

    $$("[data-pick]", resultsHost).forEach((button) => {
      button.addEventListener("click", () => {
        const id = button.dataset.pick;
        const found = endpointModels.find((model) => model.id === id);
        rerender([
          {
            id,
            name: prettyModel(id),
            contextWindow: found?.contextWindow ?? "",
            maxTokens: "",
            reasoning: false,
            vision: false,
          },
          ...readModels(),
        ]);
        renderResults();
        searchInput.focus();
      });
    });
  };

  $("[data-add-model]", overlay).addEventListener("click", async () => {
    const hidden = searchHost.classList.contains("hidden");
    searchHost.classList.toggle("hidden", !hidden);
    if (!hidden) return;

    searchInput.focus();
    if (endpointModels) return renderResults();

    const key = typedKey();
    if (!canReachEndpoint()) {
      resultsHost.innerHTML = `<p class="px-2 py-3 text-center text-sm text-ink-muted">${esc(
        t("advanced.needKeyForTest"),
      )}</p>`;
      return;
    }

    resultsHost.innerHTML = `<p class="px-2 py-3 text-center text-sm text-ink-muted">${esc(t("add.testing"))}</p>`;
    try {
      endpointModels = await api.models(endpointUrl(), endpointApi(), key, harnessId, providerId);
      renderResults();
    } catch (error) {
      endpointModels = [];
      toast(t("add.modelsFailed"), { variant: "err", detail: String(error) });
      renderResults();
    }
  });

  searchInput.addEventListener("input", renderResults);

  $("[data-toggle-key]", overlay).addEventListener("click", (event) => {
    const input = field("key");
    const shown = input.type === "text";
    input.type = shown ? "password" : "text";
    event.currentTarget.textContent = shown ? t("add.show") : t("add.hide");
  });

  // Penjelasan mode key: hanya hint dari opsi yang dipilih yang ditampilkan,
  // dan nama variabel environment ikut berubah saat ID provider diketik.
  const modeHints = $$("[data-key-mode-hint]", overlay);
  const envVarLabel = $("[data-env-var]", overlay);

  const refreshKeyMode = () => {
    const mode = $('input[name="keyMode"]:checked', overlay)?.value ?? "inline";
    // Atur display langsung: `block` vs `hidden` sama-sama utility Tailwind,
    // urutannya tidak bisa diandalkan untuk saling menimpa.
    modeHints.forEach((hint) => {
      hint.style.display = hint.dataset.keyModeHint === mode ? "block" : "none";
    });
  };
  const refreshEnvVar = () => {
    if (!envVarLabel) return;
    const typedId = field("providerId") ? field("providerId").value : "";
    envVarLabel.textContent = envVarFor({ provider, typedId, managed });
  };

  $$('input[name="keyMode"]', overlay).forEach((radio) => {
    radio.addEventListener("change", refreshKeyMode);
  });
  field("providerId")?.addEventListener("input", refreshEnvVar);
  refreshKeyMode();
  refreshEnvVar();

  $("[data-cancel]", overlay).addEventListener("click", close);

  $("[data-save]", overlay).addEventListener("click", async () => {
    // Semua nilai DOM harus dibaca SEBELUM close(): close() memanggil render()
    // yang mengganti isi #content, sehingga field() akan mengembalikan null.
    const id = providerKeyId();
    const baseUrl = endpointUrl();
    const name = providerName();
    const api = endpointApi();
    const key = field("key").value.trim();
    const keyMode = $('input[name="keyMode"]:checked', overlay)?.value ?? "inline";
    const built = readModels().filter((model) => model.id);

    if (!managed) {
      if (!/^[a-z0-9][a-z0-9._-]*$/.test(id)) return toast(t("toast.invalidId"), { variant: "err" });
      if (!name) return toast(t("toast.needName"), { variant: "err" });
      if (!/^https?:\/\//.test(baseUrl)) return toast(t("toast.needUrl"), { variant: "err" });
    }
    if (built.length === 0) return toast(t("toast.needModelId"), { variant: "err" });

    close();
    await commit({
      kind: "upsert",
      id,
      name,
      baseUrl,
      api,
      key: key || null,
      keyMode,
      models: built,
    });
  });
}

// ============================================================================
// DEFAULT MODEL
// ============================================================================

/**
 * Ubah model default harness.
 *
 * Kita sudah berada di halaman detail satu harness, jadi provider tidak perlu
 * dipilih lagi: semua model dari provider aktif diratakan ke satu daftar, dan
 * satu kotak pencarian menyaring daftar itu (pola yang sama dengan "Tambah
 * model"). Satu klik langsung menyimpan.
 *
 * Harness yang tidak menyimpan daftar model (Hermes) memakai jalur manual:
 * pilih provider (kalau lebih dari satu) lalu tulis id model.
 */
function openDefaultForm() {
  const snapshot = state.snapshot;
  const providers = snapshot.providers.filter((provider) => provider.enabled !== false);
  if (providers.length === 0) {
    toast(t("toast.noActiveProvider"), { variant: "info" });
    return;
  }

  const currentProvider = snapshot.defaultProvider ?? "";
  const currentModel = (snapshot.defaultModel ?? "").split("/").slice(1).join("/");
  const harnessName = state.harnesses.find((item) => item.id === state.activeId)?.name ?? "";

  const entries = providers.flatMap((provider) =>
    (provider.models ?? []).map((model) => ({
      providerId: provider.id,
      providerName: provider.name || provider.id,
      modelId: model.id,
      label: model.name || prettyModel(model.id),
    })),
  );
  const manual = entries.length === 0;
  const isCurrent = (entry) => entry.modelId === currentModel && entry.providerId === currentProvider;

  const { overlay, close } = openModal({
    title: t("default.title"),
    subtitle: t("default.subtitle", { name: harnessName }),
    size: "md",
    body: manual
      ? `<div class="space-y-4">
          ${
            providers.length > 1
              ? `<div>
                   <label class="hk-label">${esc(t("default.provider"))}</label>
                   <select data-provider class="hk-input">
                     ${providers
                       .map(
                         (provider) =>
                           `<option value="${esc(provider.id)}" ${
                             provider.id === (currentProvider || providers[0].id) ? "selected" : ""
                           }>${esc(provider.name || provider.id)}</option>`,
                       )
                       .join("")}
                   </select>
                 </div>`
              : ""
          }
          <div>
            <label class="hk-label">${esc(t("default.model"))}</label>
            <input data-model-input class="hk-input" value="${esc(currentModel)}"
              placeholder="${esc(t("default.typeModel"))}" style="font-family:var(--font-mono);font-size:13px" />
            <p class="hk-help">${esc(t("default.noModels"))}</p>
          </div>
        </div>`
      : `<div class="space-y-2">
          <input data-search type="search" class="hk-input" autocomplete="off" spellcheck="false"
            placeholder="${esc(t("advanced.searchModelPlaceholder"))}" />
          <div data-results class="max-h-[18.75rem] space-y-1 overflow-y-auto"></div>
          <p data-count class="px-0.5 text-xs font-medium text-ink-muted"></p>
        </div>`,
    footer: manual
      ? `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
         <button data-action data-save class="hk-btn hk-btn-primary">${esc(t("action.save"))}</button>`
      : `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  // ---- jalur manual (harness tanpa daftar model)
  if (manual) {
    const providerSelect = $("[data-provider]", overlay);
    const modelInput = $("[data-model-input]", overlay);
    $("[data-save]", overlay).addEventListener("click", async () => {
      const modelId = modelInput.value.trim();
      if (!modelId) return toast(t("toast.pickModel"), { variant: "err" });
      const providerId = providerSelect?.value || providers[0].id;
      close();
      await commit({ kind: "set_default", providerId, modelId });
    });
    return;
  }

  // ---- jalur pencarian
  const searchInput = $("[data-search]", overlay);
  const resultsHost = $("[data-results]", overlay);
  const countHost = $("[data-count]", overlay);

  const render = () => {
    const term = searchInput.value.trim().toLowerCase();
    const rows = entries.filter((entry) => {
      if (!term) return true;
      return (
        entry.modelId.toLowerCase().includes(term) ||
        entry.label.toLowerCase().includes(term) ||
        entry.providerName.toLowerCase().includes(term)
      );
    });

    resultsHost.innerHTML = rows.length
      ? rows
          .map((entry) => {
            const index = entries.indexOf(entry);
            const current = isCurrent(entry);
            return `<button data-action data-pick="${index}" aria-current="${current}"
              class="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left hover:bg-brand-soft ${
                current ? "bg-brand-soft/50" : "bg-surface"
              }">
              <span class="min-w-0 flex-1">
                <span class="block truncate text-base font-medium leading-tight text-ink">${esc(entry.label)}</span>
                <span class="block truncate font-mono text-xs leading-tight text-ink-muted">${esc(entry.modelId)}</span>
              </span>
              <span class="shrink-0">${chip("neutral", entry.providerName)}</span>
              ${
                current
                  ? `<span class="shrink-0 text-ok-text">${icon("check", "h-3.5 w-3.5")}</span>`
                  : ""
              }
            </button>`;
          })
          .join("")
      : `<p class="px-2 py-4 text-center text-sm text-ink-muted">${esc(t("advanced.noModelFound"))}</p>`;

    countHost.textContent = t("default.modelCount", { shown: rows.length, total: entries.length });

    $$("[data-pick]", resultsHost).forEach((button) => {
      button.addEventListener("click", async () => {
        const entry = entries[Number(button.dataset.pick)];
        if (!entry) return;
        if (isCurrent(entry)) {
          close();
          return;
        }
        close();
        await commit({ kind: "set_default", providerId: entry.providerId, modelId: entry.modelId });
      });
    });
  };

  searchInput.addEventListener("input", render);
  render();
  searchInput.focus();
}

// ============================================================================
// PANDUAN AWAL (halaman pertama, bukan modal)
// ----------------------------------------------------------------------------
// Alurnya sengaja terasa seperti proses: deteksi harness satu per satu, baca
// config masing-masing untuk menghitung provider + model, lalu ringkas hasilnya.
// Semua lokal dan cepat; tidak ada request jaringan.
// ============================================================================

const ONBOARD_STEP_DELAY = 220; // jeda kecil supaya baris terlihat "dikerjakan"

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Mulai (atau ulangi) panduan awal sebagai halaman penuh. */
function startOnboarding() {
  state.view = "onboarding";
  state.onboarding = { step: "welcome", harnesses: [], installed: 0, providers: 0, models: 0 };
  render();
}

/** Selesai: tandai sudah pernah lihat, lalu masuk ke halaman utama. */
function finishOnboarding() {
  savePrefs({ onboarded: true });
  state.onboarding = null;
  state.view = "home";
  render();
}

function renderOnboardingHeader() {
  $("#topbar").innerHTML = `
    <div class="mx-auto flex w-full max-w-[63rem] items-center gap-3 px-6 py-3.5">
      <div class="flex h-10 w-10 items-center justify-center rounded-xl border border-line bg-surface-muted text-ink shadow-xs">${LOGO_SVG}</div>
      <div class="min-w-0">
        <p class="text-lg font-bold leading-tight text-ink">Hakonfig</p>
        <p class="text-sm leading-tight text-ink-secondary">${esc(t("app.tagline"))}</p>
      </div>
    </div>`;
}

function renderOnboarding() {
  renderOnboardingHeader();
  const onboarding = state.onboarding;
  const container = $("#content");

  if (!onboarding || onboarding.step === "welcome") {
    container.innerHTML = `
      <div class="mx-auto flex min-h-full w-full max-w-[35rem] flex-col justify-center py-10">
        <span class="mb-5 flex h-14 w-14 items-center justify-center rounded-[18px] bg-brand text-on-brand shadow-sm">
          ${icon("wand", "h-6 w-6")}
        </span>
        <h1 class="text-4xl font-extrabold leading-tight tracking-tight text-ink">${esc(t("onboard.title"))}</h1>
        <p class="mt-3 max-w-[46ch] text-md leading-relaxed text-ink-secondary">${esc(t("onboard.subtitle"))}</p>
        <div class="mt-8">
          <button data-action data-start class="hk-btn hk-btn-primary">
            ${icon("search", "h-4 w-4")}${esc(t("onboard.start"))}
          </button>
        </div>
      </div>`;

    $("[data-start]", container).addEventListener("click", () => {
      state.onboarding = { ...(state.onboarding ?? {}), step: "scanning" };
      render();
    });
    return;
  }

  if (onboarding.step === "scanning") {
    container.innerHTML = `
      <div class="mx-auto flex min-h-full w-full max-w-[35rem] flex-col justify-center py-10">
        <div class="flex items-center gap-2.5">
          <span class="hk-spin text-brand">${icon("hourglass", "h-4 w-4")}</span>
          <h1 class="text-3xl font-bold leading-snug text-ink">${esc(t("onboard.scanning"))}</h1>
        </div>
        <p data-status class="mt-2 text-base leading-relaxed text-ink-secondary">${esc(
          t("onboard.scanningModels"),
        )}</p>
        <div class="mt-5 h-1.5 overflow-hidden rounded-full bg-surface-sunken">
          <div data-progress class="h-full w-0 rounded-full bg-brand transition-all duration-300"></div>
        </div>
        <div data-rows class="mt-5 rounded-[16px] border border-line bg-surface px-4 py-1"></div>
      </div>`;

    void runOnboardingScan();
    return;
  }

  // step: done
  const { harnesses, installed, providers, models } = onboarding;
  const found = harnesses.filter((harness) => harness.installed);
  const summary = `${t("onboard.installedCount")}: ${installed} · ${pluralProviders(
    providers,
  )} · ${pluralModels(models)}`;

  container.innerHTML = `
    <div class="mx-auto flex min-h-full w-full max-w-[35rem] flex-col justify-center py-10">
      <span class="mb-5 flex h-14 w-14 items-center justify-center rounded-[18px] ${
        installed ? "bg-ok-bg text-ok-text" : "bg-warn-bg text-warn-text"
      }">
        ${icon(installed ? "check" : "alert", "h-6 w-6")}
      </span>
      <h1 class="text-4xl font-extrabold leading-tight tracking-tight text-ink">${esc(
        t("onboard.done.title"),
      )}</h1>
      <p class="mt-3 text-md font-semibold leading-relaxed text-ink">${esc(summary)}</p>
      <p class="mt-2 max-w-[46ch] text-base leading-relaxed text-ink-secondary">${esc(
        onboarding.error ? onboarding.error : installed ? t("onboard.done.body") : t("onboard.done.none"),
      )}</p>
      ${
        found.length
          ? `<div class="mt-6 divide-y divide-line rounded-[16px] border border-line bg-surface px-4">
               ${found
                 .map(
                   (harness) => `<div class="flex items-center gap-2.5 py-3">
                     <span class="shrink-0 text-ok-text">${icon("check", "h-4 w-4")}</span>
                     <span class="min-w-0 flex-1 truncate text-base font-semibold text-ink">${esc(
                       harness.name,
                     )}</span>
                     <span class="shrink-0 text-sm font-medium text-ink-muted">${esc(
                       pluralProviders(harness.providerCount),
                     )}</span>
                   </div>`,
                 )
                 .join("")}
             </div>`
          : ""
      }
      <div class="mt-8">
        <button data-action data-open class="hk-btn hk-btn-primary">${esc(t("onboard.open"))}</button>
      </div>
    </div>`;

  $("[data-open]", container).addEventListener("click", finishOnboarding);
}

/** Deteksi harness + model, perbarui baris satu per satu supaya terasa proses. */
async function runOnboardingScan() {
  const container = $("#content");
  const rowsHost = $("[data-rows]", container);
  const progress = $("[data-progress]", container);
  const status = $("[data-status]", container);
  if (!rowsHost || !progress) return;

  const setProgress = (ratio) => {
    progress.style.width = `${Math.round(Math.min(1, Math.max(0, ratio)) * 100)}%`;
  };

  setProgress(0.08);
  let harnesses = [];
  try {
    harnesses = await api.harnesses();
  } catch (error) {
    if (status) status.textContent = String(error);
    setProgress(1);
    await wait(700);
    state.onboarding = { step: "done", harnesses: [], installed: 0, providers: 0, models: 0, error: String(error) };
    render();
    return;
  }
  setProgress(0.15);

  rowsHost.innerHTML = harnesses
    .map(
      (harness) => `
      <div data-row="${esc(harness.id)}" class="flex items-center gap-2.5 py-3">
        <span class="shrink-0 text-ink-muted">${icon("search", "h-3.5 w-3.5")}</span>
        <span class="min-w-0 flex-1 truncate text-base font-medium text-ink">${esc(harness.name)}</span>
        <span data-note class="shrink-0 text-sm font-medium text-ink-muted">...</span>
      </div>`,
    )
    .join("");

  let installed = 0;
  let providers = 0;
  let models = 0;

  for (const [index, harness] of harnesses.entries()) {
    const row = $(`[data-row="${harness.id}"]`, rowsHost);
    const note = row ? $("[data-note]", row) : null;

    if (!harness.installed) {
      if (row) row.firstElementChild.outerHTML = `<span class="shrink-0 text-ink-muted">${icon("x", "h-3.5 w-3.5")}</span>`;
      if (note) note.textContent = t("onboard.notInstalled");
    } else {
      installed += 1;
      try {
        const snapshot = await api.read(harness.id);
        const providerCount = snapshot.providers.length;
        const modelCount = snapshot.providers.reduce((sum, item) => sum + (item.models?.length ?? 0), 0);
        providers += providerCount;
        models += modelCount;
        if (row) row.firstElementChild.outerHTML = `<span class="shrink-0 text-ok-text">${icon("check", "h-3.5 w-3.5")}</span>`;
        if (note) {
          note.textContent = `${pluralProviders(providerCount)} · ${pluralModels(modelCount)}`;
          note.className = "shrink-0 text-sm font-semibold text-ink-secondary";
        }
      } catch {
        if (row) row.firstElementChild.outerHTML = `<span class="shrink-0 text-warn-text">${icon("alert", "h-3.5 w-3.5")}</span>`;
        if (note) note.textContent = t("status.readError");
      }
    }

    setProgress(0.15 + (0.8 * (index + 1)) / harnesses.length);
    if (index < harnesses.length - 1) await wait(ONBOARD_STEP_DELAY);
  }

  setProgress(1);
  await wait(200);

  state.onboarding = { step: "done", harnesses, installed, providers, models };
  render();
}

// ============================================================================
// INSTALL / UNINSTALL
// ============================================================================

async function runPackage(harnessId, mode) {
  const entry = state.catalog.find((item) => item.id === harnessId);
  const isInstall = mode === "install";
  const command = isInstall ? entry?.installCommand : entry?.uninstallCommand;
  if (!entry) return;
  if (!command) {
    toast(t("install.manualHint"), { variant: "info", detail: entry.docsUrl ?? "" });
    return;
  }

  const titleKey = isInstall ? "install.title" : "uninstall.title";
  const bodyKey = isInstall ? "install.body" : "uninstall.body";

  if (!isInstall) {
    const ok = await confirmDialog({
      title: t(titleKey, { name: entry.name }),
      body: t(bodyKey, { name: entry.name }),
      bullets: [t("uninstall.bulletConfig"), t("uninstall.bulletReinstall")],
      confirmLabel: t("uninstall.run"),
      danger: true,
    });
    if (!ok) return;
  }

  const { overlay, close } = openModal({
    title: t(titleKey, { name: entry.name }),
    size: "md",
    body: `
      <p class="text-md leading-relaxed text-ink">${esc(t(bodyKey, { name: entry.name }))}</p>
      <pre class="selectable mt-3 overflow-x-auto rounded-lg border border-line bg-surface-muted px-3 py-2 font-mono text-sm text-ink-secondary">${esc(
        command,
      )}</pre>
      ${
        isInstall && entry.kind === "planned"
          ? `<p class="mt-3 flex gap-2 rounded-lg bg-warn-bg px-3 py-2 text-sm leading-relaxed text-warn-text">
               ${icon("info", "h-4 w-4 shrink-0")}<span>${esc(t("install.warnUnsupported"))}</span>
             </p>`
          : ""
      }
      ${
        isInstall && !entry.verified
          ? `<p class="mt-2 flex gap-2 rounded-lg bg-warn-bg px-3 py-2 text-sm leading-relaxed text-warn-text">
               ${icon("alert", "h-4 w-4 shrink-0")}<span>${esc(t("install.warnUnverified"))}</span>
             </p>`
          : ""
      }
      <div data-progress class="mt-4 hidden">
        <div class="hk-bar"></div>
        <p data-phase class="mt-2 text-base text-ink-secondary">${esc(
          isInstall ? t("install.running") : t("uninstall.running"),
        )}</p>
        <details class="mt-2">
          <summary class="cursor-pointer text-sm font-semibold text-ink-secondary">${esc(t("install.showLog"))}</summary>
          <pre data-log class="selectable mt-1.5 h-36 overflow-auto rounded-lg border border-line bg-surface-sunken px-3 py-2 font-mono text-xs leading-relaxed text-ink-secondary"></pre>
        </details>
      </div>`,
    footer: `
      <button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.cancel"))}</button>
      <button data-action data-run class="hk-btn ${isInstall ? "hk-btn-primary" : "hk-btn-danger"}">${esc(
        isInstall ? t("install.run") : t("uninstall.run"),
      )}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  $("[data-run]", overlay).addEventListener("click", async () => {
    const runButton = $("[data-run]", overlay);
    runButton.disabled = true;
    $("[data-cancel]", overlay).disabled = true;
    $("[data-progress]", overlay).classList.remove("hidden");
    const details = $("details", $("[data-progress]", overlay));
    details.open = true;

    const log = $("[data-log]", overlay);
    let unsubscribe = null;
    if (listen) {
      try {
        unsubscribe = await listen("package-progress", (event) => {
          if (event.payload?.harnessId !== harnessId) return;
          log.textContent += `${event.payload.line}\n`;
          log.scrollTop = log.scrollHeight;
        });
      } catch {
        // tanpa event: bar tetap berjalan
      }
    }

    try {
      const result = isInstall ? await api.install(harnessId) : await api.uninstall(harnessId);
      close();
      if (result.ok) {
        toast(t(isInstall ? "install.done" : "uninstall.done", { name: entry.name }));
        state.versions[harnessId] = undefined;
      } else {
        toast(t(isInstall ? "install.failed" : "uninstall.failed", { name: entry.name }), {
          variant: "err",
          detail: result.output?.slice(-600) ?? "",
        });
      }
    } catch (error) {
      close();
      toast(t("toast.runFailed"), { variant: "err", detail: String(error) });
    } finally {
      if (unsubscribe) unsubscribe();
      await refreshAll();
      await loadVersions();
    }
  });
}

// ============================================================================
// SETTINGS & HISTORY
// ============================================================================

/** Panduan memasang web tool sebagai aplikasi. Tidak ada perintah yang dijalankan. */
function openPwaHelp() {
  const { overlay, close } = openModal({
    title: t("web.pwaTitle"),
    subtitle: t("web.pwaSubtitle"),
    size: "sm",
    body: `
      <div class="space-y-3 text-base leading-relaxed text-ink-secondary">
        ${["chrome", "safari", "mobile"]
          .map(
            (key) => `<div class="rounded-[14px] border border-line bg-surface px-3.5 py-3">
              <p class="mb-1 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(
                t(`web.pwa.${key}.title`),
              )}</p>
              <p>${esc(t(`web.pwa.${key}.body`))}</p>
            </div>`,
          )
          .join("")}
      </div>`,
    footer: `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.close"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);
}

function openSettings() {
  const { overlay, close } = openModal({
    title: t("settings.title"),
    size: "sm",
    body: `
      <div class="space-y-5">
        <section>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("settings.language"))}</h3>
          <div class="flex gap-2">
            <button data-action data-lang="en" class="hk-btn hk-btn-secondary hk-btn-sm ${
              getLang() === "en" ? "hk-btn-active" : ""
            }" aria-pressed="${getLang() === "en"}">English</button>
            <button data-action data-lang="id" class="hk-btn hk-btn-secondary hk-btn-sm ${
              getLang() === "id" ? "hk-btn-active" : ""
            }" aria-pressed="${getLang() === "id"}">Bahasa Indonesia</button>
          </div>
        </section>

        <section>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("settings.appearance"))}</h3>
          <div class="flex gap-2">
            <button data-action data-theme="light" class="hk-btn hk-btn-secondary hk-btn-sm ${
              state.prefs.theme !== "dark" ? "hk-btn-active" : ""
            }" aria-pressed="${state.prefs.theme !== "dark"}">${icon("sun", "h-4 w-4")}${esc(t("settings.light"))}</button>
            <button data-action data-theme="dark" class="hk-btn hk-btn-secondary hk-btn-sm ${
              state.prefs.theme === "dark" ? "hk-btn-active" : ""
            }" aria-pressed="${state.prefs.theme === "dark"}">${icon("moon", "h-4 w-4")}${esc(t("settings.dark"))}</button>
          </div>
        </section>

        <section data-scale-root>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("settings.scale"))}</h3>
          <div class="flex flex-wrap gap-2">
            <button data-action data-scale="auto" class="hk-btn hk-btn-secondary hk-btn-sm ${
              state.prefs.uiScale === "auto" ? "hk-btn-active" : ""
            }" aria-pressed="${state.prefs.uiScale === "auto"}">${esc(t("settings.scaleAuto"))}</button>
            ${SCALE_STEPS.map(
              (step) => `<button data-action data-scale="${step}" class="hk-btn hk-btn-secondary hk-btn-sm ${
                state.prefs.uiScale === step ? "hk-btn-active" : ""
              }" aria-pressed="${state.prefs.uiScale === step}">${Math.round(step * 100)}%</button>`,
            ).join("")}
          </div>
          <p data-scale-status class="mt-2 text-sm text-ink-muted">${esc(scaleStatus())}</p>
          <p data-scale-diag class="mt-1 font-mono text-xs leading-relaxed text-ink-muted">${esc(scaleDiagnostics())}</p>
        </section>

        <section>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("settings.confirmations"))}</h3>
          <label class="flex cursor-pointer items-center justify-between gap-3 rounded-lg bg-surface-muted px-3 py-2.5">
            <span class="text-base text-ink">${esc(t("settings.confirmDelete"))}</span>
            <button data-action data-confirm-toggle role="switch" aria-checked="${
              state.prefs.confirmDelete !== false
            }" class="hk-switch"></button>
          </label>
        </section>

        <section>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("onboard.section"))}</h3>
          <button data-action data-onboard class="hk-btn hk-btn-secondary hk-btn-sm">
            ${icon("wand", "h-4 w-4")}${esc(t("onboard.rerun"))}
          </button>
        </section>

        <section>
          <h3 class="mb-2 text-sm font-bold uppercase tracking-wide text-ink-muted">${esc(t("settings.about"))}</h3>
          <div class="space-y-1.5 text-base">
            <p class="text-ink-secondary">${esc(t("settings.version", { version: APP_VERSION }))}</p>
            <button data-action data-external="${esc(REPO_URL)}" class="flex items-center gap-1.5 font-semibold text-brand-text hover:underline">
              ${esc(t("settings.repo"))}${icon("arrowUpRight", "h-3.5 w-3.5")}
            </button>
            <button data-action data-external="${esc(GUIDE_URL)}" class="flex items-center gap-1.5 font-semibold text-brand-text hover:underline">
              ${esc(t("settings.guide"))}${icon("arrowUpRight", "h-3.5 w-3.5")}
            </button>
          </div>
        </section>
      </div>`,
    footer: `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("settings.close"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);
  wireExternalButtons(overlay);

  $("[data-onboard]", overlay)?.addEventListener("click", () => {
    close();
    startOnboarding();
  });

  $$("[data-lang]", overlay).forEach((button) => {
    button.addEventListener("click", () => {
      const lang = button.dataset.lang;
      if (lang === getLang()) return;
      savePrefs({ lang });
      setLang(lang);
      close();
      render();
      toast(t("settings.langSet"));
      openSettings();
    });
  });

  $$("[data-theme]", overlay).forEach((button) => {
    button.addEventListener("click", () => {
      const theme = button.dataset.theme;
      savePrefs({ theme });
      applyTheme();
      close();
      toast(theme === "dark" ? t("settings.themeDark") : t("settings.themeLight"));
      openSettings();
    });
  });

  $$("[data-scale]", overlay).forEach((button) => {
    button.addEventListener("click", async () => {
      const raw = button.dataset.scale;
      const uiScale = raw === "auto" ? "auto" : Number(raw);
      savePrefs({ uiScale });
      await applyUiScale();
      close();
      toast(
        uiScale === "auto"
          ? t("settings.scaleAutoSet")
          : t("settings.scaleSet", { percent: Math.round(uiScale * 100) }),
      );
      openSettings();
    });
  });

  $("[data-confirm-toggle]", overlay).addEventListener("click", (event) => {
    const next = state.prefs.confirmDelete === false;
    savePrefs({ confirmDelete: next });
    event.currentTarget.setAttribute("aria-checked", String(next));
    toast(next ? t("settings.confirmOn") : t("settings.confirmOff"), { variant: "info" });
  });
}

async function openHistory() {
  const harnessId = state.activeId;
  let backups = [];
  try {
    backups = await api.backups(harnessId);
  } catch (error) {
    toast(t("toast.generic"), { variant: "err", detail: String(error) });
    return;
  }

  const list = backups.length
    ? backups
        .map(
          (backup) => `<div class="flex items-start justify-between gap-3 border-b border-line px-1 py-3 last:border-0">
            <div class="min-w-0">
              <p class="text-base font-semibold text-ink">${esc(fmtDate(backup.createdAt))}</p>
              <p class="selectable truncate font-mono text-xs text-ink-muted" title="${esc(backup.dir)}">${esc(
                backup.files.map((file) => file.path.split("/").pop()).join(" · "),
              )}</p>
            </div>
            <button data-action data-restore="${esc(backup.id)}" class="hk-btn hk-btn-secondary hk-btn-sm shrink-0">${esc(
              t("history.restore"),
            )}</button>
          </div>`,
        )
        .join("")
    : `<p class="rounded-[14px] border border-dashed border-line-strong px-4 py-8 text-center text-base text-ink-muted">${esc(
        t("history.empty"),
      )}</p>`;

  const { overlay, close } = openModal({
    title: t("history.title"),
    subtitle: t("history.subtitle"),
    size: "md",
    body: `${list}
      <p class="mt-3 text-xs leading-relaxed text-ink-muted">${esc(t("history.keepNote"))}</p>`,
    footer: `<button data-action data-cancel class="hk-btn hk-btn-secondary">${esc(t("action.close"))}</button>`,
  });

  $("[data-cancel]", overlay).addEventListener("click", close);

  $$("[data-restore]", overlay).forEach((button) => {
    button.addEventListener("click", async () => {
      const backup = backups.find((item) => item.id === button.dataset.restore);
      const ok = await confirmDialog({
        title: t("history.restoreTitle"),
        body: t("history.restoreBody", { date: fmtDate(backup?.createdAt ?? "") }),
        confirmLabel: t("history.restore"),
      });
      if (!ok) return;
      try {
        await api.restore(harnessId, button.dataset.restore);
        close();
        toast(t("history.restored"));
        await refreshAll();
      } catch (error) {
        toast(t("toast.restoreFailed"), { variant: "err", detail: String(error) });
      }
    });
  });
}

// ============================================================================
// INIT
// ============================================================================

function renderFatal(message) {
  $("#content").innerHTML = `
    <div class="mx-auto max-w-lg">
      <div class="hk-card p-6">
        <p class="text-lg font-semibold text-ink">${esc(message)}</p>
        <p class="mt-2 text-base leading-relaxed text-ink-secondary">${esc(t("error.notDesktopHint"))}</p>
      </div>
    </div>`;
}

async function init() {
  setLang(state.prefs.lang ?? "en");
  applyTheme();

  // Ctrl +/-/0 mengubah skala UI. Ditangkap di fase capture supaya tidak
  // bertabrakan dengan zoom bawaan webview maupun handler fokus modal.
  document.addEventListener(
    "keydown",
    (event) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const key = event.key;
      if (key === "=" || key === "+") {
        event.preventDefault();
        stepUiScale(1);
      } else if (key === "-" || key === "_") {
        event.preventDefault();
        stepUiScale(-1);
      } else if (key === "0") {
        event.preventDefault();
        savePrefs({ uiScale: "auto" });
        applyUiScale().then(() => toast(t("settings.scaleAutoSet")));
      }
    },
    true,
  );

  if (!invoke) {
    renderFatal(t("error.notDesktop"));
    renderHomeHeader();
    return;
  }

  await applyUiScale();

  try {
    state.presets = await api.presets();
  } catch (error) {
    state.presets = [];
    toast(t("toast.presetFailed"), { variant: "err", detail: String(error) });
  }

  try {
    state.catalog = await api.catalog();
  } catch {
    state.catalog = [];
  }

  try {
    state.targets = await api.targets();
  } catch {
    state.targets = [];
  }

  await loadHarnesses();

  // Pertama kali dibuka: tampilkan panduan deteksi sebagai halaman pertama.
  if (state.prefs.onboarded !== true) {
    startOnboarding();
  }
}

init();

/** Tampilkan error apa pun di layar - jangan pernah gagal diam-diam. */
function showFatalBanner(message) {
  let banner = document.getElementById("fatal-banner");
  if (!banner) {
    banner = document.createElement("div");
    banner.id = "fatal-banner";
    banner.className =
      "fixed inset-x-0 top-0 z-[200] border-b border-line bg-danger-bg px-4 py-2 font-mono text-xs leading-relaxed text-danger-text";
    document.body.append(banner);
  }
  banner.textContent = `Error: ${message}`;
}

window.addEventListener("error", (event) => {
  showFatalBanner(event.message || String(event.error));
});
window.addEventListener("unhandledrejection", (event) => {
  showFatalBanner(String(event.reason?.message ?? event.reason));
  toast(t("toast.generic"), { variant: "err", detail: String(event.reason) });
});
