# src/Julia

Julia から `globe-mode-c` の C ABI を `ccall` だけで呼ぶ（P7、S075）。パッケージにはしていない（`globe_mode.jl` 1 本と `test.jl`）。ロジックは Rust 側に置く。

English: a single-file `ccall` binding (`globe_mode.jl`) to the GlobeMode C ABI and a test that runs `check` then `simulate` on `GlobeModeValidation.Simulation.Decay`.

```bash
mise run c:julia   # cargo build --release -p globe-mode-c → julia test.jl
```

最初の例（リポジトリの根から。`mise run c:julia` が先に `libglobe_mode_c.so` を作る。`GLOBE_MODE_LIB` はそのパス）:

```julia
include("src/Julia/globe_mode.jl")
ws = GlobeMode.Workspace(["src/Modelica/GlobeModeValidation"])
model = "GlobeModeValidation.Simulation.Decay"
occursin("\"passed\":true", GlobeMode.check(ws, model))   # true
res = GlobeMode.simulate(ws, model)                       # JSON の文字列（csv と final を含む）
parse(Float64, match(r"\"final\":\{[^}]*\"x\":([^,}]+)", res)[1])   # 0.367879468116207（e⁻¹）
```

- `globe_mode.jl`: `module GlobeMode`。結果は JSON の文字列のまま返す（JSON の標準ライブラリが無いので解析しない。上の例は正規表現で拾う）。
  - `Workspace(libraries)`（finalizer で `gm_workspace_free`）、`set_file`、`load_dir`（読んだ `.mo` の数）、`check`、`graph`。
  - `simulate`、`simulate_columns`（列の道。`(レイアウトの JSON, 列ごとの Vector{Float64})`）、`simulate_observed`（進捗と中断。`on_progress` が `true` を返すと止まる）、`export_fmu`。
  - `format`・`lint`（`Workspace` は要らない）。
  - `outline(ws, path)`・`deps(ws, model)`・`flat(ws, model)`（読み取り。S191。JSON の文字列。`globe outline` / `deps` / `flat --json` と同じ形。[対応表](../../doc/research/lang-api-matrix-2026-11-03.md)）。`simulate_columns` のレイアウトの `defaults` は既定で描く信号の名前（S190）。
- `simulate_observed` の `on_progress` が例外を投げる（Ctrl-C の `InterruptException` を含む）と、Rust のフレームを越えず、中断（`true`）として扱われて、`ccall` が返ってから投げ直される。`ws` は使用中のまま固まらない（S214。`test.jl` が確かめる）。実行中は `GC.@preserve` で `ws` の finalizer を止める。
- 共有ライブラリは環境変数 `GLOBE_MODE_LIB`（既定 `../../target/release/libglobe_mode_c.so`）。
- 進捗のコールバックの中から同じ `ws` を使うと（`check` など）`{"error": "ワークスペースは使用中…"}` が返り、`gm_workspace_free` は何もしない（S202。`test.jl` が確かめる）。
- 今後: パッケージ化と JLL（BinaryBuilder）での配布。
