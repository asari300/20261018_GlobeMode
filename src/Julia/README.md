# src/Julia

Julia から `globe-mode-c` の C ABI を `ccall` だけで呼ぶ（P7、S075）。パッケージにはしていない（`globe_mode.jl` 1 本と `test.jl`）。ロジックは Rust 側に置く。

English: a single-file `ccall` binding (`globe_mode.jl`) to the GlobeMode C ABI and a test that runs `check` then `simulate` on `GlobeModeValidation.Simulation.Decay`.

```bash
mise run c:julia   # cargo build --release -p globe-mode-c → julia test.jl
```

- `globe_mode.jl`: `module GlobeMode`。`Workspace(libraries)`（finalizer で `gm_workspace_free`）、`set_file`、`check`、`simulate`。結果は JSON の文字列のまま返す（JSON の標準ライブラリが無いので解析しない）。
- 共有ライブラリは環境変数 `GLOBE_MODE_LIB`（既定 `../../target/release/libglobe_mode_c.so`）。
- 今後: パッケージ化と JLL（BinaryBuilder）での配布。
