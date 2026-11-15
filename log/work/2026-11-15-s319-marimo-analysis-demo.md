# S319 `marimo-analysis-demo`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s319-marimo-analysis-demo` |
| worktree | `.git-worktree/s319-marimo-analysis-demo` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S319-marimo-analysis-demo/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

直近の Python の解析の機能（linearize・blt・stages・cosim・check の native）を marimo のノートブック 1 本にまとめた見本を作る。

## 決めたこと

- `src/Python/marimo/analysis_app.py`（4 節）と `test_analysis_app.py`（2 本）。Rust・依存は変えない。
- 図は plotly、表は polars。FMU か `globe` が無ければコシミュレーションの節は案内だけ。
- README の Python の節、marimo の README、features の marimo の行に追記。

## 検証

- `.venv/bin/pytest src/Python/marimo/test_analysis_app.py`: 2 本 pass（`GLOBE_BIN` は共有 target の debug の `globe`）。
- `marimo check`: 警告 0。`marimo export html`: エラー 0。
- Jacobi は 1 刻み遅れ（|差| の最大は Dahlquist の 1 刻みの減り）、Gauss–Seidel は 0。
- `uv run` は拡張を作り直しにかかったので（CARGO_TARGET_DIR を付け忘れた）止め、`.venv/bin` を直接使った。

## 残したこと

ブラウザでの表示の確認、`parameters` を動かす UI、wasm 版。
