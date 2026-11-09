<!-- S247: shape-svg.ts の要素の木（SvgNode）を SVG の要素として描く（IconView と DiagramView が使う）。VS Code の webview は同じ木を toDom で DOM にする -->
<svelte:options namespace="svg" />
<script lang="ts">
  import type { SvgNode } from "./shape-svg";

  let { nodes }: { nodes: SvgNode[] } = $props();
</script>

{#snippet el(n: SvgNode)}
  <svelte:element this={n.tag} {...n.attrs}>{#if n.text !== undefined}{n.text}{/if}{#each n.children ?? [] as c}{@render el(c)}{/each}</svelte:element>
{/snippet}
{#each nodes as n}{@render el(n)}{/each}
