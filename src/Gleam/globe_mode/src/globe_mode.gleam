//// GlobeMode の Gleam の束ね（S087、Erlang target）。NIF の関数を `@external` で宣言するだけで、ロジックは Rust 側にある。
////
//// NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス）。結果は JSON の文字列で、解析は呼ぶ側に任せる。

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

@external(erlang, "globe_mode_nif", "solvers")
pub fn solvers() -> Result(String, String)
