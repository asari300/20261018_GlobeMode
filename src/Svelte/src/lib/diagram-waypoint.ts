// S302: 結線の途中の点（S299）のつまみと、線のダブルクリックで足す点の決め方。DiagramView.svelte に埋まっていたものを、VS Code の webview と共有するために出した（diagram-select.ts と同じ置き方）。
// 値は渡された図のデータだけを見る（DOM を使わない）ので node の試験で動かせる。ドラッグは diagram-drag.ts、編集の適用は呼び出し側。
import type { Diagram } from "./backend";
import { snap, type SvgNode } from "./shape-svg.ts"; // 拡張子は node の型の除去で読む試験（VS Code の test/）のため
import type { Sel } from "./diagram-select.ts";

export interface Handle {
  p: [number, number];
  index: number;
  /** 結線の `from to` */
  key: string;
}

/** 選んだ結線の途中の点（最初と最後はコネクタに付くので出さない） */
export function handlesOf(diagram: Diagram | null | undefined, sel: Sel): Handle[] {
  const c = sel && "key" in sel ? diagram?.connections.find((c) => `${c.from} ${c.to}` === sel.key) : null;
  return c ? c.line.points.map((p, index) => ({ p, index, key: `${c.from} ${c.to}` })).slice(1, -1) : [];
}

/** つまみの半分の大きさ（図の単位。部品名の字と同じ基準） */
export const halfOf = (diagram: Diagram | null | undefined): number => (diagram ? Math.max(diagram.extent[2] - diagram.extent[0], diagram.extent[3] - diagram.extent[1]) / 80 || 0.5 : 0.5);

/** つまみの四角（`rect.wp`。DiagramView.svelte の `{#each handles}` と同じ属性）。webview が toDom で DOM にする */
export const handleNode = (h: Handle, half: number): SvgNode => ({
  tag: "rect",
  attrs: { class: "wp", "data-wp": h.key, "data-index": h.index, x: h.p[0] - half, y: -h.p[1] - half, width: half * 2, height: half * 2, role: "presentation" },
  children: [{ tag: "title", attrs: {}, text: `途中の点 ${h.index}（ドラッグで動かす）` }],
});

export interface AddWaypoint {
  from: string;
  to: string;
  index: number;
  x: number;
  y: number;
  ends: [[number, number], [number, number]];
}

/**
 * 結線（`hit` = `from to`）の線のダブルクリックの点 (x, y)（図の座標、y 上向き）から、足す点を決める。いちばん近い線分の次の番号へ、2 単位に吸着した座標で。
 * `ends` は両端のコネクタの位置（点が無い直線は、これと足す点で `Line(points)` を作る）。結線が無ければ null
 */
export function addWaypointOf(diagram: Diagram | null | undefined, hit: string | null | undefined, x: number, y: number): AddWaypoint | null {
  const c = hit && diagram?.connections.find((c) => `${c.from} ${c.to}` === hit);
  if (!c || !diagram) return null;
  const pts = c.line.points;
  const dist = (i: number) => {
    const [a, b] = [pts[i], pts[i + 1]];
    const [vx, vy, l] = [b[0] - a[0], b[1] - a[1], (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2];
    const t = l ? Math.max(0, Math.min(1, ((x - a[0]) * vx + (y - a[1]) * vy) / l)) : 0;
    return Math.hypot(x - a[0] - t * vx, y - a[1] - t * vy);
  };
  const seg = pts.slice(0, -1).map((_, i) => i).reduce((m, i) => (dist(i) < dist(m) ? i : m), 0);
  const end = (name: string, fallback: [number, number]): [number, number] => {
    const [part, ...rest] = name.split(".");
    const conn = diagram.parts.find((p) => p.name === part.split("[")[0])?.connectors?.find((k) => k.name === rest.join("."));
    return conn ? [conn.pos[0], conn.pos[1]] : fallback;
  };
  return { from: c.from, to: c.to, index: seg + 1, x: snap(x), y: snap(y), ends: [end(c.from, pts[0] as [number, number]), end(c.to, pts[pts.length - 1] as [number, number])] };
}
