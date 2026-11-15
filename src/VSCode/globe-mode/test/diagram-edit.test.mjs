// 図の編集（S264）。Svelte の UI と共有する diagram-drag.ts を偽の svg で、webview からのメッセージの検査（edit.js）を試す。
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { diagramDrag } from "../../../Svelte/src/lib/diagram-drag.ts";
import { deleteOf, markSel, pickSel } from "../../../Svelte/src/lib/diagram-select.ts";
import { addWaypointOf, halfOf, handleNode, handlesOf } from "../../../Svelte/src/lib/diagram-waypoint.ts";

const { parseEdit, parseParams, partParams, enumValue } = createRequire(import.meta.url)("../edit.js");

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

test("結線の途中の点のつまみ（.wp）のドラッグは 2 単位に吸着し、結線の端と番号つきで onWaypoint へ（S299）", async () => {
  const { svg, ev } = rig();
  const calls = [];
  const d = diagramDrag(() => svg, { onWaypoint: async (...a) => (calls.push(a), true) });
  const h = attrs({ "data-wp": "a.p b.n", "data-index": "2", setPointerCapture() {} });
  h.closest = (s) => (s === ".wp" ? h : null);
  d.down(ev(h, 10, 10));
  d.move(ev(h, 23, 3));
  assert.equal(h.transform, "translate(14 -8)");
  await d.up(ev(h, 23, 3));
  assert.deepEqual(calls, [["a.p", "b.n", 2, 14, 8]]);
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

test("削除のメッセージの検査（S293）。端は添字つきも通り、file・class・余計な字は通らない", () => {
  assert.deepEqual(parseEdit({ type: "deletePart", id: 1, part: "R1", file: "/x", class: "X" }), { id: 1, method: "globe/deletePart", args: { part: "R1" } });
  assert.deepEqual(parseEdit({ type: "deleteConnect", id: 2, from: "b[2].p", to: "c.n" }), { id: 2, method: "globe/deleteConnect", args: { from: "b[2].p", to: "c.n" } });
  assert.equal(parseEdit({ type: "deleteConnect", id: 2, from: "b[i+1].p", to: "c.n" })?.args.from, "b[i+1].p");
  for (const bad of [
    { type: "deletePart", id: 1 }, { type: "deletePart", id: 1, part: "a b" }, { type: "deletePart", id: 1, part: "a.b" }, { type: "deletePart", part: "a" },
    { type: "deleteConnect", id: 1, from: "a", to: "b.n" }, { type: "deleteConnect", id: 1, from: "a.p", to: "b.n.x" }, { type: "deleteConnect", id: 1, from: "a.p", to: "b[1.n" },
    { type: "deleteConnect", id: 1, from: "a.p", to: "b.n);x(" }, { type: "deleteConnect", id: 1, from: "a.p" },
  ]) assert.equal(parseEdit(bad), null, JSON.stringify(bad));
});

test("部品・結線の選びと削除の対象（DiagramView と共有）", () => {
  const el = (attrs, inside) => {
    const e = { classList: { s: new Set(), add(c) { this.s.add(c); }, remove(c) { this.s.delete(c); } }, getAttribute: (k) => attrs[k] ?? null };
    e.closest = (sel) => (sel.includes("data-part") && attrs["data-part"] != null ? e : sel.includes("data-hit") && attrs["data-hit"] != null ? e : inside?.closest(sel) ?? null);
    return e;
  };
  const part = el({ "data-part": "R1" }), hit = el({ "data-hit": "a.p b.n" }), conn = el({ "data-conn": "a.p b.n" }), bare = el({});
  assert.deepEqual(pickSel(part, "click", null), { part: "R1" });
  assert.deepEqual(pickSel(hit, "focusin", { part: "R1" }), { key: "a.p b.n" });
  assert.equal(pickSel(bare, "click", { part: "R1" }), null, "何も無い所のクリックで外す");
  assert.deepEqual(pickSel(bare, "focusin", { part: "R1" }), { part: "R1" }, "フォーカスでは外さない");
  const svg = { querySelectorAll: (q) => (q === ".sel" ? [part, conn].filter((e) => e.classList.s.has("sel")) : [part, conn]) };
  assert.deepEqual(markSel(svg, { part: "R1" }), { part: "R1" });
  assert.ok(part.classList.s.has("sel"));
  assert.deepEqual(markSel(svg, { key: "a.p b.n" }), { key: "a.p b.n" });
  assert.ok(!part.classList.s.has("sel") && conn.classList.s.has("sel"), "前の印は外れる");
  assert.equal(markSel(svg, { part: "Gone" }), null, "消えた対象は外れる");
  assert.deepEqual(deleteOf({ part: "R1" }, "Delete"), { part: "R1" });
  assert.deepEqual(deleteOf({ key: "a.p b.n" }, "Backspace"), { from: "a.p", to: "b.n" });
  assert.equal(deleteOf({ part: "R1" }, "x"), null);
  assert.equal(deleteOf(null, "Delete"), null);
});

test("webview は Delete / Backspace を選びのあるときだけ postMessage にし、パネルは編集のメッセージとして受ける", () => {
  const web = readFileSync(new URL("../webview.js", import.meta.url), "utf8");
  assert.ok(web.includes("deleteOf(selected, e.key)") && web.includes('type: "deletePart"') && web.includes('type: "deleteConnect"'));
  assert.ok(!web.includes("innerHTML"));
  assert.ok(readFileSync(new URL("../panel.js", import.meta.url), "utf8").includes('"deletePart", "deleteConnect"'));
});

test("途中の点のメッセージの検査（S302）。端・index・座標・ends を見て、file・class は通さない", () => {
  const mv = { type: "moveWaypoint", id: 1, from: "a.p", to: "b[2].n", index: 1, dx: 2, dy: -4 };
  assert.deepEqual(parseEdit({ ...mv, file: "/x", class: "X" }), { id: 1, method: "globe/moveWaypoint", args: { from: "a.p", to: "b[2].n", index: 1, dx: 2, dy: -4 } });
  const add = { type: "addWaypoint", id: 2, from: "a.p", to: "b.n", index: 1, x: 4, y: 6, ends: [[0, 0], [10, 10]] };
  assert.deepEqual(parseEdit(add), { id: 2, method: "globe/addWaypoint", args: { from: "a.p", to: "b.n", index: 1, x: 4, y: 6, ends: [[0, 0], [10, 10]] } });
  for (const bad of [
    { ...mv, index: 1.5 }, { ...mv, index: -1 }, { ...mv, index: "1" }, { ...mv, index: 1e9 }, { ...mv, dx: NaN }, { ...mv, dy: 1e9 }, { ...mv, from: "a" }, { ...mv, to: "b.n);x(" }, { ...mv, id: undefined },
    { ...add, x: Infinity }, { ...add, ends: [[0, 0]] }, { ...add, ends: [[0, 0], [1, 1], [2, 2]] }, { ...add, ends: [[0, 0], [1]] }, { ...add, ends: [[0, 0], [1, "1"]] }, { ...add, ends: "xx" }, { ...add, to: "b.n.x" }, { ...add, index: 0.5 },
  ]) assert.equal(parseEdit(bad), null, JSON.stringify(bad));
});

test("途中の点のつまみと足す点の決め方（DiagramView と共有）", () => {
  const line = (points) => ({ points });
  const diagram = {
    extent: [-100, -100, 100, 100],
    parts: [{ name: "a", connectors: [{ name: "p", pos: [-50, 0] }] }, { name: "b", connectors: [{ name: "n", pos: [50, 0] }] }],
    connections: [{ from: "a.p", to: "b.n", line: line([[-50, 0], [0, 0], [0, 20], [50, 0]]) }, { from: "a.p", to: "b.n2", line: line([[-50, 0], [50, 0]]) }],
  };
  assert.deepEqual(handlesOf(diagram, { key: "a.p b.n" }).map((h) => h.index), [1, 2], "最初と最後は出さない");
  assert.deepEqual(handlesOf(diagram, { part: "a" }), []);
  assert.equal(halfOf(diagram), 2.5);
  const n = handleNode(handlesOf(diagram, { key: "a.p b.n" })[1], 2.5);
  assert.deepEqual([n.attrs.class, n.attrs["data-wp"], n.attrs["data-index"], n.attrs.x, n.attrs.y], ["wp", "a.p b.n", 2, -2.5, -22.5]);
  // 点 (-3, 10) に近いのは線分 1（(0,0)-(0,20)）なので、その次の番号 2。座標は 2 単位に吸着
  assert.deepEqual(addWaypointOf(diagram, "a.p b.n", -3, 10), { from: "a.p", to: "b.n", index: 2, x: -2, y: 10, ends: [[-50, 0], [50, 0]] });
  assert.equal(addWaypointOf(diagram, "x y", 0, 0), null);
  assert.equal(addWaypointOf(diagram, null, 0, 0), null);
});

test("webview はつまみの移動と線のダブルクリックを postMessage にし、パネルは受ける（innerHTML は使わない）", () => {
  const web = readFileSync(new URL("../webview.js", import.meta.url), "utf8");
  assert.ok(web.includes('type: "moveWaypoint"') && web.includes('type: "addWaypoint"') && web.includes('"dblclick"'));
  assert.ok(!web.includes("innerHTML"));
  assert.ok(readFileSync(new URL("../panel.js", import.meta.url), "utf8").includes('"moveWaypoint", "addWaypoint"'));
});

test("パラメータの入力（S306）: webview の要求は部品の名前だけ。一覧からは部品の直下の書けるものだけを取り、列挙は型つきの名前にする", () => {
  assert.equal(parseParams({ type: "params", part: "r1", file: "/etc/passwd", class: "X" }), "r1");
  for (const bad of [null, {}, { type: "params" }, { type: "params", part: "a.b" }, { type: "params", part: "r 1" }, { type: "params", part: 1 }, { type: "move", part: "r1" }]) assert.equal(parseParams(bad), null, JSON.stringify(bad));
  const list = [
    { name: "r1.R", type: "Real", default: "1" },
    { name: "r1.mode", type: "Enumeration", literals: ["low", "high"], enum_type: "P.Mode" },
    { name: "r1.d.k", type: "Real" }, // 入れ子
    { name: "r1.v", type: "Real", dims: [3] }, // 配列
    { name: "r1.s", type: "String" },
    { name: "r10.R", type: "Real" }, // 別の部品
    { name: "R", type: "Real" }, // モデル自身
  ];
  assert.deepEqual(partParams(list, "r1").map((p) => p.key), ["R", "mode"]);
  assert.deepEqual(partParams(null, "r1"), []);
  assert.equal(enumValue(list[1], "high"), "P.Mode.high");
  assert.equal(enumValue(list[1], "a b"), "P.Mode.'a b'");
  assert.equal(enumValue({ literals: ["x"] }, "x"), null);
  const web = readFileSync(new URL("../webview.js", import.meta.url), "utf8");
  assert.ok(web.includes('addEventListener("dblclick"') && web.includes('type: "params", part'));
  assert.ok(readFileSync(new URL("../panel.js", import.meta.url), "utf8").includes("parseParams(m)"));
});
