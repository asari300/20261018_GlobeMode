<script lang="ts">
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";
  import { parseCsv } from "./csv";

  let { csv }: { csv: string } = $props();

  const COLORS = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e", "#17becf", "#8c564b", "#e377c2"];
  const DEFAULT_SHOWN = 3;

  let host: HTMLDivElement;
  let plot: uPlot | undefined;
  let shown = $state<boolean[]>([]);
  let count = $state(0);
  let names = $state<string[]>([]);
  let lastMs = $state(0);
  let parseMs = $state(0);
  let error = $state("");

  // 新しい結果が来たらグラフを作り直す。選択は最初の数本が既定
  $effect(() => {
    error = "";
    try {
      const t0 = performance.now();
      const cols = parseCsv(csv);
      parseMs = performance.now() - t0;
      names = cols.names;
      shown = cols.names.map((_, i) => i < DEFAULT_SHOWN);
      count = cols.data[0].length;
      plot = new uPlot(
        {
          width: host.clientWidth || 800,
          height: 320,
          scales: { x: { time: false } },
          series: [
            { label: "time" },
            ...cols.names.map((label, i) => ({ label, stroke: COLORS[i % COLORS.length], show: i < DEFAULT_SHOWN })),
          ],
        },
        cols.data as uPlot.AlignedData,
        host,
      );
      queueMicrotask(() => (lastMs = performance.now() - t0)); // uPlot の最初の描画は microtask なので、その後で測る
    } catch (e) {
      error = String((e as Error).message ?? e);
    }
    return () => plot?.destroy();
  });

  function toggle(i: number) {
    plot?.setSeries(i + 1, { show: shown[i] });
  }
</script>

<div class="plot" data-points={count} data-ms={lastMs.toFixed(1)} data-parse-ms={parseMs.toFixed(1)}>
  {#if error}<p class="failure" role="alert">{error}</p>{/if}
  <div class="signals" aria-label="信号">
    {#each names as n, i}
      <label><input type="checkbox" bind:checked={shown[i]} onchange={() => toggle(i)}>{n}</label>
    {/each}
  </div>
  <div bind:this={host}></div>
</div>

<style>
  .signals { display: flex; flex-wrap: wrap; gap: .25rem 1rem; margin: .5rem 0; max-height: 6rem; overflow: auto; }
  .failure { color: #b00020; white-space: pre-wrap; }
</style>
