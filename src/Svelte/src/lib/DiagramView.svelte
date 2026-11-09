<!-- S245: クラスの図（読み取り専用）。Rust が読んだ形のデータ（Diagram）を SVG の要素として描く。部品は自分のクラスの Icon を変換して置き、クリックで宣言の行へ飛ぶ -->
<script lang="ts">
  import type { Diagram } from "./backend";
  import ShapeView, { boxOf } from "./ShapeView.svelte";

  let { diagram, onJump }: { diagram: Diagram | null; onJump?: (line: number) => void } = $props();

  const view = $derived.by(() => {
    const b = boxOf(diagram?.extent ?? [-100, -100, 100, 100]);
    return `${b.x} ${b.y} ${b.w || 1} ${b.h || 1}`;
  });
  const size = $derived.by(() => { const b = boxOf(diagram?.extent ?? [-100, -100, 100, 100]); return Math.max(b.w, b.h) / 40 || 1; }); // 部品名の字の大きさ
  // Modelica の行列（y 上向き）を、y を反転した SVG の座標へ（F M F）
  const matrix = (m: number[]) => `matrix(${m[0]} ${-m[1]} ${-m[2]} ${m[3]} ${m[4]} ${-m[5]})`;
  const empty = $derived(!diagram || !(diagram.parts.length || diagram.connections.length || diagram.shapes.length));
</script>

<!-- 地は常に明るい（IconView と同じ） -->
<div class="diagram" data-diagram={diagram?.class ?? ""}>
  {#if diagram && !empty}
    <svg viewBox={view} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${diagram.class} の図`}>
      {#each diagram.shapes as s}<ShapeView {s} />{/each}
      {#each diagram.connections as c}
        <g data-conn={`${c.from} ${c.to}`}><ShapeView s={c.line} /></g>
      {/each}
      {#each diagram.parts as p}
        {@const b = boxOf(p.bbox)}
        <g class="part" data-part={p.name} role="button" tabindex="0" aria-label={`${p.name}（${p.class}）の宣言へ`} onclick={() => onJump?.(p.line)} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && onJump?.(p.line)}>
          <title>{p.name}: {p.class}</title>
          {#if p.shapes.length}
            <g transform={matrix(p.transform)}>{#each p.shapes as s}<ShapeView {s} />{/each}</g>
          {:else}
            <rect x={b.x} y={b.y} width={b.w} height={b.h} fill="none" stroke="#888" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
          {/if}
          <text x={b.x + b.w / 2} y={b.y + b.h + size} text-anchor="middle" font-size={size} fill="#333">{p.name}</text>
        </g>
      {/each}
    </svg>
  {:else}
    <p class="none" data-diagram-empty>図が無い</p>
  {/if}
</div>

<style>
  .diagram { height: 100%; min-height: 12rem; background: #fff; color: #555; border: 1px solid var(--line); display: flex; align-items: center; justify-content: center; }
  svg { width: 100%; height: 100%; display: block; }
  .part { cursor: pointer; }
  .part:focus-visible { outline: 2px solid #06c; }
  .none { margin: 0; font-size: .85em; }
</style>
