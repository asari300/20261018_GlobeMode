<!-- S257: 木の行の小さなアイコン（装飾なので aria-hidden）。行が見えたときだけ load を呼ぶ。Icon の無いクラスは何も描かない（幅だけ残す）。text は小さすぎて読めないので描かない -->
<script lang="ts">
  import type { Icon } from "./backend";
  import { iconNodes, viewBoxOf } from "./shape-svg";
  import SvgTree from "./SvgTree.svelte";

  let { load }: { load: () => Promise<Icon | null> } = $props();
  let icon = $state<Icon | null>(null);
  let box: HTMLSpanElement;
  const shapes = $derived(icon ? icon.shapes.filter((s) => s.kind !== "text") : []);

  $effect(() => {
    let alive = true;
    const go = () => void load().then((i) => { if (alive) icon = i; });
    if (typeof IntersectionObserver === "undefined") { go(); return () => { alive = false; }; }
    const io = new IntersectionObserver((es) => { if (es.some((e) => e.isIntersecting)) { io.disconnect(); go(); } });
    io.observe(box);
    return () => { alive = false; io.disconnect(); };
  });
</script>

<span class="ti" bind:this={box} aria-hidden="true">
  {#if icon && shapes.length}
    <svg viewBox={viewBoxOf(icon.extent)} preserveAspectRatio="xMidYMid meet" data-tree-icon={icon.class}><SvgTree nodes={iconNodes({ ...icon, shapes })} /></svg>
  {/if}
</span>

<style>
  .ti { display: inline-block; width: 18px; height: 18px; vertical-align: middle; margin-right: .2rem; }
  svg { width: 18px; height: 18px; display: block; background: #fff; border-radius: 2px; }
</style>
