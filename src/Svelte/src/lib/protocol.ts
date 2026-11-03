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
}

export interface SimulateRequest {
  files?: Record<string, string>;
  model: string;
  settings?: { start_time?: number; stop_time?: number; tolerance?: number; interval?: number; solver?: string; parameters?: Record<string, number> };
  force?: boolean;
  no_csv?: boolean;
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
