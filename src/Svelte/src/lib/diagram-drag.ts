// S264: 図のドラッグ（S250 の部品の移動・S261 の結線）の処理。Svelte の DiagramView と VS Code の webview が同じファイルを読む（shape-svg.ts と同じ置き方）。
// 動かすのは見た目だけで、離したら onMove（2 単位の格子に吸着した (dx, dy)。Diagram の座標、y 上向き）か onConnect を呼ぶ。編集の適用は呼び出し側。
// 型だけ DOM を使い、値は渡された svg の API だけを呼ぶ（DOMPoint・グローバルの document を使わないので node の試験で偽物を渡せる）。
import { snap } from "./shape-svg.ts"; // 拡張子は node の型の除去で読む試験（VS Code の test/diagram-edit.test.mjs）のため

export interface DragCallbacks {
  /** 離したとき。false なら（編集できなかった）見た目を元へ戻す */
  onMove?: (part: string, dx: number, dy: number) => Promise<boolean>;
  /** コネクタ `a` から `b` へ。points は両端の図の座標 */
  onConnect?: (from: string, to: string, points: [number, number][]) => Promise<boolean>;
}

const THRESHOLD = 4; // px。これ未満は移動と見ず、従来のクリック

export function diagramDrag(svg: () => SVGSVGElement | null | undefined, cb: DragCallbacks) {
  let drag: { g: Element; part: string; x: number; y: number; sx: number; sy: number; dx: number; dy: number; moved: boolean } | null = null;
  let wire: { from: Element; sx: number; sy: number; line: Element; moved: boolean } | null = null;
  let swallow = false; // ドラッグの終わりのクリックは宣言へ飛ばさない
  const far = (e: PointerEvent, s: { sx: number; sy: number }) => Math.hypot(e.clientX - s.sx, e.clientY - s.sy) >= THRESHOLD;
  // 画面の座標を図の座標（y は SVG のまま下向き）へ
  const at = (e: PointerEvent, s: SVGSVGElement) => {
    const m = s.getScreenCTM()!.inverse();
    return [m.a * e.clientX + m.c * e.clientY + m.e, m.b * e.clientX + m.d * e.clientY + m.f];
  };
  const ends = (c: Element): [number, number] => [Number(c.getAttribute("data-x")), Number(c.getAttribute("data-y"))];
  const settle = () => {
    swallow = true;
    setTimeout(() => (swallow = false), 0);
  };

  const down = (e: PointerEvent) => {
    const s = svg();
    const t = e.target as Element;
    if (!s?.getScreenCTM() || e.button !== 0) return;
    const c = t.closest(".conn");
    if (c && cb.onConnect) {
      const [x, y] = ends(c);
      const line = s.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "line");
      for (const [k, v] of Object.entries({ x1: x, y1: -y, x2: x, y2: -y, stroke: "#00f", "stroke-width": 2, "vector-effect": "non-scaling-stroke", "pointer-events": "none" })) line.setAttribute(k, String(v));
      wire = { from: c, sx: e.clientX, sy: e.clientY, line, moved: false };
      s.setPointerCapture(e.pointerId);
      return;
    }
    const g = t.closest("[data-part]");
    if (!cb.onMove || !g) return;
    const [x, y] = at(e, s);
    drag = { g, part: g.getAttribute("data-part")!, x, y, sx: e.clientX, sy: e.clientY, dx: 0, dy: 0, moved: false };
    g.setPointerCapture(e.pointerId); // 部品に捕まえる（svg だと離したあとの click の対象が svg になり、宣言へ飛べない）
  };

  const move = (e: PointerEvent) => {
    const s = svg();
    if (wire && s) {
      if (!wire.moved && !far(e, wire)) return;
      if (!wire.moved) s.append(wire.line);
      wire.moved = true;
      const [x, y] = at(e, s);
      wire.line.setAttribute("x2", String(x));
      wire.line.setAttribute("y2", String(y));
    } else if (drag && s && (drag.moved || far(e, drag))) {
      const [x, y] = at(e, s);
      drag.moved = true;
      drag.dx = snap(x - drag.x);
      drag.dy = snap(drag.y - y);
      drag.g.setAttribute("transform", `translate(${drag.dx} ${-drag.dy})`);
    }
  };

  const up = async (e: PointerEvent) => {
    const w = wire;
    const d = drag;
    wire = drag = null;
    if (w) {
      w.line.remove();
      if (!w.moved) return;
      settle();
      // 別のコネクタの上で離したときだけ。同じコネクタ・コネクタ以外なら何もしない
      const to = w.from.ownerDocument.elementFromPoint(e.clientX, e.clientY)?.closest(".conn");
      const [a, b] = [w.from.getAttribute("data-end"), to?.getAttribute("data-end")];
      if (to && a && b && a !== b) await cb.onConnect?.(a, b, [ends(w.from), ends(to)]);
    } else if (d?.moved) {
      settle();
      if (!d.dx && !d.dy) return d.g.removeAttribute("transform");
      if (!(await cb.onMove!(d.part, d.dx, d.dy))) d.g.removeAttribute("transform");
    }
  };

  return { down, move, up, swallowed: () => swallow };
}
