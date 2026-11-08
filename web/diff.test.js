// =============================================================================
// Unit test untuk web/diff.js - jalankan dengan `bun run test:js`
// =============================================================================
import test from "node:test";
import assert from "node:assert/strict";

import { MAX_DIFF_LINES, diffLines, diffStats, esc, lcsDiff, renderDiff } from "./diff.js";

// ------------------------------------------------------------------- lcsDiff

test("lcsDiff: two identical lines yield all ctx", () => {
  const ops = lcsDiff(["a", "b"], ["a", "b"]);
  assert.deepEqual(ops, [
    { type: "ctx", text: "a" },
    { type: "ctx", text: "b" },
  ]);
});

test("lcsDiff: one line added", () => {
  const ops = lcsDiff(["a"], ["a", "b"]);
  assert.deepEqual(ops, [
    { type: "ctx", text: "a" },
    { type: "add", text: "b" },
  ]);
});

test("lcsDiff: one line removed", () => {
  const ops = lcsDiff(["a", "b"], ["a"]);
  assert.deepEqual(ops, [
    { type: "ctx", text: "a" },
    { type: "del", text: "b" },
  ]);
});

test("lcsDiff: replacement yields del then add", () => {
  const ops = lcsDiff(["lama"], ["baru"]);
  assert.deepEqual(ops, [
    { type: "del", text: "lama" },
    { type: "add", text: "baru" },
  ]);
});

test("lcsDiff: totally different content loses no lines", () => {
  const before = ["1", "2", "3"];
  const after = ["x", "y"];
  const ops = lcsDiff(before, after);
  assert.equal(ops.filter((op) => op.type === "del").length, 3);
  assert.equal(ops.filter((op) => op.type === "add").length, 2);
});

// ----------------------------------------------------------------- diffLines

test("diffLines: no change yields only context lines", () => {
  const text = "a\nb\nc\nd\ne";
  const ops = diffLines(text, text);
  assert.equal(diffStats(ops).changed, 0);
  assert.ok(ops.every((op) => op.type === "ctx"));
});

test("diffLines: trims long context at the top and bottom", () => {
  const before = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
  const after = [...before];
  after[5] = "6-diubah";

  const ops = diffLines(before.join("\n"), after.join("\n"));
  const del = ops.filter((op) => op.type === "del");
  const add = ops.filter((op) => op.type === "add");
  assert.equal(del.length, 1);
  assert.equal(add.length, 1);
  assert.equal(del[0].text, "6");
  assert.equal(add[0].text, "6-diubah");

  // hanya 3 baris konteks sebelum & sesudah, bukan seluruh file
  assert.ok(ops.length <= 1 + 1 + 3 + 3, `ops terlalu banyak: ${ops.length}`);
});

test("diffLines: new file (empty before) is all add", () => {
  const ops = diffLines("", '{\n  "a": 1\n}');
  assert.equal(ops.filter((op) => op.type === "del").length, 0);
  assert.ok(ops.filter((op) => op.type === "add").length >= 3);
});

test("diffLines: deleted file (empty after) is all del", () => {
  const ops = diffLines('{\n  "a": 1\n}', "");
  assert.equal(ops.filter((op) => op.type === "add").length, 0);
  assert.ok(ops.filter((op) => op.type === "del").length >= 3);
});

test("diffLines: null/undefined is treated as empty", () => {
  assert.doesNotThrow(() => diffLines(null, undefined));
  assert.equal(diffStats(diffLines(null, undefined)).changed, 0);
});

test("diffLines: giant block falls back to replacement mode without hanging", () => {
  const before = Array.from({ length: 1200 }, (_, i) => `baris-${i}`);
  const after = Array.from({ length: 1200 }, (_, i) => `baru-${i}`);
  const started = Date.now();
  const ops = diffLines(before.join("\n"), after.join("\n"));
  const elapsed = Date.now() - started;

  assert.ok(elapsed < 2000, `diff terlalu lambat: ${elapsed}ms`);
  // 1200*1200 > MAX_LCS_CELLS → seluruh blok ditandai del+add
  assert.equal(ops.filter((op) => op.type === "del").length, 1200);
  assert.equal(ops.filter((op) => op.type === "add").length, 1200);
});

// -------------------------------------------------------------------- render

test("renderDiff: assigns the right class for each line kind", () => {
  const html = renderDiff('{"a":0}', '{"a":1}');
  assert.ok(html.includes("diff-del"), "harus ada baris dihapus");
  assert.ok(html.includes("diff-add"), "harus ada baris ditambah");
  assert.ok(html.includes("diff-line"));
});

test("renderDiff: escapes HTML to prevent injection", () => {
  const html = renderDiff("lama", '<img src=x onerror="alert(1)">');
  assert.ok(!html.includes("<img"), "tag mentah tidak boleh lolos");
  assert.ok(html.includes("&lt;img"));
  assert.ok(html.includes("&quot;"));
});

test("renderDiff: notes when no line changed", () => {
  const html = renderDiff('{"a":1}', '{"a":1}');
  assert.ok(html.includes("Tidak ada baris yang berubah"));
});

test("renderDiff: trims output and reports the total count", () => {
  const before = Array.from({ length: 50 }, (_, i) => `a${i}`).join("\n");
  const after = Array.from({ length: 50 }, (_, i) => `b${i}`).join("\n");
  const html = renderDiff(before, after, 10);
  assert.ok(html.includes("diff dipotong pada 10 baris"));
  assert.ok(html.includes("total"));
});

test("renderDiff: empty lines still become line elements", () => {
  const html = renderDiff("a\n\nb", "a\n\nb\nc");
  assert.ok(html.includes("diff-line"));
  assert.ok(html.includes("diff-add"));
});

test("renderDiff: default limit uses MAX_DIFF_LINES", () => {
  assert.equal(typeof MAX_DIFF_LINES, "number");
  assert.ok(MAX_DIFF_LINES > 0);
});

// ----------------------------------------------------------------------- esc

test("esc: escapes the five dangerous characters", () => {
  assert.equal(esc(`<>&"'`), "&lt;&gt;&amp;&quot;&#39;");
});

test("esc: null/undefined becomes an empty string", () => {
  assert.equal(esc(null), "");
  assert.equal(esc(undefined), "");
});

test("esc: leaves safe text unchanged", () => {
  assert.equal(esc("groq"), "groq");
  assert.equal(esc(42), "42");
});
