import type { Backend, CompletionEntry, Definition, Hover, LintMessage, Progress, TreeEntry } from "./backend";
import type { Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse, SolverEntry, TunableParameter } from "./protocol";

// Tauri が `withGlobalTauri` で注入する関数だけを使う（@tauri-apps/api は入れない。npm の依存を増やさないため）。
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>;
type Dialog = { open(o: object): Promise<string | null>; save(o: object): Promise<string | null> };
type Fs = { readTextFile(p: string): Promise<string>; writeTextFile(p: string, t: string): Promise<void>; writeFile(p: string, b: Uint8Array): Promise<void> };
const tauriGlobal = () => (window as unknown as { __TAURI__: { core: { invoke: Invoke }; event: { listen: Listen }; dialog: Dialog; fs: Fs } }).__TAURI__;
const MO = [{ name: "Modelica", extensions: ["mo"] }];
const extFilter = (name: string) => [{ name: "File", extensions: [name.split(".").pop() ?? ""] }]; // 保存ダイアログの拡張子の絞り込み
const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
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
  // sidecar の `globe/simulateColumns`（S076）。列は JSON の数の配列で、有限でない値は既に null（uPlot の欠損）。
  // S136（ADR-0033）: workDoneToken を付け、サーバの `$/progress`（イベント `globe-notification`）の value を onProgress に渡す
  async simulateColumns(req: SimulateRequest, onProgress?: (p: Progress) => void): Promise<{ response: SimulateResponse; columns: Columns }> {
    this.#listening ??= tauriGlobal().event.listen<{ method?: string; params?: { token?: string; value?: Progress } }>("globe-notification", (e) => {
      const p = e.payload;
      if (p.method === "$/progress" && p.params?.token === RUN_TOKEN && p.params.value?.stage) this.#onProgress?.(p.params.value);
    });
    await this.#listening;
    this.#onProgress = onProgress;
    this.#cancelling = false;
    try {
      const r = await invoke<{ response: SimulateResponse; layout: { names: string[]; points: number }; columns: (number | null)[][] }>(
        "rpc",
        { method: "globe/simulateColumns", params: { request: req, workDoneToken: RUN_TOKEN } },
      );
      return { response: r.response, columns: { names: r.layout.names.slice(1), points: r.layout.points, data: r.columns } };
    } catch (e) {
      // 止めても応えない sidecar は作り直される（command `cancel_run`）。UI には止めたとして返す
      if (this.#cancelling) throw Object.assign(new Error(String(e)), { cancelled: true });
      throw e;
    } finally {
      this.#onProgress = undefined;
    }
  }
  /** 実行中の simulateColumns を止める（S136）。サーバが止めれば途中までの列と GM-I11004 の普通の応答、止まらなければ sidecar を作り直して cancelled で reject。 */
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
  librarySource(path: string): Promise<{ file: string; text: string }> {
    return invoke("rpc", { method: "globe/librarySource", params: { path } });
  }
  completion(file: string, line: number, character: number): Promise<CompletionEntry[]> {
    return invoke("rpc", { method: "globe/completion", params: { file, line, character } }); // S171（textDocument/completion と同じ計算）
  }
  definition(file: string, line: number, character: number): Promise<Definition | null> {
    return invoke("rpc", { method: "globe/definition", params: { file, line, character } }); // S138（textDocument/definition と同じ解決。本文付き）
  }

  // S141: 開く・保存は dialog / fs プラグイン（window.__TAURI__.dialog・fs。権限は capabilities/desktop.json）。dialog で選んだパスは fs の許可に自動で入る
  #paths = new Map<string, string>(); // UI の文書の名前 → 開いた・保存したパス（「保存」は同じパスに上書きする）
  async openFile(): Promise<{ name: string; text: string; path: string } | null> {
    const path = await tauriGlobal().dialog.open({ multiple: false, filters: MO });
    if (!path) return null;
    const name = baseName(path), text = await tauriGlobal().fs.readTextFile(path); // dialog で選んだパスは fs の許可に入っている
    this.#paths.set(name, path);
    return { name, text, path };
  }
  /** S151: 最近使ったファイルを path から開く。dialog を通していない path は fs の許可の外なので、Rust の command `read_recent` が読む（S154。`.mo` の通常のファイルだけ）。 */
  async openPath(path: string): Promise<{ name: string; text: string }> {
    const name = baseName(path);
    const text = await invoke<string>("read_recent", { path });
    this.#paths.set(name, path);
    return { name, text };
  }
  async saveFile(name: string, text: string, saveAs = false): Promise<string | null> {
    const t = tauriGlobal();
    const path = (!saveAs && this.#paths.get(name)) || (await t.dialog.save({ defaultPath: name, filters: extFilter(name) }));
    if (!path) return null;
    await t.fs.writeTextFile(path, text);
    this.#paths.set(name, path);
    return path;
  }
  async saveBinary(name: string, bytes: Uint8Array): Promise<string | null> {
    const t = tauriGlobal();
    const path = await t.dialog.save({ defaultPath: name, filters: extFilter(name) });
    if (!path) return null;
    await t.fs.writeFile(path, bytes);
    return path;
  }

  /** S144: ソースの FMU。dialog.save で選んだ場所のフォルダを outDir にして sidecar に書かせる（`<outDir>/<モデル名の . を _ にしたもの>.fmu` と展開した同名のフォルダ。選んだファイル名は使わない）。 */
  references(file: string, line: number, character: number) {
    return invoke<Definition["range"][]>("rpc", { method: "globe/references", params: { file, line, character } }); // S161
  }
  models(file: string) {
    return invoke<string[]>("rpc", { method: "globe/models", params: { file } }); // S170
  }
  rename(file: string, line: number, character: number, newName: string) {
    return invoke<{ range: Definition["range"]; newText: string }[]>("rpc", { method: "globe/rename", params: { file, line, character, newName } });
  }
  hover(file: string, line: number, character: number): Promise<Hover | null> {
    return invoke("rpc", { method: "globe/hover", params: { file, line, character } }); // S149（globe/definition と同じ解決で宣言の 1 行）
  }
  async exportFmu(model: string, fmiVersion: number): Promise<string | null> {
    const path = await tauriGlobal().dialog.save({ defaultPath: `${model.replaceAll(".", "_")}.fmu`, filters: [{ name: "FMU", extensions: ["fmu"] }] });
    if (!path) return null;
    const r = await invoke<{ path: string }>("rpc", { method: "globe/exportFmu", params: { model, fmiVersion, outDir: path.replace(/[\\/][^\\/]*$/, "") || "/" } });
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
