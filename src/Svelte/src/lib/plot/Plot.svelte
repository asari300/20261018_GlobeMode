<script lang="ts">
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";
  import { toCsv, type Columns } from "./csv";
  import type { Backend } from "../backend";
  import { TauriBackend, inTauri } from "../tauri-backend";
  import { cssVar, theme } from "../../theme.svelte";
  import { browserSave, browserSaveBinary } from "../browser-file";

  let { columns }: { columns: Columns } = $props();

  const COLORS = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e", "#17becf", "#8c564b", "#e377c2"];
  const DEFAULT_SHOWN = 3;

  let host: HTMLDivElement;
  let plot: uPlot | undefined;
  // 新しい結果が来たら選択・軸・x を既定に戻す（書き換えは次の結果まで残る）
  let shown = $derived(columns.names.map((_, i) => i < DEFAULT_SHOWN));
  let right = $derived(columns.names.map(() => false));
  let xi = $derived.by(() => (void columns, -1)); // -1 は time、それ以外は names の添字（パラメトリック）
  let lastMs = $state(0);
  let error = $state("");
  let saved = $state("");
  // Plot の中で完結させる（App から backend を受けない）。Desktop は dialog / fs、ブラウザ・Android はダウンロード
  const out: Pick<Backend, "saveFile" | "saveBinary"> = inTauri() ? new TauriBackend() : { saveFile: browserSave, saveBinary: browserSaveBinary };

  const done = (p: string | null) => void (p && (saved = p));
  const fail = (e: unknown) => (error = String((e as Error).message ?? e));
  // 選んだ系列と x の列（画面と同じ）。欠損は空欄
  function exportCsv() {
    const ys = columns.names.flatMap((_, i) => (shown[i] ? [i] : []));
    const head = [xi < 0 ? "time" : columns.names[xi], ...ys.map((i) => columns.names[i])];
    out.saveFile("result.csv", toCsv(head, [columns.data[xi + 1], ...ys.map((i) => columns.data[i + 1])]), true).then(done, fail);
  }
  // shortcut: canvas だけ（凡例と DOM の軸ラベルは入らない）。背景色で塗るのは、透明のままだと線が見えないため（明は白のまま。暗は暗い地に明るい軸の文字で書き出す）
  function exportPng() {
    if (!plot) return;
    const src = plot.ctx.canvas;
    const c = Object.assign(document.createElement("canvas"), { width: src.width, height: src.height });
    const g = c.getContext("2d")!;
    g.fillStyle = cssVar("--bg");
    g.fillRect(0, 0, c.width, c.height);
    g.drawImage(src, 0, 0);
    c.toBlob(async (b) => {
      try { done(await out.saveBinary!("result.png", new Uint8Array(await b!.arrayBuffer()))); } catch (e) { fail(e); }
    }, "image/png");
  }

  // 選んだ系列だけを uPlot に渡す（列が数百あっても重くしない）。選択が変わるたびに作り直す
  $effect(() => {
    error = "";
    void theme.dark; // 明暗が変わったら軸の色を渡して作り直す（S152）
    try {
      const ax = { stroke: cssVar("--fg"), grid: { stroke: cssVar("--line"), width: 1 }, ticks: { stroke: cssVar("--border"), width: 1 } };
      const t0 = performance.now();
      const cols = columns;
      const ys = cols.names.flatMap((_, i) => (shown[i] ? [i] : []));
      const scale = (i: number) => (right[i] ? "y2" : "y");
      const used = new Set(ys.map(scale));
      const xs = cols.data[xi + 1];
      let xr: [number, number] | null = null;
      if (xi >= 0) {
        // 時刻以外の x は単調でないので、uPlot が両端の値から取る範囲の代わりに最小・最大を渡す
        let lo = Infinity, hi = -Infinity;
        for (let k = 0; k < xs.length; k++) {
          const v = xs[k];
          if (v != null) (lo = Math.min(lo, v)), (hi = Math.max(hi, v));
        }
        if (lo <= hi) xr = [lo, hi > lo ? hi : lo + 1];
      }
      plot = new uPlot(
        {
          width: host.clientWidth || 800,
          height: 320,
          scales: { x: xr ? { time: false, range: () => xr } : { time: false } },
          // shortcut: パラメトリックでは拡大を止める（uPlot の拡大は x が昇順の前提）。要るなら mode 2 に移す
          cursor: xr ? { drag: { x: false, y: false } } : {},
          axes: [
            { ...ax, label: xi < 0 ? "time" : cols.names[xi] },
            ...(used.has("y") ? [{ ...ax, scale: "y" }] : []),
            ...(used.has("y2") ? [{ ...ax, scale: "y2", side: 1, grid: { show: false } }] : []),
          ],
          series: [
            { label: xi < 0 ? "time" : cols.names[xi] },
            ...ys.map((i) => ({ label: cols.names[i], stroke: COLORS[i % COLORS.length], scale: scale(i) })),
          ],
        },
        [xs, ...ys.map((i) => cols.data[i + 1])] as uPlot.AlignedData,
        host,
      );
      queueMicrotask(() => (lastMs = performance.now() - t0)); // uPlot の最初の描画は microtask なので、その後で測る
    } catch (e) {
      error = String((e as Error).message ?? e);
    }
    return () => plot?.destroy();
  });
</script>

<div class="plot" data-points={columns.points} data-ms={lastMs.toFixed(1)} data-typed={columns.data[0] instanceof Float64Array}
  data-x={xi < 0 ? "time" : columns.names[xi]} data-series={shown.filter(Boolean).length} data-right={shown.filter((s, i) => s && right[i]).length}>
  {#if error}<p class="failure" role="alert">{error}</p>{/if}
  <label>x 軸
    <select value={xi} onchange={(e) => (xi = Number(e.currentTarget.value))}>
      <option value={-1}>time</option>
      {#each columns.names as n, i}<option value={i}>{n}</option>{/each}
    </select>
  </label>
  <div class="signals" aria-label="信号">
    {#each columns.names as n, i}
      <span>
        <label><input type="checkbox" name="signal" checked={shown[i]} onchange={(e) => (shown = shown.map((v, k) => (k === i ? e.currentTarget.checked : v)))}>{n}</label>
        {#if shown[i]}<label class="side"><input type="checkbox" aria-label="{n} を右の軸に" checked={right[i]} onchange={(e) => (right = right.map((v, k) => (k === i ? e.currentTarget.checked : v)))}>右</label>{/if}
      </span>
    {/each}
  </div>
  <div class="export">
    <button onclick={exportCsv}>CSV</button>
    <button onclick={exportPng}>PNG</button>
    {#if saved}<span data-exported>{saved}</span>{/if}
  </div>
  <div bind:this={host}></div>
</div>

<style>
  .signals { display: flex; flex-wrap: wrap; gap: .25rem 1rem; margin: .5rem 0; max-height: 6rem; overflow: auto; }
  .export { display: flex; gap: .5rem; align-items: center; margin-bottom: .5rem; }
  .side { font-size: .85em; opacity: .8; }
  .failure { color: var(--err); white-space: pre-wrap; }
</style>
