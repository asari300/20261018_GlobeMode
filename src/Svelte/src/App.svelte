<script lang="ts">
  import { onMount } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, drawSelection } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { bracketMatching } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import { modelicaCompletion, modelicaLinter, toCmDiagnostics } from "./lib/editor";
  import { WorkerBackend } from "./lib/worker-backend";
  import { TauriBackend, inTauri } from "./lib/tauri-backend";
  import type { Backend } from "./lib/backend";
  import type { Columns } from "./lib/plot/csv";
  import type { Diagnostic } from "./lib/protocol";
  import Plot from "./lib/plot/Plot.svelte";
  import Graph from "./lib/graph/Graph.svelte";

  const FILE = "Model.mo";
  const desktop = inTauri();
  const tauri = desktop ? new TauriBackend() : null;
  const backend: Backend = tauri ?? new WorkerBackend(); // S049: Tauri の中では sidecar
  const severityRank = { error: 0, warning: 1, info: 2 };

  let host: HTMLDivElement;
  let view: EditorView;
  let model = $state("Decay");
  let busy = $state(false);
  let failure = $state("");
  let diagnostics = $state<Diagnostic[]>([]);
  let columns = $state.raw<Columns | null>(null); // $state だと配列を丸ごと Proxy にする。10 万点では遅い
  // MSL（public/msl/MSL.json。pack-msl.mjs が data/msl から作る）。起動時には読まず、ボタンで読む
  let msl = $state<"none" | "loading" | "loaded">("none");
  let mslNote = $state("");
  let graphModel = $state<string | null>(null); // 「グラフ」を押したときのモデル名（入力のたびに描き直さない）

  onMount(() => {
    // S069: 取得済みの MSL を sidecar が読んでいれば、最初から「読んだ」状態にする
    void tauri?.hasLibrary().then((has) => {
      if (has && msl === "none") { msl = "loaded"; mslNote = "MSL を読んだ（取得済み）"; }
    });
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n",
        extensions: [
          lineNumbers(), history(), drawSelection(), bracketMatching(), keymap.of([...defaultKeymap, ...historyKeymap]),
          modelicaCompletion(), modelicaLinter(backend, FILE), // S053
        ],
      }),
    });
    return () => view.destroy();
  });

  async function loadMsl() {
    msl = "loading";
    failure = "";
    const t = performance.now();
    try {
      let json = "";
      if (!desktop) { // Desktop は JSON を使わない（backend.library が MSL を取得して sidecar に渡す）
        const res = await fetch(`${import.meta.env.BASE_URL}msl/MSL.json`);
        if (!res.ok) throw new Error(`MSL.json を取得できない（${res.status}）。data/msl を取得して pnpm prepare-wasm（cargo xtask fetch-msl）`);
        json = await res.text();
      }
      await backend.library("MSL", json);
      msl = "loaded";
      const size = desktop ? "" : `${(json.length / 2 ** 20).toFixed(0)} MiB、`;
      mslNote = `MSL 4.1.0 を読んだ（${size}${((performance.now() - t) / 1000).toFixed(1)} 秒）`;
      diagnostics = [];
      columns = null;
      graphModel = null;
    } catch (e) {
      msl = "none";
      failure = String((e as Error).message ?? e);
    }
  }

  const bySeverity = (ds: Diagnostic[]) =>
    // S0…S10 の順、同じ段階では error > warning > info
    [...ds].sort((a, b) => Number(a.stage.slice(1)) - Number(b.stage.slice(1)) || severityRank[a.severity] - severityRank[b.severity]);

  async function run(job: () => Promise<void>) {
    busy = true;
    failure = "";
    try {
      await backend.setFile(FILE, view.state.doc.toString());
      await job();
    } catch (e) {
      failure = String((e as Error).message ?? e);
    } finally {
      busy = false;
    }
  }

  const check = () => run(async () => {
    diagnostics = bySeverity((await backend.check(model)).diagnostics);
    view.dispatch(setDiagnostics(view.state, toCmDiagnostics(view.state.doc.toString(), diagnostics, FILE)));
  });

  // 整形（S053、doc/format-style.md）。文法が壊れていれば本文は変えず、理由を出す
  const format = () => run(async () => {
    const text = view.state.doc.toString();
    const out = await backend.format(text, 2, true);
    if (out !== text) view.dispatch({ changes: { from: 0, to: text.length, insert: out } });
  });

  const simulate = () =>
    run(async () => {
      const { response: r, columns: c } = await backend.simulateColumns({ model });
      diagnostics = bySeverity(r.diagnostics);
      columns = c.points > 0 ? c : null;
      if (r.refused) failure = r.refused;
      else if (!r.ok && !r.diagnostics.length) failure = "実行に失敗した（診断なし）";
    });

  const graph = () => run(async () => {
    graphModel = model;
  });

  // ノードのキー（`stage1.resistor` など）の先頭の名前を、エディタの本文で最初に出る所で選ぶ
  // shortcut: 宣言を名前の字面で探す。同じ名前が先に出ると外れる。outline の位置が要るなら LSP の documentSymbol を使う
  function select(node: string) {
    const name = node.split(".")[0];
    const m = new RegExp(`\\b${name.replace(/[^\w]/g, "")}\\b`).exec(view.state.doc.toString());
    if (!m) return;
    view.dispatch({ selection: { anchor: m.index, head: m.index + name.length }, scrollIntoView: true });
    view.focus();
  }

  // range は UTF-8 のバイト位置。CodeMirror は UTF-16 の位置なので変換する
  function jump(d: Diagnostic) {
    if (!d.range || (d.file && d.file !== FILE)) return;
    const bytes = new TextEncoder().encode(view.state.doc.toString());
    const at = (n: number) => new TextDecoder().decode(bytes.slice(0, n)).length;
    view.dispatch({ selection: { anchor: at(d.range.start), head: at(d.range.end) }, scrollIntoView: true });
    view.focus();
  }
</script>

<main>
  <div class="bar">
    <label>モデル名 <input bind:value={model} aria-label="モデル名"></label>
    <button onclick={check} disabled={busy}>検査</button>
    <button onclick={simulate} disabled={busy}>実行</button>
    <button onclick={graph} disabled={busy}>グラフ</button>
    <button onclick={format} disabled={busy}>整形</button>
    <button onclick={loadMsl} disabled={busy || msl !== "none"}>{msl === "loading" ? (desktop ? "MSL を取得中…" : "MSL を読み込み中…") : desktop ? "MSL を取得する（29 MB、初回のみ）" : "MSL を読む"}</button>
  </div>
  <p class="note" data-msl={msl}>
    {#if msl === "loaded"}{mslNote}。モデル名にライブラリのモデル（例 Modelica.Electrical.Analog.Examples.ChuaCircuit）を入れて検査・実行できる。グラフは編集中のファイルのモデルだけ（ライブラリのモデルは未対応）。
    {:else if msl === "loading"}{desktop ? "MSL 4.1.0 を取得して展開している（回線しだい）。" : "MSL.json を取得して読み込んでいる（十数秒）。"}
    {:else}MSL は未読み込み。{/if}
  </p>
  <div class="editor" bind:this={host}></div>
  {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  <table class="diagnostics" aria-label="診断">
    <tbody>
      {#each diagnostics as d}
        <tr class={d.severity} onclick={() => jump(d)}>
          <td>{d.code}</td><td>{d.stage}</td><td>{d.severity}</td><td>{d.message}{#each d.notes ?? [] as n}<div class="dnote">{n}</div>{/each}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if columns}<Plot {columns} />{/if}
  {#if graphModel}<Graph {backend} model={graphModel} onSelect={select} />{/if}
</main>

<style>
  main { font: 14px/1.4 system-ui, sans-serif; max-width: 60rem; margin: 0 auto; padding: 1rem; }
  .bar { display: flex; flex-wrap: wrap; gap: 1rem; margin-bottom: .5rem; }
  .note { color: #555; margin: .25rem 0; }
  .editor { border: 1px solid #888; }
  .editor :global(.cm-editor) { height: 16rem; }
  .diagnostics { width: 100%; border-collapse: collapse; margin-top: .5rem; }
  td { border-bottom: 1px solid #ddd; padding: .2rem .5rem; vertical-align: top; }
  .dnote { color: #555; font-size: .9em; }
  tr { cursor: pointer; }
  tr.error { color: #b00020; }
  tr.warning { color: #8a6100; }
  .failure { color: #b00020; white-space: pre-wrap; }
</style>
