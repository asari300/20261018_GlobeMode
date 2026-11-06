<script lang="ts">
  import { untrack } from "svelte";
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";
  import { toCsv, type Columns } from "./csv";
  import { exportName, isInternal, matches, plotLabel } from "./signals";
  import type { Backend } from "../backend";
  import { TauriBackend, inTauri } from "../tauri-backend";
  import { cssVar, theme } from "../../theme.svelte";
  import { browserSave, browserSaveBinary } from "../browser-file";

  let { columns, model = "" }: { columns: Columns; model?: string } = $props();

  const COLORS = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e", "#17becf", "#8c564b", "#e377c2"];
  const DEFAULT_SHOWN = 3;
  const LIST_MAX = 200; // 一覧に並べる最大（信号が千本あっても入力が引っかからない。残りは絞り込みで出す。仮想スクロールは入れない）

  let host: HTMLDivElement;
  let plot: uPlot | undefined;
  // 選択・右の軸・x は名前で持つ。新しい結果が来たら既定から決め直すが、同じモデルの再実行で利用者が選び直していたときは、その選択を保つ（S201。下の $effect.pre）
  // shortcut: 保つのは Plot が載っている間だけ（タブの切り替え・読み込み・失敗で結果が消えると既定に戻る）。タブごとの記憶や再読み込みをまたぐ保存は持ち越し
  const defaultShown = (c: Columns) => {
    const d = c.defaults; // 既定は Rust の選ぶ信号（状態、足りなければ出力）。応答に無ければ内部名を除く先頭の 3 本
    let n = 0;
    return c.names.map((name) => (d ? d.includes(name) : !isInternal(name) && n++ < DEFAULT_SHOWN));
  };
  let shown = $state.raw<boolean[]>([]);
  let right = $state.raw<boolean[]>([]);
  let xi = $state(-1); // -1 は time、それ以外は names の添字（パラメトリック）
  let touched = false; // 利用者が既定から選び直したか
  let prev = { model: "", names: [] as string[] }; // 前の結果
  let query = $state("");
  let internal = $state(false); // 内部の名前（$ で始まるもの）も一覧と書き出しに含める
  let lastMs = $state(0);
  let error = $state("");
  let saved = $state("");
  // 一覧に出す信号の添字（絞り込みと内部名）。選択（shown）はこれと独立で、絞り込みで一覧から消えても残る
  let matched = $derived(columns.names.flatMap((n, i) => ((internal || !isInternal(n)) && matches(n, query) ? [i] : [])));
  let listed = $derived(matched.slice(0, LIST_MAX));
  // プロットと CSV に使う系列。内部名は、チェックを外していれば選択されていても出さない
  let ys = $derived(columns.names.flatMap((n, i) => (shown[i] && (internal || !isInternal(n)) ? [i] : [])));
  const setListed = (on: boolean) => ((touched = true), (shown = shown.map((v, i) => (listed.includes(i) ? on : v))));
  const resetPicks = () => {
    touched = false;
    query = "";
    internal = false;
    shown = defaultShown(columns);
    right = columns.names.map(() => false);
    xi = -1;
  };
  $effect.pre(() => {
    const cols = columns;
    untrack(() => {
      const names = (flags: boolean[]) => new Set(prev.names.filter((_, i) => flags[i]));
      const [mine, side, x] = [names(shown), names(right), xi < 0 ? "" : prev.names[xi]];
      const same = model === prev.model;
      // 同じモデルで選び直していたら、同じ名前の信号を選ぶ。無くなった信号は外し、何も残らなければ既定に戻す（何も選んでいなかったなら、そのまま）
      const hit = touched && same && cols.names.map((n) => mine.has(n));
      if (hit && (mine.size === 0 || hit.some(Boolean))) {
        shown = hit;
        right = cols.names.map((n) => side.has(n));
        xi = cols.names.indexOf(x);
      } else {
        if (!same || touched) (query = ""), (internal = false); // 別のモデル、または保てるものが何も残らなかった
        touched = false;
        shown = defaultShown(cols);
        right = cols.names.map(() => false);
        xi = -1;
      }
      prev = { model, names: cols.names };
    });
  });

  // Plot の中で完結させる（App から backend を受けない）。Desktop は dialog / fs、ブラウザ・Android はダウンロード
  const out: Pick<Backend, "saveFile" | "saveBinary"> = inTauri() ? new TauriBackend() : { saveFile: browserSave, saveBinary: browserSaveBinary };

  const done = (p: string | null) => void (p && (saved = p));
  const fail = (e: unknown) => (error = String((e as Error).message ?? e));
  // 選んだ系列と x の列（画面と同じ）。欠損は空欄
  function exportCsv() {
    const head = [xi < 0 ? "time" : columns.names[xi], ...ys.map((i) => columns.names[i])];
    out.saveFile(exportName(model, "csv"), toCsv(head, [columns.data[xi + 1], ...ys.map((i) => columns.data[i + 1])]), true).then(done, fail);
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
      try { done(await out.saveBinary!(exportName(model, "png"), new Uint8Array(await b!.arrayBuffer()))); } catch (e) { fail(e); }
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
      // S175: 読み上げ用の代替テキスト。model は読むだけ（入力のたびに作り直さない）
      const alt = plot.ctx.canvas;
      alt.setAttribute("role", "img");
      alt.setAttribute("aria-label", plotLabel(untrack(() => model), xi < 0 ? "time" : cols.names[xi], xs, ys.map((i) => ({ name: cols.names[i], data: cols.data[i + 1] }))));
      queueMicrotask(() => (lastMs = performance.now() - t0)); // uPlot の最初の描画は microtask なので、その後で測る
    } catch (e) {
      error = String((e as Error).message ?? e);
    }
    return () => (plot?.destroy(), (plot = undefined)); // 作り直しが投げても古い図を残さない（PNG・二重の destroy）
  });
</script>

<div class="plot" data-points={columns.points} data-ms={lastMs.toFixed(1)} data-typed={columns.data[0] instanceof Float64Array}
  data-x={xi < 0 ? "time" : columns.names[xi]} data-series={ys.length} data-right={ys.filter((i) => right[i]).length} data-listed={listed.length} data-matched={matched.length}>
  {#if error}<p class="failure" role="alert">{error}</p>{/if}
  <label>x 軸
    <select value={xi} onchange={(e) => ((touched = true), (xi = Number(e.currentTarget.value)))}>
      <option value={-1}>time</option>
      {#each columns.names as n, i}{#if internal || !isInternal(n) || i === xi}<option value={i}>{n}</option>{/if}{/each}
    </select>
  </label>
  <div class="filter">
    <input type="search" aria-label="信号を絞り込む" placeholder="信号を絞り込む（空白区切りで AND）" bind:value={query}>
    <label><input type="checkbox" name="internal" bind:checked={internal}>内部の変数も表示</label>
    <button onclick={() => setListed(true)}>表示中をすべて選ぶ</button>
    <button onclick={() => setListed(false)}>表示中をすべて外す</button>
    <button onclick={resetPicks}>既定に戻す</button>
    <span class="count">選択 {ys.length} 本 / 一致 {matched.length} 本</span>
  </div>
  <div class="signals" aria-label="信号">
    {#each listed as i (i)}
      {@const n = columns.names[i]}
      <span>
        <label><input type="checkbox" name="signal" checked={shown[i]} onchange={(e) => ((touched = true), (shown = shown.map((v, k) => (k === i ? e.currentTarget.checked : v))))}>{n}</label>
        {#if shown[i]}<label class="side"><input type="checkbox" aria-label="{n} を右の軸に" checked={right[i]} onchange={(e) => ((touched = true), (right = right.map((v, k) => (k === i ? e.currentTarget.checked : v))))}>右</label>{/if}
      </span>
    {/each}
    {#if matched.length > listed.length}<span class="more">ほか {matched.length - listed.length} 件（絞り込むと出る）</span>{/if}
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
  .filter { display: flex; flex-wrap: wrap; gap: .25rem .75rem; align-items: center; margin-top: .5rem; }
  .filter input[type=search] { flex: 1 1 14rem; }
  .count, .more { color: var(--muted); font-size: .85em; }
  .export { display: flex; gap: .5rem; align-items: center; margin-bottom: .5rem; }
  .side { font-size: .85em; opacity: .8; }
  .failure { color: var(--err); white-space: pre-wrap; }
</style>
