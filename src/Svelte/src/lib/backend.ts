import type { Columns } from "./plot/csv";
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
  /** simulate と同じ実行で、時系列を CSV でなく列ごとに返す（S065）。response.csv は無い（null）。実行できなければ points 0。 */
  simulateColumns(req: SimulateRequest): Promise<{ response: SimulateResponse; columns: Columns }>;
  /** 整形（S053、globe-mode-fmt）。文法が壊れていれば reject で、本文は変えない。 */
  format(text: string, indent: number, insertOrigin: boolean): Promise<string>;
  lint(text: string): Promise<LintMessage[]>;
  /** ライブラリの木の prefix（空ならトップレベル）の直下の子（S127）。ライブラリが無ければ空。 */
  libraryTree(prefix: string): Promise<TreeEntry[]>;
  /** ライブラリのクラス path を定義するファイルの名前と本文（S127）。その名前で setFile するとライブラリのファイルを差し替える。 */
  librarySource(path: string): Promise<{ file: string; text: string }>;
}

export interface TreeEntry {
  name: string;
  path: string;
  kind: string;
  description: string;
  file: string;
  hasChildren: boolean;
}

export interface LintMessage {
  rule: string;
  line: number;
  column: number;
  message: string;
}
