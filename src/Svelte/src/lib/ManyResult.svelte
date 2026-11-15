<script lang="ts">
  import type { SimulateManyResponse } from "./protocol";
  import { pairSummary, relText } from "./many";

  // 複数のソルバで回した結果の表（S323）。合否は色でなく文字で出す。重ね描きは Plot が受け持つ
  let { result }: { result: SimulateManyResponse } = $props();
  const STATUS = { ok: "成功", failed: "失敗", cancelled: "中断" } as const;
  const num = (v: number) => v.toPrecision(3);
  const why = (r: SimulateManyResponse["runs"][number]) => r.response.refused ?? r.response.diagnostics.find((d) => d.severity === "error")?.message.split("\n")[0] ?? "";
  const c = $derived(result.comparison);
</script>

<section class="many" data-many aria-label="複数のソルバの結果">
  <p data-many-mode>
    実行の仕方: {result.mode === "parallel" ? `並列（${result.workers} 本）` : "1 つずつ"}、{(result.elapsed_ms / 1000).toFixed(2)} 秒{#if result.mode_note}。<span data-mode-note>{result.mode_note}</span>{/if}
  </p>
  <table aria-label="ソルバごとの実行">
    <caption>ソルバごとの実行</caption>
    <thead><tr><th scope="col">ソルバ</th><th scope="col">状態</th><th scope="col">時間（ms）</th><th scope="col">注記</th></tr></thead>
    <tbody>
      {#each result.runs as r}
        <tr data-run-solver={r.solver} data-run-status={r.status}>
          <th scope="row">{r.solver}</th><td>{STATUS[r.status]}</td><td>{r.elapsed_ms.toFixed(1)}</td>
          <td>{#if r.status !== "ok" && why(r)}{why(r)}{/if}{#each r.notes as n}<div><small>{n}</small></div>{/each}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if c}
    <p data-compare-verdict role="status">
      比較: 閾値 10<sup>-{c.threshold_exp}</sup>（{c.threshold}）、基準 {c.reference}、全体で <strong>{c.passed ? "合格" : "不合格"}</strong>
    </p>
    {#if c.worst}
      <p data-compare-worst>全変数の最大の相対差: {relText(c.max_rel, c.threshold_exp)}（{c.worst.solver}、{c.worst.name}{#if c.worst.time != null}、t = {num(c.worst.time)}{/if}）</p>
    {/if}
    <table aria-label="基準との比較">
      <caption>基準 {c.reference} との比較</caption>
      <thead><tr><th scope="col">ソルバ</th><th scope="col">合否</th><th scope="col">外れた信号</th><th scope="col">最大のはみ出し量</th><th scope="col">最初に外れた時刻</th><th scope="col">最大の相対差</th></tr></thead>
      <tbody>
        {#each c.pairs as p}
          {@const s = pairSummary(p)}
          <tr data-pair={p.solver} data-passed={p.passed}>
            <th scope="row">{p.solver}</th><td>{p.passed ? "合格" : "不合格"}</td>
            <td>{p.failed_signals} / {p.signals}{#if p.failures.length}（{p.failures.slice(0, 3).map((f) => f.name).join("、")}{p.failures.length > 3 ? " ほか" : ""}）{/if}</td>
            <td>{s ? num(s.excess) : "-"}</td><td>{s && Number.isFinite(s.first) ? num(s.first) : "-"}</td>
            <td data-max-rel>{relText(p.max_rel, c.threshold_exp)}{#if p.worst}<small>（{p.worst.name}{#if p.worst.time != null}、t = {num(p.worst.time)}{/if}）</small>{/if}</td>
          </tr>
        {/each}
        {#each c.skipped as k}<tr data-skipped={k.solver}><th scope="row">{k.solver}</th><td colspan="5">比べなかった: {k.reason}</td></tr>{/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .many { margin: .5rem 0; }
  table { border-collapse: collapse; margin: .25rem 0; }
  th, td { border: 1px solid var(--border); padding: .15rem .5rem; text-align: left; }
  caption { text-align: left; color: var(--muted); }
  small { color: var(--muted); }
</style>
