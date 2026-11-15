// 側のパネルの webview の中（S247）。dist/webview.js に束ねる。拡張ホストから届いた形のデータを DOM にする。
// 説明の HTML の無害化とアイコン・図の SVG の組み立ては、Svelte の UI と同じファイルを読む（写しを作らない）。
// HTML の文字列は作らない。文字は textContent、要素は createElement / createElementNS で作る。
import { sanitize } from "../../Svelte/src/lib/html-sanitize.ts";
import { diagramDrag } from "../../Svelte/src/lib/diagram-drag.ts";
import { deleteOf, markSel, pickSel } from "../../Svelte/src/lib/diagram-select.ts";
import { addWaypointOf, halfOf, handleNode, handlesOf } from "../../Svelte/src/lib/diagram-waypoint.ts";
import { diagramNodes, iconNodes, toDom, viewBoxOf } from "../../Svelte/src/lib/shape-svg.ts";

const vscode = acquireVsCodeApi();
const $ = (id) => document.getElementById(id);

/** 洗った木（Safe）を要素と文字にする */
const safeDom = (list) =>
  list.map((n) => {
    if (typeof n === "string") return document.createTextNode(n);
    const e = document.createElement(n.tag);
    if (n.href) (e.href = n.href), (e.rel = "noopener noreferrer");
    e.append(...safeDom(n.children));
    return e;
  });

const note = (text) => Object.assign(document.createElement("p"), { textContent: text, className: "none" });

const svg = (label, extent, nodes) =>
  toDom({ tag: "svg", attrs: { viewBox: viewBoxOf(extent), preserveAspectRatio: "xMidYMid meet", role: "img", "aria-label": label }, children: nodes });

let selected = null; // 選んだ部品か結線（S293。DiagramView と同じ diagram-select.ts）
let current = null; // 描いている図のデータ（S302: 選んだ結線の途中の点のつまみと、ダブルクリックで足す点の決め方に使う）

// S302: 選んだ結線の途中の点のつまみ（rect.wp）を図の最後に足す。つまみの決め方と形は DiagramView と同じ diagram-waypoint.ts
function handles() {
  const s = $("diagram").querySelector("svg");
  s?.querySelectorAll(".wp").forEach((e) => e.remove());
  const half = halfOf(current);
  for (const h of s ? handlesOf(current, selected) : []) s.append(toDom(handleNode(h, half)));
}

function show({ doc, icon, diagram }) {
  $("icon").replaceChildren(icon?.shapes.length ? svg(`${icon.class} のアイコン`, icon.extent, iconNodes(icon)) : note("アイコンが無い"));
  const empty = !diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length);
  $("diagram").replaceChildren(empty ? note("図が無い") : svg(`${diagram.class} の図`, diagram.extent, diagramNodes(diagram)));
  current = empty ? null : diagram;
  selected = markSel($("diagram").querySelector("svg"), selected); // 図は作り直したので選びの印を付け直す。消えた対象は外れる
  handles();
  $("cls").textContent = doc?.class ?? "";
  const info = doc ? sanitize(doc.info) : [];
  const revisions = doc ? sanitize(doc.revisions) : [];
  $("docs").replaceChildren(...(info.length ? safeDom(info) : [note("Documentation が無い")]));
  $("rev").replaceChildren(...safeDom(revisions));
  $("revisions").hidden = !revisions.length;
  vscode.postMessage({
    type: "rendered",
    class: doc?.class ?? icon?.class ?? diagram?.class ?? "",
    icon: $("icon").querySelectorAll("svg *").length > 0,
    parts: $("diagram").querySelectorAll("g[data-part]").length,
    connections: $("diagram").querySelectorAll("g[data-conn]").length,
  });
}

// 編集の結果の待ち（S264）。拡張が edited で返す。返らない（拡張が落ちた）ときは 10 秒で失敗にして見た目を戻す
let seq = 0;
const pending = new Map();
const ask = (msg) =>
  new Promise((resolve) => {
    const id = ++seq;
    const t = setTimeout(() => (pending.delete(id), resolve(false)), 10_000);
    pending.set(id, (ok) => (clearTimeout(t), resolve(ok)));
    vscode.postMessage({ ...msg, id });
  });

window.addEventListener("message", (e) => {
  if (e.data?.type === "show") show(e.data);
  else if (e.data?.type === "edited") pending.get(e.data.id)?.(e.data.ok === true), pending.delete(e.data.id);
});

// 部品のドラッグ移動と結線（S264）。Svelte の DiagramView と同じ diagram-drag.ts。図（svg）は show のたびに作り直すので、受けるのは外側の #diagram
const dnd = diagramDrag(() => $("diagram").querySelector("svg"), {
  onMove: (part, dx, dy) => ask({ type: "move", part, dx, dy }),
  onConnect: (from, to, points) => ask({ type: "connect", from, to, points }),
  onWaypoint: (from, to, index, dx, dy) => ask({ type: "moveWaypoint", from, to, index, dx, dy }), // S302
});
for (const [ev, f] of [["pointerdown", dnd.down], ["pointermove", dnd.move], ["pointerup", dnd.up], ["pointercancel", dnd.up]]) $("diagram").addEventListener(ev, f);

// 部品のクリック・Enter・Space で宣言の行へ（svg の中は一括で受ける）
const jump = (e) => {
  if (dnd.swallowed()) return; // ドラッグの終わりのクリックは宣言へ飛ばない
  pick(e);
  const line = e.target.closest?.("[data-part]")?.getAttribute("data-line");
  if (line != null) vscode.postMessage({ type: "reveal", line: Number(line) });
};
$("diagram").addEventListener("click", jump);
$("diagram").addEventListener("keydown", (e) => (e.key === "Enter" || e.key === " ") && jump(e));

// 選びと削除（S293）。部品か結線（`.hit`）のクリック・Tab で選び、何も無い所のクリックで外す。Delete / Backspace は図の中でだけ受ける（エディタの文字は消さない）
function pick(e) {
  const was = JSON.stringify(selected);
  selected = markSel($("diagram").querySelector("svg"), pickSel(e.target, e.type, selected));
  if (JSON.stringify(selected) !== was) handles(); // 変わらないとき（つまみのクリックなど）は作り直さない
}
// 結線の線（`.hit`）のダブルクリックで、いちばん近い線分に途中の点を足す（S302。右クリックでは足さない）
$("diagram").addEventListener("dblclick", (e) => {
  const s = $("diagram").querySelector("svg");
  if (!s?.getScreenCTM()) return;
  const q = new DOMPoint(e.clientX, e.clientY).matrixTransform(s.getScreenCTM().inverse());
  const w = addWaypointOf(current, e.target.closest?.("[data-hit]")?.getAttribute("data-hit"), q.x, -q.y);
  if (!w) return;
  e.preventDefault();
  void ask({ type: "addWaypoint", ...w });
});
$("diagram").addEventListener("focusin", pick);
$("diagram").addEventListener("keydown", async (e) => {
  const d = deleteOf(selected, e.key);
  if (!d) return;
  e.preventDefault();
  await ask("part" in d ? { type: "deletePart", part: d.part } : { type: "deleteConnect", from: d.from, to: d.to });
});
