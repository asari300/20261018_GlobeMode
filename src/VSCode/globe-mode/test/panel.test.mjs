// 側のパネル（S247）。Svelte の UI と共有する 2 つの純粋な関数（html-sanitize.ts・shape-svg.ts）を、拡張の側から読んで試す。
// node 24 は .ts をそのまま読む（型は消すだけ）。DOM は無いので、木と document は最小の偽物を渡す。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { sanitize } from "../../../Svelte/src/lib/html-sanitize.ts";
import { diagramNodes, iconNodes, shapeNode, toDom, viewBoxOf } from "../../../Svelte/src/lib/shape-svg.ts";

const txt = (v) => ({ nodeType: 3, nodeName: "#text", nodeValue: v, childNodes: [] });
const el = (name, attrs = {}, ...kids) => ({ nodeType: 1, nodeName: name.toUpperCase(), nodeValue: null, childNodes: kids, getAttribute: (k) => attrs[k] ?? null });

test("説明の HTML は白リストで洗われる（script・on* 属性・javascript: は出ない）", () => {
  const tree = el("body", {},
    el("p", { onclick: "x()" }, txt("本文 "), el("b", {}, txt("太字"))),
    el("script", {}, txt("alert(1)")),
    el("img", { src: "x", onerror: "x()", alt: "図" }),
    el("a", { href: "javascript:alert(1)" }, txt("悪")),
    el("a", { href: "https://example.com/" }, txt("良")),
    el("blink", {}, txt("残る")),
  );
  const out = sanitize("", () => tree);
  assert.deepEqual(out, [
    { tag: "p", href: undefined, children: ["本文 ", { tag: "b", href: undefined, children: ["太字"] }] },
    "図",
    "悪", // javascript: の a は外して中身だけ
    { tag: "a", href: "https://example.com/", children: ["良"] },
    "残る",
  ]);
  assert.ok(!JSON.stringify(out).includes("alert") && !JSON.stringify(out).includes("onclick"));
});

const shape = (o) => ({ kind: "rect", origin: [0, 0], rotation: 0, extent: [-10, -5, 10, 5], points: [], line: [0, 0, 255], fill: null, thickness: 0.25, dash: "solid", radius: 0, text: "", fontSize: 0, align: "center", arrows: ["none", "none"], arrowSize: 3, ...o });

test("アイコンと図は SVG の要素の木になり、DOM には setAttribute で入る", () => {
  const icon = { class: "A", extent: [-100, -100, 100, 100], shapes: [shape({}), shape({ kind: "line", points: [[0, 0], [10, 10]], arrows: ["none", "filled"] })] };
  assert.equal(viewBoxOf(icon.extent), "-100 -100 200 200");
  const [r, l] = iconNodes(icon);
  assert.deepEqual(r.children[0].attrs.y, -5, "y は反転");
  assert.equal(l.children.length, 2, "線と終端の矢印");

  const d = { class: "M", extent: [-100, -100, 100, 100], connections: [{ from: "a.p", to: "b.p", line: shape({ kind: "line", points: [[0, 0], [5, 5]] }) }], shapes: [],
    parts: [{ name: `<img onerror=x>`, class: "A", line: 3, transform: [1, 0, 0, 1, 0, 0], bbox: [-5, -5, 5, 5], shapes: icon.shapes }] };
  const nodes = diagramNodes(d);
  const part = nodes.find((n) => n.attrs["data-part"]);
  assert.equal(part.attrs["data-line"], 3);

  // 偽の document: 属性は setAttribute、文字は textContent だけ（名前に < があっても要素にならない）
  const doc = { createElementNS: (ns, tag) => ({ ns, tag, attrs: {}, kids: [], setAttribute(k, v) { this.attrs[k] = v; }, append(c) { this.kids.push(c); } }) };
  const dom = toDom(part, doc);
  assert.equal(dom.attrs["data-part"], "<img onerror=x>");
  assert.equal(dom.kids[0].tag, "title");
  assert.equal(dom.kids[0].textContent, "<img onerror=x>: A");
  assert.equal(shapeNode(shape({ kind: "text", text: "hi" })).children[0].text, "hi");
});

test("パネルは innerHTML を使わず、CSP は script を nonce だけに絞る", () => {
  for (const f of ["../panel.js", "../webview.js"]) assert.ok(!/innerHTML|insertAdjacentHTML|document\.write/.test(readFileSync(new URL(f, import.meta.url), "utf8")), f);
  const panel = readFileSync(new URL("../panel.js", import.meta.url), "utf8");
  assert.ok(panel.includes("default-src 'none'; style-src ${web.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'"));
});
