// 側のパネルの webview の中（S247）。dist/webview.js に束ねる。拡張ホストから届いた形のデータを DOM にする。
// 説明の HTML の無害化とアイコン・図の SVG の組み立ては、Svelte の UI と同じファイルを読む（写しを作らない）。
// HTML の文字列は作らない。文字は textContent、要素は createElement / createElementNS で作る。
import { sanitize } from "../../Svelte/src/lib/html-sanitize.ts";
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

function show({ doc, icon, diagram }) {
  $("icon").replaceChildren(icon?.shapes.length ? svg(`${icon.class} のアイコン`, icon.extent, iconNodes(icon)) : note("アイコンが無い"));
  const empty = !diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length);
  $("diagram").replaceChildren(empty ? note("図が無い") : svg(`${diagram.class} の図`, diagram.extent, diagramNodes(diagram)));
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

window.addEventListener("message", (e) => e.data?.type === "show" && show(e.data));

// 部品のクリック・Enter・Space で宣言の行へ（svg の中は一括で受ける）
const jump = (e) => {
  const line = e.target.closest?.("[data-part]")?.getAttribute("data-line");
  if (line != null) vscode.postMessage({ type: "reveal", line: Number(line) });
};
$("diagram").addEventListener("click", jump);
$("diagram").addEventListener("keydown", (e) => (e.key === "Enter" || e.key === " ") && jump(e));
