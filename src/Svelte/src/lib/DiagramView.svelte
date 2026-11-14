<!-- S245: クラスの図。S250: 部品をドラッグして動かせる（離したら onMove。書き換えるのは Rust の編集をエディタに入れる側）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import { diagramNodes, snap, viewBoxOf } from "./shape-svg";
  import SvgTree from "./SvgTree.svelte";

  let { diagram, onJump, onParams, onMove }: { diagram: Diagram | null; onJump?: (line: number) => void; onParams?: (part: string) => void; onMove?: (part: string, dx: number, dy: number) => Promise<boolean> } = $props();
  let svg = $state<SVGSVGElement>();

  const empty = $derived(!diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length));
  // 部品のクリックとキーは svg で受ける（部品の <g> は SvgTree が組む）
  const jump = (e: Event) => {
    if (swallow) return; // ドラッグの終わりのクリックは宣言へ飛ばない
    const line = (e.target as Element).closest("[data-part]")?.getAttribute("data-line");
    if (line != null) onJump?.(Number(line));
  };
  // S250: ドラッグ。動かすのは見た目の transform だけ（図が読み直されたら消える）。離したら 2 単位の格子に吸着した (dx, dy)（Diagram の座標、y 上向き）を onMove へ。編集できなければ元へ戻す
  let drag: { g: Element; part: string; x: number; y: number; sx: number; sy: number; dx: number; dy: number; moved: boolean } | null = null;
  let swallow = false;
  const at = (e: PointerEvent) => {
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg!.getScreenCTM()!.inverse());
    return [p.x, p.y];
  };
  const down = (e: PointerEvent) => {
    const g = (e.target as Element).closest("[data-part]");
    if (!onMove || e.button !== 0 || !g || !svg?.getScreenCTM()) return;
    const [x, y] = at(e);
    drag = { g, part: g.getAttribute("data-part")!, x, y, sx: e.clientX, sy: e.clientY, dx: 0, dy: 0, moved: false };
    svg.setPointerCapture(e.pointerId);
  };
  const move = (e: PointerEvent) => {
    if (!drag || (!drag.moved && Math.hypot(e.clientX - drag.sx, e.clientY - drag.sy) < 4)) return;
    const [x, y] = at(e);
    drag.moved = true;
    drag.dx = snap(x - drag.x);
    drag.dy = snap(drag.y - y);
    drag.g.setAttribute("transform", `translate(${drag.dx} ${-drag.dy})`);
  };
  const up = async () => {
    const d = drag;
    drag = null;
    if (!d?.moved) return;
    swallow = true;
    setTimeout(() => (swallow = false), 0);
    if (!d.dx && !d.dy) return d.g.removeAttribute("transform");
    if (!(await onMove!(d.part, d.dx, d.dy))) d.g.removeAttribute("transform");
  };
  // 図が読み直されたら（動かした結果が入ったら）見た目の transform は捨てる
  $effect(() => {
    diagram;
    svg?.querySelectorAll(".part[transform]").forEach((g) => g.removeAttribute("transform"));
  });
  // S248: ダブルクリックと右クリックで、その部品のパラメータのダイアログへ
  const params = (e: Event) => {
    const part = (e.target as Element).closest("[data-part]")?.getAttribute("data-part");
    if (part == null) return;
    e.preventDefault();
    onParams?.(part);
  };
</script>

<!-- 地は常に明るい（IconView と同じ） -->
<div class="diagram" data-diagram={diagram?.class ?? ""}>
  {#if diagram && !empty}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <svg bind:this={svg} viewBox={viewBoxOf(diagram.extent)} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${diagram.class} の図`} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onclick={jump} ondblclick={params} oncontextmenu={params} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && jump(e)}>
      <SvgTree nodes={diagramNodes(diagram)} />
    </svg>
  {:else}
    <p class="none" data-diagram-empty>図が無い</p>
  {/if}
</div>

<style>
  .diagram { height: 100%; min-height: 12rem; background: #fff; color: #555; border: 1px solid var(--line); display: flex; align-items: center; justify-content: center; }
  svg { width: 100%; height: 100%; display: block; }
  svg :global(.part) { cursor: grab; touch-action: none; }
  svg :global(.part:focus-visible) { outline: 2px solid #06c; }
  .none { margin: 0; font-size: .85em; }
</style>
