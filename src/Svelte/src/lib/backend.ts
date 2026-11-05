import type { Columns } from "./plot/csv";
import type { CheckResponse, GraphJson, SimulateRequest, SimulateResponse, SolverEntry, TunableParameter } from "./protocol";

/** UI が計算を頼む相手。WorkerBackend（ブラウザの wasm）と TauriBackend（sidecar の `globe lsp`）が実装する。 */
export interface Backend {
  /** ライブラリ（{ パス: 本文 } の JSON）を読む。今の Workspace は捨てて作り直す。 */
  library(name: string, json: string): Promise<void>;
  setFile(name: string, text: string): Promise<void>;
  check(model: string): Promise<CheckResponse>;
  /** connect グラフ（graphology の JSON）。depth は 1 なら直下の部品だけ。モデルが無ければ reject。 */
  graph(model: string, depth: number): Promise<GraphJson>;
  simulate(req: SimulateRequest): Promise<SimulateResponse>;
  /** simulate と同じ実行で、時系列を CSV でなく列ごとに返す（S065）。response.csv は無い（null）。実行できなければ points 0。 */
  simulateColumns(req: SimulateRequest, onProgress?: (p: Progress) => void): Promise<{ response: SimulateResponse; columns: Columns }>;
  /** 実行中の simulateColumns を止める（S133、ADR-0033）。止めた呼び出しは cancelled: true の Error で reject する。無ければ止められない。 */
  cancel?(): void;
  /** ソルバの登録表（S142）。 */
  solvers(): Promise<SolverEntry[]>;
  /** 上書きできるパラメータの一覧（S150）。モデルを DAE まで変換できなければ reject、lowering に失敗したら空。 */
  tunableParameters(model: string): Promise<TunableParameter[]>;
  /** 整形（S053、globe-mode-fmt）。文法が壊れていれば reject で、本文は変えない。indent などを省くと、Desktop は globe-mode.toml（S143）、ブラウザは 2 と true。 */
  format(text: string, indent?: number, insertOrigin?: boolean): Promise<string>;
  lint(text: string): Promise<LintMessage[]>;
  /** ライブラリの木の prefix（空ならトップレベル）の直下の子（S127）。ライブラリが無ければ空。 */
  libraryTree(prefix: string): Promise<TreeEntry[]>;
  /** ライブラリのクラス（ファイルの中の入れ子を含む）を名前の部分一致で探す（S167。大文字小文字を区別せず、`.` を含めると完全修飾名に当てる。最大 50 件、近い順）。 */
  librarySearch(query: string): Promise<TreeEntry[]>;
  /** ライブラリのクラス path を定義するファイルの名前と本文（S127）。その名前で setFile するとライブラリのファイルを差し替える。 */
  librarySource(path: string): Promise<{ file: string; text: string }>;
  /** setFile 済みの file の位置（行は 0 始まり、列は UTF-16）にある名前の定義（S138）。名前が引けなければ null。 */
  definition(file: string, line: number, character: number): Promise<Definition | null>;
  /** setFile 済みの file の位置にある名前の宣言（S149）。contents は `Real x(start = 1) "説明"` のような 1 行、range はカーソルの名前。引けなければ null。 */
  hover(file: string, line: number, character: number): Promise<Hover | null>;
  /** setFile 済みの file の位置（行は 0 始まり、列は UTF-16）の補完候補（S171。LSP の CompletionItem。最大 200 件、打ちかけの語で絞り済み。文字列・コメントの中は空）。 */
  completion(file: string, line: number, character: number): Promise<CompletionEntry[]>;
  /** setFile 済みの file の位置にある名前の参照（S161）。同じ文書の同じ名前の範囲（LSP の Range）。部品はそのいちばん内側のクラスの中、クラス名（S166）は同じクラスに行き着く全部。 */
  references(file: string, line: number, character: number): Promise<Definition["range"][]>;
  /** 位置の名前を newName に変える編集（S161。LSP の TextEdit）。いちばん内側のクラスで宣言した部品と、この文書で定義したクラス（S166。宣言・`end`・使っている所）。部品でもクラスでもない・別のファイル（ライブラリ）のクラス・識別子でない・予約語・同じクラスに既にある名前は、理由つきで reject。 */
  rename(file: string, line: number, character: number, newName: string): Promise<{ range: Definition["range"]; newText: string }[]>;
  /** 手元のファイルを 1 つ選んで読む（S141）。ブラウザ・Android は `<input type=file>`、Desktop は dialog / fs プラグイン。選ばなければ null。 */
  openFile(): Promise<{ name: string; text: string; path?: string } | null>;
  /** 最近使ったファイルを path から開く（S151。Desktop だけ。ブラウザは本文を localStorage に持つので無い）。 */
  openPath?(path: string): Promise<{ name: string; text: string }>;
  /** 本文を手元に書く。保存先（Desktop はパス、ブラウザは名前）を返す。取り消しは null。saveAs は Desktop で保存先を選び直す（ブラウザは毎回ダウンロードなので同じ）。 */
  saveFile(name: string, text: string, saveAs?: boolean): Promise<string | null>;
  /** ソースの FMU（FMI 2 か 3、build 無し）を書き出す（S144）。保存先（Desktop はパス、ブラウザは名前）を返す。取り消しは null。作れなければ reject。 */
  exportFmu(model: string, fmiVersion: number): Promise<string | null>;
  /** バイナリ（PNG など）を手元に書く（S146）。保存先と取り消しは saveFile と同じ。 */
  saveBinary?(name: string, bytes: Uint8Array): Promise<string | null>;
}

/** 定義の場所。text は定義のあるファイルの本文（ライブラリのファイルならエディタに開く）。range は LSP の Range。 */
export interface Definition {
  file: string;
  text: string;
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
}

/** 補完の候補（S171。LSP の CompletionItem のうち使う分）。kind は LSP の CompletionItemKind の番号。 */
export interface CompletionEntry {
  label: string;
  kind?: number;
  detail?: string;
  documentation?: string;
}

/** ホバーの内容（S149）。range は LSP の Range（カーソルの名前）。 */
export interface Hover {
  contents: string;
  range: Definition["range"];
}

/** 実行系の進捗（ADR-0033）。stage は "lowering"・"integration" など、time は積分の t（無ければ undefined）。 */
export interface Progress {
  stage: string;
  time?: number;
  start_time: number;
  stop_time: number;
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
