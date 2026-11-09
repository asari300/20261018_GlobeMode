// S244・S245・S247: アイコンと図を SVG の「要素の木」にする純粋な関数。Svelte の UI（SvgTree.svelte が要素として描く）と
// VS Code の拡張の webview（toDom で DOM に組む）が同じファイルを読む。文字列の SVG は作らない（属性の値は DOM の API が入れる）。
// 座標は Modelica のまま（y は上向き）なので y を反転して描く。

import type { Diagram, Icon, IconShape } from "./backend";

export interface SvgNode {
  tag: string;
  attrs: Record<string, string | number>;
  text?: string;
  children?: SvgNode[];
}

type Extent = [number, number, number, number];

export const boxOf = (e: Extent) => ({ x: Math.min(e[0], e[2]), y: -Math.max(e[1], e[3]), w: Math.abs(e[2] - e[0]), h: Math.abs(e[3] - e[1]) });

/** `<svg viewBox>` の値。extent が無ければ既定の枠 */
export const viewBoxOf = (e: Extent | undefined) => {
  const b = boxOf(e ?? [-100, -100, 100, 100]);
  return `${b.x} ${b.y} ${b.w || 1} ${b.h || 1}`;
};

const rgb = (c: [number, number, number] | null) => (c ? `rgb(${c[0]},${c[1]},${c[2]})` : "none");
const DASH = { solid: "", dash: "4 3", dot: "1 3", dashdot: "4 3 1 3", dashdotdot: "4 3 1 3 1 3" };
const pts = (p: [number, number][]) => p.map(([x, y]) => `${x},${-y}`).join(" ");
const VE = "non-scaling-stroke"; // 線の太さは画面の px（拡大に連れない）

/** 図形 1 つ。原点と回転の `<g>` の中に、種類ごとの要素が入る */
export function shapeNode(s: IconShape): SvgNode {
  const b = boxOf(s.extent);
  const width = Math.max(1, s.thickness * 4);
  const stroke = { stroke: rgb(s.line), "stroke-width": width, "stroke-dasharray": DASH[s.dash], "vector-effect": VE };
  const g = (children: SvgNode[]): SvgNode => ({ tag: "g", attrs: { transform: `translate(${s.origin[0]} ${-s.origin[1]}) rotate(${-s.rotation})` }, children });
  const el = (tag: string, attrs: SvgNode["attrs"], text?: string): SvgNode => ({ tag, attrs, text });
  switch (s.kind) {
    case "rect":
      return g([el("rect", { x: b.x, y: b.y, width: b.w, height: b.h, rx: s.radius, fill: rgb(s.fill), ...stroke })]);
    case "ellipse":
      return g([el("ellipse", { cx: b.x + b.w / 2, cy: b.y + b.h / 2, rx: b.w / 2, ry: b.h / 2, fill: rgb(s.fill), ...stroke })]);
    case "polygon":
      return g([el("polygon", { points: pts(s.points), fill: rgb(s.fill), "stroke-linejoin": "round", ...stroke })]);
    case "line": {
      const kids = [el("polyline", { points: pts(s.points), fill: "none", "stroke-linejoin": "round", ...stroke })];
      // 線の端の矢印（端の点 a、一つ手前の点 c）。open・half は V 字、filled は三角
      for (const end of [0, 1] as const) {
        const k = s.arrows[end];
        const p = s.points;
        if (k === "none" || p.length < 2) continue;
        const [a, c] = end === 0 ? [p[0], p[1]] : [p[p.length - 1], p[p.length - 2]];
        const d = Math.hypot(c[0] - a[0], c[1] - a[1]) || 1;
        const [ux, uy] = [(c[0] - a[0]) / d, (c[1] - a[1]) / d];
        const n = s.arrowSize * 2;
        const left: [number, number] = [a[0] + n * (ux * 0.87 - uy * 0.5), a[1] + n * (uy * 0.87 + ux * 0.5)];
        const right: [number, number] = [a[0] + n * (ux * 0.87 + uy * 0.5), a[1] + n * (uy * 0.87 - ux * 0.5)];
        kids.push(el("polyline", { points: pts([left, a, right]), fill: k === "filled" ? rgb(s.line) : "none", stroke: rgb(s.line), "stroke-width": width, "vector-effect": VE }));
      }
      return g(kids);
    }
    case "text": {
      // 文字の大きさ: 指定が無ければ枠の高さと、文字数から見た幅に収まる大きさ
      const size = s.fontSize > 0 ? s.fontSize : Math.max(1, Math.min(b.h, (b.w / Math.max(1, s.text.length)) * 1.7));
      const x = s.align === "left" ? b.x : s.align === "right" ? b.x + b.w : b.x + b.w / 2;
      const anchor = s.align === "left" ? "start" : s.align === "right" ? "end" : "middle";
      return g([el("text", { x, y: b.y + b.h / 2, "text-anchor": anchor, "dominant-baseline": "central", "font-size": size, fill: rgb(s.line) }, s.text)]);
    }
    case "bitmap":
      return g([
        el("rect", { x: b.x, y: b.y, width: b.w, height: b.h, fill: "none", stroke: rgb(s.line), "stroke-width": 1, "stroke-dasharray": "4 3", "vector-effect": VE }),
        el("text", { x: b.x + b.w / 2, y: b.y + b.h / 2, "text-anchor": "middle", "dominant-baseline": "central", "font-size": Math.max(1, Math.min(b.h / 4, 10)), fill: rgb(s.line) }, s.text),
      ]);
  }
}

/** アイコンの図形を描く順に並べたもの（`<svg viewBox={viewBoxOf(icon.extent)}>` の子） */
export const iconNodes = (icon: Icon): SvgNode[] => icon.shapes.map(shapeNode);

// Modelica の行列（y 上向き）を、y を反転した SVG の座標へ（F M F）
const matrix = (m: number[]) => `matrix(${m[0]} ${-m[1]} ${-m[2]} ${m[3]} ${m[4]} ${-m[5]})`;

/**
 * 図の中身（`<svg viewBox={viewBoxOf(diagram.extent)}>` の子）。図の図形、connect の線、部品の順。
 * 部品の `<g>` には `data-part`（名前）と `data-line`（宣言の行、0 始まり）が付き、クリックで宣言へ飛ぶのは呼び出し側（イベントの委譲）。
 */
export function diagramNodes(d: Diagram): SvgNode[] {
  const e = boxOf(d.extent);
  const size = Math.max(e.w, e.h) / 40 || 1; // 部品名の字の大きさ
  const parts = d.parts.map((p): SvgNode => {
    const b = boxOf(p.bbox);
    return {
      tag: "g",
      attrs: { class: "part", "data-part": p.name, "data-line": p.line, role: "button", tabindex: 0, "aria-label": `${p.name}（${p.class}）の宣言へ` },
      children: [
        { tag: "title", attrs: {}, text: `${p.name}: ${p.class}` },
        // 線だけのアイコンでも、枠の中を押せるように（S248。rect にすると Icon の rect と数え違うので path）
        { tag: "path", attrs: { d: `M${b.x} ${b.y}h${b.w}v${b.h}h${-b.w}z`, fill: "#fff", "fill-opacity": 0, stroke: "none" } },
        p.shapes.length
          ? { tag: "g", attrs: { transform: matrix(p.transform) }, children: p.shapes.map(shapeNode) }
          : { tag: "rect", attrs: { x: b.x, y: b.y, width: b.w, height: b.h, fill: "none", stroke: "#888", "stroke-dasharray": "4 3", "vector-effect": VE } },
        { tag: "text", attrs: { x: b.x + b.w / 2, y: b.y + b.h + size, "text-anchor": "middle", "font-size": size, fill: "#333" }, text: p.name },
      ],
    };
  });
  return [
    ...d.shapes.map(shapeNode),
    ...d.connections.map((c): SvgNode => ({ tag: "g", attrs: { "data-conn": `${c.from} ${c.to}` }, children: [shapeNode(c.line)] })),
    ...parts,
  ];
}

/** 要素の木を DOM にする（webview 用）。属性は setAttribute、文字は textContent で入れる（innerHTML を通らない） */
export function toDom(n: SvgNode, doc: Document = document): Element {
  const e = doc.createElementNS("http://www.w3.org/2000/svg", n.tag);
  for (const [k, v] of Object.entries(n.attrs)) e.setAttribute(k, String(v));
  if (n.text !== undefined) e.textContent = n.text;
  for (const c of n.children ?? []) e.append(toDom(c, doc));
  return e;
}
