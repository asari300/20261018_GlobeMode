<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, drawSelection, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { bracketMatching } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import { darkSlot, modelicaCompletion, modelicaHighlight, modelicaDefinition, modelicaLinter, modelicaRefactor, modelicaTheme, themeSlot, toCmDiagnostics } from "./lib/editor";
  import { setPref, theme, type Pref } from "./theme.svelte";
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
  const STORE = "globe-mode.doc"; // S141 の 1 文書（S151 からは TABS。読むだけ）
  const TABS = "globe-mode.tabs", RECENT = "globe-mode.recent", CAP = 2 * 2 ** 20; // 本文の合計の上限（字数。UTF-16 の長さで数える近似）
  const LONG = "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation(experiment(StopTime = 1e6));\nend Long;\n"; // S147: smoke の「止める」用。止めるまで終わらない長さ
  const DECAY = "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
  let own = true;
  let saved = $state(""); // 保存した先（Desktop はパス、ブラウザは名前）
  const store = (key: string, v: unknown) => {
    try { localStorage.setItem(key, JSON.stringify(v)); } catch { /* 保存できなくても使える */ }
  };
  // 合計が CAP を超えたら、keep 以外を古い（先頭）ものから落とす
  function cap<T extends { text?: string }>(xs: T[], keep: T): T[] {
    const out = [...xs];
    let n = out.reduce((a, x) => a + (x.text?.length ?? 0), 0);
    for (let i = 0; n > CAP && i < out.length; ) {
      if (out[i] === keep) i++;
      else n -= out.splice(i, 1)[0].text?.length ?? 0;
    }
    return out;
  }

  // S151: タブ。いまの文書の file / model / own / diagnostics / saved はこれまでどおり大域の変数で、離れるときに tab へ写す（state は EditorState。元に戻す履歴も文書ごと）
  type Tab = { file: string; model: string; own: boolean; state: EditorState; diagnostics: Diagnostic[]; saved: string; clean: string }; // clean: 開いた・保存した本文（閉じるとき、違えば確かめる。ライブラリは見ない）
  let tabs = $state.raw<Tab[]>([]);
  let cur = $state(0);
  // 最近使ったファイル（古い順。上限 10）。Desktop は path だけ、ブラウザは名前と本文
  type Recent = { path: string; text?: string };
  let recent = $state.raw<Recent[]>([]);
  const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
  const remember = (path: string, text: string) => {
    const r = { path, text: desktop ? undefined : text };
    recent = cap([...recent.filter((x) => x.path !== path), r].slice(-10), r);
    store(RECENT, recent);
  };
  // 自分のファイルのタブだけを本文ごと保存する（localStorage が使えなくても動く。S102）
  const persist = () => {
    const docs = tabs
      .map((t, i) => (i === cur ? { file, own, text: view.state.doc.toString(), a: true } : { file: t.file, own: t.own, text: t.state.doc.toString(), a: false }))
      .filter((t) => t.own);
    const kept = cap(docs, docs.find((t) => t.a) ?? docs[0]);
    store(TABS, { active: Math.max(0, kept.findIndex((t) => t.a)), tabs: kept.map(({ file, text }) => ({ file, text })) });
  };
  const guessModel = (t: string) => /^\s*(?:(?:partial|encapsulated)\s+)*(?:model|block|class|package)\s+(\w+)/m.exec(t)?.[1];

  const mkState = (doc: string) =>
    EditorState.create({
      doc,
      extensions: [
        lineNumbers(), history(), drawSelection(), highlightActiveLine(), bracketMatching(), keymap.of([...defaultKeymap, ...historyKeymap]),
        modelicaHighlight, modelicaCompletion(), modelicaLinter(backend, file), // S053（S163: 色付け）
        modelicaDefinition(definition, goBack, { backend, file: () => file }), // S138（S149: ホバー）
        modelicaRefactor(references, askRename), // S161: Shift+F12 と F2
        EditorView.updateListener.of((u) => { if (u.docChanged) persist(); }),
        modelicaTheme, darkSlot(theme.dark), // S152
      ],
    });

  // S152: 明暗が変わったら、開いている EditorView と、他のタブの state の darkTheme を差し替える
  $effect(() => {
    const effects = themeSlot.reconfigure(EditorView.darkTheme.of(theme.dark));
    untrack(() => {
      if (!view) return;
      view.dispatch({ effects });
      for (const t of tabs) if (t.state !== view.state) t.state = t.state.update({ effects }).state;
    });
  });

  onMount(() => {
    let docs: { file: string; text: string }[] = [];
    let active = 0;
    try {
      const d = JSON.parse(localStorage.getItem(TABS) ?? "null");
      const old = JSON.parse(localStorage.getItem(STORE) ?? "null"); // S141 の保存
      docs = d?.tabs ?? (old ? [{ file: old.name, text: old.text }] : []);
      docs = docs.filter((t) => typeof t?.text === "string" && typeof t.file === "string");
      active = Number(d?.active) || 0;
      const r = JSON.parse(localStorage.getItem(RECENT) ?? "[]");
      if (Array.isArray(r)) recent = r.filter((x) => typeof x?.path === "string");
    } catch { /* 無ければ Decay から */ }
    if (!docs.length) docs = [{ file, text: DECAY }];
    tabs = docs.map((t) => ({ file: t.file, model: guessModel(t.text) ?? model, own: true, state: mkState(t.text), diagnostics: [], saved: "", clean: t.text })); // 戻した本文は保存済みとみなす（shortcut: 再読み込みをまたぐ未保存は数えない）
    cur = Math.min(Math.max(0, active), tabs.length - 1);
    file = tabs[cur].file;
    model = tabs[cur].model;
    // S069: 取得済みの MSL を sidecar が読んでいれば、最初から「読んだ」状態にする
    void tauri?.hasLibrary().then((has) => {
      if (has && msl === "none") { msl = "loaded"; mslNote = "MSL を読んだ（取得済み）"; }
    });
    void tauri?.onMessage((m) => { configNote = m; });
    const unlisten = tauri?.onMslProgress(({ received, total }) => {
      const mib = (n: number) => (n / 2 ** 20).toFixed(1);
      mslProgress = total ? `${mib(received)} / ${mib(total)} MiB（${Math.floor((100 * received) / total)}%）` : `${mib(received)} MiB`;
    });
    view = new EditorView({ parent: host, state: tabs[cur].state });
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
    const mslMs = performance.now() - t;
    const fail0 = failure;
    // S147: UI の操作（ライブラリの木・整形・グラフ・実行の設定・プロットの選択・定義ジャンプ・止める）を UI のコードの関数と DOM で回す。落ちた項目は =ng と理由（failure=）
    // MSL が無い（ブラウザで MSL.json が無い）ときは木・グラフ・ライブラリへの定義ジャンプを飛ばす（=skip）
    const mslUp = msl === "loaded";
    const file0 = file;
    const ng: string[] = [];
    const step = async (name: string, f: () => Promise<unknown>) => {
      failure = "";
      try {
        if (await f()) return `${name}=ok`;
        ng.push(`${name}: ${failure || "条件に合わない"}`);
      } catch (e) {
        ng.push(`${name}: ${e}`);
      }
      return `${name}=ng`;
    };
    const wait = async (f: () => unknown, limit = 60_000) => {
      const t0 = performance.now();
      while (!f() && performance.now() - t0 < limit) await new Promise((r) => setTimeout(r, 50));
      return !!f();
    };
    const restore = (text = DECAY) => { const i = tabs.findIndex((t, j) => j !== cur && t.file === file0); if (file !== file0 && i >= 0) activate(i); own = true; file = file0; model = "Decay"; back.length = 0; replaceDoc(text); diagnostics = []; columns = null; graphModel = null; };
    const setField = async (label: string, v: string) => {
      const el = document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
      el.value = v;
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
      await tick();
    };
    const doc = () => view.state.doc.toString();
    const tree = !mslUp ? "tree=skip" : await step("tree", async () => {
      const e = (await backend.libraryTree("Modelica.Electrical.Analog.Examples")).find((x) => x.name === "ChuaCircuit");
      if (!e) return false;
      await openLibrary(e);
      return !failure && !own && model === e.path && doc().includes("model ChuaCircuit");
    });
    const graphed = !mslUp ? "graph=skip" : await step("graph", async () => {
      await graph();
      const drawn = await wait(() => { const n = document.querySelector<HTMLElement>(".canvas[data-nodes]")?.dataset.nodes; return n && n !== "0"; });
      const alert = document.querySelector(".wrap [role=alert]")?.textContent; // Graph 自身の失敗（WebGL など）
      graphModel = null;
      await tick();
      if (alert) failure = alert;
      return drawn && !alert;
    });
    const def = await step("def", async () => {
      if (mslUp) { // ライブラリのファイルが開き、Alt+← 相当で戻る
        const f = file, i = doc().indexOf("Modelica.Electrical.Analog.Basic.Resistor") + 36; // Resistor の 3 文字目
        const l = view.state.doc.lineAt(i);
        await definition(l.number - 1, i - l.from);
        const opened = !failure && file !== f && doc().includes("model Resistor");
        goBack();
        if (!opened || file !== f || !doc().includes("model ChuaCircuit")) return false;
      }
      restore();
      await definition(3, 13); // Decay の `  der(x) = -x;` の x のすぐ後ろ
      return !failure && view.state.doc.lineAt(view.state.selection.main.head).text.includes("Real x");
    });
    const fmt = await step("fmt", async () => {
      restore("model C\nReal x;\nend C;\n");
      model = "C";
      await format();
      return !failure && /\n {2,4}Real x;/.test(doc());
    });
    restore();
    const settings = await step("settings", async () => {
      await setField("終了時刻", "abc");
      const refused = settingsProblem !== ""; // 数でない欄は問題として出る
      await setField("終了時刻", "2");
      await simulate();
      await tick();
      const t2 = columns ? Number(columns.data[0][columns.points - 1]) : NaN;
      await setField("終了時刻", "");
      return refused && !failure && lastSettings?.stop_time === 2 && lastSettings.origins.stop_time === "requested" && Math.abs(t2 - 2) < 1e-3;
    });
    const plot = await step("plot", async () => { // 直前の実行の列。系列を外す・戻す・右の軸へ
      const p = document.querySelector<HTMLElement>(".plot");
      const box = p?.querySelector<HTMLInputElement>("input[name=signal]");
      if (!p || !box) return false;
      const n = Number(p.dataset.series);
      box.click();
      await tick();
      const off = Number(p.dataset.series);
      box.click();
      await tick();
      p.querySelector<HTMLInputElement>('input[aria-label$="を右の軸に"]')?.click();
      await tick();
      return n >= 1 && off === n - 1 && Number(p.dataset.series) === n && p.dataset.right === "1";
    });
    let cancelPath = "sidecar"; // S159: 止めた道。flag は SharedArrayBuffer のフラグ（途中の列が残る）、restart は Worker の作り直し（残らない）、sidecar は Desktop
    const cancel = await step("cancel", async () => { // Desktop は TauriBackend.cancel、Android・ブラウザは Worker のフラグ（cross-origin isolated のとき）か terminate
      replaceDoc(LONG);
      model = "Long";
      const done = simulate();
      const running = await wait(() => runNote.includes("integration"), 120_000);
      const button = [...document.querySelectorAll("button")].find((b) => b.textContent === "止める");
      if (button) button.click();
      else backend.cancel?.();
      await done;
      if (!tauri) cancelPath = runNote.includes("途中の結果は出ない") ? "restart" : "flag";
      const stopped = runState === "cancelled" || diagnostics.some((d) => d.code === "GM-I11004"); // sidecar が止めれば途中までの結果と GM-I11004
      await check(); // 止めたあとも次の呼び出しが回る
      return running && !!button && stopped && !failure && !diagnostics.some((d) => d.severity === "error");
    });
    // S154: Desktop は作業ディレクトリの Recent.mo（desktop.yml が置く）を最近使ったファイルとして開く（dialog を通さない path。Rust の read_recent）
    const recentStep = !tauri ? "" : " " + await step("recent", async () => {
      const saved = recent;
      recent = [{ path: "Recent.mo" }];
      await openRecent("Recent.mo");
      recent = saved;
      return !failure && file === "Recent.mo" && doc().includes("model Recent");
    });
    restore();
    const ms = (x: number) => x.toFixed(0);
    const line = `GM-SMOKE check=${checked ? "ok" : "ng"} simulate=${simulated ? "ok" : "ng"} x1=${x1} msl=${mslOk ? "ok" : "ng"}${cfg}`
      + ` ${tree} ${fmt} ${graphed} ${cancel} ${settings} ${plot} ${def}${recentStep}`
      + ` isolated=${crossOriginIsolated ? 1 : 0} cancel_path=${cancelPath}`
      + ` ready_ms=${ms(ready)} decay_ms=${ms(decay)} msl_ms=${mslOk ? ms(mslMs) : "-"}`
      + (fail0 || ng.length ? ` failure=${[fail0, ...ng].filter(Boolean).join(" / ").replace(/\s+/g, " ")}` : "");
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
      // S157: 止めた実行はそこまでの列を出し、止めた旨を残す（wasm は SharedArrayBuffer のフラグ、Desktop は $/cancelRequest。どちらも GM-I11004 の普通の応答）
      const stopped = r.diagnostics.find((d) => d.code === "GM-I11004");
      if (stopped) { runState = "cancelled"; runNote = `止めた（${c.points > 0 ? `${stopped.message}、そこまでの ${c.points} 点を表示` : "結果は出ない"}）`; }
      if (r.refused) failure = r.refused;
      else if (!r.ok && !r.diagnostics.length) failure = "実行に失敗した（診断なし）";
    });

  // ライブラリの木で選んだクラスのファイルをタブに開き、モデル名をそのクラスにする（S127）
  const openLibrary = (e: TreeEntry) => run(async () => {
    const src = await backend.librarySource(e.path);
    openDoc(src.file, src.text, e.path, false);
  });

  // S138: 定義ジャンプ。別のファイル（ライブラリ）なら openLibrary と同じく同じ名前の文書として開く。入る前の文書と位置は back に積む（Alt+←）
  const back: string[] = []; // 入る前のタブの file（S151 からは文書そのものはタブが持つ）
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
        back.push(file);
        openDoc(d.file, d.text, undefined, false);
      }
      const at = ({ line, character }: { line: number; character: number }) => view.state.doc.line(line + 1).from + character;
      moveTo(at(d.range.start), at(d.range.end));
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
  }
  // S161: 参照の一覧（Shift+F12）。行を押すとその範囲へ。範囲の計算は Rust（backend.references）
  let refs = $state<{ n: number; text: string; from: number; to: number }[]>([]);
  const offsetOf = ({ line, character }: { line: number; character: number }) => view.state.doc.line(line + 1).from + character;
  async function references(line: number, character: number) {
    failure = "";
    try {
      await backend.setFile(file, view.state.doc.toString());
      refs = (await backend.references(file, line, character)).map((r) => {
        const from = offsetOf(r.start), l = view.state.doc.lineAt(from);
        return { n: l.number, text: l.text.trim(), from, to: offsetOf(r.end) };
      });
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
  }
  // S161: リネーム（F2）。部品とクラス名（S166）。入力欄を出し、確定すると backend.rename の編集を 1 回の dispatch（undo 1 回）で入れる。断られたら理由を failure に出す
  let renaming = $state<{ line: number; character: number } | null>(null);
  let newName = $state("");
  let nameInput = $state<HTMLInputElement>();
  async function askRename(line: number, character: number) {
    const w = view.state.wordAt(view.state.selection.main.head);
    newName = w ? view.state.sliceDoc(w.from, w.to) : "";
    failure = "";
    try {
      // S166: リネームできない所は入力欄を出さずに理由を出す（今の名前のまま rename を試し、断られたら理由が返る。編集は入れない）
      await backend.setFile(file, view.state.doc.toString());
      await backend.rename(file, line, character, newName);
    } catch (e) {
      failure = String((e as Error).message ?? e);
      return;
    }
    renaming = { line, character };
    await tick();
    nameInput?.select();
  }
  async function rename() {
    const at = renaming;
    renaming = null;
    if (!at) return;
    failure = "";
    try {
      await backend.setFile(file, view.state.doc.toString());
      const edits = await backend.rename(file, at.line, at.character, newName);
      view.dispatch({ changes: edits.map((e) => ({ from: offsetOf(e.range.start), to: offsetOf(e.range.end), insert: e.newText })) });
      refs = [];
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
    view.focus();
  }
  function goBack() {
    let i = -1;
    while (i < 0 && back.length) { const f = back.pop(); i = tabs.findIndex((t, j) => j !== cur && t.file === f); } // 閉じたタブは飛ばす
    if (i < 0) return;
    activate(i);
    view.focus();
  }

  // S151: タブの切り替え。離れる文書を tab に写し、次の tab を読む。Workspace へは送らない（検査・実行・定義ジャンプが、開いている文書を setFile する）
  function activate(i: number) {
    if (i === cur || !tabs[i]) return;
    const prev = tabs[cur];
    Object.assign(prev, { file, model, own, state: view.state, diagnostics, saved });
    const t = tabs[i];
    if (prev.own && prev.file !== t.file) void backend.setFile(prev.file, "").catch(() => {}); // 離れた自分のファイルは Workspace から外す（同じクラスの二重定義を避ける。S141）
    cur = i;
    ({ file, model, own, diagnostics, saved } = t);
    columns = null;
    graphModel = null;
    tabs = [...tabs];
    view.setState(t.state);
    persist();
  }
  // 文書 f を開く。いまの文書なら本文を差し替え、同じ名前のタブがあればそこへ、無ければ新しいタブにする。自分のファイル（isOwn）は渡された本文が正、ライブラリのファイルは開いているタブのまま
  function openDoc(f: string, text: string, m: string | undefined, isOwn: boolean) {
    if (f === file) {
      if (isOwn) { own = true; replaceDoc(text); diagnostics = []; columns = null; graphModel = null; saved = ""; tabs[cur].clean = text; }
      if (m) model = m;
      return;
    }
    let i = tabs.findIndex((t, j) => j !== cur && t.file === f);
    if (i < 0) {
      tabs = [...tabs, { file: f, model: m ?? model, own: isOwn, state: mkState(text), diagnostics: [], saved: "", clean: text }];
      i = tabs.length - 1;
    } else {
      if (isOwn) Object.assign(tabs[i], { own: true, state: mkState(text), diagnostics: [], saved: "", clean: text });
      if (m) tabs[i].model = m;
    }
    activate(i);
  }
  function closeTab(i: number) {
    if (tabs.length < 2) return;
    const t = tabs[i], text = i === cur ? view.state.doc.toString() : t.state.doc.toString();
    if ((i === cur ? own : t.own) && text !== t.clean && !confirm(`${baseName(i === cur ? file : t.file)} は保存していない。閉じる？`)) return;
    if (i === cur) activate(i ? i - 1 : 1);
    const here = tabs[cur];
    tabs = tabs.filter((_, j) => j !== i);
    cur = tabs.indexOf(here);
    persist();
  }

  // S141: 開く。S151 から新しいタブに開く（同じ名前のタブがあればその本文を差し替える）。Workspace の二重定義は activate が避ける
  const openOwn = (name: string, text: string) => openDoc(name, text, guessModel(text), true);
  const openFromDisk = () => run(async () => {
    const f = await backend.openFile();
    if (!f) return;
    openOwn(f.name, f.text);
    remember(f.path ?? f.name, f.text);
  });
  const openRecent = (path: string) => run(async () => {
    const r = recent.find((x) => x.path === path);
    const f = r?.text !== undefined ? { name: path, text: r.text } : await backend.openPath?.(path);
    if (f) openOwn(f.name, f.text);
  });
  // S144: ソースの FMU。ブラウザはダウンロード、Desktop は保存先のフォルダへ sidecar が書く。S154: FMI の版（2.0 / 3.0）を選べる
  let fmiVersion = $state(2);
  const exportFmu = () => run(async () => {
    const to = await backend.exportFmu(model, fmiVersion);
    if (to) saved = `FMU を書き出した: ${to}`;
  });
  async function save(saveAs = false) {
    failure = "";
    try {
      const to = await backend.saveFile(file.split("/").pop()!, view.state.doc.toString(), saveAs);
      if (to) {
        saved = `保存した: ${to}`;
        if (own) { remember(to, view.state.doc.toString()); tabs[cur].clean = view.state.doc.toString(); }
      }
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
    {#if recent.length}
      <select aria-label="最近使ったファイル" disabled={busy} onchange={(e) => { const p = e.currentTarget.value; e.currentTarget.value = ""; if (p) void openRecent(p); }}>
        <option value="">最近使ったファイル</option>
        {#each [...recent].reverse() as r}<option value={r.path} title={r.path}>{baseName(r.path)}</option>{/each}
      </select>
    {/if}
    <button onclick={() => save()}>保存</button>
    {#if desktop}<button onclick={() => save(true)}>名前を付けて保存</button>{/if}
    <button onclick={exportFmu} disabled={busy}>FMU を書き出す</button>
    <select aria-label="FMI の版" bind:value={fmiVersion}><option value={2}>FMI 2.0</option><option value={3}>FMI 3.0</option></select>
    <label>モデル名 <input bind:value={model} aria-label="モデル名"></label>
    <select aria-label="テーマ" value={theme.pref} onchange={(e) => setPref(e.currentTarget.value as Pref)}>
      <option value="auto">OS に合わせる</option>
      <option value="light">明</option>
      <option value="dark">暗</option>
    </select>
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
  <div class="tabs" role="tablist">
    {#each tabs as t, i}
      {@const f = i === cur ? file : t.file}
      <span class="tab" class:active={i === cur}>
        <button role="tab" aria-selected={i === cur} title={f} disabled={busy} onclick={() => activate(i)}>{baseName(f)}</button>
        {#if tabs.length > 1}<button aria-label="{baseName(f)} を閉じる" disabled={busy} onclick={() => closeTab(i)}>×</button>{/if}
      </span>
    {/each}
  </div>
  <div class="work">
    {#if msl === "loaded"}<aside class="library"><LibraryTree {backend} onOpen={openLibrary} /></aside>{/if}
    <div class="editor" bind:this={host}></div>
  </div>
  {#if saved}<p class="note" role="status" data-saved>{saved}</p>{/if}
  {#if lastFinal.length}<p class="final" data-final={JSON.stringify(Object.fromEntries(lastFinal))}>最後の時刻の値: {lastFinal.map(([k, v]) => `${k} = ${v.toPrecision(6)}`).join("、")}</p>{/if}
  {#if runNote}<p class="run" role="status" data-run={runState}>{runNote}</p>{/if}
  {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  {#if renaming}
    <form class="rename" onsubmit={(e) => { e.preventDefault(); void rename(); }}>
      <label>新しい名前 <input bind:this={nameInput} bind:value={newName} onkeydown={(e) => { if (e.key === "Escape") { renaming = null; view.focus(); } }} /></label>
      <button type="submit">リネーム</button>
    </form>
  {/if}
  {#if refs.length}
    <table class="refs" aria-label="参照">
      <tbody>
        {#each refs as r}
          <tr onclick={() => moveTo(r.from, r.to)}><td>{r.n}</td><td>{r.text}</td></tr>
        {/each}
      </tbody>
    </table>
    <button type="button" onclick={() => (refs = [])}>参照を閉じる</button>
  {/if}
  <table class="diagnostics" aria-label="診断">
    <tbody>
      {#each diagnostics as d}
        <tr class={d.severity} onclick={() => jump(d)}>
          <td>{d.code}</td><td>{d.stage}</td><td>{d.severity}</td><td>{d.message}{#each d.notes ?? [] as n}<div class="dnote">{n}</div>{/each}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if columns}<Plot {columns} {model} />{/if}
  {#if graphModel}<Graph {backend} model={graphModel} onSelect={select} />{/if}
</main>

<style>
  main { font: 14px/1.4 system-ui, sans-serif; max-width: 60rem; margin: 0 auto; padding: 1rem; }
  .bar { display: flex; flex-wrap: wrap; gap: 1rem; margin-bottom: .5rem; }
  .note, .run, .final { color: var(--muted); margin: .25rem 0; }
  .tabs { display: flex; flex-wrap: wrap; gap: .25rem; margin-bottom: .25rem; }
  .tab.active button[role=tab] { font-weight: bold; }
  .work { display: flex; gap: .5rem; }
  .library { flex: 0 0 16rem; height: 16rem; overflow: auto; border: 1px solid var(--border); font-size: .9em; }
  .editor { flex: 1; min-width: 0; border: 1px solid var(--border); }
  .editor :global(.cm-editor) { height: 16rem; }
  .diagnostics, .refs { width: 100%; border-collapse: collapse; margin-top: .5rem; }
  td { border-bottom: 1px solid var(--line); padding: .2rem .5rem; vertical-align: top; }
  .rename { margin: .5rem 0; }
  .dnote { color: var(--muted); font-size: .9em; }
  tr { cursor: pointer; }
  tr.error { color: var(--err); }
  tr.warning { color: var(--warn); }
  .failure { color: var(--err); white-space: pre-wrap; }
  /* 小画面（S145）: 左の木を上に積み、診断の長い語を折り返し、横スクロールを出さない */
  @media (max-width: 600px) {
    main { padding: .5rem; }
    .work { flex-direction: column; }
    .library { flex: none; height: 10rem; }
    td { overflow-wrap: anywhere; }
  }
</style>
