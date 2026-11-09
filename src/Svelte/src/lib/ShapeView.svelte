<!-- S244・S245: 図形 1 つを SVG の要素として描く（IconView と DiagramView が使う）。座標は Modelica のまま（y は上向き）なので y を反転して描く -->
<svelte:options namespace="svg" />
<script lang="ts" module>
  import type { IconShape } from "./backend";
  export const boxOf = (e: IconShape["extent"]) => ({ x: Math.min(e[0], e[2]), y: -Math.max(e[1], e[3]), w: Math.abs(e[2] - e[0]), h: Math.abs(e[3] - e[1]) });
</script>

<script lang="ts">
  let { s }: { s: IconShape } = $props();

  const rgb = (c: [number, number, number] | null) => (c ? `rgb(${c[0]},${c[1]},${c[2]})` : "none");
  const DASH = { solid: "", dash: "4 3", dot: "1 3", dashdot: "4 3 1 3", dashdotdot: "4 3 1 3 1 3" };
  const pts = (p: [number, number][]) => p.map(([x, y]) => `${x},${-y}`).join(" ");
  const b = $derived(boxOf(s.extent));
  const width = $derived(Math.max(1, s.thickness * 4)); // 画面の px（vector-effect で拡大に連れない）
  // 文字の大きさ: 指定が無ければ枠の高さと、文字数から見た幅に収まる大きさ
  const size = $derived(s.fontSize > 0 ? s.fontSize : Math.max(1, Math.min(b.h, (b.w / Math.max(1, s.text.length)) * 1.7)));
  // 線の端の矢印（端の点 a、一つ手前の点 b）。open・half は V 字、filled は三角
  const arrow = (end: 0 | 1) => {
    const k = s.arrows[end];
    const p = s.points;
    if (k === "none" || p.length < 2) return null;
    const [a, c] = end === 0 ? [p[0], p[1]] : [p[p.length - 1], p[p.length - 2]];
    const d = Math.hypot(c[0] - a[0], c[1] - a[1]) || 1;
    const [ux, uy] = [(c[0] - a[0]) / d, (c[1] - a[1]) / d];
    const n = s.arrowSize * 2;
    const left: [number, number] = [a[0] + n * (ux * 0.87 - uy * 0.5), a[1] + n * (uy * 0.87 + ux * 0.5)];
    const right: [number, number] = [a[0] + n * (ux * 0.87 + uy * 0.5), a[1] + n * (uy * 0.87 - ux * 0.5)];
    return { points: pts([left, a, right]), filled: k === "filled" };
  };
</script>

<g transform={`translate(${s.origin[0]} ${-s.origin[1]}) rotate(${-s.rotation})`}>
  {#if s.kind === "rect"}
    <rect x={b.x} y={b.y} width={b.w} height={b.h} rx={s.radius} fill={rgb(s.fill)} stroke={rgb(s.line)} stroke-width={width} stroke-dasharray={DASH[s.dash]} vector-effect="non-scaling-stroke" />
  {:else if s.kind === "ellipse"}
    <ellipse cx={b.x + b.w / 2} cy={b.y + b.h / 2} rx={b.w / 2} ry={b.h / 2} fill={rgb(s.fill)} stroke={rgb(s.line)} stroke-width={width} stroke-dasharray={DASH[s.dash]} vector-effect="non-scaling-stroke" />
  {:else if s.kind === "polygon"}
    <polygon points={pts(s.points)} fill={rgb(s.fill)} stroke={rgb(s.line)} stroke-width={width} stroke-dasharray={DASH[s.dash]} stroke-linejoin="round" vector-effect="non-scaling-stroke" />
  {:else if s.kind === "line"}
    <polyline points={pts(s.points)} fill="none" stroke={rgb(s.line)} stroke-width={width} stroke-dasharray={DASH[s.dash]} stroke-linejoin="round" vector-effect="non-scaling-stroke" />
    {#each [0, 1] as const as end}
      {@const a = arrow(end)}
      {#if a}<polyline points={a.points} fill={a.filled ? rgb(s.line) : "none"} stroke={rgb(s.line)} stroke-width={width} vector-effect="non-scaling-stroke" />{/if}
    {/each}
  {:else if s.kind === "text"}
    <text x={s.align === "left" ? b.x : s.align === "right" ? b.x + b.w : b.x + b.w / 2} y={b.y + b.h / 2} text-anchor={s.align === "left" ? "start" : s.align === "right" ? "end" : "middle"} dominant-baseline="central" font-size={size} fill={rgb(s.line)}>{s.text}</text>
  {:else if s.kind === "bitmap"}
    <rect x={b.x} y={b.y} width={b.w} height={b.h} fill="none" stroke={rgb(s.line)} stroke-width="1" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
    <text x={b.x + b.w / 2} y={b.y + b.h / 2} text-anchor="middle" dominant-baseline="central" font-size={Math.max(1, Math.min(b.h / 4, 10))} fill={rgb(s.line)}>{s.text}</text>
  {/if}
</g>
