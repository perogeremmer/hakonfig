// =============================================================================
// Rekonfig — frontend (vanilla JS, tanpa framework)
// Kontrak dengan backend: docs/UI-CONTRACT.md
// =============================================================================

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

const MAX_DIFF_LINES = 400;

// ------------------------------------------------------------------- helper

const $ = (selector, root = document) => root.querySelector(selector);

function esc(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
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
    ok: "border-emerald-500/40 bg-emerald-500/10 text-emerald-100",
    err: "border-red-500/40 bg-red-500/10 text-red-100",
    info: "border-zinc-600/60 bg-zinc-800/90 text-zinc-100",
  };
  const element = document.createElement("div");
  element.className = `pointer-events-auto selectable rounded-lg border px-3 py-2 text-xs shadow-lg backdrop-blur ${
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
    "fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/60 p-6 backdrop-blur-sm";

  overlay.innerHTML = `
    <div class="my-6 w-full ${
      wide ? "max-w-4xl" : "max-w-2xl"
    } rounded-xl border border-zinc-700/80 bg-zinc-900 shadow-2xl">
      <div class="flex items-start justify-between gap-4 border-b border-zinc-800 px-5 py-4">
        <div class="min-w-0">
          <h3 class="text-sm font-semibold">${esc(title)}</h3>
          ${subtitle ? `<p class="mt-0.5 text-xs text-zinc-500">${esc(subtitle)}</p>` : ""}
        </div>
        <button data-action class="rounded px-2 py-1 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200" data-close>✕</button>
      </div>
      <div class="max-h-[64vh] overflow-y-auto px-5 py-4">${body}</div>
      <div class="flex items-center justify-end gap-2 border-t border-zinc-800 px-5 py-3">${footer}</div>
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
      body: `<p class="selectable text-sm leading-relaxed text-zinc-300">${esc(message)}</p>`,
      footer: `
        <button data-action class="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800" data-cancel>Batal</button>
        <button data-action class="rounded-lg px-3 py-1.5 text-xs font-medium text-white ${
          danger ? "bg-red-600 hover:bg-red-500" : "bg-emerald-600 hover:bg-emerald-500"
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

// -------------------------------------------------------------------- diff

function lcsDiff(before, after) {
  const n = before.length;
  const m = after.length;
  const table = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));

  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      table[i][j] =
        before[i] === after[j]
          ? table[i + 1][j + 1] + 1
          : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  const ops = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (before[i] === after[j]) {
      ops.push({ type: "ctx", text: before[i] });
      i += 1;
      j += 1;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      ops.push({ type: "del", text: before[i] });
      i += 1;
    } else {
      ops.push({ type: "add", text: after[j] });
      j += 1;
    }
  }
  while (i < n) ops.push({ type: "del", text: before[i++] });
  while (j < m) ops.push({ type: "add", text: after[j++] });
  return ops;
}

/** Diff hemat memori: potong konteks yang sama di awal & akhir lebih dulu. */
function diffLines(beforeText, afterText) {
  const before = String(beforeText ?? "").split("\n");
  const after = String(afterText ?? "").split("\n");

  let start = 0;
  while (start < before.length && start < after.length && before[start] === after[start]) {
    start += 1;
  }
  let endBefore = before.length - 1;
  let endAfter = after.length - 1;
  while (endBefore >= start && endAfter >= start && before[endBefore] === after[endAfter]) {
    endBefore -= 1;
    endAfter -= 1;
  }

  const contextBefore = before.slice(Math.max(0, start - 3), start);
  const contextAfter = before.slice(endBefore + 1, Math.min(before.length, endBefore + 4));
  const midBefore = before.slice(start, endBefore + 1);
  const midAfter = after.slice(start, endAfter + 1);

  const trimmed = midBefore.length === 0 && midAfter.length === 0;
  let middle;
  if (trimmed) {
    middle = [];
  } else if (midBefore.length * midAfter.length > 250000) {
    middle = [
      ...midBefore.map((text) => ({ type: "del", text })),
      ...midAfter.map((text) => ({ type: "add", text })),
    ];
  } else {
    middle = lcsDiff(midBefore, midAfter);
  }

  const ops = [
    ...contextBefore.map((text) => ({ type: "ctx", text })),
    ...middle,
    ...contextAfter.map((text) => ({ type: "ctx", text })),
  ];
  return { ops, truncated: false };
}

function renderDiff(beforeText, afterText) {
  const { ops } = diffLines(beforeText, afterText);
  const shown = ops.slice(0, MAX_DIFF_LINES);
  const className = { add: "diff-add", del: "diff-del", ctx: "diff-ctx" };
  const body = shown
    .map(
      (op) =>
        `<span class="diff-line ${className[op.type]}">${esc(op.text) || " "}</span>`,
    )
    .join("");
  const note =
    ops.length > MAX_DIFF_LINES
      ? `<p class="mt-2 text-[11px] text-amber-400/90">… diff dipotong pada ${MAX_DIFF_LINES} baris dari total ${ops.length} baris.</p>`
      : "";
  const changes = ops.filter((op) => op.type !== "ctx").length;
  return `<pre class="selectable overflow-x-auto rounded-lg border border-zinc-800 bg-zinc-950 py-2 font-mono text-[11px] leading-relaxed">${body}</pre>${note}${
    changes === 0
      ? `<p class="mt-2 text-[11px] text-amber-400/90">Tidak ada baris yang berubah pada pratinjau — kemungkinan hanya nilai rahasia (disamarkan).</p>`
      : ""
  }`;
}

// ============================================================================
// RENDER
// ============================================================================

function renderSidebar() {
  const container = $("#harness-list");
  if (state.loadError) {
    container.innerHTML = `<p class="px-2 py-4 text-xs text-red-300">${esc(state.loadError)}</p>`;
    return;
  }

  container.innerHTML = state.harnesses
    .map((harness) => {
      const active = harness.id === state.activeId;
      const files = harness.files
        .map(
          (file) => `
          <div class="flex items-start gap-1.5">
            <span class="mt-1.5 h-1 w-1 shrink-0 rounded-full ${
              file.exists ? "bg-emerald-400" : "bg-zinc-600"
            }"></span>
            <span class="min-w-0 flex-1 font-mono text-[10px] leading-relaxed break-all ${
              file.exists ? "text-zinc-400" : "text-zinc-600 line-through"
            }">${esc(file.path)}</span>
            <button data-action data-reveal="${esc(file.path)}" title="Buka folder"
              class="shrink-0 rounded p-0.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                class="h-3 w-3"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>
            </button>
          </div>`,
        )
        .join("");

      const badge = harness.installed
        ? `<span class="rounded bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-medium text-emerald-300">terpasang</span>`
        : `<span class="rounded bg-zinc-700/40 px-1.5 py-0.5 text-[10px] font-medium text-zinc-400">belum ada</span>`;
      const experimental = harness.experimental
        ? `<span class="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">eksperimental</span>`
        : "";

      return `
        <div data-harness="${esc(harness.id)}" role="button" tabindex="0"
          class="w-full cursor-pointer rounded-lg border px-2.5 py-2 text-left transition ${
            active
              ? "border-emerald-500/50 bg-emerald-500/10"
              : "border-transparent hover:border-zinc-700 hover:bg-zinc-800/60"
          }">
          <div class="flex items-center gap-1.5">
            <span class="truncate text-xs font-medium ${
              active ? "text-emerald-200" : "text-zinc-200"
            }">${esc(harness.name)}</span>
            ${badge}
          </div>
          <div class="mt-1.5 space-y-1">${files}</div>
          ${experimental ? `<div class="mt-1.5">${experimental}</div>` : ""}
        </div>`;
    })
    .join("");

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

function renderTopbar() {
  const harness = state.harnesses.find((item) => item.id === state.activeId);
  $("#active-name").textContent = harness ? harness.name : "Pilih harness";
  $("#active-notes").textContent = harness
    ? harness.notes
    : "Deteksi konfigurasi harness di mesin ini.";

  const actions = $("#topbar-actions");
  if (!harness) {
    actions.innerHTML = "";
    return;
  }
  actions.innerHTML = `
    <button data-action data-refresh class="rounded-lg border border-zinc-700 px-2.5 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800">Muat ulang</button>
    <button data-action data-backups class="rounded-lg border border-zinc-700 px-2.5 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800">Backup</button>
    <button data-action data-add class="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500">+ Tambah Provider</button>`;

  $("[data-refresh]", actions).addEventListener("click", () => loadSnapshot(state.activeId));
  $("[data-backups]", actions).addEventListener("click", () => openBackups());
  $("[data-add]", actions).addEventListener("click", () => openProviderForm("add"));
}

function providerRow(provider) {
  const expanded = state.expanded.has(provider.id);
  const modelsDetail = expanded
    ? `<tr class="bg-zinc-900/60">
         <td colspan="7" class="px-4 py-3">
           ${
             provider.models.length
               ? `<div class="grid grid-cols-1 gap-1 sm:grid-cols-2">
                    ${provider.models
                      .map(
                        (model) => `<div class="flex items-baseline gap-2 rounded bg-zinc-800/50 px-2 py-1">
                          <span class="selectable font-mono text-[11px] text-zinc-200">${esc(model.id)}</span>
                          <span class="ml-auto shrink-0 font-mono text-[10px] text-zinc-500">${fmtTokens(
                            model.contextWindow,
                          )} · out ${fmtTokens(model.maxTokens)}${model.reasoning ? " · reasoning" : ""}</span>
                        </div>`,
                      )
                      .join("")}
                  </div>`
               : `<p class="text-xs text-zinc-500">Tidak ada model yang didefinisikan di config ini.</p>`
           }
         </td>
       </tr>`
    : "";

  return `
    <tr class="border-b border-zinc-800/70 hover:bg-zinc-800/30">
      <td class="px-4 py-2.5">
        <div class="selectable font-mono text-xs text-zinc-200">${esc(provider.id)}</div>
        <div class="text-[11px] text-zinc-500">${esc(provider.name)}</div>
      </td>
      <td class="max-w-[16rem] truncate px-4 py-2.5">
        <span class="selectable font-mono text-[11px] text-zinc-400" title="${esc(
          provider.baseUrl ?? "",
        )}">${esc(provider.baseUrl ?? "—")}</span>
      </td>
      <td class="px-4 py-2.5 font-mono text-[11px] text-zinc-400">${esc(provider.api ?? "—")}</td>
      <td class="px-4 py-2.5">
        ${
          provider.hasKey
            ? `<span class="rounded bg-emerald-500/10 px-1.5 py-0.5 font-mono text-[10px] text-emerald-300">${esc(
                provider.keyDisplay ?? "ada",
              )}</span>`
            : `<span class="rounded bg-amber-500/10 px-1.5 py-0.5 text-[10px] text-amber-300">belum ada</span>`
        }
      </td>
      <td class="px-4 py-2.5">
        <button data-action data-expand="${esc(provider.id)}"
          class="rounded px-1.5 py-0.5 text-[11px] text-zinc-400 hover:bg-zinc-700/60 hover:text-zinc-200">
          ${provider.models.length} model ${expanded ? "▾" : "▸"}
        </button>
      </td>
      <td class="px-4 py-2.5">
        <div class="flex items-center justify-end gap-1">
          <button data-action data-edit="${esc(provider.id)}"
            class="rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800">Ubah</button>
          <button data-action data-remove="${esc(provider.id)}"
            class="rounded border border-red-900/60 px-2 py-1 text-[11px] text-red-300 hover:bg-red-950/50">Hapus</button>
        </div>
      </td>
    </tr>
    ${modelsDetail}`;
}

function renderContent() {
  const container = $("#content");
  const snapshot = state.snapshot;

  if (state.loadError) {
    container.innerHTML = `<div class="mx-auto max-w-5xl rounded-lg border border-red-900/50 bg-red-950/30 p-4 text-sm text-red-200">${esc(
      state.loadError,
    )}</div>`;
    return;
  }
  if (!snapshot) {
    container.innerHTML = `<p class="text-sm text-zinc-500">Memuat konfigurasi…</p>`;
    return;
  }

  const warnings = snapshot.warnings.length
    ? `<div class="mb-4 space-y-1 rounded-lg border border-amber-600/40 bg-amber-500/10 p-3">
         ${snapshot.warnings
           .map((warning) => `<p class="text-xs leading-relaxed text-amber-200">• ${esc(warning)}</p>`)
           .join("")}
       </div>`
    : "";

  const defaults =
    snapshot.defaultProvider || snapshot.defaultModel
      ? `<div class="mb-4 flex flex-wrap items-center gap-x-6 gap-y-1 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-2 text-xs">
           <span class="text-zinc-500">Default aktif:</span>
           <span class="selectable font-mono text-zinc-300">${esc(snapshot.defaultProvider ?? "—")}</span>
           <span class="selectable font-mono text-zinc-300">${esc(snapshot.defaultModel ?? "—")}</span>
         </div>`
      : "";

  const table =
    snapshot.providers.length === 0
      ? `<div class="rounded-xl border border-dashed border-zinc-700 px-6 py-12 text-center">
           <p class="text-sm font-medium text-zinc-300">Belum ada provider di harness ini</p>
           <p class="mx-auto mt-1 max-w-md text-xs leading-relaxed text-zinc-500">
             Tambahkan provider gratis (Groq, Gemini, Cerebras, dan lainnya) dengan menempel API key —
             Rekonfig menulis konfigurasi ke file yang benar untukmu.
           </p>
           <button data-action data-add-empty class="mt-4 rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500">
             + Tambah Provider
           </button>
         </div>`
      : `<div class="overflow-hidden rounded-xl border border-zinc-800">
           <table class="w-full text-left text-sm">
             <thead class="bg-zinc-900/70 text-[11px] tracking-wide text-zinc-500 uppercase">
               <tr>
                 <th class="px-4 py-2 font-medium">Provider</th>
                 <th class="px-4 py-2 font-medium">Base URL</th>
                 <th class="px-4 py-2 font-medium">API</th>
                 <th class="px-4 py-2 font-medium">Key</th>
                 <th class="px-4 py-2 font-medium">Model</th>
                 <th class="px-4 py-2"></th>
               </tr>
             </thead>
             <tbody>${snapshot.providers.map(providerRow).join("")}</tbody>
           </table>
         </div>`;

  const fileList = snapshot.files
    .map(
      (file) => `
      <div class="flex items-center gap-2 border-b border-zinc-800/60 px-3 py-1.5 last:border-0">
        <span class="w-32 shrink-0 text-[11px] text-zinc-500">${esc(
          ROLE_LABEL[file.role] ?? file.role,
        )}</span>
        <span class="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-400">${esc(file.path)}</span>
        <span class="shrink-0 text-[10px] ${file.exists ? "text-emerald-400" : "text-zinc-600"}">${
          file.exists ? "ada" : "belum ada"
        }</span>
        <button data-action data-reveal="${esc(file.path)}" class="shrink-0 rounded p-1 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200" title="Buka folder">
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
      ${table}
      <div class="mt-5 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/40">
        <div class="border-b border-zinc-800 px-3 py-2 text-[11px] font-semibold tracking-wide text-zinc-500 uppercase">
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

function renderAll() {
  renderSidebar();
  renderTopbar();
  renderContent();
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
  renderSidebar();
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
  renderAll();
  await loadSnapshot(harnessId);
}

async function refreshEverything() {
  await loadHarnesses();
  if (state.activeId) await loadSnapshot(state.activeId);
  else renderTopbar();
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
        class="rounded border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-[11px] outline-none focus:border-emerald-500" />
      <input data-field="name" value="${esc(model.name ?? "")}" placeholder="Nama tampilan"
        class="rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-[11px] outline-none focus:border-emerald-500" />
      <input data-field="contextWindow" value="${esc(model.contextWindow ?? "")}" placeholder="ctx"
        inputmode="numeric" class="rounded border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-[11px] outline-none focus:border-emerald-500" />
      <input data-field="maxTokens" value="${esc(model.maxTokens ?? "")}" placeholder="out"
        inputmode="numeric" class="rounded border border-zinc-700 bg-zinc-950 px-2 py-1 font-mono text-[11px] outline-none focus:border-emerald-500" />
      <label class="flex items-center justify-center gap-1 text-[10px] text-zinc-400">
        <input data-field="reasoning" type="checkbox" ${model.reasoning ? "checked" : ""}
          class="h-3 w-3 rounded border-zinc-600 bg-zinc-950 accent-emerald-500" />R
      </label>
      <button data-action data-model-remove="${index}" title="Hapus baris"
        class="rounded p-1 text-zinc-500 hover:bg-red-950/60 hover:text-red-300">✕</button>
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
              <label class="mb-1 block text-[11px] font-medium text-zinc-400">Preset provider</label>
              <select data-preset class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 text-xs outline-none focus:border-emerald-500">
                <option value="">Custom / lainnya</option>
                ${presetOptions}
              </select>
              <p data-preset-note class="mt-1 text-[11px] leading-relaxed text-zinc-500"></p>
            </div>`
      }

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="mb-1 block text-[11px] font-medium text-zinc-400">ID provider</label>
          <input data-input="id" value="${esc(draft.id)}" placeholder="groq" ${isEdit ? "readonly" : ""}
            class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 font-mono text-xs outline-none focus:border-emerald-500 ${
              isEdit ? "opacity-60" : ""
            }" />
        </div>
        <div>
          <label class="mb-1 block text-[11px] font-medium text-zinc-400">Nama tampilan</label>
          <input data-input="name" value="${esc(draft.name)}" placeholder="Groq"
            class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 text-xs outline-none focus:border-emerald-500" />
        </div>
      </div>

      <div>
        <label class="mb-1 block text-[11px] font-medium text-zinc-400">Base URL</label>
        <input data-input="baseUrl" value="${esc(draft.baseUrl)}" placeholder="https://api.groq.com/openai/v1"
          class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 font-mono text-xs outline-none focus:border-emerald-500" />
        <p class="mt-1 text-[11px] text-zinc-500">Endpoint OpenAI-compatible biasanya berakhiran <span class="font-mono">/v1</span>.</p>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="mb-1 block text-[11px] font-medium text-zinc-400">Jenis API</label>
          <select data-input="api" class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 text-xs outline-none focus:border-emerald-500">
            ${API_OPTIONS.map(
              ([value, label]) =>
                `<option value="${value}" ${draft.api === value ? "selected" : ""}>${label}</option>`,
            ).join("")}
          </select>
        </div>
        <div>
          <label class="mb-1 block text-[11px] font-medium text-zinc-400">Mode penyimpanan key</label>
          <select data-input="keyMode" class="w-full rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 text-xs outline-none focus:border-emerald-500">
            <option value="inline">Simpan key (paling mudah)</option>
            <option value="env">Rujuk variabel environment</option>
          </select>
        </div>
      </div>

      <div>
        <label class="mb-1 block text-[11px] font-medium text-zinc-400">API key</label>
        <div class="flex gap-2">
          <input data-input="key" type="password" value="" autocomplete="off" spellcheck="false"
            placeholder="${esc(
              isEdit ? "biarkan kosong untuk mempertahankan key saat ini" : "tempel API key di sini",
            )}"
            class="min-w-0 flex-1 rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-1.5 font-mono text-xs outline-none focus:border-emerald-500" />
          <button data-action data-toggle-key class="rounded-lg border border-zinc-700 px-2.5 py-1.5 text-[11px] text-zinc-300 hover:bg-zinc-800">Lihat</button>
          <button data-action data-test class="rounded-lg border border-zinc-700 px-2.5 py-1.5 text-[11px] text-zinc-300 hover:bg-zinc-800">Test koneksi</button>
        </div>
        <div data-test-result class="mt-1.5 hidden rounded-lg border px-2.5 py-2 text-[11px] leading-relaxed"></div>
      </div>

      <div>
        <div class="mb-1.5 flex items-center justify-between">
          <label class="text-[11px] font-medium text-zinc-400">Model</label>
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-zinc-600">R = reasoning</span>
            <button data-action data-add-model class="rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:bg-zinc-800">+ Baris</button>
          </div>
        </div>
        <div class="grid grid-cols-[1fr_1fr_5rem_5rem_2.5rem_1.75rem] gap-1.5 px-0.5 pb-1 text-[10px] text-zinc-600">
          <span>ID model</span><span>Nama</span><span>Context</span><span>Max out</span><span class="text-center">R</span><span></span>
        </div>
        <div data-models class="space-y-1.5">${draft.models
          .map((model, index) => modelRowHtml(index, model))
          .join("")}</div>
      </div>
    </div>`;

  const footer = `
    <button data-action data-cancel class="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800">Batal</button>
    <button data-action data-save class="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500">
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
        note.innerHTML = `Dapatkan API key di <span class="selectable font-mono text-emerald-300">${esc(
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
      "mt-1.5 rounded-lg border border-zinc-700 bg-zinc-950 px-2.5 py-2 text-[11px] leading-relaxed text-zinc-400";
    result.textContent = "Menguji koneksi…";
    try {
      const output = await api.test(baseUrl, input("api").value, key);
      const tone = output.ok
        ? "border-emerald-600/50 bg-emerald-500/10 text-emerald-200"
        : "border-red-700/60 bg-red-950/40 text-red-200";
      result.className = `mt-1.5 rounded-lg border px-2.5 py-2 text-[11px] leading-relaxed ${tone}`;
      const latency = output.latencyMs != null ? ` · ${output.latencyMs} ms` : "";
      const sample = output.modelsSample?.length
        ? `<div class="mt-1 selectable font-mono text-[10px] opacity-80">${output.modelsSample
            .map(esc)
            .join(" · ")}</div>`
        : "";
      result.innerHTML = `${esc(output.message)}${latency}${sample}`;
    } catch (error) {
      result.className =
        "mt-1.5 rounded-lg border border-red-700/60 bg-red-950/40 px-2.5 py-2 text-[11px] text-red-200";
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
              <span class="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-400">${esc(
                ROLE_LABEL[file.role] ?? file.role,
              )}</span>
              <span class="selectable min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-300">${esc(
                file.path,
              )}</span>
              ${
                file.sensitive
                  ? `<span class="shrink-0 rounded bg-amber-500/10 px-1.5 py-0.5 text-[10px] text-amber-300">rahasia disamarkan</span>`
                  : ""
              }
            </div>
            ${renderDiff(file.before ?? "", file.after ?? "")}
          </div>`,
        )
        .join("")
    : `<p class="rounded-lg border border-zinc-800 bg-zinc-900/60 px-3 py-6 text-center text-xs text-zinc-500">
         Tidak ada file yang akan berubah.
       </p>`;

  const warnings = preview.warnings.length
    ? `<div class="mb-4 space-y-1 rounded-lg border border-amber-600/40 bg-amber-500/10 p-3">
         ${preview.warnings
           .map((warning) => `<p class="text-xs leading-relaxed text-amber-200">• ${esc(warning)}</p>`)
           .join("")}
       </div>`
    : "";

  const summary = `
    <div class="mb-4 flex flex-wrap gap-x-6 gap-y-1 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-2 text-[11px]">
      ${preview.willCreate.length ? `<span class="text-emerald-300">dibuat: ${preview.willCreate.length}</span>` : ""}
      ${preview.willModify.length ? `<span class="text-amber-300">diubah: ${preview.willModify.length}</span>` : ""}
      <span class="text-zinc-500">backup otomatis dibuat sebelum menulis</span>
    </div>`;

  const { overlay, close } = openModal({
    title: isRemove ? "Konfirmasi hapus provider" : "Pratinjau perubahan",
    subtitle: "Periksa diff di bawah. Belum ada yang ditulis ke disk.",
    body: summary + warnings + fileBlocks,
    footer: `
      <button data-action data-cancel class="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800">Batal</button>
      <button data-action data-write ${
        changed.length ? "" : "disabled"
      } class="rounded-lg px-3 py-1.5 text-xs font-medium text-white ${
        isRemove ? "bg-red-600 hover:bg-red-500" : "bg-emerald-600 hover:bg-emerald-500"
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
          <div class="mb-2 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-2">
            <div class="flex items-center justify-between gap-3">
              <div class="min-w-0">
                <p class="selectable text-xs text-zinc-200">${esc(fmtDate(backup.createdAt))}</p>
                <p class="selectable truncate font-mono text-[10px] text-zinc-500">${esc(backup.dir)}</p>
              </div>
              <button data-action data-restore="${esc(backup.id)}"
                class="shrink-0 rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800">Pulihkan</button>
            </div>
            <div class="mt-1.5 space-y-0.5">
              ${backup.files
                .map(
                  (file) =>
                    `<p class="selectable truncate font-mono text-[10px] text-zinc-600">${esc(file.path)}</p>`,
                )
                .join("")}
            </div>
          </div>`,
        )
        .join("")
    : `<p class="rounded-lg border border-dashed border-zinc-700 px-3 py-8 text-center text-xs text-zinc-500">Belum ada backup untuk harness ini.</p>`;

  const { overlay, close } = openModal({
    title: "Backup",
    subtitle: "Rekonfig menyimpan salinan setiap file sebelum menulis.",
    body: list,
    footer: `<button data-action data-cancel class="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800">Tutup</button>`,
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
    <div class="mx-auto max-w-lg rounded-xl border border-red-900/50 bg-red-950/30 p-5">
      <p class="text-sm font-medium text-red-100">${esc(message)}</p>
      <p class="mt-2 text-xs leading-relaxed text-red-200/80">
        Rekonfig perlu dijalankan sebagai aplikasi desktop (Tauri), bukan dibuka langsung di browser.
      </p>
    </div>`;
}

async function init() {
  if (!invoke) {
    renderFatal("Rekonfig harus dijalankan sebagai aplikasi desktop.");
    renderSidebar();
    return;
  }

  try {
    state.presets = await api.presets();
  } catch (error) {
    state.presets = [];
    toast("Preset provider tidak bisa dimuat", "err", String(error));
  }

  await loadHarnesses();

  if (state.harnesses.length === 0) {
    renderTopbar();
    $("#content").innerHTML = `
      <div class="mx-auto max-w-lg rounded-xl border border-zinc-800 bg-zinc-900/50 p-5">
        <p class="text-sm font-medium text-zinc-200">Tidak ada harness yang dikenali</p>
        <p class="mt-1 text-xs leading-relaxed text-zinc-500">
          Rekonfig belum menemukan konfigurasi Pi, oh-my-pi, OpenCode, atau Hermes di mesin ini.
          Pastikan salah satu harness sudah pernah dijalankan minimal sekali.
        </p>
      </div>`;
    return;
  }

  // pilih harness terpasang pertama, atau yang pertama di daftar
  const installed = state.harnesses.find((harness) => harness.installed);
  await selectHarness((installed ?? state.harnesses[0]).id);
}

init();

// tangkap error di luar promise agar tidak hilang diam-diam
window.addEventListener("unhandledrejection", (event) => {
  toast("Terjadi kesalahan tak terduga", "err", String(event.reason));
});
