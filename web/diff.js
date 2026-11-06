// =============================================================================
// Utilitas diff untuk pratinjau perubahan.
// Dipisah dari main.js agar bisa diuji tanpa menjalankan Tauri/DOM.
// =============================================================================

export const MAX_DIFF_LINES = 400;

/** Batas sel matriks LCS; di atas ini diff dianggap penggantian blok penuh. */
const MAX_LCS_CELLS = 250_000;

/** Jumlah baris konteks di atas/bawah blok yang berubah. */
const CONTEXT_LINES = 3;

const CLASS_NAME = { add: "diff-add", del: "diff-del", ctx: "diff-ctx" };

export function esc(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

/** Diff berbasis longest-common-subsequence atas dua array baris. */
export function lcsDiff(before, after) {
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

/**
 * Diff hemat memori: konteks yang sama di awal & akhir dipotong lebih dulu,
 * sehingga file besar (katalog model) tidak membangun matriks LCS raksasa.
 */
/**
 * String kosong berarti "tidak ada baris", bukan "satu baris kosong".
 * Tanpa ini, diff file baru/terhapus memunculkan satu baris `+`/`-` palsu.
 */
function splitLines(text) {
  const value = String(text ?? "");
  return value === "" ? [] : value.split("\n");
}

export function diffLines(beforeText, afterText) {
  const before = splitLines(beforeText);
  const after = splitLines(afterText);

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

  const contextBefore = before.slice(Math.max(0, start - CONTEXT_LINES), start);
  const contextAfter = before.slice(
    endBefore + 1,
    Math.min(before.length, endBefore + 1 + CONTEXT_LINES),
  );
  const midBefore = before.slice(start, endBefore + 1);
  const midAfter = after.slice(start, endAfter + 1);

  let middle;
  if (midBefore.length === 0 && midAfter.length === 0) {
    middle = [];
  } else if (midBefore.length * midAfter.length > MAX_LCS_CELLS) {
    middle = [
      ...midBefore.map((text) => ({ type: "del", text })),
      ...midAfter.map((text) => ({ type: "add", text })),
    ];
  } else {
    middle = lcsDiff(midBefore, midAfter);
  }

  return [
    ...contextBefore.map((text) => ({ type: "ctx", text })),
    ...middle,
    ...contextAfter.map((text) => ({ type: "ctx", text })),
  ];
}

export function diffStats(ops) {
  let added = 0;
  let removed = 0;
  for (const op of ops) {
    if (op.type === "add") added += 1;
    else if (op.type === "del") removed += 1;
  }
  return { added, removed, changed: added + removed };
}

/** Render diff menjadi HTML siap tempel (sudah di-escape). */
export function renderDiff(beforeText, afterText, maxLines = MAX_DIFF_LINES) {
  const ops = diffLines(beforeText, afterText);
  const shown = ops.slice(0, maxLines);

  const body = shown
    .map((op) => `<span class="diff-line ${CLASS_NAME[op.type]}">${esc(op.text) || " "}</span>`)
    .join("");

  const truncated =
    ops.length > maxLines
      ? `<p class="mt-2 text-[11px] text-amber-600">… diff dipotong pada ${maxLines} baris dari total ${ops.length} baris.</p>`
      : "";

  const kosong =
    diffStats(ops).changed === 0
      ? `<p class="mt-2 text-[11px] text-amber-600">Tidak ada baris yang berubah pada pratinjau — kemungkinan hanya nilai rahasia (disamarkan).</p>`
      : "";

  return `<pre class="selectable overflow-x-auto rounded-xl border border-stone-200 bg-stone-50 py-2 font-mono text-[11px] leading-relaxed">${body}</pre>${truncated}${kosong}`;
}
