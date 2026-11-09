<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, drawSelection, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { bracketMatching } from "@codemirror/language";
  import { setDiagnostics } from "@codemirror/lint";
  import { closeCompletion, completionStatus } from "@codemirror/autocomplete";
  import { EDITOR_HELP, byteToChar, darkSlot, makeEditable, readOnlySlot, modelFollow, modelRenamed, modelicaCompletion, modelicaHighlight, modelicaDefinition, modelicaFolding, modelicaLinter, modelicaRefactor, modelicaTab, modelicaTheme, themeSlot, toCmDiagnostics } from "./lib/editor";
  import { setPref, theme, type Pref } from "./theme.svelte";
  import { WorkerBackend } from "./lib/worker-backend";
  import { TauriBackend, inDesktop, tabKey } from "./lib/tauri-backend";
  import type { Backend, Documentation, FmuInfo, FmuRunRequest, Icon, TreeEntry } from "./lib/backend";
  import type { Columns } from "./lib/plot/csv";
  import type { CheckResponse, Diagnostic, ResolvedSettings, SimulateRequest } from "./lib/protocol";
  import Plot from "./lib/plot/Plot.svelte";
  import Graph from "./lib/graph/Graph.svelte";
  import LibraryTree from "./lib/LibraryTree.svelte";
  import SimSettings from "./lib/SimSettings.svelte";
  import DocsPanel from "./lib/DocsPanel.svelte";
  import { modelicaDocs } from "./lib/docs";
  import { isInternal } from "./lib/plot/signals";
  import FmuForm from "./lib/FmuForm.svelte";
  import { isFmuFile, isResultsFile, type ResultColumns } from "./lib/results";

  // エディタの文書の名前。ライブラリの木で開くと、そのライブラリのファイルの名前（同じ名前で差し替える。S127）
  let file = $state("Model.mo");
  // S092: Android の Tauri には sidecar が無い（ADR-0023）ので、ブラウザと同じ Worker（wasm）を使う
  const desktop = inDesktop();
  const tauri = desktop ? new TauriBackend() : null;
  const backend: Backend = tauri ?? new WorkerBackend(); // S049: Tauri の中では sidecar
  const severityRank = { error: 0, warning: 1, info: 2 };

  let host: HTMLDivElement;
  let view: EditorView;
  let model = $state("Decay");
  let busy = $state(false);
  let failure = $state("");
  let live = $state(""); // S175: 検査・実行の完了を読み上げる 1 行（aria-live="polite"）。失敗は role="alert"、未対応の要約・止めたは role="status" が読むので、ここには入れない
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
  let lastFinal = $state<[string, number][]>([]); // 最後の時刻の値（先頭の 8 本。$ で始まる内部名は除く）
  // S173: コンパイラが未対応で止まった要約（Rust が決める。空なら無い）、強行で進める見込みのモデル（Rust の can_force）、直近の実行が強行か
  let unsupported = $state("");
  let canForce = $state<string | null>(null);
  let forced = $state(false);
  $effect(() => { void model; experiment = undefined; }); // 別のモデルにしたら、次の検査まで experiment は分からない
  let configNote = $state(""); // S143: Desktop の globe-mode.toml を読み直した結果、または sidecar の showMessage（誤り）
  // S170: 開いている文書のモデルの候補（Rust の outline。2 つ以上のときだけ select を出す）。strayLib は、候補に無いモデル名がライブラリにあるとき（その名前）
  let models = $state<string[]>([]);
  let strayLib = $state("");
  async function refreshModels(set = true) {
    const f = file;
    try {
      if (set && !isResults) await backend.setFile(f, view.state.doc.toString());
      const m = own ? await backend.models(f) : [];
      if (f === file) models = m;
    } catch {
      if (f === file) models = [];
    }
  }
  let docs = $state.raw<Documentation | null>(null); // S232: 説明パネルの Documentation
  let icon = $state.raw<Icon | null>(null); // S244: 説明パネルの上のアイコン
  let graphModel = $state<string | null>(null); // 「グラフ」を押したときのモデル名（入力のたびに描き直さない）

  // S141: 自分のファイルを開く・保存する。本文とファイル名は localStorage に自動保存して起動時に戻す（使えなくても動く。S102）。モデル名は戻さず本文から入れ直す（smoke が書き換えたモデル名を次の起動で拾わないため）
  // own はエディタの文書が自分のファイルか（ライブラリのファイルを開いている間は保存も上書きもしない）
  const STORE = "globe-mode.doc"; // S141 の 1 文書（S151 からは TABS。読むだけ）
  const TABS = "globe-mode.tabs", RECENT = "globe-mode.recent", CAP = 2 * 2 ** 20; // 本文の合計の上限（字数。UTF-16 の長さで数える近似）
  const LONG = "model Long\n  Real x(start = 0, fixed = true);\nequation\n  der(x) = sin(100 * time);\n  annotation(experiment(StopTime = 1e6));\nend Long;\n"; // S147: smoke の「止める」用。止めるまで終わらない長さ
  const DECAY = "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
  const WELCOME = "globe-mode.welcome", smokeMode = import.meta.env.VITE_GM_SMOKE === "1" || new URLSearchParams(location.search).has("smoke");
  let welcome = $state(false); // S179: 起動直後の案内（保存したタブが無い初回だけ。閉じたことは localStorage）
  const closeWelcome = () => { welcome = false; store(WELCOME, 1); view.focus(); };
  let rev = $state(0); // 本文か保存済みの本文が変わった合図（タブの未保存の印がこれを読む。view は $state でない）
  let own = $state(true);
  let saved = $state(""); // 保存した先（Desktop はパス、ブラウザは名前）
  const store = (key: string, v: unknown) => {
    try { localStorage.setItem(key, JSON.stringify(v)); return true; } catch { return false; } // 保存できなくても使える。persist は失敗を利用者に言う
  };
  let storeNote = $state(""); // S196: 自動保存（localStorage）が落ちたとき（書けない・上限で古いタブを落とした）の 1 文
  let otherWindow = $state(false); // S196: 別のウィンドウが同じ保存を書いた（保存は最後に書いたほうだけが残る）
  let roHit = $state(false); // S196: 読み取り専用のタブに打とうとした
  const RO_MSG = "このタブはライブラリのファイルで読み取り専用。手元に写すには「保存」（名前を付けて書き出す）。";
  // 合計が CAP を超えたら、keep 以外を古い（先頭）ものから落とす
  function cap<T extends { text?: string; clean?: string }>(xs: T[], keep: T): T[] {
    const out = [...xs];
    const len = (x: T) => (x.text?.length ?? 0) + (x.clean?.length ?? 0);
    let n = out.reduce((a, x) => a + len(x), 0);
    for (let i = 0; n > CAP && i < out.length; ) {
      if (out[i] === keep) i++;
      else n -= len(out.splice(i, 1)[0]);
    }
    return out;
  }

  // S151: タブ。いまの文書の file / model / own / diagnostics / saved はこれまでどおり大域の変数で、離れるときに tab へ写す（state は EditorState。元に戻す履歴も文書ごと）
  type Tab = { file: string; model: string; own: boolean; state: EditorState; diagnostics: Diagnostic[]; saved: string; clean: string; path?: string; results?: ResultColumns }; // results: 開いた結果のファイル（.mat・.csv。S222）のタブ。本文は無く、プロットだけを持つ。S236: Desktop は path（結果のファイルのパス）を残し、再読み込みで読み直す。 // path: Desktop でこのタブが本文を受け取った（開いた・保存した）ファイルのパス。保存はここに書く（S196）。 // clean: 開いた・保存した本文（閉じるとき、違えば確かめる。ライブラリは見ない）
  let tabs = $state.raw<Tab[]>([]);
  let cur = $state(0);
  const isResults = $derived(!!tabs[cur]?.results);
  const resultParams = $derived(isResults ? (columns as ResultColumns | null)?.parameters ?? [] : []); // S236: MAT のパラメータ（折りたたみの表）
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
  // S196: 打鍵ごとの書き込み（全タブの文字列化と同期の localStorage）は persistSoon で 300 ms まとめる。閉じる・保存・タブの切り替えと、ページを離れる前（pagehide・隠れたとき・beforeunload）は persist ですぐ書く
  let persistTimer: ReturnType<typeof setTimeout> | undefined;
  const persistSoon = () => { clearTimeout(persistTimer); persistTimer = setTimeout(persist, 300); };
  const flush = () => { if (persistTimer) persist(); }; // 遅らせた書き込みがあるときだけ（何も編集していない初回の状態は保存しない。初回の案内が出続ける）
  const persist = () => {
    clearTimeout(persistTimer);
    persistTimer = undefined;
    if (!view || !tabs.length) return;
    const docs = tabs.flatMap((t, i) => { // ライブラリのタブは本文を文字列にしない
      const a = i === cur;
      if (t.results && t.path) return [{ file: a ? file : t.file, model: t.model, text: "", a, clean: undefined, results: t.path }]; // S236: 結果のファイルのタブはパスだけ（列は再読み込みで読み直す。FMU の結果は path が無く残さない）
      if (!(a ? own : t.own)) return [];
      const text = (a ? view.state : t.state).doc.toString();
      return [{ file: a ? file : t.file, model: a ? model : t.model, text, a, clean: t.clean === text ? undefined : t.clean, results: undefined as string | undefined }]; // S179: 保存済みの本文は、編集中（本文と違う）のときだけ持つ。再読み込みのあとも未保存の印が正しい
    });
    const kept = cap(docs, docs.find((t) => t.a) ?? docs[0]);
    const ok = store(TABS, { active: Math.max(0, kept.findIndex((t) => t.a)), tabs: kept.map(({ file, text, model, clean, results }) => ({ file, text, model, clean, results })) }); // S170: モデル名もタブごとに残す
    storeNote = !ok ? "自動保存できない（ブラウザの保存領域に書けない）。閉じる前に「保存」でファイルに書き出す" : kept.length < docs.length ? "自動保存の上限を超えたので、古いタブの本文は再読み込みで戻らない。「保存」でファイルに書き出す" : "";
  };
  const guessModel = (t: string) => /^\s*(?:(?:partial|encapsulated)\s+)*(?:model|block|class|package)\s+(\w+)/m.exec(t)?.[1];

  const mkState = (doc: string, readOnly = false) =>
    EditorState.create({
      doc,
      extensions: [
        lineNumbers(), history(), drawSelection(), highlightActiveLine(), bracketMatching(), keymap.of([...defaultKeymap, ...historyKeymap]),
        modelicaHighlight, modelicaCompletion(), modelicaLinter(backend), // S053（S163: 色付け）
        modelicaDefinition(definition, goBack, { backend, file: () => file }), // S138（S149: ホバー）
        modelicaFolding(backend, () => file), // S225: 折りたたみ（範囲は Rust）
        modelicaDocs(backend, () => file, (d, i) => ((docs = d), (icon = i))), // S232: 説明パネル（カーソルのクラスの Documentation。引くのは Rust）
        modelicaRefactor(references, askRename), // S161: Shift+F12 と F2
        modelicaTab, // S188: Tab は字下げ。出る手は Esc のあと Tab
        modelFollow((from, to) => { if (model === from) model = to; }), // S186: クラス名のリネーム（と元に戻す）でモデル名も追う
        EditorView.updateListener.of((u) => { if (u.docChanged) { persistSoon(); rev++; } }),
        readOnlySlot(readOnly, () => { roHit = true; live = RO_MSG; }), // S196: ライブラリのタブ
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

  $effect(() => { void model; untrack(() => { if (view) persist(); }); }); // S170: モデル名を直したら残す

  onMount(() => {
    let docs: { file: string; text: string; model?: string; clean?: string; results?: string }[] = [];
    let active = 0;
    let closed = true; // S179: 案内を閉じたか。読めなければ閉じたことにする（毎回は出さない）
    try {
      const d = JSON.parse(localStorage.getItem(TABS) ?? "null");
      const old = JSON.parse(localStorage.getItem(STORE) ?? "null"); // S141 の保存
      docs = d?.tabs ?? (old ? [{ file: old.name, text: old.text }] : []);
      docs = docs.filter((t) => typeof t?.text === "string" && typeof t.file === "string");
      active = Number(d?.active) || 0;
      closed = localStorage.getItem(WELCOME) !== null;
      const r = JSON.parse(localStorage.getItem(RECENT) ?? "[]");
      if (Array.isArray(r)) recent = r.filter((x) => typeof x?.path === "string");
    } catch { /* 無ければ Decay から */ }
    welcome = !smokeMode && !closed && !docs.length; // S179: 保存したタブが無い初回だけ案内を出す（試験の smoke では出さない）
    if (!docs.length) docs = [{ file, text: DECAY }];
    if (docs.every((t) => typeof t.results === "string")) docs.push({ file, text: DECAY }); // S236: 結果のタブが読み直せなくて閉じても、タブが残るように
    tabs = docs.map((t) => typeof t.results === "string" ? { file: t.file, model: typeof t.model === "string" ? t.model : model, own: false, state: mkState("", true), diagnostics: [], saved: "", clean: "", path: t.results, results: { names: [], points: 0, data: [[]] } } : ({ file: t.file, model: (typeof t.model === "string" && t.model && t.text.includes(t.model.split(".").pop()!) ? t.model : guessModel(t.text)) ?? model, own: true, state: mkState(t.text), diagnostics: [], saved: "", clean: typeof t.clean === "string" ? t.clean : t.text })); // S179: 保存済みの本文を持っていなければ（初回・S151 までの保存）戻した本文を保存済みとみなす
    cur = Math.min(Math.max(0, active), tabs.length - 1);
    file = tabs[cur].file;
    model = tabs[cur].model; // S170: 残したモデル名が本文に無ければ（本文を書き換えた・古い保存）、本文の最初のクラス
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
    void refreshModels();
    // S236: 結果のファイルのタブは、保存したパスから読み直す（読めなければ閉じて理由を出す）
    for (const t of tabs) if (t.results && t.path) {
      void backend.readResultsAt?.(t.path).then(
        (c) => { t.results = c; if (tabs[cur] === t) columns = c; },
        (e) => { const i = tabs.indexOf(t); failure = `${baseName(t.path!)} を読み直せなかったので、そのタブを閉じた: ${String((e as Error).message ?? e)}`; if (i >= 0) { t.results = undefined; closeTab(i); } },
      );
    }
    // S100: Android のエミュレータの smoke（android.yml が logcat で GM-SMOKE の行を待つ）。クエリを渡せない Tauri の Android はビルド時の VITE_GM_SMOKE=1
    if (smokeMode) void smoke();
    else void tauri?.bench().then((on) => { if (on) { welcome = false; void smoke(); } }); // S125: Desktop の測定（GLOBE_DESKTOP_BENCH）
    return () => { flush(); view.destroy(); void unlisten?.then((f) => f()); };
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
      await tick(); // S228: モデル名を直した直後。設定の入れ替え（モデルごとの保存）を済ませてから欄を打つ
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
    const a = document.activeElement; // S186: キーボードで押したなら、読み終えたあとフォーカスを次の操作（ライブラリの検索欄）へ移す（ボタンは無効になりフォーカスが body に落ちる）
    const byKeyboard = a instanceof HTMLButtonElement && a.matches(":focus-visible");
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
      if (byKeyboard) {
        await tick();
        if (!document.activeElement || document.activeElement === document.body) document.querySelector<HTMLElement>(".library input")?.focus();
      }
    } catch (e) {
      msl = prev; // 取り消し・失敗のときは前の MSL のまま
      failure = String((e as Error).message ?? e);
    }
  }

  const bySeverity = (ds: Diagnostic[]) =>
    // S0…S10 の順、同じ段階では error > warning > info
    [...ds].sort((a, b) => Number(a.stage.slice(1)) - Number(b.stage.slice(1)) || severityRank[a.severity] - severityRank[b.severity]);

  async function run(job: () => Promise<void>) {
    if (busy) return; // S196: 実行中の検査・実行・木から開くは二重に動かさない（busy が早く戻り、結果が別のタブに書かれる）
    busy = true;
    failure = "";
    live = ""; // 同じ文が続いても、消してから入れ直すと読まれる
    try {
      if (!isResults) {
        await backend.setFile(file, view.state.doc.toString());
        await refreshModels(false);
        strayLib = "";
        if (own && models.length && !models.includes(model) && msl === "loaded") await backend.librarySource(model).then(() => (strayLib = model), () => {});
      }
      await job();
    } catch (e) {
      failure = String((e as Error).message ?? e);
    } finally {
      busy = false;
    }
  }

  // S170: 実行・検査が終わったとき 1 回だけ、失敗の理由・結果（プロット、無ければ診断の表）を画面に入れる。もう見えていれば動かさず（nearest）、利用者が動かした後は引き戻さない
  async function reveal(sel: string) {
    await tick();
    if (sel) document.querySelector(sel)?.scrollIntoView({ block: "nearest" }); // 滑らかなスクロールにしない（prefers-reduced-motion も満たし、動いている間に押した操作とも重ならない）
  }

  const check = () => { const m = model; return run(async () => { // S196: 押した時点のモデル名（検査の途中で欄を打ち換えても、canForce・checkedModel は検査したモデル）
    const checked = await backend.check(m);
    diagnostics = bySeverity(checked.diagnostics);
    experiment = checked.experiment;
    unsupported = checked.unsupported ?? "";
    canForce = checked.can_force ? m : null;
    checkedModel = { model: m };
    view.dispatch(setDiagnostics(view.state, toCmDiagnostics(view.state.doc.toString(), diagnostics, file)));
  }).then(() => {
    if (!failure && !unsupported) live = `検査が完了した: error ${diagnostics.filter((d) => d.severity === "error").length} 件、warning ${diagnostics.filter((d) => d.severity === "warning").length} 件、診断は全部で ${diagnostics.length} 件`;
    return reveal(failure ? ".failure" : diagnostics.length ? ".diagnostics" : "");
  }); };

  // 整形（S053、doc/format-style.md）。文法が壊れていれば本文は変えず、理由を出す
  const format = () => run(async () => {
    if (!own) { failure = RO_MSG; return; } // S196: ライブラリのタブは読み取り専用（dispatch は readOnly を通り抜けるので、ここで断る）
    const text = view.state.doc.toString();
    const out = await backend.format(text);
    if (out !== text) view.dispatch({ changes: { from: 0, to: text.length, insert: out } });
  });

  // S173: 失敗の 1 行目（実行しなかった理由）の下に、最初の error の診断の文面を 1 行足す（同じ文なら足さない。未対応の要約があるときは原文を詳細に畳むので足さない）
  const firstError = (head: string, ds: Diagnostic[], summary?: string) => {
    const e = summary ? undefined : ds.find((d) => d.severity === "error");
    const line = e?.message.split("\n")[0].slice(0, 200);
    return e && line && !head.includes(line) ? `${head}\n${e.code} ${line}` : head;
  };

  const simulate = (force = false) => {
    const m = model; // S196: 押した時点のモデル名
    return run(async () => {
      // S173: 数でない欄は SimSettings が欄の近くに理由を出している（aria-invalid）。ここでは二重に出さず、その欄へ移る
      if (settingsProblem) { document.querySelector<HTMLElement>(".settings [aria-invalid=true]")?.focus(); return; }
      unsupported = "";
      canForce = null;
      runState = "running";
      runNote = "実行中";
      let out;
      try {
        out = await backend.simulateColumns({ model: m, settings: simSettings, force }, (p) => {
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
      lastFinal = Object.entries(r.final ?? {}).filter(([k]) => !isInternal(k)).slice(0, 8);
      unsupported = r.unsupported ?? "";
      canForce = r.can_force ? m : null;
      forced = force;
      columns = c.points > 0 ? c : null;
      // S157: 止めた実行はそこまでの列を出し、止めた旨を残す（wasm は SharedArrayBuffer のフラグ、Desktop は $/cancelRequest。どちらも GM-I11004 の普通の応答）
      const stopped = r.diagnostics.find((d) => d.code === "GM-I11004");
      if (stopped) { runState = "cancelled"; runNote = `止めた（${c.points > 0 ? `${stopped.message}、そこまでの ${c.points} 点を表示` : "結果は出ない"}）`; }
      if (r.refused) failure = firstError(r.refused, r.diagnostics, r.unsupported);
      else if (!r.ok && !r.diagnostics.length) failure = "実行に失敗した（診断なし）";
    }).then(() => {
      if (!failure && !settingsProblem && !unsupported && runState !== "cancelled") live = `実行が完了した: 信号 ${columns ? columns.names.filter((n) => !isInternal(n)).length : 0} 本、${columns?.points ?? 0} 点`;
      return reveal(settingsProblem ? ".settings .problem" : diagnostics.some((d) => d.code === "GM-E10004") ? ".settings .refusal" : failure ? ".failure" : columns ? ".plot" : diagnostics.length ? ".diagnostics" : "");
    });
  };

  // ライブラリの木で選んだクラスのファイルをタブに開き、モデル名をそのクラスにする（S127）
  const openLibrary = (e: Pick<TreeEntry, "path">) => run(async () => {
    const src = await backend.librarySource(e.path);
    openDoc(src.file, src.text, e.path, false);
    if (src.range) view.dispatch({ selection: { anchor: offsetOf(src.range.start) }, scrollIntoView: true }); // S186: 入れ子のクラスはファイルの途中にある。宣言の行にカーソルを置いて画面に入れる
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
    if (busy) return; // S196: 実行中にタブを切り替えない（結果が別のタブに書かれる）
    failure = "";
    try {
      await backend.setFile(file, view.state.doc.toString());
      const d = await backend.definition(file, line, character);
      if (!d) return;
      if (d.file !== file) {
        back.push(file);
        openDoc(d.file, d.text, undefined, false);
      }
      moveTo(offsetOf(d.range.start), offsetOf(d.range.end));
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
    if (!own) { failure = RO_MSG; return; } // S196
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
      const spec = { changes: edits.map((e) => ({ from: offsetOf(e.range.start), to: offsetOf(e.range.end), insert: e.newText })) };
      // S186: リネームしたクラスがいまのモデル（かその途中の成分）なら、モデル名も同じ編集で追う（元に戻す Ctrl+Z も）。新しい名前は Rust の models が決める（順序はリネームで変わらないので、同じ番号の名前）
      const m = model, before = await backend.models(file), i = before.indexOf(m);
      let effects: ReturnType<typeof modelRenamed.of>[] = [];
      if (i >= 0) {
        await backend.setFile(file, view.state.update(spec).state.doc.toString());
        const to = (await backend.models(file))[i];
        if (to && to !== m) effects = [modelRenamed.of({ from: m, to })];
      }
      view.dispatch({ ...spec, effects });
      refs = [];
    } catch (e) {
      failure = String((e as Error).message ?? e);
    }
    view.focus();
  }
  function goBack() {
    if (busy) return; // S196
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
    // S196: 前のタブのものは持ち越さない（リネームの入力欄は前のタブの位置、参照の一覧の行は前の本文の位置、未対応・強行・最後の値は前のタブの実行の結果）
    refs = []; renaming = null; unsupported = ""; canForce = null; forced = false; lastFinal = []; strayLib = ""; roHit = false;
    const t = tabs[i];
    if (prev.own && prev.file !== t.file) void backend.setFile(prev.file, "").catch(() => {}); // 離れた自分のファイルは Workspace から外す（同じクラスの二重定義を避ける。S141）
    cur = i;
    ({ file, model, own, diagnostics, saved } = t);
    columns = t.results ?? null; // S222: 結果のファイルのタブはプロットを持つ
    graphModel = null;
    tabs = [...tabs];
    view.setState(t.state);
    persist();
    void refreshModels();
  }
  // 文書 f を開く。いまの文書なら本文を差し替え、同じ名前のタブがあればそこへ、無ければ新しいタブにする。自分のファイル（isOwn）は渡された本文が正、ライブラリのファイルは開いているタブのまま
  // S170: 未保存の編集があるタブと同じ名前の自分のファイルを開くときは、本文を差し替えず（編集を黙って失わない）そのタブへ切り替えて理由を出す
  const kept = (f: string) => `${baseName(f)} は開いていて未保存の編集がある。ディスクの内容は読み込まず、そのタブのままにした（読み直すにはタブを閉じてから開く）`;
  function openDoc(f: string, text: string, m: string | undefined, isOwn: boolean, path?: string) {
    if (f === file) {
      if (isOwn && isDirty(cur)) { saved = kept(f); return; }
      if (isOwn) { if (!own) makeEditable(view); own = true; tabs[cur].path = path; replaceDoc(text); diagnostics = []; columns = null; graphModel = null; saved = ""; tabs[cur].clean = text; persist(); void refreshModels(); }
      if (m) model = m;
      return;
    }
    let i = tabs.findIndex((t, j) => j !== cur && t.file === f);
    if (i < 0) {
      tabs = [...tabs, { file: f, model: m ?? model, own: isOwn, state: mkState(text, !isOwn), diagnostics: [], saved: "", clean: text, path }];
      i = tabs.length - 1;
    } else {
      if (isOwn && isDirty(i)) { activate(i); saved = kept(f); return; }
      if (isOwn) Object.assign(tabs[i], { own: true, state: mkState(text), diagnostics: [], saved: "", clean: text, path });
      if (m) tabs[i].model = m;
    }
    activate(i);
  }
  // S170: 自分のファイルのタブが、開いた・保存した本文から変わっているか（閉じる・開き直すときの確認に使う）
  // ライブラリのタブは読み取り専用で変わらないので、未保存にならない
  const isDirty = (i: number) => (i === cur ? own : tabs[i].own) && (i === cur ? view.state.doc.toString() : tabs[i].state.doc.toString()) !== tabs[i].clean;
  function closeTab(i: number) {
    if (tabs.length < 2) return;
    if (isDirty(i) && !confirm(`${baseName(i === cur ? file : tabs[i].file)} は保存していない。閉じる？`)) return;
    if (i === cur) activate(i ? i - 1 : 1);
    const here = tabs[cur];
    tabs = tabs.filter((_, j) => j !== i);
    cur = tabs.indexOf(here);
    persist();
  }

  // S228: タブの並べ替え（ドラッグ&ドロップと Ctrl+Shift+PageUp / PageDown）。cur は動かしたあとも同じタブを指し、順序は persist（300 ms）で残る
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  function moveTab(from: number, to: number) {
    if (busy || from === to || !tabs[from] || !tabs[to]) return;
    const here = tabs[cur], moved = tabs[from];
    const next = [...tabs];
    next.splice(to, 0, next.splice(from, 1)[0]);
    tabs = next;
    cur = next.indexOf(here);
    persistSoon();
    live = `${baseName(moved === here ? file : moved.file)} を ${to + 1} 番目へ移した（全 ${next.length} 枚）`;
  }

  async function tabKeys(e: KeyboardEvent) {
    if ((e.target as HTMLElement).getAttribute("role") !== "tab" || e.altKey || e.ctrlKey || e.metaKey || busy) return;
    const n = tabs.length;
    const i = { ArrowRight: (cur + 1) % n, ArrowLeft: (cur + n - 1) % n, Home: 0, End: n - 1 }[e.key];
    if (i === undefined) return;
    e.preventDefault();
    activate(i);
    await tick();
    document.querySelectorAll<HTMLElement>("[role=tab]")[i]?.focus();
  }

  // S141: 開く。S151 から新しいタブに開く（同じ名前のタブがあればその本文を差し替える）。Workspace の二重定義は activate が避ける
  // S196: Desktop は path を受け取り、同じ名前の別のファイルは別の key のタブにして、保存先はタブの path で決める（tabKey）
  const openOwn = (name: string, text: string, path?: string) => openDoc(tabKey(name, path, tabs), text, guessModel(text), true, path);
  // S222: 結果のファイル（.mat・.csv）は読んだ列を持つタブに開く（同じファイルならそのタブの列を差し替える）。エディタは隠し、プロットは実行の結果と同じ
  function openResults(name: string, c: ResultColumns, path?: string) {
    const key = `結果 ${name}`;
    if (key === file) { tabs[cur].results = c; tabs[cur].path = path; columns = c; persistSoon(); return; }
    let i = tabs.findIndex((t, j) => j !== cur && t.file === key);
    if (i < 0) {
      tabs = [...tabs, { file: key, model: name.replace(/\.[^.]*$/, ""), own: false, state: mkState("", true), diagnostics: [], saved: "", clean: "", results: c, path }];
      i = tabs.length - 1;
    } else Object.assign(tabs[i], { results: c, path });
    activate(i);
  }
  // S229: FMU（Desktop だけ）。要約と警告で確かめてから、sidecar のワーカー（別プロセス）で回し、結果は結果のタブに出す。「止める」でワーカーを kill する
  // S236: 確認ダイアログの代わりに、警告と実行の設定（FmuForm。FMU のパスごとに保つ）を出す。実行かやめるまで待つ
  let fmuAsk = $state<{ name: string; path: string; info: FmuInfo; settle: (r: FmuRunRequest | null) => void } | null>(null);
  async function openFmu(name: string, path?: string) {
    if (!path || !backend.fmuInfo || !backend.runFmu) { failure = `${name} を回せるのは Desktop だけ（ブラウザ・Android はネイティブコードを動かせない）`; return; }
    const info = await backend.fmuInfo(path);
    remember(path, "");
    const request = await new Promise<FmuRunRequest | null>((settle) => (fmuAsk = { name, path, info, settle }));
    fmuAsk = null;
    if (!request) return;
    runState = "running";
    runNote = "FMU を実行中";
    try {
      openResults(name, await backend.runFmu(path, request, (t) => { runNote = `FMU を実行中: t = ${t.toPrecision(4)}`; }));
    } catch (e) {
      if (!(e as { cancelled?: boolean }).cancelled) throw e;
      runState = "cancelled";
      runNote = "止めた（FMU の結果は出ない）";
    } finally {
      if (runState === "running") { runState = "idle"; runNote = ""; }
    }
  }
  const openFromDisk = () => run(async () => {
    const f = await backend.openFile();
    if (!f) return;
    if (isFmuFile(f.name)) return openFmu(f.name, f.path);
    if (f.columns) { if (f.path) remember(f.path, ""); return openResults(f.name, f.columns, f.path); } // S236: Desktop は最近使ったファイルにも入れる
    openOwn(f.name, f.text, f.path);
    remember(f.path ?? f.name, f.text);
  });
  const openRecent = (path: string) => run(async () => {
    // S236: 結果と FMU の最近使ったファイル（Desktop だけ。読むのは sidecar）
    if (desktop && isResultsFile(path) && backend.readResultsAt) { openResults(baseName(path), await backend.readResultsAt(path), path); return remember(path, ""); }
    if (desktop && isFmuFile(path)) return openFmu(baseName(path), path);
    const r = recent.find((x) => x.path === path);
    const f = r?.text !== undefined ? { name: path, text: r.text } : await backend.openPath?.(path);
    if (f) openOwn(f.name, f.text, desktop ? path : undefined);
  });
  // S144: ソースの FMU。ブラウザはダウンロード、Desktop は保存先のフォルダへ sidecar が書く。S154: FMI の版（2.0 / 3.0）を選べる
  let fmiVersion = $state(2);
  let fmuRuntime = $state<"p5a" | "p5b">("p5a"); // S239: P5b は Desktop だけ（cdylib を sidecar が cargo で作る。ブラウザ・Android には無い）
  const FMU_BUILD = "globe fmu export --build"; // S173: バイナリ入りは CLI だけ（cc か zig cc が要る。S144 の書き出しは build 無し）
  const exportFmu = () => run(async () => {
    const to = await backend.exportFmu(model, fmiVersion, fmuRuntime);
    if (to) saved = fmuRuntime === "p5b"
      ? `FMU を書き出した: ${to}。自前の実行時（P5b）の共有ライブラリ入りで、この OS 用だけ`
      : `FMU を書き出した: ${to}。ソースだけでバイナリは入っていない（Desktop も同じ）。他のツールで読むには CLI の ${FMU_BUILD} で作る`;
  });
  async function save(saveAs = false) {
    failure = "";
    if (isResults) { failure = "結果のファイルのタブは保存できない（プロットの CSV・PNG の書き出しを使う）"; return; }
    try {
      // #70: 開いた・保存したあとに外（別のエディタ・git）で書き換わっていたら、確かめてから上書きする。読めなければ（消えた）そのまま書く
      const at = own && desktop && !saveAs ? tabs[cur].path : undefined;
      if (at) {
        const disk = await backend.openPath?.(at).then((f) => f.text, () => undefined);
        if (disk !== undefined && disk !== tabs[cur].clean && !confirm(`${baseName(file)} は開いたあとに外で書き換わっている。上書きする？`)) return;
      }
      const to = await backend.saveFile(baseName(file), view.state.doc.toString(), saveAs, own ? tabs[cur].path : undefined);
      if (to) {
        saved = `保存した: ${to}`;
        if (own) { if (desktop) tabs[cur].path = to; remember(to, view.state.doc.toString()); tabs[cur].clean = view.state.doc.toString(); rev++; persist(); }
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
    const at = byteToChar(view.state.doc.toString());
    view.dispatch({ selection: { anchor: at(d.range.start), head: at(d.range.end) }, scrollIntoView: true });
    view.focus();
  }

  // S175: ショートカット。Ctrl（macOS は Cmd）+ Enter 実行、+ Shift+Enter 検査、+ . 止める、+ S 保存（Desktop は + Shift+S 名前を付けて保存）、+ O 開く。
  // ブラウザが握る Ctrl+R・W・F5 などは使わない。修飾キーつきなので入力欄で文字を打っていても誤爆しない。エディタの中でも効くよう capture で受け、CodeMirror の Mod-Enter（空行を挿入）は覆う
  function shortcut(e: KeyboardEvent) {
    // S228: Esc で実行を止める（Ctrl+. と同じ）。入力欄・選択・ダイアログの中では奪わず、補完の一覧が開いていれば CodeMirror が閉じるだけ（応答待ちで見えていないものは数えない。実行中の Worker は答えない）
    if (e.key === "Escape" && !e.isComposing && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey && runState === "running" && completionStatus(view.state) !== "active" && !(e.target as HTMLElement).closest?.("input, textarea, select, dialog, [role=dialog]")) backend.cancel?.();
    if (e.isComposing || e.repeat || e.altKey || !(e.ctrlKey || e.metaKey)) return;
    const idle = (f: () => unknown) => () => { if (!busy) void f(); };
    const act = {
      "Enter": idle(() => simulate()),
      "Shift+Enter": idle(check),
      ".": () => { if (runState === "running") backend.cancel?.(); },
      "Shift+PageUp": idle(() => moveTab(cur, cur - 1)), // S228: タブを左へ・右へ
      "Shift+PageDown": idle(() => moveTab(cur, cur + 1)),
      "s": () => void save(),
      "Shift+s": desktop ? () => void save(true) : undefined,
      "o": idle(openFromDisk),
    }[(e.shiftKey ? "Shift+" : "") + (e.key.length === 1 ? e.key.toLowerCase() : e.key)];
    if (!act) return;
    e.preventDefault();
    e.stopPropagation();
    closeCompletion(view); // S186: 候補の一覧が開いたまま実行しても、終わったあとに残らない
    act();
  }
</script>

<!-- S179: 保存していない編集があるときだけ、ブラウザのタブを閉じる・再読み込みの前に確かめる。Tauri（Desktop・Android）と試験の smoke では付けない -->
<!-- S196: 離れる前に、遅らせた自動保存をすぐ書く（pagehide・隠れたとき・beforeunload）。別のウィンドウが同じ保存を書いたら知らせる -->
<svelte:window onkeydowncapture={shortcut} onbeforeunload={(e) => { flush(); if (!tauri && !smokeMode && tabs.some((_, i) => isDirty(i))) e.preventDefault(); }} onpagehide={flush} onstorage={(e) => { if (e.key === TABS) otherWindow = true; }} />
<svelte:document onvisibilitychange={() => { if (document.hidden) flush(); }} />

<main>
  <!-- S179: スキップリンク（Tab 1〜2 回でエディタか結果へ。フォーカスで見える） -->
  <nav class="skip" aria-label="ページ内の移動">
    <a href="#editor" onclick={(e) => { e.preventDefault(); view.focus(); }}>エディタへ</a>
    <a href="#results" onclick={(e) => { e.preventDefault(); const h = document.getElementById("results"); h?.focus({ preventScroll: true }); h?.scrollIntoView({ block: "start" }); }}>結果へ</a>
  </nav>
  <h1 class="sr-only">GlobeMode</h1>
  <div class="bar">
    <button onclick={openFromDisk} disabled={busy} title="開く（Ctrl/Cmd+O）" aria-keyshortcuts="Control+O Meta+O">開く</button>
    {#if recent.length}
      <select aria-label="最近使ったファイル" disabled={busy} onchange={(e) => { const p = e.currentTarget.value; e.currentTarget.value = ""; if (p) void openRecent(p); }}>
        <option value="">最近使ったファイル</option>
        {#each [...recent].reverse() as r}<option value={r.path} title={r.path}>{baseName(r.path)}</option>{/each}
      </select>
    {/if}
    <button onclick={() => save()} title="保存（Ctrl/Cmd+S）" aria-keyshortcuts="Control+S Meta+S">保存</button>
    {#if desktop}<button onclick={() => save(true)} title="名前を付けて保存（Ctrl/Cmd+Shift+S）" aria-keyshortcuts="Control+Shift+S Meta+Shift+S">名前を付けて保存</button>{/if}
    <button onclick={exportFmu} disabled={busy} aria-describedby="fmu-note">FMU を書き出す</button>
    <select aria-label="FMI の版" bind:value={fmiVersion}><option value={2}>FMI 2.0</option><option value={3}>FMI 3.0</option></select>
    {#if desktop}<select aria-label="実行時" bind:value={fmuRuntime}><option value="p5a">P5a（Rumoca の C のソース）</option><option value="p5b">P5b（自前の実行時。cargo が要る）</option></select>{/if}
    <small id="fmu-note" data-fmu-note>FMU はソースだけ（バイナリ無し）。他のツールで読むには CLI の <code>{FMU_BUILD}</code></small>
    <label>モデル名 <input bind:value={model} aria-label="モデル名"></label>
    {#if models.length > 1}
      <select aria-label="ファイルのモデル" value={models.includes(model) ? model : ""} onchange={(e) => { if (e.currentTarget.value) model = e.currentTarget.value; }}>
        {#if !models.includes(model)}<option value="">（このファイルのモデルから選ぶ）</option>{/if}
        {#each models as m}<option value={m}>{m}</option>{/each}
      </select>
    {/if}
    <select aria-label="テーマ" value={theme.pref} onchange={(e) => setPref(e.currentTarget.value as Pref)}>
      <option value="auto">OS に合わせる</option>
      <option value="light">明</option>
      <option value="dark">暗</option>
    </select>
    <button onclick={check} disabled={busy} title="検査（Ctrl/Cmd+Shift+Enter）" aria-keyshortcuts="Control+Shift+Enter Meta+Shift+Enter">検査</button>
    <button onclick={() => simulate()} disabled={busy} title="実行（Ctrl/Cmd+Enter）" aria-keyshortcuts="Control+Enter Meta+Enter">実行</button>
    <button onclick={graph} disabled={busy}>グラフ</button>
    <button onclick={format} disabled={busy}>整形</button>
    {#if runState === "running" && backend.cancel}<button onclick={() => backend.cancel?.()} title="止める（Ctrl/Cmd+.）" aria-keyshortcuts="Control+. Meta+.">止める</button>{/if}
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
  {#if welcome}
    <!-- S179: 初回の案内。ショートカットは各ボタンの aria-keyshortcuts と同じキー -->
    <section class="welcome" aria-label="はじめに">
      <strong>はじめに</strong>
      <ol>
        <li>モデルを書く（いまは例の Decay）か、「開く」（Ctrl/Cmd+O）でファイルを開く{#if msl === "none"}。MSL のモデルを使うなら{desktop ? "「MSL を取得する」" : "「MSL を読む」"}を押してライブラリの木から選ぶ{/if}</li>
        <li>「検査」（Ctrl/Cmd+Shift+Enter）で診断を見る</li>
        <li>「実行」（Ctrl/Cmd+Enter）で結果のプロットを見る</li>
      </ol>
      <button onclick={closeWelcome}>閉じる</button>
    </section>
  {/if}
  <!-- S170: 進捗・失敗の理由・モデル名の食い違いは、押したボタンのすぐ下（画面の上）に出す。下に置くとスクロールしないと見えない -->
  <p class="sr-only" aria-live="polite" aria-atomic="true">{live}</p>
  {#if runNote}<p class="run" role={runState === "running" ? undefined : "status"} data-run={runState}>{runNote}</p>{/if}
  {#if failure}<p class="failure" class:unsup={!!unsupported} role="alert">{failure}</p>{/if}
  {#if canForce === model}<p class="note" data-can-force><button onclick={() => simulate(true)} disabled={busy}>それでも実行する</button> 検査の Error を無視して実行する（強行）。結果は信頼できないことがある。</p>{/if}
  {#if forced}<p class="note forced" role="status" data-forced>強行した実行: 検査の Error を無視して実行した結果。信頼できないことがある。</p>{/if}
  {#if strayLib === model && own && !models.includes(model)}<p class="note" role="status" data-stray>モデル「{model}」はライブラリのもので、エディタの {baseName(file)} には無い。<button onclick={() => openLibrary({ path: model })} disabled={busy}>ライブラリのファイルを開く</button></p>{/if}
  <SimSettings {backend} {model} {experiment} errors={diagnostics.filter((d) => d.code === "GM-E10004")} checked={checkedModel} last={lastSettings} bind:settings={simSettings} bind:problem={settingsProblem} />
  {#if tauri}<p class="note" data-config>
    <button onclick={async () => { configNote = ""; try { await tauri.reloadConfig(); configNote ||= "globe-mode.toml を読み直した（無ければ既定）"; } catch (e) { configNote = String(e); } }}>設定を読み直す</button> {configNote}
  </p>{/if}
  <p class="note" data-msl={msl}>
    {#if msl === "loaded"}{mslNote}。左の木でモデルを選ぶか、モデル名にライブラリのモデル（例 Modelica.Electrical.Analog.Examples.ChuaCircuit）を入れて検査・実行できる。グラフも描ける。
    {:else if msl === "loading"}{desktop ? `MSL ${mslVersion} を取得して展開している（${mslProgress || "回線しだい"}）。もう一度押すと取り消す。` : "MSL.json を取得して読み込んでいる（十数秒）。"}
    {:else}MSL は未読み込み。{/if}
  </p>
  <!-- S179: 矢印キー（左右・Home・End）でタブを移る（roving tabindex。選ぶと開く）。Delete や Ctrl+W は足さない（ブラウザが握る） -->
  <div class="tabs" role="tablist" aria-label="開いているファイル" tabindex="-1" onkeydown={tabKeys}>
    {#each tabs as t, i (t)}
      {@const f = i === cur ? file : t.file}
      {@const label = (i === cur ? own : t.own) ? f : baseName(f)} <!-- 自分のファイルは key（同じ名前の別のファイルは「フォルダ/名前」）、ライブラリは末尾 -->
      {@const dirty = rev >= 0 && isDirty(i)} <!-- rev は再計算の合図 -->
      <span
        class="tab" role="presentation" class:active={i === cur} class:over={dragOver === i && dragFrom !== i} draggable={!busy && tabs.length > 1}
        ondragstart={(e) => { dragFrom = i; e.dataTransfer?.setData("text/plain", String(i)); if (e.dataTransfer) e.dataTransfer.effectAllowed = "move"; }}
        ondragover={(e) => { if (dragFrom !== null) { e.preventDefault(); dragOver = i; } }}
        ondrop={(e) => { e.preventDefault(); if (dragFrom !== null) moveTab(dragFrom, i); dragFrom = dragOver = null; }}
        ondragend={() => (dragFrom = dragOver = null)}
      >
        <button role="tab" aria-selected={i === cur} tabindex={i === cur ? 0 : -1} title={f} aria-keyshortcuts="Control+Shift+PageUp Control+Shift+PageDown" disabled={busy} onclick={() => activate(i)}>{#if dirty}<span class="dirty" role="img" aria-label="未保存">●</span> {/if}{label}</button>
        {#if tabs.length > 1}<button aria-label="{label} を閉じる" disabled={busy} onclick={() => closeTab(i)}>×</button>{/if}
      </span>
    {/each}
  </div>
  {#if fmuAsk}<FmuForm name={fmuAsk.name} path={fmuAsk.path} info={fmuAsk.info} onrun={(r) => fmuAsk?.settle(r)} oncancel={() => fmuAsk?.settle(null)} />{/if}
  {#if isResults}<p class="note" data-results>開いた結果のファイル: {file}（{columns?.points ?? 0} 点、{columns?.names.length ?? 0} 信号）。プロットの CSV・PNG の書き出しはそのまま使える。実行・保存はできない。{tabs[cur]?.path ? "再読み込みでは同じファイルを読み直す。" : "再読み込みで閉じる（ファイルのパスを持たない）。"}</p>
    {#if resultParams.length}
      <details class="params" data-result-params>
        <summary>パラメータ（{resultParams.length} 個）</summary>
        <div class="rows"><table><tbody>
          {#each resultParams as p (p.name)}
            <tr data-param={p.name}><th scope="row">{p.name}</th><td>{p.value}</td><td>{p.unit ?? ""}</td><td><small>{p.description ?? ""}</small></td></tr>
          {/each}
        </tbody></table></div>
      </details>
    {/if}
  {:else if !own}<p class="note" class:hit={roHit} data-readonly>{RO_MSG}</p>{/if}
  {#if otherWindow}<p class="note" role="status" data-other-window>別のウィンドウでも開いている。自動保存は最後に編集したほうだけが残る。</p>{/if}
  {#if storeNote}<p class="failure" role="status" data-store-note>{storeNote}</p>{/if}
  <div class="work">
    {#if msl === "loaded"}<aside class="library"><LibraryTree {backend} onOpen={openLibrary} /></aside>{/if}
    <div class="editor" id="editor" hidden={isResults} bind:this={host}></div>
    <DocsPanel doc={docs} {icon} />
  </div>
  <p class="hint" id="editor-help">{EDITOR_HELP}</p>
  {#if saved}<p class="note" role="status" data-saved>{saved}</p>{/if}
  {#if lastFinal.length}<p class="final" data-final={JSON.stringify(Object.fromEntries(lastFinal))}>最後の時刻の値: {lastFinal.map(([k, v]) => `${k} = ${v.toPrecision(6)}`).join("、")}</p>{/if}
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
          <tr onclick={() => moveTo(r.from, r.to)}><td><button type="button" class="jump" aria-label="{r.n} 行目の参照へ移る">{r.n}</button></td><td>{r.text}</td></tr>
        {/each}
      </tbody>
    </table>
    <button type="button" onclick={() => (refs = [])}>参照を閉じる</button>
  {/if}
  <h2 id="results" class="sr-only" tabindex="-1">結果</h2>
  {#if unsupported}<p class="unsup-summary" role="status" data-unsupported><strong>未対応</strong> {unsupported}</p>{/if}
  <table class="diagnostics" aria-label="診断">
    <tbody>
      {#each diagnostics as d}
        {@const unsup = !!unsupported && d.severity === "error"}
        <tr class={unsup ? "unsup" : d.severity} onclick={() => jump(d)}>
          <td>{#if d.range && (!d.file || d.file === file)}<button type="button" class="jump" title="エディタの該当位置へ移る（Enter）">{d.code}</button>{:else}{d.code}{/if}</td><td>{d.stage}</td><td>{unsup ? "未対応" : d.severity}</td>
          <td>{#if unsup}<details><summary>Rumoca の原文</summary>{d.message}</details>{:else}{d.message}{/if}{#each d.notes ?? [] as n}<div class="dnote">{n}</div>{/each}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if columns}<Plot {columns} {model} />{/if}
  {#if graphModel}<Graph {backend} model={graphModel} onSelect={select} />{/if}
</main>

<style>
  .params .rows { max-height: 12rem; overflow-y: auto; }
  .params th { text-align: left; font-weight: normal; padding-right: .5rem; }
  main { font: 14px/1.4 system-ui, sans-serif; max-width: 76rem; margin: 0 auto; padding: 1rem; }
  .bar { display: flex; flex-wrap: wrap; gap: 1rem; margin-bottom: .5rem; }
  .note, .run, .final, .hint { color: var(--muted); margin: .25rem 0; }
  .tabs { display: flex; flex-wrap: wrap; gap: .25rem; margin-bottom: .25rem; }
  .skip a { position: absolute; left: -999px; top: .5rem; z-index: 10; padding: .25rem .5rem; background: var(--bg); color: var(--fg); border: 1px solid var(--accent); }
  .skip a:focus { left: .5rem; }
  .welcome { border: 1px solid var(--accent); padding: .25rem .75rem; margin: .5rem 0; }
  .welcome ol { margin: .25rem 0; padding-left: 1.5rem; }
  .dirty, .note.hit { color: var(--warn); }
  .tab.over { box-shadow: -3px 0 0 var(--accent); }
  .tab.active button[role=tab] { font-weight: bold; }
  .work { display: flex; gap: .5rem; }
  .library { flex: 0 0 16rem; height: 16rem; overflow: auto; border: 1px solid var(--border); font-size: .9em; }
  .editor[hidden] { display: none; }
  .editor { flex: 1; min-width: 0; border: 1px solid var(--border); }
  .editor :global(.cm-editor) { height: 16rem; }
  .diagnostics, .refs { width: 100%; border-collapse: collapse; margin-top: .5rem; }
  td { border-bottom: 1px solid var(--line); padding: .2rem .5rem; vertical-align: top; }
  .rename { margin: .5rem 0; }
  .dnote { color: var(--muted); font-size: .9em; }
  tr { cursor: pointer; }
  .jump { border: 0; background: none; color: inherit; font: inherit; padding: 0; cursor: pointer; } /* S175: 行の中の本物の button（Tab・Enter・Space で押せる）。見た目は文字のまま */
  tr.error { color: var(--err); }
  tr.warning { color: var(--warn); }
  tr.unsup, .unsup-summary, .failure.unsup { color: var(--unsup); }
  .unsup-summary { margin: .5rem 0 0; }
  .failure { color: var(--err); white-space: pre-wrap; }
  /* 小画面（S145）: 左の木を上に積み、診断の長い語を折り返し、横スクロールを出さない */
  @media (max-width: 600px) {
    main { padding: .5rem; }
    .work { flex-direction: column; }
    .library { flex: none; height: 10rem; }
    td { overflow-wrap: anywhere; }
  }
</style>
