import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse } from "./protocol";

/** UI が計算を頼む相手。WorkerBackend（ブラウザの wasm）と、あとで TauriBackend（sidecar）が実装する。 */
export interface Backend {
  /** ライブラリ（{ パス: 本文 } の JSON）を読む。今の Workspace は捨てて作り直す。 */
  library(name: string, json: string): Promise<void>;
  setFile(name: string, text: string): Promise<void>;
  check(model: string): Promise<CheckResponse>;
  /** connect グラフ（graphology の JSON）。depth は 1 なら直下の部品だけ。モデルが無ければ reject。 */
  graph(model: string, depth: number): Promise<GraphJson>;
  simulate(req: SimulateRequest): Promise<SimulateResponse>;
  /** 整形（S053、globe-mode-fmt）。文法が壊れていれば reject で、本文は変えない。 */
  format(text: string, indent: number, insertOrigin: boolean): Promise<string>;
  lint(text: string): Promise<LintMessage[]>;
}

export interface LintMessage {
  rule: string;
  line: number;
  column: number;
  message: string;
}
