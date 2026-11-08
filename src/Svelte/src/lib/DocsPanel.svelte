<!-- S232: 説明（Documentation）のパネル。HTML は信用せず、sanitize が白リストで組み直した木を要素と文字列として描く（{@html} は使わない） -->
<script lang="ts">
  import type { Documentation } from "./backend";
  import { sanitize, type Safe } from "./html-sanitize";

  let { doc }: { doc: Documentation | null } = $props();
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

<aside class="docs" aria-label="説明" data-docs>
  <h2>説明</h2>
  {#if doc}<p class="cls"><code>{doc.class}</code></p>{/if}
  {#if info.length}
    <div class="body" data-docs-info>{@render nodes(info)}</div>
  {:else}
    <p class="empty" data-docs-empty>Documentation が無い</p>
  {/if}
  {#if revisions.length}<details><summary>改訂履歴</summary><div class="body" data-docs-revisions>{@render nodes(revisions)}</div></details>{/if}
</aside>

<style>
  .docs { flex: 0 0 16rem; height: 16rem; overflow: auto; border: 1px solid var(--border); padding: 0 .5rem; font-size: .9em; overflow-wrap: anywhere; }
  h2 { font-size: 1em; margin: .25rem 0; }
  .cls, .empty { margin: .25rem 0; color: var(--muted); }
  .body :global(a) { color: var(--accent); }
  .body :global(pre) { white-space: pre-wrap; background: var(--hover); padding: .25rem; }
  .body :global(table) { border-collapse: collapse; }
  .body :global(td), .body :global(th) { border: 1px solid var(--line); padding: .1rem .3rem; }
  @media (max-width: 600px) {
    .docs { flex: none; height: 10rem; }
  }
</style>
