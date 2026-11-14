<!-- S245: クラスの図。S261: コネクタから別のコネクタへドラッグすると結線（connect）を足す（離したら onConnect）。S265: 木の行をドロップすると部品を足す（onAdd）。S250: 部品をドラッグして動かせる（離したら onMove。書き換えるのは Rust の編集をエディタに入れる側）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import { diagramDrag } from "./diagram-drag";
  import { diagramNodes, viewBoxOf } from "./shape-svg";
  import SvgTree from "./SvgTree.svelte";

  let { diagram, onJump, onParams, onMove, onConnect, onAdd }: { diagram: Diagram | null; onJump?: (line: number) => void; onParams?: (part: string) => void; onMove?: (part: string, dx: number, dy: number) => Promise<boolean>; onConnect?: (from: string, to: string, points: [number, number][]) => Promise<boolean>; onAdd?: (type: string, x: number, y: number) => Promise<boolean> } = $props();
  let svg = $state<SVGSVGElement>();

  const empty = $derived(!diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length));
  // 部品のクリックとキーは svg で受ける（部品の <g> は SvgTree が組む）
  const jump = (e: Event) => {
    if (dnd.swallowed()) return; // ドラッグの終わりのクリックは宣言へ飛ばない
    const line = (e.target as Element).closest("[data-part]")?.getAttribute("data-line");
    if (line != null) onJump?.(Number(line));
  };
  // S250・S261・S264: ドラッグ（部品の移動と結線）は diagram-drag.ts（VS Code の webview と共通）。動かすのは見た目の transform だけで、離したら onMove・onConnect。編集できなければ元へ戻す
  const dnd = diagramDrag(() => svg, { get onMove() { return onMove; }, get onConnect() { return onConnect; } });
  // 図が読み直されたら（動かした結果が入ったら）見た目の transform は捨てる
  $effect(() => {
    diagram;
    svg?.querySelectorAll(".part[transform]").forEach((g) => g.removeAttribute("transform"));
  });
  // S265: 木の行のドロップ。ドロップした点（Diagram の座標、y 上向き）を onAdd へ
  const CLASS = "application/x-globe-class";
  const over = (e: DragEvent) => {
    if (onAdd && e.dataTransfer?.types.includes(CLASS)) e.preventDefault();
  };
  const drop = (e: DragEvent) => {
    const type = e.dataTransfer?.getData(CLASS);
    if (!onAdd || !type || !svg?.getScreenCTM()) return;
    e.preventDefault();
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg.getScreenCTM()!.inverse());
    onAdd(type, p.x, -p.y);
  };
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
    <svg bind:this={svg} viewBox={viewBoxOf(diagram.extent)} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${diagram.class} の図`} onpointerdown={dnd.down} onpointermove={dnd.move} onpointerup={dnd.up} onpointercancel={dnd.up} ondragover={over} ondrop={drop} onclick={jump} ondblclick={params} oncontextmenu={params} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && jump(e)}>
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
  svg :global(.conn) { fill: #06c; fill-opacity: 0; cursor: crosshair; }
  svg :global(.conn:hover) { fill-opacity: .35; }
  svg :global(.part:focus-visible) { outline: 2px solid #06c; }
  .none { margin: 0; font-size: .85em; }
</style>
