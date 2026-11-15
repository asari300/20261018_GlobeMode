<!-- S299: 選んだ結線の途中の点に小さな四角のつまみ（`.wp`）を出し、ドラッグで動かす（onWaypoint）。線のダブルクリックで点を足す（onAddWaypoint）。S245: クラスの図。S261: コネクタから別のコネクタへドラッグすると結線（connect）を足す（離したら onConnect）。S265: 木の行をドロップすると部品を足す（onAdd）。S281: 部品か結線を選び（クリック・Tab）、Delete / Backspace で消す（onDelete）。S250: 部品をドラッグして動かせる（離したら onMove。書き換えるのは Rust の編集をエディタに入れる側）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import { diagramDrag } from "./diagram-drag";
  import { deleteOf, markSel, pickSel, type Sel } from "./diagram-select";
  import { diagramNodes, snap, viewBoxOf } from "./shape-svg";
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
  const key = async (e: KeyboardEvent) => {
    const del = onDelete && deleteOf(selected, e.key);
    if (del) {
      e.preventDefault();
      await onDelete!(del);
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
  // S299: 選んだ結線の途中の点（最初と最後はコネクタに付くので出さない）。つまみの大きさは部品名の字と同じ基準
  const handles = $derived.by(() => {
    const sel = selected;
    const c = sel && "key" in sel ? diagram?.connections.find((c) => `${c.from} ${c.to}` === sel.key) : null;
    return c ? c.line.points.map((p, index) => ({ p, index, key: `${c.from} ${c.to}` })).slice(1, -1) : [];
  });
  const half = $derived(diagram ? Math.max(diagram.extent[2] - diagram.extent[0], diagram.extent[3] - diagram.extent[1]) / 80 || 0.5 : 0.5);
  // 線のダブルクリック: いちばん近い線分の次へ点を足す（点が無い直線は 2 つのコネクタの位置を始めと終わりにして `Line(points)` を作る）
  const addWaypoint = (e: MouseEvent) => {
    const hit = (e.target as Element).closest("[data-hit]")?.getAttribute("data-hit");
    const c = hit && diagram?.connections.find((c) => `${c.from} ${c.to}` === hit);
    if (!c || !diagram || !onAddWaypoint || !svg?.getScreenCTM()) return;
    const q = new DOMPoint(e.clientX, e.clientY).matrixTransform(svg.getScreenCTM()!.inverse());
    const [x, y] = [q.x, -q.y];
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
      const conn = diagram!.parts.find((p) => p.name === part.split("[")[0])?.connectors?.find((k) => k.name === rest.join("."));
      return conn ? [conn.pos[0], conn.pos[1]] : fallback;
    };
    e.preventDefault();
    onAddWaypoint(c.from, c.to, seg + 1, snap(x), snap(y), [end(c.from, pts[0] as [number, number]), end(c.to, pts[pts.length - 1] as [number, number])]);
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
  svg :global(.part:focus-visible), svg :global(.hit:focus-visible) { outline: 2px solid #06c; }
  svg :global(.part.sel > path) { stroke: #06c; stroke-width: 2; stroke-dasharray: 4 2; vector-effect: non-scaling-stroke; fill: #06c; fill-opacity: .08; }
  svg :global([data-conn].sel polyline) { stroke: #06c; stroke-width: 3; }
  svg :global(.hit) { cursor: pointer; }
  svg :global(.wp) { fill: #fff; stroke: #06c; stroke-width: 2; vector-effect: non-scaling-stroke; cursor: move; touch-action: none; }
  .none { margin: 0; font-size: .85em; }
</style>
