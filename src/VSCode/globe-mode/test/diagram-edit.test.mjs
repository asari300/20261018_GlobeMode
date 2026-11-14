// 図の編集（S264）。Svelte の UI と共有する diagram-drag.ts を偽の svg で、webview からのメッセージの検査（edit.js）を試す。
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { diagramDrag } from "../../../Svelte/src/lib/diagram-drag.ts";

const { parseEdit } = createRequire(import.meta.url)("../edit.js");

// 偽の要素・svg。画面の座標 = 図の座標（CTM は単位行列）
const attrs = (o = {}) => ({ ...o, setAttribute(k, v) { this[k] = v; }, getAttribute(k) { return this[k] ?? null; }, removeAttribute(k) { delete this[k]; } });
const part = (name) => { const g = attrs({ "data-part": name, setPointerCapture() {} }); g.closest = (s) => (s === "[data-part]" ? g : null); return g; };
const conn = (end, x, y, owner) => { const c = attrs({ "data-end": end, "data-x": x, "data-y": y }); c.closest = (s) => (s === ".conn" ? c : null); c.ownerDocument = owner; return c; };
function rig() {
  const doc = { under: null, createElementNS: () => { const l = attrs(); l.remove = () => (l.removed = true); return l; }, elementFromPoint() { return doc.under; } };
  const svg = { ownerDocument: doc, kids: [], getScreenCTM: () => ({ inverse: () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 }) }), setPointerCapture() {}, append(c) { this.kids.push(c); } };
  const calls = [];
  const d = diagramDrag(() => svg, { onMove: async (...a) => (calls.push(["move", ...a]), true), onConnect: async (...a) => (calls.push(["connect", ...a]), true) });
  const ev = (target, x, y) => ({ target, clientX: x, clientY: y, button: 0, pointerId: 1 });
  return { doc, svg, calls, d, ev };
}

test("部品のドラッグは 2 単位に吸着し、y は上向きの量で onMove へ（4 px 未満はクリック）", async () => {
  const { d, ev, calls } = rig();
  const g = part("R1");
  d.down(ev(g, 10, 10));
  d.move(ev(g, 12, 11)); // 3.6 px 未満
  await d.up(ev(g, 12, 11));
  assert.deepEqual(calls, []);
  assert.equal(d.swallowed(), false, "クリックは宣言へ飛べる");
  assert.equal(g.transform, undefined);

  d.down(ev(g, 10, 10));
  d.move(ev(g, 23, 3)); // 右へ 13、画面の上へ 7 → dx 14（13 を 2 の格子へ）、dy 8（7 → 8）
  assert.equal(g.transform, "translate(14 -8)");
  await d.up(ev(g, 23, 3));
  assert.deepEqual(calls, [["move", "R1", 14, 8]]);
  assert.equal(d.swallowed(), true, "離したあとのクリックは飛ばさない");
});

test("onMove が false なら見た目を戻す。コネクタから始めたら結線で、別のコネクタに離したときだけ onConnect", async () => {
  const r = rig();
  const g = part("R1");
  r.d = diagramDrag(() => r.svg, { onMove: async () => false });
  r.d.down(r.ev(g, 0, 0));
  r.d.move(r.ev(g, 20, 0));
  assert.equal(g.transform, "translate(20 0)");
  await r.d.up(r.ev(g, 20, 0));
  assert.equal(g.transform, undefined, "編集できなければ戻す");

  const { d, ev, doc, svg, calls } = rig();
  const a = conn("R1.p", 1, 2, doc), b = conn("R2.n", 30, 40, doc);
  d.down(ev(a, 0, 0));
  d.move(ev(a, 50, 50));
  assert.equal(svg.kids.length, 1, "仮の線が出る");
  doc.under = a; // 同じコネクタで離す
  await d.up(ev(a, 50, 50));
  assert.deepEqual(calls, []);
  assert.equal(svg.kids[0].removed, true);
  d.down(ev(a, 0, 0));
  d.move(ev(a, 50, 50));
  doc.under = b;
  await d.up(ev(b, 50, 50));
  assert.deepEqual(calls, [["connect", "R1.p", "R2.n", [[1, 2], [30, 40]]]]);
});

test("webview からのメッセージは型と値を検査する（file・class は受け取らない）", () => {
  assert.deepEqual(parseEdit({ type: "move", id: 1, part: "R1", dx: 2, dy: -4, file: "/etc/passwd", class: "X" }), { id: 1, method: "globe/movePart", args: { part: "R1", dx: 2, dy: -4 } });
  assert.deepEqual(parseEdit({ type: "connect", id: 2, from: "a.p", to: "b.n", points: [[0, 0], [1.5, 2]] }), { id: 2, method: "globe/addConnect", args: { from: "a.p", to: "b.n", points: [[0, 0], [1.5, 2]] } });
  for (const bad of [
    null, {}, { type: "move", part: "R1", dx: 1, dy: 1 }, // id が無い
    { type: "move", id: 1, dx: 1, dy: 1 }, { type: "move", id: 1, part: "a b", dx: 1, dy: 1 }, { type: "move", id: 1, part: "R1", dx: "1", dy: 1 },
    { type: "move", id: 1, part: "R1", dx: Infinity, dy: 1 }, { type: "move", id: 1, part: "R1", dx: 1e9, dy: 1 },
    { type: "connect", id: 1, from: "a", to: "b.n", points: [] }, { type: "connect", id: 1, from: "a.p", to: "b.n.x", points: [] },
    { type: "connect", id: 1, from: "a.p", to: "b.n);x(", points: [] }, { type: "connect", id: 1, from: "a.p", to: "b.n", points: [[0]] },
    { type: "connect", id: 1, from: "a.p", to: "b.n", points: "x" }, { type: "delete", id: 1 },
  ]) assert.equal(parseEdit(bad), null, JSON.stringify(bad));
});

test("webview は結果（edited）を待ち、拡張は webview の file・class を使わない", () => {
  const panel = readFileSync(new URL("../panel.js", import.meta.url), "utf8");
  assert.ok(panel.includes("parseEdit(m)") && panel.includes("t.doc.uri.toString(), class: t.class, ...args"));
});
