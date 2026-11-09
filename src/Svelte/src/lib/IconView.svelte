<!-- S244: クラスのアイコン。Rust が読んだ形のデータ（Icon）を SVG の要素として描く（文字列を {@html} に渡さない）。座標は Modelica のまま（y は上向き）なので y を反転して描く -->
<script lang="ts">
  import type { Icon } from "./backend";
  import ShapeView, { boxOf } from "./ShapeView.svelte";

  let { icon }: { icon: Icon | null } = $props();

  const view = $derived.by(() => {
    const b = boxOf(icon?.extent ?? [-100, -100, 100, 100]);
    return `${b.x} ${b.y} ${b.w || 1} ${b.h || 1}`;
  });
</script>

<!-- 暗でも見えるよう、地は常に明るい（Modelica の色は白地を前提にしている） -->
<div class="icon" data-icon={icon?.class ?? ""}>
  {#if icon && icon.shapes.length}
    <svg viewBox={view} preserveAspectRatio="xMidYMid meet" role="img" aria-label={`${icon.class} のアイコン`}>
      {#each icon.shapes as s}<ShapeView {s} />{/each}
    </svg>
  {:else}
    <p class="none" data-icon-empty>アイコンが無い</p>
  {/if}
</div>

<style>
  .icon { height: 120px; background: #fff; color: #555; border: 1px solid var(--line); margin: .25rem 0; display: flex; align-items: center; justify-content: center; }
  svg { width: 100%; height: 100%; display: block; }
  .none { margin: 0; font-size: .85em; }
</style>
