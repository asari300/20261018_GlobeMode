<script lang="ts">
  import type { Backend, TreeEntry } from "./backend";
  import { getContext, setContext } from "svelte";
  import LibraryTree from "./LibraryTree.svelte";
  import TreeIcon from "./TreeIcon.svelte";
  import { treeIconLoader } from "./tree-icons";

  // ライブラリの木（S127）。展開したときに直下の子だけを引く。子の無いクラス（と、子を持つ package 以外のクラス）を押すと onOpen。
  // 最上位（prefix が空）には検索欄（S167）。一致の計算は Rust（backend.librarySearch）で、ここは結果の選択だけ
  let { backend, prefix = "", onOpen }: { backend: Backend; prefix?: string; onOpen: (e: TreeEntry) => void } = $props();
  let open = $state<Record<string, boolean>>({});
  // アイコンは最上位の木が持つ loader（memo と同時 4 本）を、入れ子の木が共有する（S257）
  const icons = getContext<ReturnType<typeof treeIconLoader> | undefined>("tree-icons") ?? treeIconLoader({ icon: (c) => backend.icon(c) });
  setContext("tree-icons", icons);
  const entries = $derived(backend.libraryTree(prefix));

  let query = $state("");
  let hits = $state<TreeEntry[]>([]);
  let active = $state(0);
  let asked = 0; // 遅れて返った古い問い合わせを捨てる
  async function search() {
    const n = ++asked;
    const r = query.trim() ? await backend.librarySearch(query).catch(() => []) : [];
    if (n === asked) { hits = r; active = 0; }
  }
  function pick(e: TreeEntry) {
    query = ""; hits = []; asked++;
    onOpen(e);
  }
  function keydown(ev: KeyboardEvent) {
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      if (!hits.length) return;
      ev.preventDefault();
      active = (active + (ev.key === "ArrowDown" ? 1 : hits.length - 1)) % hits.length;
    } else if (ev.key === "Enter" && hits[active]) {
      ev.preventDefault();
      pick(hits[active]);
    } else if (ev.key === "Escape") { query = ""; hits = []; asked++; }
  }
</script>

{#if !prefix}
  <div class="search">
    <input type="search" role="combobox" aria-label="ライブラリを検索" aria-expanded={hits.length > 0} aria-controls="library-hits"
      aria-activedescendant={hits.length ? `library-hit-${active}` : undefined} placeholder="検索（例 PID_Con）"
      bind:value={query} oninput={search} onkeydown={keydown} />
    {#if hits.length}
      <ul id="library-hits" role="listbox" aria-label="検索結果">
        {#each hits as h, i (h.path)}
          <li id="library-hit-{i}" role="option" aria-selected={i === active} class:active={i === active} title={h.description}>
            <button tabindex="-1" onclick={() => pick(h)}>{h.path} <span class="kind">{h.kind}</span></button>
          </li>
        {/each}
      </ul>
    {:else if query.trim()}<p class="kind" role="status">一致するクラスは無い</p>{/if}
  </div>
{/if}

{#await entries then list}
  <ul class="tree" aria-label={prefix ? undefined : "ライブラリ"}>
    {#each list as e (e.path)}
      <li data-path={e.path}>
        {#if e.hasChildren}
          <button aria-expanded={!!open[e.path]} title={e.description} onclick={() => (open[e.path] = !open[e.path])}>
            <span aria-hidden="true">{open[e.path] ? "▾" : "▸"}</span> <TreeIcon load={() => icons(e.path)} />{e.name}
          </button>
          {#if e.kind !== "package"}<button title={e.description} onclick={() => onOpen(e)}>開く <span class="kind">{e.kind}</span></button>{/if}
          {#if open[e.path]}<LibraryTree {backend} prefix={e.path} {onOpen} />{/if}
        {:else}
          <button title={e.description} onclick={() => onOpen(e)}><TreeIcon load={() => icons(e.path)} />{e.name} <span class="kind">{e.kind}</span></button>
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
  .search { position: sticky; top: 0; background: var(--bg); padding: .2rem; }
  .search input { width: 100%; box-sizing: border-box; font: inherit; }
  .search ul { list-style: none; margin: 0; padding: 0; }
  .search li.active { background: var(--hover); }
</style>
