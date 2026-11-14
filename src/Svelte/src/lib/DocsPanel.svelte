<!-- S232: 説明（Documentation）のパネル。HTML は信用せず、sanitize が白リストで組み直した木を要素と文字列として描く（{@html} は使わない） -->
<script lang="ts">
  import type { Diagram, Documentation, Icon } from "./backend";
  import DiagramView from "./DiagramView.svelte";
  import IconView from "./IconView.svelte";
  import { sanitize, type Safe } from "./html-sanitize";

  let { doc, icon = null, diagram = null, onJump, onParams, onMove, onConnect }: { doc: Documentation | null; icon?: Icon | null; diagram?: Diagram | null; onJump?: (line: number) => void; onParams?: (part: string) => void; onMove?: (part: string, dx: number, dy: number) => Promise<boolean>; onConnect?: (from: string, to: string, points: [number, number][]) => Promise<boolean> } = $props();
  let tab = $state<"docs" | "diagram">("docs"); // S245: 説明と図のタブ
  const info = $derived(doc ? sanitize(doc.info) : []);
  const revisions = $derived(doc ? sanitize(doc.revisions) : []);
</script>

{#snippet nodes(list: Safe[])}
  {#each list as n}
    {#if typeof n === "string"}{n}
    {:else if n.tag === "br" || n.tag === "hr"}<svelte:element this={n.tag} />
    {:else if n.tag === "a"}<a href={n.href} target="_blank" rel="noopener noreferrer">{@render nodes(n.children)}</a>
    {:else}<svelte:element this={n.tag}>{@render nodes(n.children)}</svelte:element>{/if}
  {/each}
{/snippet}

<aside class="docs" class:wide={tab === "diagram"} aria-label="説明" data-docs>
  <div class="tabs" role="group" aria-label="パネルの表示">
    <button aria-pressed={tab === "docs"} onclick={() => (tab = "docs")}>説明</button>
    <button aria-pressed={tab === "diagram"} data-tab-diagram onclick={() => (tab = "diagram")}>図</button>
  </div>
  {#if tab === "diagram"}
  <DiagramView {diagram} {onJump} {onParams} {onMove} {onConnect} />
  {:else}
  <IconView {icon} />
  {#if doc}<p class="cls"><code>{doc.class}</code></p>{/if}
  {#if info.length}
    <div class="body" data-docs-info>{@render nodes(info)}</div>
  {:else}
    <p class="empty" data-docs-empty>Documentation が無い</p>
  {/if}
  {#if revisions.length}<details><summary>改訂履歴</summary><div class="body" data-docs-revisions>{@render nodes(revisions)}</div></details>{/if}
  {/if}
</aside>

<style>
  .docs { flex: 0 0 16rem; height: 16rem; overflow: auto; border: 1px solid var(--border); padding: 0 .5rem; font-size: .9em; overflow-wrap: anywhere; }
  .docs.wide { flex-basis: 30rem; height: 22rem; display: flex; flex-direction: column; }
  .tabs { display: flex; gap: .25rem; margin: .25rem 0; }
  .tabs button { font: inherit; padding: .1rem .5rem; }
  .tabs button[aria-pressed="true"] { font-weight: bold; border-color: var(--accent); }
  .docs.wide :global(.diagram) { flex: 1; min-height: 0; }
  .cls, .empty { margin: .25rem 0; color: var(--muted); }
  .body :global(a) { color: var(--accent); }
  .body :global(pre) { white-space: pre-wrap; background: var(--hover); padding: .25rem; }
  .body :global(table) { border-collapse: collapse; }
  .body :global(td), .body :global(th) { border: 1px solid var(--line); padding: .1rem .3rem; }
  @media (max-width: 600px) {
    .docs { flex: none; height: 10rem; }
  }
</style>
