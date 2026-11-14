import type { Backend, CompletionEntry, Definition, Diagram, Documentation, FmuInfo, FmuRunRequest, FmuRuntime, Hover, Icon, LintMessage, Progress, TreeEntry } from "./backend";
import type { Columns } from "./plot/csv";
import { isFmuFile, isResultsFile, toColumns, type ResultColumns, type ResultParam } from "./results.ts"; // node の型の除去（tests/test_tauri.mjs）が拡張子を要る
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse, SolverEntry, ModelParameter, TunableParameter } from "./protocol";

// Tauri が `withGlobalTauri` で注入する関数だけを使う（@tauri-apps/api は入れない。npm の依存を増やさないため）。
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>;
type Dialog = { open(o: object): Promise<string | null>; save(o: object): Promise<string | null> };
type Fs = { readTextFile(p: string): Promise<string>; writeTextFile(p: string, t: string): Promise<void>; writeFile(p: string, b: Uint8Array): Promise<void> };
const tauriGlobal = () => (window as unknown as { __TAURI__: { core: { invoke: Invoke }; event: { listen: Listen }; dialog: Dialog; fs: Fs } }).__TAURI__;
const OPEN = [{ name: "Modelica・結果・FMU", extensions: ["mo", "mat", "csv", "fmu"] }]; // S222: 結果のファイルも開く。S229: FMU も（確認のあと回して結果のタブに出す）
const extFilter = (name: string) => [{ name: "File", extensions: [name.split(".").pop() ?? ""] }]; // 保存ダイアログの拡張子の絞り込み
const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
/** 保存ダイアログで選んだファイルのフォルダ（Windows のドライブの根は `C:\`。`C:` だとドライブ相対になる。S196）。 */
export const dirOf = (p: string) => p.replace(/[\\/][^\\/]*$/, "").replace(/^[A-Za-z]:$/, "$&\\") || "/";
/**
 * 開いたファイルのタブの key（Workspace の名前でもある）。ふつうはファイル名。同じ名前で別のパスのタブがあれば `フォルダ名/ファイル名`、それも使われていれば `名前 (n)`。
 * 同じパスのタブがあればその key。パスの無い（保存前の）タブとは同じ名前で重ねる。保存先はタブが持つパスで決め、名前では決めない（S196）。
 */
export function tabKey(name: string, path: string | undefined, tabs: { file: string; path?: string }[]): string {
  if (!path) return name;
  const same = tabs.find((t) => t.path === path);
  if (same) return same.file;
  const dir = path.split(/[\\/]/).slice(-2, -1)[0];
  const free = (k: string) => { const t = tabs.find((t) => t.file === k); return !t || !t.path; };
  let key = [name, dir && `${dir}/${name}`].find((k) => k && free(k));
  for (let n = 2; !key; n++) if (free(`${name} (${n})`)) key = `${name} (${n})`;
  return key;
}
const invoke: Invoke = (cmd, args) => tauriGlobal().core.invoke(cmd, args);

/** `$/progress` の token（S136。実行は 1 度に 1 つなので固定）。 */
const RUN_TOKEN = "globe-run";

/** MSL の取得の進捗（イベント `msl-progress`）。total は Content-Length、分からなければ null。 */
export type MslProgress = { received: number; total: number | null };

/** 同梱の `globe` の sidecar（`globe lsp`、stdio の JSON-RPC。ADR-0023）。Rust 側（src/Rust/apps/desktop）は中継だけ。 */
export class TauriBackend implements Backend {
  #version = 0;

  // Desktop ではライブラリは sidecar の起動時の --library / GLOBE_LIBRARY で読む。JSON を送る道は無い（S056）。
  // 何も読んでいなければ、MSL 4.1.0 を取得して展開し、sidecar を作り直させる（S069、command `fetch_msl`。S084 から版を選べる）。JSON は捨てる。
  async library(_name: string): Promise<void> {
    const libs = await invoke<string[]>("libraries");
    if (libs.length === 0) await this.fetchMsl("4.1.0");
  }

  /** その版の MSL を取得し、sidecar をそれで作り直させる（S084。版の切り替えも同じ）。取得済みなら展開を省く。 */
  async fetchMsl(version: string): Promise<void> {
    await invoke("fetch_msl", { version });
  }
  /** 取得中の fetchMsl を止める（fetchMsl は取り消しのエラーで終わる）。 */
  cancelMsl(): Promise<void> {
    return invoke("cancel_msl");
  }
  onMslProgress(cb: (p: MslProgress) => void): Promise<() => void> {
    return tauriGlobal().event.listen<MslProgress>("msl-progress", (e) => cb(e.payload));
  }

  /** S125: UI の測定（`GLOBE_DESKTOP_BENCH`）。line なしは測定中か、line ありはその行を書いてアプリを終える。 */
  bench(line?: string): Promise<boolean> {
    return invoke("bench", { line: line ?? null });
  }

  /** globe-mode.toml を読み直す（S143）。誤りは `window/showMessage` で届く（onMessage）。 */
  reloadConfig(): Promise<void> {
    return invoke("reload_config");
  }
  /** sidecar の `window/showMessage`（設定ファイルの誤りなど）を受ける。 */
  onMessage(cb: (message: string) => void): Promise<() => void> {
    return tauriGlobal().event.listen<{ method?: string; params?: { message?: string } }>("globe-notification", (e) => {
      if (e.payload.method === "window/showMessage") cb(e.payload.params?.message ?? "");
    });
  }

  /** sidecar が既に何かライブラリを読んでいるか（起動時の MSL の表示用）。 */
  async hasLibrary(): Promise<boolean> {
    return (await invoke<string[]>("libraries")).length > 0;
  }

  /** `textDocument/didOpen` で文書を（差し替えて）開く。サーバは診断も出し直し、イベント `globe-notification` で届く。 */
  async setFile(name: string, text: string): Promise<void> {
    await invoke("notify", {
      method: "textDocument/didOpen",
      params: { textDocument: { uri: name, languageId: "modelica", version: ++this.#version, text } },
    });
  }
  check(model: string): Promise<CheckResponse> {
    return invoke("rpc", { method: "globe/check", params: { model } });
  }
  graph(model: string, depth: number): Promise<GraphJson> {
    return invoke("rpc", { method: "globe/graph", params: { model, depth } });
  }
  simulate(req: SimulateRequest): Promise<SimulateResponse> {
    return invoke("rpc", { method: "globe/simulate", params: { request: req } });
  }
  #onProgress?: (p: Progress) => void;
  #listening?: Promise<unknown>;
  #cancelling = false;
  // `$/progress`（イベント `globe-notification`）の value を onProgress に渡す口。実行は 1 度に 1 つなので 1 つだけ張る
  #listen(onProgress?: (p: Progress) => void): Promise<unknown> {
    this.#listening ??= tauriGlobal().event.listen<{ method?: string; params?: { token?: string; value?: Progress } }>("globe-notification", (e) => {
      const p = e.payload;
      if (p.method === "$/progress" && p.params?.token === RUN_TOKEN && p.params.value?.stage) this.#onProgress?.(p.params.value);
    });
    this.#onProgress = onProgress;
    this.#cancelling = false;
    return this.#listening;
  }
  // sidecar の `globe/simulateColumns`（S076）。列は JSON の数の配列で、有限でない値は既に null（uPlot の欠損）。
  // S136（ADR-0033）: workDoneToken を付け、サーバの `$/progress`（イベント `globe-notification`）の value を onProgress に渡す
  async simulateColumns(req: SimulateRequest, onProgress?: (p: Progress) => void): Promise<{ response: SimulateResponse; columns: Columns }> {
    await this.#listen(onProgress);
    try {
      const r = await invoke<{ response: SimulateResponse; layout: { names: string[]; points: number; defaults?: string[] }; columns: (number | null)[][] }>(
        "rpc",
        { method: "globe/simulateColumns", params: { request: req, workDoneToken: RUN_TOKEN } },
      );
      return { response: r.response, columns: { names: r.layout.names.slice(1), points: r.layout.points, defaults: r.layout.defaults, data: r.columns } };
    } catch (e) {
      // 止めても応えない sidecar は作り直される（command `cancel_run`）。UI には止めたとして返す
      if (this.#cancelling) throw Object.assign(new Error(String(e)), { cancelled: true });
      throw e;
    } finally {
      this.#onProgress = undefined;
    }
  }
  /** S229: FMU の要約（`globe fmu info` と同じ。ネイティブコードは実行しない）と、実行前に見せる警告。 */
  fmuInfo(path: string): Promise<FmuInfo> {
    return invoke("rpc", { method: "globe/fmuInfo", params: { path } });
  }
  /** S229: FMU を sidecar のワーカー（別プロセス）で回す。`cancel` で止められ（ワーカーを kill）、止めた呼び出しは cancelled で reject する。 */
  async runFmu(path: string, request: FmuRunRequest = {}, onProgress?: (t: number) => void): Promise<Columns> {
    await this.#listen((p) => p.time !== undefined && onProgress?.(p.time));
    try {
      const r = await invoke<{ layout: { names: string[]; points: number; defaults?: string[] }; columns: (number | null)[][] }>("rpc", { method: "globe/fmuRun", params: { path, request, workDoneToken: RUN_TOKEN } });
      return toColumns(r.layout, r.columns);
    } catch (e) {
      if (this.#cancelling) throw Object.assign(new Error(String(e)), { cancelled: true });
      throw e;
    } finally {
      this.#onProgress = undefined;
    }
  }
  /** 実行中の simulateColumns・runFmu を止める（S136）。サーバが止めれば途中までの列と GM-I11004 の普通の応答（FMU はワーカーを kill して cancelled で reject）、止まらなければ sidecar を作り直して cancelled で reject。 */
  cancel(): void {
    this.#cancelling = true;
    void invoke("cancel_run");
  }
  solvers(): Promise<SolverEntry[]> {
    return invoke("rpc", { method: "globe/solvers", params: {} }); // S142
  }
  async tunableParameters(model: string): Promise<TunableParameter[]> {
    const r = await invoke<{ parameters: TunableParameter[] | null }>("rpc", { method: "globe/tunableParameters", params: { model } }); // S150
    return r.parameters ?? [];
  }
  async parameters(model: string): Promise<ModelParameter[]> {
    return (await invoke<{ parameters: ModelParameter[] }>("rpc", { method: "globe/parameters", params: { model } })).parameters; // S248
  }
  // indent などを省くと params から落ち（JSON）、sidecar が globe-mode.toml（S143）の値を使う
  format(text: string, indent?: number, insertOrigin?: boolean): Promise<string> {
    return invoke("rpc", { method: "globe/format", params: { text, indent, insertOrigin } });
  }
  lint(text: string): Promise<LintMessage[]> {
    return invoke("rpc", { method: "globe/lint", params: { text } });
  }
  libraryTree(prefix: string): Promise<TreeEntry[]> {
    return invoke("rpc", { method: "globe/libraryTree", params: { prefix } });
  }
  librarySearch(query: string): Promise<TreeEntry[]> {
    return invoke("rpc", { method: "globe/librarySearch", params: { query } });
  }
  librarySource(path: string): ReturnType<Backend["librarySource"]> {
    return invoke("rpc", { method: "globe/librarySource", params: { path } });
  }
  completion(file: string, line: number, character: number): ReturnType<Backend["completion"]> {
    return invoke("rpc", { method: "globe/completion", params: { file, line, character } }); // S171（textDocument/completion と同じ計算）
  }
  definition(file: string, line: number, character: number): Promise<Definition | null> {
    return invoke("rpc", { method: "globe/definition", params: { file, line, character } }); // S138（textDocument/definition と同じ解決。本文付き）
  }

  // S141: 開く・保存は dialog / fs プラグイン（window.__TAURI__.dialog・fs。権限は capabilities/desktop.json）。dialog で選んだパスは fs の許可に自動で入る
  // S196: パスは持たない。UI のタブが本文を受け取ったときのパスを覚え、saveFile に渡す（名前で覚えると、同じ名前の別のファイルを上書きした）
  async openFile(): Promise<{ name: string; text: string; path: string; columns?: ResultColumns } | null> {
    const path = await tauriGlobal().dialog.open({ multiple: false, filters: OPEN });
    if (!path) return null;
    const name = baseName(path);
    if (isFmuFile(name)) return { name, text: "", path }; // S229: 読むのは sidecar（確認のあと App が fmuInfo・runFmu を呼ぶ）
    if (isResultsFile(name)) return { name, text: "", path, columns: await this.readResultsAt(path) };
    const text = await tauriGlobal().fs.readTextFile(path); // dialog で選んだパスは fs の許可に入っている
    return { name, text, path };
  }
  /** S222: sidecar が path を読む（bytes を JSON で往復しない）。列は simulateColumns と同じ形。S236: 最近使ったファイルと再読み込みも同じ口（dialog を通していなくても、読むのは sidecar） */
  async readResultsAt(path: string): Promise<ResultColumns> {
    const r = await invoke<{ layout: { names: string[]; points: number; defaults?: string[] }; columns: (number | null)[][]; parameters?: ResultParam[] }>("rpc", { method: "globe/readResults", params: { path } });
    return toColumns(r.layout, r.columns, r.parameters);
  }
  /** S151: 最近使ったファイルを path から開く。dialog を通していない path は fs の許可の外なので、Rust の command `read_recent` が読む（S154。`.mo` の通常のファイルだけ）。 */
  async openPath(path: string): Promise<{ name: string; text: string }> {
    return { name: baseName(path), text: await invoke<string>("read_recent", { path }) };
  }
  /** path があって saveAs でなければそこへ上書きする。無ければ（か saveAs なら）保存ダイアログで選ぶ。書いたパスを返す。 */
  async saveFile(name: string, text: string, saveAs = false, path?: string): Promise<string | null> {
    if (!saveAs && path) {
      // #67: 最近使ったファイルなど dialog を通していない path は fs の許可の外なので、Rust の command `write_recent` が書く（.mo だけ）
      await invoke("write_recent", { path, text });
      return path;
    }
    const to = await tauriGlobal().dialog.save({ defaultPath: name, filters: extFilter(name) });
    if (!to) return null;
    await tauriGlobal().fs.writeTextFile(to, text);
    return to;
  }
  async saveBinary(name: string, bytes: Uint8Array): Promise<string | null> {
    const t = tauriGlobal();
    const path = await t.dialog.save({ defaultPath: name, filters: extFilter(name) });
    if (!path) return null;
    await t.fs.writeFile(path, bytes);
    return path;
  }

  /** S144: ソースの FMU。dialog.save で選んだ場所のフォルダを outDir にして sidecar に書かせる（`<outDir>/<モデル名の . を _ にしたもの>-fmi<版>.fmu` と展開した同名のフォルダ。選んだファイル名は使わない）。 */
  references(file: string, line: number, character: number) {
    return invoke<Definition["range"][]>("rpc", { method: "globe/references", params: { file, line, character } }); // S161
  }
  models(file: string) {
    return invoke<string[]>("rpc", { method: "globe/models", params: { file } }); // S170
  }
  movePart(cls: string, part: string, dx: number, dy: number, at: { file: string }) {
    return invoke<{ range: Definition["range"]; newText: string }[] | null>("rpc", { method: "globe/movePart", params: { file: at.file, class: cls, part, dx, dy } }); // S250
  }
  rename(file: string, line: number, character: number, newName: string) {
    return invoke<{ range: Definition["range"]; newText: string }[]>("rpc", { method: "globe/rename", params: { file, line, character, newName } });
  }
  documentation(cls: string, at?: { file: string; line: number; character: number }): Promise<Documentation | null> {
    return invoke("rpc", { method: "globe/documentation", params: { class: cls, file: at?.file, line: at?.line, character: at?.character } }); // S232（setFile の didOpen の本文を引く）
  }
  icon(cls: string, at?: { file: string; line: number; character: number }): Promise<Icon | null> {
    return invoke("rpc", { method: "globe/icon", params: { class: cls, file: at?.file, line: at?.line, character: at?.character } }); // S244（documentation と同じ）
  }
  diagram(cls: string, at?: { file: string; line: number; character: number }): Promise<Diagram | null> {
    return invoke("rpc", { method: "globe/diagram", params: { class: cls, file: at?.file, line: at?.line, character: at?.character } }); // S245（icon と同じ）
  }
  async foldingRanges(file: string): Promise<{ startLine: number; endLine: number }[]> {
    return (await invoke<{ startLine: number; endLine: number }[] | null>("rpc", { method: "textDocument/foldingRange", params: { textDocument: { uri: file } } })) ?? []; // S225（setFile の didOpen と同じ uri）
  }
  hover(file: string, line: number, character: number): Promise<Hover | null> {
    return invoke("rpc", { method: "globe/hover", params: { file, line, character } }); // S149（globe/definition と同じ解決で宣言の 1 行）
  }
  async exportFmu(model: string, fmiVersion: number, runtime: FmuRuntime = "p5a"): Promise<string | null> {
    const stem = `${model.replaceAll(".", "_")}${runtime === "p5b" ? "-p5b" : ""}-fmi${fmiVersion}`; // sidecar が書く名前（p5b は <名前>-p5b-fmi<版>.fmu）
    const path = await tauriGlobal().dialog.save({ defaultPath: `${stem}.fmu`, filters: [{ name: "FMU", extensions: ["fmu"] }] });
    if (!path) return null;
    const r = await invoke<{ path: string }>("rpc", { method: "globe/exportFmu", params: { model, fmiVersion, runtime, outDir: dirOf(path) } });
    return r.path;
  }

  /** 更新を確かめ、あれば確かめてから入れる（S135、updater プラグインの `window.__TAURI__.updater`）。結果を短い文で返す。 */
  async checkUpdate(): Promise<string> {
    if (!(await invoke<boolean>("updater_configured"))) return "未設定"; // 公開鍵の無いビルド（release.yml の署名なし）
    type Update = { version: string; downloadAndInstall(): Promise<void> };
    const updater = (window as unknown as { __TAURI__: { updater: { check(): Promise<Update | null> } } }).__TAURI__.updater;
    try {
      const u = await updater.check();
      if (!u) return "最新";
      if (!confirm(`GlobeMode ${u.version} に更新しますか`)) return `${u.version} あり`;
      await u.downloadAndInstall(); // Windows は passive の msi が入れてアプリを閉じる。Linux は pkexec dpkg -i
      return `${u.version} を入れた。再起動で反映`;
    } catch (e) {
      return `失敗: ${e}`;
    }
  }
}

/** Tauri のウィンドウの中か。 */
export const inTauri = (): boolean => "__TAURI__" in window;

/** Desktop（dialog / fs プラグインと sidecar がある Tauri）か。Android の Tauri はブラウザと同じ（ダウンロードと Worker）。 */
export const inDesktop = (): boolean => inTauri() && !/Android/i.test(navigator.userAgent);
