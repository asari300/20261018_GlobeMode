<script lang="ts">
  import { onMount, tick } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, drawSelection, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { bracketMatching } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import { modelicaCompletion, modelicaDefinition, modelicaLinter, toCmDiagnostics } from "./lib/editor";
  import { WorkerBackend } from "./lib/worker-backend";
  import { TauriBackend, inTauri } from "./lib/tauri-backend";
  import type { Backend, TreeEntry } from "./lib/backend";
  import type { Columns } from "./lib/plot/csv";
  import type { CheckResponse, Diagnostic, ResolvedSettings, SimulateRequest } from "./lib/protocol";
  import Plot from "./lib/plot/Plot.svelte";
  import Graph from "./lib/graph/Graph.svelte";
  import LibraryTree from "./lib/LibraryTree.svelte";
  import SimSettings from "./lib/SimSettings.svelte";

  // エディタの文書の名前。ライブラリの木で開くと、そのライブラリのファイルの名前（同じ名前で差し替える。S127）
  let file = $state("Model.mo");
  // S092: Android の Tauri には sidecar が無い（ADR-0023）ので、ブラウザと同じ Worker（wasm）を使う
  const desktop = inTauri() && !/Android/i.test(navigator.userAgent);
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
  // S084: Desktop だけ版を選べる（4.0.0 は公開ライブラリの前提の版）。loadedVersion は起動時に取得済みを読んだときは不明（null）
  let mslVersion = $state("4.1.0");
  let loadedVersion = $state<string | null>(null);
  let mslProgress = $state("");
  // S133（ADR-0033）: 実行中の進捗の 1 行と、終わり方（cancelled なら「止めた」）
  let runNote = $state("");
  let runState = $state<"idle" | "running" | "cancelled">("idle");
  // S142: 実行の設定（SimSettings が作る）、検査で分かった experiment、直近の実行の設定と出どころ
  let simSettings = $state<NonNullable<SimulateRequest["settings"]>>({});
  let settingsProblem = $state("");
  let experiment = $state<CheckResponse["experiment"]>();
  let checkedModel = $state<{ model: string } | null>(null); // S150: 検査したモデル（SimSettings が上書きできるパラメータの一覧を取る合図。検査のたびに新しい値）
  let lastSettings = $state<ResolvedSettings | null>(null);
  let lastFinal = $state<[string, number][]>([]); // 最後の時刻の値（先頭の 8 本）
  $effect(() => { void model; experiment = undefined; }); // 別のモデルにしたら、次の検査まで experiment は分からない
  let configNote = $state(""); // S143: Desktop の globe-mode.toml を読み直した結果、または sidecar の showMessage（誤り）
  let graphModel = $state<string | null>(null); // 「グラフ」を押したときのモデル名（入力のたびに描き直さない）

  // S141: 自分のファイルを開く・保存する。本文とファイル名は localStorage に自動保存して起動時に戻す（使えなくても動く。S102）。モデル名は戻さず本文から入れ直す（smoke が書き換えたモデル名を次の起動で拾わないため）
  // own はエディタの文書が自分のファイルか（ライブラリのファイルを開いている間は保存も上書きもしない）
  const STORE = "globe-mode.doc";
  const DECAY = "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
  let own = true;
  let saved = $state(""); // 保存した先（Desktop はパス、ブラウザは名前）
  const persist = () => {
    if (!own) return;
    try { localStorage.setItem(STORE, JSON.stringify({ name: file, text: view.state.doc.toString() })); } catch { /* 保存できなくても使える */ }
  };
  const guessModel = (t: string) => /^\s*(?:(?:partial|encapsulated)\s+)*(?:model|block|class|package)\s+(\w+)/m.exec(t)?.[1];

  onMount(() => {
    let text = DECAY;
    try {
      const d = JSON.parse(localStorage.getItem(STORE) ?? "null");
      if (typeof d?.text === "string" && typeof d.name === "string") { text = d.text; file = d.name; model = guessModel(text) ?? model; }
    } catch { /* 無ければ Decay から */ }
    // S069: 取得済みの MSL を sidecar が読んでいれば、最初から「読んだ」状態にする
    void tauri?.hasLibrary().then((has) => {
      if (has && msl === "none") { msl = "loaded"; mslNote = "MSL を読んだ（取得済み）"; }
    });
    void tauri?.onMessage((m) => { configNote = m; });
    const unlisten = tauri?.onMslProgress(({ received, total }) => {
      const mib = (n: number) => (n / 2 ** 20).toFixed(1);
      mslProgress = total ? `${mib(received)} / ${mib(total)} MiB（${Math.floor((100 * received) / total)}%）` : `${mib(received)} MiB`;
    });
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: text,
        extensions: [
          lineNumbers(), history(), drawSelection(), highlightActiveLine(), bracketMatching(), keymap.of([...defaultKeymap, ...historyKeymap]),
          modelicaCompletion(), modelicaLinter(backend, file), // S053
          modelicaDefinition(definition, goBack), // S138
          EditorView.updateListener.of((u) => { if (u.docChanged) persist(); }),
        ],
      }),
    });
    // S100: Android のエミュレータの smoke（android.yml が logcat で GM-SMOKE の行を待つ）。クエリを渡せない Tauri の Android はビルド時の VITE_GM_SMOKE=1
    if (import.meta.env.VITE_GM_SMOKE === "1" || new URLSearchParams(location.search).has("smoke")) void smoke();
    else void tauri?.bench().then((on) => { if (on) void smoke(); }); // S125: Desktop の測定（GLOBE_DESKTOP_BENCH）
    return () => { view.destroy(); void unlisten?.then((f) => f()); };
  });

  // 起動時の Decay を検査・実行して、結果を console に 1 行出す。x1 は x(1)（e⁻¹ ≈ 0.3679）
  // S125: 末尾に時刻（ms、ADR-0034）。ready は最初の往復（ページの読み込みの始まりから）、decay は検査・実行・プロットの反映まで、msl は MSL を読み ChuaCircuit を検査するまで
  async function smoke() {
    await backend.setFile(file, view.state.doc.toString()).catch((e) => { failure = String(e); });
    const ready = performance.now();
    await check();
    const checked = !failure && !diagnostics.some((d) => d.severity === "error");
    await simulate();
    await tick();
    const decay = performance.now() - ready;
    const i = columns ? columns.names.indexOf("x") : -1;
    const x1 = columns && i >= 0 ? Number(columns.data[i + 1][columns.points - 1]) : NaN;
    const simulated = !failure && Math.abs(x1 - Math.exp(-1)) < 1e-3;
    // S103: 同梱の MSL.json を読み、MSL の例題を 1 つ検査する（Android の APK は android.yml が MSL.json を入れる）
    const t = performance.now();
    await loadMsl();
    if (msl === "loaded") { model = "Modelica.Electrical.Analog.Examples.ChuaCircuit"; await check(); }
    const mslOk = msl === "loaded" && !failure && !diagnostics.some((d) => d.severity === "error");
    // S143: Desktop は globe-mode.toml（desktop.yml が indent = 4 を置いた作業ディレクトリ）が整形に効くか。無ければ none
    const cfg = tauri ? ` cfg=${(await backend.format("model C\nReal x;\nend C;\n").catch(() => "")).includes("\n    Real x;") ? "ok" : "none"}` : "";
    const ms = (x: number) => x.toFixed(0);
    const line = `GM-SMOKE check=${checked ? "ok" : "ng"} simulate=${simulated ? "ok" : "ng"} x1=${x1} msl=${mslOk ? "ok" : "ng"}${cfg}`
      + ` ready_ms=${ms(ready)} decay_ms=${ms(decay)} msl_ms=${mslOk ? ms(performance.now() - t) : "-"}${failure ? ` failure=${failure}` : ""}`;
    console.log(line);
    void tauri?.bench(line);
  }

  async function loadMsl() {
    if (tauri && msl === "loading") { void tauri.cancelMsl(); return; } // 取得中にもう一度押すと取り消す
    const prev = msl;
    msl = "loading";
    mslProgress = "";
    failure = "";
    const t = performance.now();
    try {
      let json = "";
      if (tauri) await tauri.fetchMsl(mslVersion); // Desktop は JSON を使わない（sidecar が取得した MSL を読む）
      else {
        const res = await fetch(`${import.meta.env.BASE_URL}msl/MSL.json`);
        if (!res.ok) throw new Error(`MSL.json を取得できない（${res.status}）。data/msl を取得して pnpm prepare-wasm（cargo xtask fetch-msl）`);
        json = await res.text();
        await backend.library("MSL", json);
      }
      msl = "loaded";
      loadedVersion = desktop ? mslVersion : "4.1.0";
      const size = desktop ? "" : `${(json.length / 2 ** 20).toFixed(0)} MiB、`;
      mslNote = `MSL ${loadedVersion} を読んだ（${size}${((performance.now() - t) / 1000).toFixed(1)} 秒）`;
      diagnostics = [];
      columns = null;
      graphModel = null;
    } catch (e) {
      msl = prev; // 取り消し・失敗のときは前の MSL のまま
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
      await backend.setFile(file, view.state.doc.toString());
      await job();
    } catch (e) {
      failure = String((e as Error).message ?? e);
    } finally {
      busy = false;
    }
  }

  const check = () => run(async () => {
    const checked = await backend.check(model);
    diagnostics = bySeverity(checked.diagnostics);
    experiment = checked.experiment;
    checkedModel = { model };
    view.dispatch(setDiagnostics(view.state, toCmDiagnostics(view.state.doc.toString(), diagnostics, file)));
  });

  // 整形（S053、doc/format-style.md）。文法が壊れていれば本文は変えず、理由を出す
  const format = () => run(async () => {
    const text = view.state.doc.toString();
    const out = await backend.format(text);
    if (out !== text) view.dispatch({ changes: { from: 0, to: text.length, insert: out } });
  });

  const simulate = () =>
    run(async () => {
      if (settingsProblem) { failure = `実行の設定: ${settingsProblem}`; return; }
      runState = "running";
      runNote = "実行中";
      let out;
      try {
        out = await backend.simulateColumns({ model, settings: simSettings }, (p) => {
          const pct = p.time === undefined ? "" : `、t = ${p.time.toPrecision(4)} / ${p.stop_time}（${Math.floor((100 * (p.time - p.start_time)) / (p.stop_time - p.start_time || 1))}%）`;
          runNote = `実行中: ${p.stage}${pct}`;
        });
      } catch (e) {
        if (!(e as { cancelled?: boolean }).cancelled) throw e;
        runState = "cancelled";
        runNote = "止めた（途中の結果は出ない）";
        return;
      } finally {
        if (runState === "running") { runState = "idle"; runNote = ""; }
      }
      const { response: r, columns: c } = out;
      diagnostics = bySeverity(r.diagnostics);
      lastSettings = r.settings ?? null;
      lastFinal = Object.entries(r.final ?? {}).slice(0, 8);
      columns = c.points > 0 ? c : null;
      if (r.refused) failure = r.refused;
      else if (!r.ok && !r.diagnostics.length) failure = "実行に失敗した（診断なし）";
    });

  // ライブラリの木で選んだクラスのファイルをエディタに開き、モデル名をそのクラスにする（S127）
  const openLibrary = (e: TreeEntry) => run(async () => {
    const src = await backend.librarySource(e.path);
    own = false;
    file = src.file;
    model = e.path;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: src.text } });
    diagnostics = [];
    columns = null;
    graphModel = null;
  });

  // S138: 定義ジャンプ。別のファイル（ライブラリ）なら openLibrary と同じく同じ名前の文書として開く。入る前の文書と位置は back に積む（Alt+←）
  const back: { file: string; text: string; anchor: number; own: boolean }[] = [];
  // 範囲を選び、カーソル（head）は宣言の頭に置く（クラスは長いので、末尾だと宣言が画面の外に出る）
  const moveTo = (from: number, to: number) => {
    view.dispatch({ selection: { anchor: to, head: from }, scrollIntoView: true });
    view.focus();
  };
  const replaceDoc = (text: string) => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
  async function definition(line: number, character: number) {
    failure = "";
    try {
      await backend.setFile(file, view.state.doc.toString());
      const d = await backend.definition(file, line, character);
      if (!d) return;
      if (d.file !== file) {
        back.push({ file, text: view.state.doc.toString(), anchor: view.state.selection.main.head, own });
        own = false;
        file = d.file;
        replaceDoc(d.text);
        diagnostics = [];
        columns = null;
        graphModel = null;
      }
      const at = ({ line, character }: { line: number; character: number }) => view.state.doc.line(line + 1).from + character;
      moveTo(at(d.range.start), at(d.range.end));
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
  }
  function goBack() {
    const b = back.pop();
    if (!b) return;
    file = b.file;
    own = b.own;
    replaceDoc(b.text);
    moveTo(b.anchor, b.anchor);
  }

  // S141: 開く。いまの自分のファイルと名前が違えば、バックエンドの Workspace に残らないよう空にする（同じクラスの二重定義を避ける）
  const openFromDisk = () => run(async () => {
    const f = await backend.openFile();
    if (!f) return;
    if (own && f.name !== file) await backend.setFile(file, "");
    own = true;
    file = f.name;
    model = guessModel(f.text) ?? model;
    back.length = 0;
    replaceDoc(f.text);
    diagnostics = [];
    columns = null;
    graphModel = null;
    saved = "";
  });
  // S144: ソースの FMU（FMI 2.0）。ブラウザはダウンロード、Desktop は保存先のフォルダへ sidecar が書く
  const exportFmu = () => run(async () => {
    const to = await backend.exportFmu(model, 2);
    if (to) saved = `FMU を書き出した: ${to}`;
  });
  async function save(saveAs = false) {
    failure = "";
    try {
      const to = await backend.saveFile(file.split("/").pop()!, view.state.doc.toString(), saveAs);
      if (to) saved = `保存した: ${to}`;
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
  }

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
    if (!d.range || (d.file && d.file !== file)) return;
    const bytes = new TextEncoder().encode(view.state.doc.toString());
    const at = (n: number) => new TextDecoder().decode(bytes.slice(0, n)).length;
    view.dispatch({ selection: { anchor: at(d.range.start), head: at(d.range.end) }, scrollIntoView: true });
    view.focus();
  }
</script>

<main>
  <div class="bar">
    <button onclick={openFromDisk} disabled={busy}>開く</button>
    <button onclick={() => save()}>保存</button>
    {#if desktop}<button onclick={() => save(true)}>名前を付けて保存</button>{/if}
    <button onclick={exportFmu} disabled={busy}>FMU を書き出す</button>
    <label>モデル名 <input bind:value={model} aria-label="モデル名"></label>
    <button onclick={check} disabled={busy}>検査</button>
    <button onclick={simulate} disabled={busy}>実行</button>
    <button onclick={graph} disabled={busy}>グラフ</button>
    <button onclick={format} disabled={busy}>整形</button>
    {#if runState === "running" && backend.cancel}<button onclick={() => backend.cancel?.()}>止める</button>{/if}
    {#if desktop}
      <select bind:value={mslVersion} disabled={msl === "loading"} aria-label="MSL の版">
        <option value="4.1.0">MSL 4.1.0</option>
        <option value="4.0.0">MSL 4.0.0</option>
      </select>
      <button onclick={loadMsl} disabled={busy || (msl === "loaded" && loadedVersion === mslVersion)}>{msl === "loading" ? "取り消す" : msl === "loaded" ? `MSL ${mslVersion} に切り替える` : `MSL ${mslVersion} を取得する（約 29 MB、初回のみ）`}</button>
      <button onclick={async (e) => { const b = e.currentTarget; b.textContent = `更新: ${await tauri!.checkUpdate()}`; }}>更新を確認</button>
    {:else}
      <button onclick={loadMsl} disabled={busy || msl !== "none"}>{msl === "loading" ? "MSL を読み込み中…" : "MSL を読む"}</button>
    {/if}
  </div>
  <SimSettings {backend} {experiment} checked={checkedModel} last={lastSettings} bind:settings={simSettings} bind:problem={settingsProblem} />
  {#if tauri}<p class="note" data-config>
    <button onclick={async () => { configNote = ""; try { await tauri.reloadConfig(); configNote ||= "globe-mode.toml を読み直した（無ければ既定）"; } catch (e) { configNote = String(e); } }}>設定を読み直す</button> {configNote}
  </p>{/if}
  <p class="note" data-msl={msl}>
    {#if msl === "loaded"}{mslNote}。左の木でモデルを選ぶか、モデル名にライブラリのモデル（例 Modelica.Electrical.Analog.Examples.ChuaCircuit）を入れて検査・実行できる。グラフも描ける。
    {:else if msl === "loading"}{desktop ? `MSL ${mslVersion} を取得して展開している（${mslProgress || "回線しだい"}）。もう一度押すと取り消す。` : "MSL.json を取得して読み込んでいる（十数秒）。"}
    {:else}MSL は未読み込み。{/if}
  </p>
  <div class="work">
    {#if msl === "loaded"}<aside class="library"><LibraryTree {backend} onOpen={openLibrary} /></aside>{/if}
    <div class="editor" bind:this={host}></div>
  </div>
  {#if saved}<p class="note" role="status" data-saved>{saved}</p>{/if}
  {#if lastFinal.length}<p class="final" data-final={JSON.stringify(Object.fromEntries(lastFinal))}>最後の時刻の値: {lastFinal.map(([k, v]) => `${k} = ${v.toPrecision(6)}`).join("、")}</p>{/if}
  {#if runNote}<p class="run" role="status" data-run={runState}>{runNote}</p>{/if}
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
  .note, .run, .final { color: #555; margin: .25rem 0; }
  .work { display: flex; gap: .5rem; }
  .library { flex: 0 0 16rem; height: 16rem; overflow: auto; border: 1px solid #888; font-size: .9em; }
  .editor { flex: 1; min-width: 0; border: 1px solid #888; }
  .editor :global(.cm-editor) { height: 16rem; }
  .diagnostics { width: 100%; border-collapse: collapse; margin-top: .5rem; }
  td { border-bottom: 1px solid #ddd; padding: .2rem .5rem; vertical-align: top; }
  .dnote { color: #555; font-size: .9em; }
  tr { cursor: pointer; }
  tr.error { color: #b00020; }
  tr.warning { color: #8a6100; }
  .failure { color: #b00020; white-space: pre-wrap; }
  /* 小画面（S145）: 左の木を上に積み、診断の長い語を折り返し、横スクロールを出さない */
  @media (max-width: 600px) {
    main { padding: .5rem; }
    .work { flex-direction: column; }
    .library { flex: none; height: 10rem; }
    td { overflow-wrap: anywhere; }
  }
</style>
