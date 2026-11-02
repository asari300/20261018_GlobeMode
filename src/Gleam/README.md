# src/Gleam

Gleam のラッパ（P7）。Erlang target では `@external` で `globe_mode_nif` を呼び、JS target では `globe-mode-wasm` を呼ぶ（JS target は未）。ロジックは Rust 側に置く。

| ディレクトリ | 内容 |
| --- | --- |
| `globe_mode/` | S087。`src/globe_mode.gleam` が `@external(erlang, "globe_mode_nif", …)` で Rustler の NIF（`globe-mode-nif`）の `new` / `set_file` / `load_dir` / `check` / `graph` / `simulate` / `solvers` を宣言する。結果は JSON の文字列。`.so` は `src/globe_mode_nif.erl` が環境変数 `GLOBE_MODE_NIF`（拡張子を除いたパス）から読む |
| `wisp_fmt/` | CLI を呼ぶウェブアプリ |

```bash
mise run gleam:test   # cargo build --release -p globe-mode-nif → gleam test（Decay を check → simulate）
```
