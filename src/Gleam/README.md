# src/Gleam

Gleam のラッパ（P7）。Erlang target では `@external` で `globe_mode_nif` を呼び、JS target では `globe-mode-wasm` を呼ぶ（JS target は未）。ロジックは Rust 側に置く。

| ディレクトリ | 内容 |
| --- | --- |
| `globe_mode/` | S087。`src/globe_mode.gleam` が `@external(erlang, "globe_mode_nif", …)` で Rustler の NIF（`globe-mode-nif`）の `new` / `set_file` / `load_dir` / `check` / `graph` / `simulate` / `solvers` を宣言する。結果は JSON の文字列。`.so` は `src/globe_mode_nif.erl` が環境変数 `GLOBE_MODE_NIF`（拡張子を除いたパス）から読む |
| `wisp_fmt/` | CLI を呼ぶウェブアプリ |

```bash
mise run gleam:test   # cargo build --release -p globe-mode-nif → gleam test（Decay を check → simulate）
```

最初の例（`src/Gleam/globe_mode/src/first.gleam` に置き、`src/Gleam/globe_mode` で `GLOBE_MODE_NIF=<target/release/libglobe_mode_nif> gleam run -m first`）。JSON の解析は呼ぶ側の仕事で、標準ライブラリだけなので文字列で切り出している:

```gleam
import gleam/io
import gleam/string
import globe_mode

pub fn main() {
  let assert Ok(ws) = globe_mode.new(["../../Modelica/GlobeModeValidation"])
  let model = "GlobeModeValidation.Simulation.Decay"
  let assert Ok(report) = globe_mode.check(ws, model)
  assert string.contains(report, "\"passed\":true")
  let assert Ok(res) = globe_mode.simulate(ws, model, "{}")
  let assert Ok(#(_, rest)) = string.split_once(res, "\"final\":{\"x\":")
  let assert Ok(#(x, _)) = string.split_once(rest, "}")
  io.println("x(1) = " <> x)   // 0.367879468116207（e⁻¹）
}
```
