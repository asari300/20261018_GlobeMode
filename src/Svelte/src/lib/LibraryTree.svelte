<script lang="ts">
  import type { Backend, TreeEntry } from "./backend";
  import LibraryTree from "./LibraryTree.svelte";

  // ライブラリの木（S127）。展開したときに直下の子だけを引く。ファイル単位の子の無いクラスを押すと onOpen
  let { backend, prefix = "", onOpen }: { backend: Backend; prefix?: string; onOpen: (e: TreeEntry) => void } = $props();
  let open = $state<Record<string, boolean>>({});
  const entries = $derived(backend.libraryTree(prefix));
</script>

{#await entries then list}
  <ul class="tree" aria-label={prefix ? undefined : "ライブラリ"}>
    {#each list as e (e.path)}
      <li data-path={e.path}>
        {#if e.hasChildren}
          <button aria-expanded={!!open[e.path]} title={e.description} onclick={() => (open[e.path] = !open[e.path])}>
            <span aria-hidden="true">{open[e.path] ? "▾" : "▸"}</span> {e.name}
          </button>
          {#if open[e.path]}<LibraryTree {backend} prefix={e.path} {onOpen} />{/if}
        {:else}
          <button title={e.description} onclick={() => onOpen(e)}>{e.name} <span class="kind">{e.kind}</span></button>
        {/if}
      </li>
    {/each}
  </ul>
{:catch err}
  <p class="failure">{String(err?.message ?? err)}</p>
{/await}

<style>
  .tree { list-style: none; margin: 0; padding-left: .8rem; }
  button { border: 0; background: none; color: inherit; padding: .05rem .2rem; cursor: pointer; text-align: left; font: inherit; }
  button:hover { background: var(--hover); }
  .kind { color: var(--muted); font-size: .85em; }
  .failure { color: var(--err); }
</style>
