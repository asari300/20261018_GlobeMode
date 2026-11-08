# S240 `fmu-cdylib-split`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s240-fmu-cdylib-split` |
| worktree | `.git-worktree/s240-fmu-cdylib-split` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S240-fmu-cdylib-split/` |
| セッション記録 | （親が写す） |

## 依頼

S239 の持ち越し。cdylib を別 crate に出し、facade は rlib だけに依存する。S239 で置いた `doctest = false`（facade・c・lsp・py・wasm）を外す。

## 決めたこと

- `globe-mode-fmu` は rlib だけ（本体と `extern "C"` の入口は `Instance` の内部と密なのでここに残す）。新しい `globe-mode-fmu-cdylib` は cdylib だけで `pub use globe_mode_fmu::*;` の 1 行。入口は共有ライブラリから出る（`nm -D` で `fmi2*`・`fmi3*` が 119 個）。
- 共有ライブラリの名前は `libglobe_mode_fmu_cdylib.so`（Windows `globe_mode_fmu_cdylib.dll`、macOS `.dylib`）。rlib と lib 名が同じだと出力名がぶつかるため。FMU の中は `globe_mode_fmu.<拡張子>` のまま。
- 入れ子の `cargo build -p` は `globe-mode-fmu-cdylib`（`tests/fmpy.rs`、CLI の試験、facade の `build_runtime`、`fmu-export.yml`、説明文）。
- `ci_plan.py` は `Cargo.toml` から依存を読むので不変。`test_ci_plan.py` の期待に新 crate を足した。

## 検証

- `cargo test --workspace` を 2 回続けて: どちらも exit 0、849 passed / 0 failed、E0460・E0463 は 0 件。Doc-tests は globe_mode・c・lsp・py（`_native`）・wasm を含めて実行（facade の Decay の例が戻った）。
- `test_ci_plan.py` 28 本 OK。
- `mise run check`（2 回）: （下に追記）
- CI: （下に追記）

## 残したこと

- 閉包のクラス単位化、配布物への cdylib の同梱、cdylib の大きさ（`opt-level`・LTO）、cross。
