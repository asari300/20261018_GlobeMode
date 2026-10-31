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
  diagnostics: Diagnostic[];
  notes: string[];
}

export interface SimulateRequest {
  files?: Record<string, string>;
  model: string;
  settings?: { start_time?: number; stop_time?: number; tolerance?: number; interval?: number; solver?: string };
  force?: boolean;
  no_csv?: boolean;
}

export interface SimulateResponse {
  model: string;
  ok: boolean;
  refused: string | null;
  diagnostics: Diagnostic[];
  final: Record<string, number> | null;
  csv: string | null;
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
