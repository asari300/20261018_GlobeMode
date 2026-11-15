<!-- S316: 部品を選んで c でコネクタを Tab・矢印で選び、Enter で始点・終点を決めて結線を足す（Esc でやめる。diagram-connect.ts）。選んだ結線で + は中点に途中の点を足す。部品で Enter はパラメータのダイアログ（宣言の行へは Space）。S299: 選んだ結線の途中の点に小さな四角のつまみ（`.wp`）を出し、ドラッグで動かす（onWaypoint）。線のダブルクリックで点を足す（onAddWaypoint）。S245: クラスの図。S261: コネクタから別のコネクタへドラッグすると結線（connect）を足す（離したら onConnect）。S265: 木の行をドロップすると部品を足す（onAdd）。S281: 部品か結線を選び（クリック・Tab）、Delete / Backspace で消す（onDelete）。S250: 部品をドラッグして動かせる（離したら onMove。書き換えるのは Rust の編集をエディタに入れる側）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import { diagramDrag } from "./diagram-drag";
  import { deleteOf, markSel, nudgeOf, pickSel, type Sel } from "./diagram-select";
  import { markWire, wireKey, type Wire } from "./diagram-connect";
  import { addWaypointOf, halfOf, handlesOf, midWaypointOf } from "./diagram-waypoint";
  import { diagramNodes, viewBoxOf } from "./shape-svg";
  import SvgTree from "./SvgTree.svelte";

  let { diagram, onJump, onParams, onMove, onConnect, onAdd, onDelete, onWaypoint, onAddWaypoint }: { diagram: Diagram | null; onJump?: (line: number) => void; onParams?: (part: string) => void; onMove?: (part: string, dx: number, dy: number) => Promise<boolean>; onConnect?: (from: string, to: string, points: [number, number][]) => Promise<boolean>; onAdd?: (type: string, x: number, y: number) => Promise<boolean>; onDelete?: (what: { part: string } | { from: string; to: string }) => Promise<boolean>; onWaypoint?: (from: string, to: string, index: number, dx: number, dy: number) => Promise<boolean>; onAddWaypoint?: (from: string, to: string, index: number, x: number, y: number, ends: [[number, number], [number, number]]) => Promise<boolean> } = $props();
  let svg = $state<SVGSVGElement>();

  const empty = $derived(!diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length));
  // 部品のクリックとキーは svg で受ける（部品の <g> は SvgTree が組む）
  const jump = (e: Event) => {
    if (dnd.swallowed()) return; // ドラッグの終わりのクリックは宣言へ飛ばない
    const t = e.target as Element;
    const line = t.closest("[data-part]")?.getAttribute("data-line");
    if (line != null) onJump?.(Number(line));
    pick(e);
    // 飛ぶとエディタにフォーカスが移り、Delete がテキストを消してしまう。選んだ部品・結線へフォーカスを戻す
    (t.closest("[data-part], [data-hit]") as SVGElement | null)?.focus({ preventScroll: true });
  };
  // S281: 選び。部品か結線（`.hit`）のクリック・フォーカス（Tab）で選び、何も無い所のクリックで外す。キーは svg だけで受ける（エディタの Delete は触らない）
  let selected = $state<Sel>(null);
  const pick = (e: Event) => (selected = pickSel(e.target as Element, e.type, selected));
  $effect(() => {
    diagram;
    selected = markSel(svg, selected); // 編集で消えた対象は外れる
  });
  // S316: キーボードでの結線。選び中のコネクタは枠（`.cur`・`.from`）と、下の role="status" の文で伝える
  let wire = $state<Wire>(null);
  let say = $state("");
  $effect(() => {
    diagram;
    markWire(svg, wire, say); // 図が作り直されたら付け直す
  });
  const key = async (e: KeyboardEvent) => {
    const step = onConnect ? wireKey(wire, diagram, selected, e) : null;
    if (step?.handled) {
      e.preventDefault();
      [wire, say] = [step.wire, step.say || say];
      if (step.connect) await onConnect!(step.connect.from, step.connect.to, step.connect.points);
      return;
    }
    const mid = onAddWaypoint && midWaypointOf(diagram, selected, e.key);
    if (mid) {
      e.preventDefault();
      await onAddWaypoint!(mid.from, mid.to, mid.index, mid.x, mid.y, mid.ends);
      return;
    }
    const del = onDelete && deleteOf(selected, e.key);
    if (del) {
      e.preventDefault();
      await onDelete!(del);
    } else if (onMove && nudgeOf(selected, e.key, e.shiftKey)) {
      e.preventDefault(); // S309: 矢印キーで選んだ部品を動かす（ドラッグと同じ 2 単位、Shift で 10 単位）
      const n = nudgeOf(selected, e.key, e.shiftKey)!;
      await onMove(n.part, n.dx, n.dy);
    } else if (e.key === "Enter" && onParams && (e.target as Element).closest("[data-part]")) {
      e.preventDefault(); // S316: 部品で Enter はパラメータのダイアログ（webview と同じ。宣言の行へは Space）
      onParams((e.target as Element).closest("[data-part]")!.getAttribute("data-part")!);
    } else if (e.key === "Enter" || e.key === " ") jump(e);
  };
  // S250・S261・S264: ドラッグ（部品の移動と結線）は diagram-drag.ts（VS Code の webview と共通）。動かすのは見た目の transform だけで、離したら onMove・onConnect。編集できなければ元へ戻す
  const dnd = diagramDrag(() => svg, { get onMove() { return onMove; }, get onConnect() { return onConnect; }, get onWaypoint() { return onWaypoint; } });
  // 図が読み直されたら（動かした結果が入ったら）見た目の transform は捨てる
  $effect(() => {
    diagram;
    svg?.querySelectorAll(".part[transform], .wp[transform]").forEach((g) => g.removeAttribute("transform"));
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
  // S299・S302: 選んだ結線の途中の点のつまみと、線のダブルクリックで足す点の決め方は diagram-waypoint.ts（VS Code の webview と共通）
  const handles = $derived(handlesOf(diagram, selected));
  const half = $derived(halfOf(diagram));
  const addWaypoint = (e: MouseEvent) => {
    if (!onAddWaypoint || !svg?.getScreenCTM()) return;
    const q = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg.getScreenCTM()!.inverse());
    const w = addWaypointOf(diagram, (e.target as Element).closest("[data-hit]")?.getAttribute("data-hit"), q.x, -q.y);
    if (!w) return;
    e.preventDefault();
    onAddWaypoint(w.from, w.to, w.index, w.x, w.y, w.ends);
  };
  // S248: ダブルクリックと右クリックで、その部品のパラメータのダイアログへ
  const params = (e: Event) => {
    const part = (e.target as Element).closest("[data-part]")?.getAttribute("data-part");
    if (part == null) return e.type === "dblclick" ? addWaypoint(e as MouseEvent) : undefined; // 右クリックでは足さない
    e.preventDefault();
    onParams?.(part);
  };
</script>

<!-- 地は常に明るい（IconView と同じ） -->
<div class="diagram" data-diagram={diagram?.class ?? ""}>
  {#if diagram && !empty}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <svg bind:this={svg} viewBox={viewBoxOf(diagram.extent)} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${diagram.class} の図`} onpointerdown={dnd.down} onpointermove={dnd.move} onpointerup={dnd.up} onpointercancel={dnd.up} ondragover={over} ondrop={drop} onclick={jump} onfocusin={pick} ondblclick={params} oncontextmenu={params} onkeydown={key}>
      <SvgTree nodes={diagramNodes(diagram)} />
      {#if onWaypoint}
        {#each handles as h (`${h.key}:${h.index}:${h.p}`)}
          <rect class="wp" data-wp={h.key} data-index={h.index} x={h.p[0] - half} y={-h.p[1] - half} width={half * 2} height={half * 2} role="presentation"><title>途中の点 {h.index}（ドラッグで動かす）</title></rect>
        {/each}
      {/if}
    </svg>
    <p class="say" role="status">{say}</p>
  {:else}
    <p class="none" data-diagram-empty>図が無い</p>
  {/if}
</div>

<style>
  .diagram { position: relative; height: 100%; min-height: 12rem; background: #fff; color: #555; border: 1px solid var(--line); display: flex; align-items: center; justify-content: center; }
  svg { width: 100%; height: 100%; display: block; }
  svg :global(.part) { cursor: grab; touch-action: none; }
  svg :global(.conn) { fill: #06c; fill-opacity: 0; cursor: crosshair; }
  svg :global(.conn:hover) { fill-opacity: .35; }
  svg :global(.part:focus-visible), svg :global(.hit:focus-visible) { outline: 2px solid #06c; }
  svg :global(.part.sel > path) { stroke: #06c; stroke-width: 2; stroke-dasharray: 4 2; vector-effect: non-scaling-stroke; fill: #06c; fill-opacity: .08; }
  svg :global([data-conn].sel polyline) { stroke: #06c; stroke-width: 3; }
  svg :global(.hit) { cursor: pointer; }
  svg :global(.wp) { fill: #fff; stroke: #06c; stroke-width: 2; vector-effect: non-scaling-stroke; cursor: move; touch-action: none; }
  svg :global(.conn.cur) { stroke: #f60; stroke-width: 3; fill-opacity: .35; vector-effect: non-scaling-stroke; }
  svg :global(.conn.from) { stroke: #06c; stroke-width: 3; fill-opacity: .6; vector-effect: non-scaling-stroke; }
  .say { position: absolute; left: 0; bottom: 0; margin: 0; padding: 0 .3em; font-size: .8em; background: #fff; }
  .none { margin: 0; font-size: .85em; }
</style>
