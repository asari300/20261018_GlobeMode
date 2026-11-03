//// GlobeMode の Gleam の束ね（S087、Erlang target）。NIF の関数を `@external` で宣言するだけで、ロジックは Rust 側にある。
////
//// NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス）。結果は JSON の文字列で、解析は呼ぶ側に任せる。

import gleam/dynamic.{type Dynamic}

/// NIF の resource（Rust の `globe_mode::Workspace`）。
pub type Workspace

/// `libraries` はライブラリのディレクトリ（MSL など）。
@external(erlang, "globe_mode_nif", "new")
pub fn new(libraries: List(String)) -> Result(Workspace, String)

/// Rustler は `Ok(())` を `{ok, {}}` で返すので `#()`。
@external(erlang, "globe_mode_nif", "set_file")
pub fn set_file(
  ws: Workspace,
  path: String,
  text: String,
) -> Result(#(), String)

/// ディレクトリの下の `.mo` をすべて読み、読んだ数を返す。
@external(erlang, "globe_mode_nif", "load_dir")
pub fn load_dir(ws: Workspace, dir: String) -> Result(Int, String)

@external(erlang, "globe_mode_nif", "check")
pub fn check(ws: Workspace, model: String) -> Result(String, String)

@external(erlang, "globe_mode_nif", "graph")
pub fn graph(ws: Workspace, model: String, depth: Int) -> Result(String, String)

/// `settings_json` は `{"stop_time": 2, "solver": "bdf"}` など。
@external(erlang, "globe_mode_nif", "simulate")
pub fn simulate(
  ws: Workspace,
  model: String,
  settings_json: String,
) -> Result(String, String)

/// 列の道（S116）。`#(レイアウトの JSON, 列)`。レイアウトは `{"response", "names", "points"}`（`response` に `csv` は無い）、
/// 列は `names` と同じ順の f64（little-endian）の BitArray。
@external(erlang, "globe_mode_nif", "simulate_columns")
pub fn simulate_columns(
  ws: Workspace,
  model: String,
  settings_json: String,
) -> Result(#(String, List(BitArray)), String)

/// 中断の合図（S131、ADR-0033。Rust の `CancelToken`）。
pub type CancelToken

/// Erlang のプロセス（進捗の送り先）。gleam_erlang を足さないので不透明な型にする。
pub type Pid

/// 呼んだプロセス（`erlang:self/0`）。
@external(erlang, "erlang", "self")
pub fn self() -> Pid

@external(erlang, "globe_mode_nif", "cancel_token")
pub fn cancel_token() -> CancelToken

/// 合図を立てる。実行は次の区切りで止まる。戻り値は Erlang の `ok`。
@external(erlang, "globe_mode_nif", "cancel")
pub fn cancel(token: CancelToken) -> Dynamic

/// `simulate` と同じだが、進捗のたびに `pid` へ `{globe_progress, #{stage, time, start_time, stop_time}}` を送り、
/// `token` が立てられたら止める（S131）。止めたら `termination` が `cancelled` で途中までの CSV と `GM-I11004`。
/// 終わるまで戻らないので、別のプロセスで回す。
@external(erlang, "globe_mode_nif", "simulate_observed")
pub fn simulate_observed(
  ws: Workspace,
  model: String,
  settings_json: String,
  pid: Pid,
  token: CancelToken,
) -> Result(String, String)

/// FMU を書き出して `.fmu` のパス（S118。`globe fmu export` と同じ）。`options_json` は
/// `{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}`（`"{}"` で既定）。
@external(erlang, "globe_mode_nif", "export_fmu")
pub fn export_fmu(
  ws: Workspace,
  model: String,
  out_dir: String,
  options_json: String,
) -> Result(String, String)

@external(erlang, "globe_mode_nif", "solvers")
pub fn solvers() -> Result(String, String)

/// 本文を整形する（S111）。文法が壊れていれば `Error(理由)`。
@external(erlang, "globe_mode_nif", "format")
pub fn format(
  text: String,
  indent: Int,
  insert_origin: Bool,
) -> Result(String, String)

/// lint の結果（`[{rule, line, column, message}]` の JSON。S111）。
@external(erlang, "globe_mode_nif", "lint")
pub fn lint(text: String) -> String
