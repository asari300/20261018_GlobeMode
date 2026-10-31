import type { Backend } from "./backend";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse } from "./protocol";

// Tauri が `withGlobalTauri` で注入する関数だけを使う（@tauri-apps/api は入れない。npm の依存を増やさないため）。
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
const invoke: Invoke = (cmd, args) =>
  (window as unknown as { __TAURI__: { core: { invoke: Invoke } } }).__TAURI__.core.invoke(cmd, args);

/** 同梱の `globe` の sidecar（`globe lsp`、stdio の JSON-RPC。ADR-0023）。Rust 側（src/Rust/apps/desktop）は中継だけ。 */
export class TauriBackend implements Backend {
  #version = 0;

  // shortcut: Desktop ではライブラリは sidecar の起動時の --library で読む。JSON を送る道は無い。MSL の置き場を決めたら足す
  library(name: string): Promise<void> {
    return Promise.reject(new Error(`Desktop ではライブラリ ${name} を JSON では読めない（sidecar の --library で読む）`));
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
}

/** Tauri のウィンドウの中か。 */
export const inTauri = (): boolean => "__TAURI__" in window;
