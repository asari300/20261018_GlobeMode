# S207 `lint-performance`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s207-lint-performance` |
| worktree | `.git-worktree/s207-lint-performance` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S207-lint-performance/` |
| セッション記録 | （親が写す） |

## 依頼

`globe_mode_fmt::lint`（と同じ原因があれば `format`）が本文の長さの 2 乗で遅くなる原因を突き止めて直す。S202 の観測は 1 MB で 2.2 s、5.4 MB で 52 s。lint・format の結果は 1 文字も変えない。

## 決めたこと

- 計測は crate に `examples/`・`benches/` を足さず、`~/.cache/globe-mode-s207/` の使い捨てのプログラム 2 つ（main の fmt と作業中の fmt を別々に指す）で行った。
- 原因は `lint.rs` の 2 か所だけで、`format`・字句解析・配置は線形だった。直しは最小（改行の位置の表 + 二分探索、最初の行の `HashMap`）。増分の lint は範囲外。
- 歯止めの試験は比（2 倍の本文で 3 倍未満、3 回の最小値）。直す前の lint.rs に戻すと debug で 4.08 倍になって落ちることを確かめた。

## やったこと

- `src/Rust/mod/globe-mode-fmt/src/lint.rs`: `line_col` を改行位置の表と `partition_point` に。`line-too-long` の元の行の探索を最初の行の `HashMap` に。
- `src/Rust/mod/globe-mode-fmt/tests/format.rs`: `lint_time_grows_linearly`。
- 文書: 解説書・スライド、CHANGELOG、roadmap、features、索引。

## 検証

- 前後の時間は [解説書](../../doc/sessions/S207-lint-performance/解説書.md) の 3 章。4.4 MB の lint が 88.7 s から 0.57 s、wasm の `Fluid/Dissipation.mo`（630 KB）が 508 ms から 43 ms。
- 結果の一致: MSL 2,556 本 + `src/Modelica`・`doc/research/upstream` 105 本 = 2,661 本 × 8 通り（lint・format、幅 80・30、LF・CRLF）で不一致 0。
- 共有 target の罠: 誤って `.git-worktree` で cargo を呼ぶと main の crate が共有 target の同じ名前の rlib を上書きし、worktree の変更が古いビルドのまま「fresh」と見なされた。`touch` で直した。
- `mise run check`: 通った（fmt・clippy・test・deny・`globe fmt --check src/Modelica`（0 ファイルが要整形）・`uv sync --locked`）。最初の 1 回は新しい試験の rustfmt で落ち、整形して回し直した。
- `mise run wasm:node`: 通った（EXIT=0）。
- `check_docs.py`: デッキ OK / リンク 4102 件中の切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件。
- CI: `gh workflow run ci.yml --ref feature/s207-lint-performance -f full=true`（Windows を含む）run 42312995721 は success（全 11 ジョブ: plan・lang・docs・rust-windows・python・deny・wasm-node・ui・rust-sundials・msl・rust-linux）。
