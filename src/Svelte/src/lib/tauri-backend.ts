import type { Backend, LintMessage } from "./backend";
import type { Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse } from "./protocol";

// Tauri が `withGlobalTauri` で注入する関数だけを使う（@tauri-apps/api は入れない。npm の依存を増やさないため）。
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>;
const tauriGlobal = () => (window as unknown as { __TAURI__: { core: { invoke: Invoke }; event: { listen: Listen } } }).__TAURI__;
const invoke: Invoke = (cmd, args) => tauriGlobal().core.invoke(cmd, args);

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
  // sidecar の `globe/simulateColumns`（S076）。列は JSON の数の配列で、有限でない値は既に null（uPlot の欠損）
  async simulateColumns(req: SimulateRequest): Promise<{ response: SimulateResponse; columns: Columns }> {
    const r = await invoke<{ response: SimulateResponse; layout: { names: string[]; points: number }; columns: (number | null)[][] }>(
      "rpc",
      { method: "globe/simulateColumns", params: { request: req } },
    );
    return { response: r.response, columns: { names: r.layout.names.slice(1), points: r.layout.points, data: r.columns } };
  }
  format(text: string, indent: number, insertOrigin: boolean): Promise<string> {
    return invoke("rpc", { method: "globe/format", params: { text, indent, insertOrigin } });
  }
  lint(text: string): Promise<LintMessage[]> {
    return invoke("rpc", { method: "globe/lint", params: { text } });
  }
}

/** Tauri のウィンドウの中か。 */
export const inTauri = (): boolean => "__TAURI__" in window;
