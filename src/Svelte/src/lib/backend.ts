import type { Columns } from "./plot/csv";
import type { ResultColumns } from "./results";
import type { CheckResponse, GraphJson, BltReport, SimulateRequest, SimulateResponse, SolverEntry, ModelParameter, TunableParameter } from "./protocol";

/** UI が計算を頼む相手。WorkerBackend（ブラウザの wasm）と TauriBackend（sidecar の `globe lsp`）が実装する。 */
export interface Backend {
  /** ライブラリ（{ パス: 本文 } の JSON）を読む。今の Workspace は捨てて作り直す。 */
  library(name: string, json: string): Promise<void>;
  setFile(name: string, text: string): Promise<void>;
  check(model: string): Promise<CheckResponse>;
  /** connect グラフ（graphology の JSON）。depth は 1 なら直下の部品だけ。モデルが無ければ reject。 */
  graph(model: string, depth: number): Promise<GraphJson>;
  /** 方程式の BLT 分解（S262。解く順のブロック）。検査が通らなければ reject。 */
  blt(model: string): Promise<BltReport>;
  simulate(req: SimulateRequest): Promise<SimulateResponse>;
  /** simulate と同じ実行で、時系列を CSV でなく列ごとに返す（S065）。response.csv は無い（null）。実行できなければ points 0。 */
  simulateColumns(req: SimulateRequest, onProgress?: (p: Progress) => void): Promise<{ response: SimulateResponse; columns: Columns }>;
  /** 実行中の simulateColumns を止める（S133、ADR-0033）。止めた呼び出しは cancelled: true の Error で reject する。無ければ止められない。 */
  cancel?(): void;
  /** ソルバの登録表（S142）。 */
  solvers(): Promise<SolverEntry[]>;
  /** 上書きできるパラメータの一覧（S150）。モデルを DAE まで変換できなければ reject、lowering に失敗したら空。 */
  tunableParameters(model: string): Promise<TunableParameter[]>;
  /** モデルのパラメータ宣言の一覧（S248。部品ごとの入れ子の名前・型・既定値・単位）。DAE まで変換できなければ reject。 */
  parameters(model: string): Promise<ModelParameter[]>;
  /** 整形（S053、globe-mode-fmt）。文法が壊れていれば reject で、本文は変えない。indent などを省くと、Desktop は globe-mode.toml（S143）、ブラウザは 2 と true。 */
  format(text: string, indent?: number, insertOrigin?: boolean): Promise<string>;
  lint(text: string): Promise<LintMessage[]>;
  /** ライブラリの木の prefix（空ならトップレベル）の直下の子（S127）。ライブラリが無ければ空。 */
  libraryTree(prefix: string): Promise<TreeEntry[]>;
  /** ライブラリのクラス（ファイルの中の入れ子を含む）を名前の部分一致で探す（S167。大文字小文字を区別せず、`.` を含めると完全修飾名に当てる。最大 50 件、近い順）。 */
  librarySearch(query: string): Promise<TreeEntry[]>;
  /** ライブラリのクラス path を定義するファイルの名前と本文（S127）。その名前で setFile するとライブラリのファイルを差し替える。range はそのファイルの中のクラスの宣言の範囲（S186。引けなければ無い）。 */
  librarySource(path: string): Promise<{ file: string; text: string; range?: Definition["range"] }>;
  /** setFile 済みの file の位置（行は 0 始まり、列は UTF-16）にある名前の定義（S138）。名前が引けなければ null。 */
  definition(file: string, line: number, character: number): Promise<Definition | null>;
  /** setFile 済みの file の位置にある名前の宣言（S149）。contents は `Real x(start = 1) "説明"` のような 1 行、range はカーソルの名前。引けなければ null。 */
  hover(file: string, line: number, character: number): Promise<Hover | null>;
  /** setFile 済みの file の位置（行は 0 始まり、列は UTF-16）の補完候補（S171。LSP の CompletionItem。最大 200 件、打ちかけの語で絞り済み。文字列・コメントの中は空。配列か LSP の CompletionList。S196）。 */
  completion(file: string, line: number, character: number): Promise<CompletionEntry[] | { isIncomplete?: boolean; items: CompletionEntry[] }>;
  /** setFile 済みの file の位置にある名前の参照（S161）。同じ文書の同じ名前の範囲（LSP の Range）。部品はそのいちばん内側のクラスの中、クラス名（S166）は同じクラスに行き着く全部。 */
  references(file: string, line: number, character: number): Promise<Definition["range"][]>;
  /** setFile 済みの file の折りたたみの範囲（S225。LSP の foldingRange と同じ関数。行は 0 始まりで endLine を含む。クラス・節・複数行の annotation・連続コメント。構文が壊れていればクラスと節は無い）。 */
  foldingRanges(file: string): Promise<{ startLine: number; endLine: number }[]>;
  /** クラスの Documentation（`annotation(Documentation(info, revisions))` の生の HTML。S232）。cls は完全修飾名（ライブラリのクラスもよい）。空なら at（setFile 済みの file の位置にあるいちばん内側のクラス）。引けなければ null。HTML は信用しない（表示は lib/html-sanitize.ts を通す）。 */
  documentation(cls: string, at?: { file: string; line: number; character: number }): Promise<Documentation | null>;
  /** クラスのアイコン（`annotation(Icon(…))`）の形のデータ（S244）。cls・at は `documentation` と同じ。`extends` の基底クラスのアイコンを 1 段重ねてある。引けなければ null。描くのは UI（`lib/IconView.svelte`）。 */
  icon(cls: string, at?: { file: string; line: number; character: number }): Promise<Icon | null>;
  /** クラスの図（部品の配置と `connect`）の形のデータ（S245）。cls・at は `documentation` と同じ。引けなければ null。描くのは UI（`lib/DiagramView.svelte`）。 */
  diagram(cls: string, at?: { file: string; line: number; character: number }): Promise<Diagram | null>;
  /** 図の部品 part を Diagram の座標で (dx, dy) だけ動かす編集（S250。LSP の TextEdit。`Placement` の `origin` を書き換える）。cls は diagram.class、at.file は setFile 済み。動かせなければ null。ファイルは書かない（適用は UI のエディタ）。 */
  movePart(cls: string, part: string, dx: number, dy: number, at: { file: string }): Promise<{ range: Definition["range"]; newText: string }[] | null>;
  /** 図に結線 `connect(from, to)` を足す編集（S261。LSP の TextEdit。クラスの最後の `equation` 節の末尾へ、`Line(points)` つきで）。from・to は `部品.コネクタ`、points は図の座標。cls は diagram.class、at.file は setFile 済み。足せなければ null。型の互換は見ない。ファイルは書かない（適用は UI のエディタ）。 */
  addConnect(cls: string, from: string, to: string, points: [number, number][], at: { file: string }): Promise<{ range: Definition["range"]; newText: string }[] | null>;
  /** setFile 済みの file の実行できるクラス（model・block・class。partial は除く）の修飾名（S170。UI のモデル名の候補。構文が壊れていれば空）。 */
  models(file: string): Promise<string[]>;
  /** 位置の名前を newName に変える編集（S161。LSP の TextEdit）。いちばん内側のクラスで宣言した部品と、この文書で定義したクラス（S166。宣言・`end`・使っている所）。部品でもクラスでもない・別のファイル（ライブラリ）のクラス・識別子でない・予約語・同じクラスに既にある名前は、理由つきで reject。 */
  rename(file: string, line: number, character: number, newName: string): Promise<{ range: Definition["range"]; newText: string }[]>;
  /** 手元のファイルを 1 つ選んで読む（S141）。ブラウザ・Android は `<input type=file>`、Desktop は dialog / fs プラグイン。選ばなければ null。結果のファイル（.mat・.csv。S222）は text が空で、読んだ列が columns に入る（読めなければ reject）。 */
  openFile(): Promise<{ name: string; text: string; path?: string; columns?: ResultColumns } | null>;
  /** FMU（.fmu）の要約と、実行前に見せる警告（S229。Desktop だけ。ネイティブコードは実行しない）。読めなければ reject。 */
  fmuInfo?(path: string): Promise<FmuInfo>;
  /** FMU を別プロセスで回し、時刻と出力の列を返す（S229。Desktop だけ）。request は `FmuRunRequest`（S236。kind・stop_time・step・timeout だけ使う。省くと FMU の既定の実験設定）。onProgress は FMU の時刻。止めた呼び出しは cancelled: true の Error で reject する。 */
  runFmu?(path: string, request?: FmuRunRequest, onProgress?: (t: number) => void): Promise<Columns>;
  /** パスの結果のファイル（.mat・.csv）を読み直す（S236。Desktop だけ。最近使ったファイルと再読み込みで戻すタブ。読めなければ reject）。 */
  readResultsAt?(path: string): Promise<ResultColumns>;
  /** 最近使ったファイルを path から開く（S151。Desktop だけ。ブラウザは本文を localStorage に持つので無い）。 */
  openPath?(path: string): Promise<{ name: string; text: string }>;
  /** 本文を手元に書く。保存先（Desktop はパス、ブラウザは名前）を返す。取り消しは null。saveAs は Desktop で保存先を選び直す（ブラウザは毎回ダウンロードなので同じ）。path は Desktop で、そのタブが本文を受け取った（開いた・保存した）パス（S196。あれば saveAs でない限りそこへ上書きする）。 */
  saveFile(name: string, text: string, saveAs?: boolean, path?: string): Promise<string | null>;
  /** ソースの FMU（FMI 2 か 3、build 無し）を書き出す（S144）。保存先（Desktop はパス、ブラウザは名前）を返す。取り消しは null。作れなければ reject。runtime（S239）の "p5b" は自前の実行時の共有ライブラリ入りの FMU（Desktop だけ。sidecar が cargo で作るので globe のソースの作業ツリーが要る。ブラウザは無視してソースの FMU）。 */
  exportFmu(model: string, fmiVersion: number, runtime?: FmuRuntime): Promise<string | null>;
  /** バイナリ（PNG など）を手元に書く（S146）。保存先と取り消しは saveFile と同じ。 */
  saveBinary?(name: string, bytes: Uint8Array): Promise<string | null>;
}

/** FMU の実行時（S239）。p5a は Rumoca の C のソース、p5b は自前の共有ライブラリ入り。 */
export type FmuRuntime = "p5a" | "p5b";

/** `globe/fmuRun` の request のうち UI が送る分（S236。空の鍵は送らない）。kind は "cs" か "me"、timeout は壁時計の秒（予算）。 */
export interface FmuRunRequest { kind?: "cs" | "me"; stop_time?: number; step?: number; timeout?: number }

/** `globe/fmuInfo` の要約（S229。使う分だけ）。warning は実行前に見せる警告（Rust の `FMU_TRUST_WARNING`）。 */
export interface FmuInfo {
  model_name: string;
  fmi_version: string;
  interfaces: { kind: string; model_identifier: string }[];
  warning: string;
}

/** 定義の場所。text は定義のあるファイルの本文（ライブラリのファイルならエディタに開く）。range は LSP の Range。 */
export interface Definition {
  file: string;
  text: string;
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
}

/** `Backend.documentation` の結果（S232）。info・revisions は Modelica のエスケープを戻した生の HTML（無ければ空）。 */
export interface Documentation {
  class: string;
  info: string;
  revisions: string;
}

/** `Backend.icon` の図形（S244）。座標は Modelica のまま（y は上向き）。line・fill は `[r, g, b]` か null（text の色は line）。 */
export interface IconShape {
  kind: "rect" | "ellipse" | "line" | "polygon" | "text" | "bitmap";
  origin: [number, number];
  rotation: number;
  extent: [number, number, number, number];
  points: [number, number][];
  line: [number, number, number] | null;
  fill: [number, number, number] | null;
  thickness: number;
  dash: "solid" | "dash" | "dot" | "dashdot" | "dashdotdot";
  radius: number;
  text: string;
  fontSize: number;
  align: "left" | "center" | "right";
  arrows: ["none" | "open" | "filled" | "half", "none" | "open" | "filled" | "half"];
  arrowSize: number;
}

/** `Backend.diagram` の結果（S245）。座標は Modelica のまま（y は上向き）。 */
export interface Diagram {
  class: string;
  extent: [number, number, number, number];
  parts: DiagramPart[];
  connections: { from: string; to: string; line: IconShape }[];
  shapes: IconShape[];
}

/** 部品 1 つ。`transform` はアイコンの座標から図の座標への `[a, b, c, d, e, f]`（`x' = a x + c y + e`、`y' = b x + d y + f`）。`line` は宣言の行（0 始まり）。 */
export interface DiagramPart {
  name: string;
  class: string;
  line: number;
  transform: [number, number, number, number, number, number];
  bbox: [number, number, number, number];
  shapes: IconShape[];
  /** コネクタ（部品のクラスで `Placement` を持つ宣言。S261）の名前と図の座標。型が引けて connector でなければ除く（引けなければ残す） */
  connectors: { name: string; pos: [number, number] }[];
}

/** `Backend.icon` の結果（S244）。extent は `[x1, y1, x2, y2]`。shapes は描く順。 */
export interface Icon {
  class: string;
  extent: [number, number, number, number];
  shapes: IconShape[];
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
