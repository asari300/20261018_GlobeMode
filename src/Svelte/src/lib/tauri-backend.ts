import type { Backend, LintMessage } from "./backend";
import { parseCsv, type Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse } from "./protocol";

// Tauri が `withGlobalTauri` で注入する関数だけを使う（@tauri-apps/api は入れない。npm の依存を増やさないため）。
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
const invoke: Invoke = (cmd, args) =>
  (window as unknown as { __TAURI__: { core: { invoke: Invoke } } }).__TAURI__.core.invoke(cmd, args);

/** 同梱の `globe` の sidecar（`globe lsp`、stdio の JSON-RPC。ADR-0023）。Rust 側（src/Rust/apps/desktop）は中継だけ。 */
export class TauriBackend implements Backend {
  #version = 0;

  // Desktop ではライブラリは sidecar の起動時の --library / GLOBE_LIBRARY で読む。JSON を送る道は無い（S056）:
  // 既に何か読んでいれば JSON は捨てて resolve、無ければ reject。
  async library(name: string): Promise<void> {
    const libs = await invoke<string[]>("libraries");
    if (libs.length === 0) {
      throw new Error(`Desktop ではライブラリ ${name} を JSON では読めない（--library か GLOBE_LIBRARY で sidecar に渡す）`);
    }
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
  // sidecar には列の道が無い（S065）。CSV を解析して同じ形にする
  async simulateColumns(req: SimulateRequest): Promise<{ response: SimulateResponse; columns: Columns }> {
    const response = await this.simulate(req);
    const columns = response.csv ? parseCsv(response.csv) : { names: [], points: 0, data: [] };
    return { response: { ...response, csv: null }, columns };
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
