<!-- S291: 変換の段（flat → DAE → インデックスの低減 → BLT → tearing）の要約。BltPanel と同じ流儀。DAE の増減と微分した方程式・増えた未知数は名前の差分で出す -->
<script lang="ts">
  import type { Backend } from "./backend";
  import type { StagesReport } from "./protocol";

  let { backend, model }: { backend: Backend; model: string } = $props();
  let report = $state<StagesReport | null>(null);
  let failure = $state("");
  const names = (v: string[]) => (v.length ? v.join(", ") : "なし");
  const delta = (n: number) => (n > 0 ? `+${n}` : String(n));

  $effect(() => {
    const m = model;
    let cancelled = false;
    report = null;
    failure = "";
    backend.stages(m).then((r) => { if (!cancelled) report = r; }, (e) => { if (!cancelled) failure = String(e?.message ?? e); });
    return () => { cancelled = true; };
  });
</script>

<section class="stages" aria-label="変換の段" data-stages>
  <h3>変換の段</h3>
  <div aria-live="polite">
    {#if failure}<p class="failure" role="alert" data-stages-failure>{failure}</p>
    {:else if !report}<p class="note">解析している…</p>
    {:else}
      {@const r = report}
      <ol>
        {#if r.flat}
          <li data-stage="flat"><strong>flat</strong>: 変数 {r.flat.variables}・方程式 {r.flat.equations}・初期方程式 {r.flat.initial_equations}</li>
        {/if}
        {#if r.dae}
          <li data-stage="dae"><strong>DAE</strong>: 状態 {r.dae.states}・代数 {r.dae.algebraics}・出力 {r.dae.outputs}・入力 {r.dae.inputs}・パラメータ {r.dae.parameters}（変数 {r.dae.variables}、flat から {delta(r.dae.variables_delta)}）。
            方程式 {r.dae.equations}（flat から {delta(r.dae.equations_delta)}）・未知数 {r.dae.unknowns}・der {r.dae.derivatives}</li>
        {/if}
        <li data-stage="index" data-index={r.index.index}>
          <strong>インデックスの低減</strong>:
          {#if r.index.failed}打ち切った（構造特異など）
          {:else}
            インデックス {r.index.index}・微分した方程式 {r.index.differentiated.length} 本（方程式 +{r.index.added_equations}・未知数 +{r.index.added_unknowns.length}）・動的な状態 {r.index.dynamic_states}
            {#if r.index.added_equations > 0}
              <ul>
                <li>微分した方程式: <code>{names(r.index.differentiated)}</code></li>
                <li>増えた未知数: <code>{names(r.index.added_unknowns)}</code></li>
                <li>状態選択の候補: <code>{names(r.index.dummy_candidates)}</code></li>
              </ul>
            {/if}
          {/if}
        </li>
        <li data-stage="blt">
          <strong>BLT</strong>:
          {#if r.blt.skipped}作らなかった: {r.blt.skipped}
          {:else}ブロック {r.blt.blocks}（大きさ×個数: {r.blt.size_histogram.map(([s, n]) => `${s}×${n}`).join(" ") || "なし"}）・代数ループ {r.blt.algebraic_loops}{r.blt.reduced ? "（インデックス 2 以上: 見込み）" : ""}{/if}
        </li>
        {#if !r.blt.skipped}
          <li data-stage="tearing">
            <strong>tearing</strong>:
            {#if !r.tearing.loops.length}ループなし
            {:else}
              ループ {r.tearing.loops.length}・非線形の反復変数 {r.tearing.iteration_variables}
              <ul>
                {#each r.tearing.loops as l, i}
                  <li>ループ {i + 1}: 大きさ {l.size}・{l.nonlinear ? "非線形" : "線形"}・反復変数 <code>{names(l.variables)}</code>・残差 <code>{names(l.residuals)}</code></li>
                {/each}
              </ul>
            {/if}
          </li>
        {/if}
      </ol>
    {/if}
  </div>
</section>

<style>
  .stages { margin-top: .5rem; }
  .note { color: var(--muted); }
  .failure { color: var(--err); white-space: pre-wrap; }
  ol { padding-left: 2rem; max-height: 24rem; overflow-y: auto; }
  li { overflow-wrap: anywhere; margin: .2rem 0; }
</style>
