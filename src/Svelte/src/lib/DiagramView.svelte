<!-- S245: クラスの図（読み取り専用）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import { diagramNodes, viewBoxOf } from "./shape-svg";
  import SvgTree from "./SvgTree.svelte";

  let { diagram, onJump, onParams }: { diagram: Diagram | null; onJump?: (line: number) => void; onParams?: (part: string) => void } = $props();

  const empty = $derived(!diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length));
  // 部品のクリックとキーは svg で受ける（部品の <g> は SvgTree が組む）
  const jump = (e: Event) => {
    const line = (e.target as Element).closest("[data-part]")?.getAttribute("data-line");
    if (line != null) onJump?.(Number(line));
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
    <svg viewBox={viewBoxOf(diagram.extent)} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${diagram.class} の図`} onclick={jump} ondblclick={params} oncontextmenu={params} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && jump(e)}>
      <SvgTree nodes={diagramNodes(diagram)} />
    </svg>
  {:else}
    <p class="none" data-diagram-empty>図が無い</p>
  {/if}
</div>

<style>
  .diagram { height: 100%; min-height: 12rem; background: #fff; color: #555; border: 1px solid var(--line); display: flex; align-items: center; justify-content: center; }
  svg { width: 100%; height: 100%; display: block; }
  svg :global(.part) { cursor: pointer; }
  svg :global(.part:focus-visible) { outline: 2px solid #06c; }
  .none { margin: 0; font-size: .85em; }
</style>
