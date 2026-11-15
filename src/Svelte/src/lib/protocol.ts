// globe-mode-protocol / globe-mode-core の JSON の形を、UI が使う項目だけ手で写したもの（JSON Schema ができたら置き換える）。
export type Stage = "S0" | "S1" | "S2" | "S3" | "S4" | "S5" | "S6" | "S7" | "S8" | "S9" | "S10";
export type Severity = "info" | "warning" | "error";

export interface Diagnostic {
  code: string;
  severity: Severity;
  stage: Stage;
  message: string;
  file?: string;
  /** ソース中の UTF-8 のバイト範囲（半開区間） */
  range?: { start: number; end: number };
  notes?: string[];
}

export interface CheckResponse {
  model: string;
  passed: boolean;
  failed_stage: Stage | null;
  /** モデルの experiment annotation の数の字面（S142。annotation が無ければ無い。書かれていない項目は null） */
  experiment?: Partial<Record<"start_time" | "stop_time" | "tolerance" | "interval", number | null>>;
  diagnostics: Diagnostic[];
  notes: string[];
  /** Error がすべて GM-U（コンパイラが未対応）のときの要約（S173。Rust が決める。無ければ無い） */
  unsupported?: string;
  /** 強行（force）で実行に進める見込み（S173。true のときだけ出る） */
  can_force?: boolean;
}

export interface SimulateRequest {
  files?: Record<string, string>;
  model: string;
  settings?: { start_time?: number; stop_time?: number; tolerance?: number; interval?: number; step?: number; max_order?: number; max_wall_seconds?: number; solver?: string; parameters?: Record<string, number> };
  force?: boolean;
  no_csv?: boolean;
}

/** S322（ADR-0040）: 複数のソルバで回す依頼。`settings`（`tolerance`・`max_order` を含む）は全部の実行に同じものを渡し、`settings.solver` は使わない。 */
export interface SimulateManyRequest {
  files?: Record<string, string>;
  model: string;
  settings?: SimulateRequest["settings"];
  /** 回すソルバの名前（1〜16 個、重複なし） */
  solvers: string[];
  /** 同時に回す（ネイティブは最大 4 本。ブラウザ・wasm は逐次に落ちて response.mode_note に出る）。既定は 1 つずつ */
  parallel?: boolean;
  /** 全部が終わってから比べる。閾値は 10^-threshold_exp（1〜15、既定 6）の矩形の管の幅。reference は基準のソルバ（省略は最初に成功した実行） */
  compare?: { threshold_exp?: number; reference?: string };
  force?: boolean;
}

export type ManyStatus = "ok" | "failed" | "cancelled";
export interface ManyRun {
  /** 依頼の名前（別名のまま。使ったソルバは response.settings.solver） */
  solver: string;
  status: ManyStatus;
  /** その実行の応答（csv 無し。diagnostics はこの実行の分だけ） */
  response: SimulateResponse;
  elapsed_ms: number;
  /** 例: 「指定した最大次数（3）はソルバ bdf では使わない…」 */
  notes: string[];
}
export interface SignalDiff { name: string; passed: boolean; errors: number; max_excess: number; first_error_time?: number; missing?: boolean }
export interface PairComparison { solver: string; passed: boolean; signals: number; failed_signals: number; failures: SignalDiff[] }
export interface ManyComparison {
  threshold_exp: number;
  threshold: number;
  reference: string;
  passed: boolean;
  pairs: PairComparison[];
  skipped: { solver: string; reason: string }[];
}
export interface SimulateManyResponse {
  model: string;
  /** すべての実行が ok（比較の合否は comparison.passed） */
  ok: boolean;
  refused: string | null;
  /** 実行の前の共通の診断（検査・コンパイル） */
  diagnostics: Diagnostic[];
  mode: "sequential" | "parallel";
  workers: number;
  mode_note?: string;
  runs: ManyRun[];
  comparison?: ManyComparison;
  elapsed_ms: number;
}

export interface SimulateResponse {
  model: string;
  ok: boolean;
  refused: string | null;
  diagnostics: Diagnostic[];
  /** 実行に使った設定と各項目の出どころ（S142。実行まで進まなければ null） */
  settings?: ResolvedSettings | null;
  final: Record<string, number> | null;
  csv: string | null;
  /** CheckResponse と同じ（この実行の診断について）。can_force は検査で断った（強行していない）ときだけ */
  unsupported?: string;
  can_force?: boolean;
}

export type SettingOrigin = "requested" | "experiment" | "default";
export interface ResolvedSettings {
  start_time: number;
  stop_time: number;
  tolerance: number;
  interval: number;
  solver: string;
  origins: Record<"start_time" | "stop_time" | "tolerance" | "interval" | "solver", SettingOrigin>;
}

/** 再コンパイル無しに上書きできるパラメータの 1 行（`TunableParameter`。S150。default はモデルの値） */
export interface TunableParameter {
  name: string;
  default: number;
  unit?: string;
  min?: number;
  max?: number;
  description?: string;
}

/** モデルのパラメータ宣言の 1 行（`ModelParameter`。S248）。name は `comp.k` の形、value は数の既定値（Boolean は 0/1、Enumeration は 1 始まりの ordinal）。 */
export interface ModelParameter {
  name: string;
  type: "Real" | "Integer" | "Boolean" | "String" | "Enumeration";
  dims?: number[];
  default?: string;
  value?: number;
  literals?: string[];
  enum_type?: string; // S298: Enumeration の型の完全修飾名（修飾に `型.リテラル` と書く）
  unit?: string;
  description?: string;
  min?: number;
  max?: number;
  fixed?: boolean;
}

/** 線形化の結果（S260。`globe linearize --json` と同じ。`dx/dt ≈ A·dx + B·du`、`dy ≈ C·dx + D·du`。A は x×x、B は x×u、C は y×x、D は y×u の行ごとの配列） */
export interface Linearization {
  model: string;
  time: number;
  x: string[];
  u: string[];
  y: string[];
  x0: number[];
  u0: number[];
  y0: number[];
  A: number[][];
  B: number[][];
  C: number[][];
  D: number[][];
  /** A の固有値（極。S286）と安定性（stable / unstable / marginal / none） */
  eigenvalues: { re: number; im: number }[];
  stability: "stable" | "unstable" | "marginal" | "none";
}

/** ソルバの登録表の 1 行（`globe_mode_sim::solvers::SolverEntry`）。使えるのは status.kind が available のもの。 */
export interface SolverEntry {
  name: string;
  status: { kind: "available" | "alias" | "planned" };
  description: string;
}

/** `ConnectGraph::to_graphology()` の JSON（graphology の `Graph.from` で読める形）。 */
export interface GraphJson {
  attributes: { model: string; depth: number; unresolved: string[] };
  options: { type: "undirected"; multi: boolean; allowSelfLoops: boolean };
  nodes: { key: string; attributes: { label: string; class: string; kind: "component" | "connector"; level: number } }[];
  edges: {
    key: string;
    source: string;
    target: string;
    attributes: { lhs: string; rhs: string; domain: string; connector: string };
  }[];
}

/** BLT のブロック 1 つ（S256。`globe blt --json` と同じ。size 2 以上が代数ループ）。 */
export interface BltBlock { size: number; algebraic_loop: boolean; nonlinear: boolean; variables: string[]; equations: string[] }
/** 変換の段ごとの要約（S280・S282。`globe stages --json` と同じ）。flat → DAE は数だけ。 */
export interface StagesReport {
  flat: { variables: number; equations: number; initial_equations: number } | null;
  dae: Record<string, number> | null;
  index: { index: number; failed: boolean; differentiated: string[]; added_equations: number; added_unknowns: string[]; dynamic_states: number; dummy_candidates: string[] };
  blt: { reduced: boolean; blocks: number; algebraic_loops: number; max_size: number; size_histogram: [number, number][]; skipped?: string };
  tearing: { loops: { size: number; nonlinear: boolean; variables: string[]; residuals: string[] }[]; iteration_variables: number };
}
/** BLT（S262）。blocks は解く順。skipped があれば解析しなかった理由で blocks は空。 */
export interface BltReport { reduced: boolean; blocks: BltBlock[]; skipped?: string }
