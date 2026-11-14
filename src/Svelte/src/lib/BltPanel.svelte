<!-- S262: BLT（方程式のブロック下三角分解）のパネル。ブロックを解く順に並べ、大きさ 2 以上（代数ループ）は色と見出しで目立たせる -->
<script lang="ts">
  import type { Backend } from "./backend";
  import type { BltReport } from "./protocol";

  let { backend, model }: { backend: Backend; model: string } = $props();
  let report = $state<BltReport | null>(null);
  let failure = $state("");
  const loops = $derived(report?.blocks.filter((b) => b.algebraic_loop).length ?? 0);

  $effect(() => {
    const m = model;
    let cancelled = false;
    report = null;
    failure = "";
    backend.blt(m).then((r) => { if (!cancelled) report = r; }, (e) => { if (!cancelled) failure = String(e?.message ?? e); });
    return () => { cancelled = true; };
  });
</script>

<section class="blt" aria-label="BLT" data-blt>
  <h3>BLT（解く順）</h3>
  {#if failure}<p class="failure" role="alert" data-blt-failure>{failure}</p>
  {:else if !report}<p class="note">解析している…</p>
  {:else if report.skipped}<p class="note" data-blt-skipped>BLT を作らなかった: {report.skipped}</p>
  {:else}
    <p class="note" data-blt-summary>{report.blocks.length} ブロック・代数ループ {loops}{report.reduced ? "（インデックス 2 以上: 最高階の系・状態選択の前の見込み）" : ""}</p>
    <ol>
      {#each report.blocks as b}
        <li class:loop={b.algebraic_loop} data-blt-block data-size={b.size}>
          {#if b.algebraic_loop}
            <strong>代数ループ（大きさ {b.size}・{b.nonlinear ? "非線形" : "線形"}）</strong>
            <div>解く変数: <code>{b.variables.join(", ")}</code></div>
            <ul>{#each b.equations as e}<li><code>{e}</code></li>{/each}</ul>
          {:else}
            <code>{b.variables[0]}</code> ← <code>{b.equations[0]}</code>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .blt { margin-top: .5rem; }
  .note { color: var(--muted); }
  .failure { color: var(--err); white-space: pre-wrap; }
  ol { padding-left: 2rem; max-height: 20rem; overflow-y: auto; }
  li { overflow-wrap: anywhere; }
  li.loop { color: var(--warn); border-left: 3px solid var(--warn); padding-left: .5rem; margin: .25rem 0; }
  li.loop ul { margin: .1rem 0; }
</style>
