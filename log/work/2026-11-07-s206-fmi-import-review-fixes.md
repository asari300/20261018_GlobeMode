# S206 `fmi-import-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s206-fmi-import-review-fixes` |
| worktree | `.git-worktree/s206-fmi-import-review-fixes` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S206-fmi-import-review-fixes/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S205（FMI Import の ME）への ponytail-review（Fable、親）の指摘 1〜4 を、1 件ずつ失敗する試験で再現してから直す。`Discard` を同じ道で扱えるか調べる。ユーザーが 2026-11-07 に決めた「FMI Import の案 B: 自前で作成」を ADR-0026 の (m) に書き、案 B にあって自前に無いものの表を作る。

## 決めたこと

- ユーザーの決定（案 B は自前で作成）: 親の解釈で、`fmi` crate は入れず FMI Import は全部、自前の `globe-mode-fmi-import` で進める。ADR (m) に「解釈であること」を明記し、(g) の戻す条件は消さずに打ち消した。
- `Discard`: 縮めて再試行にした（最小で済んだため）。`Error::Discard` を足し、`check` が status 2 で返す。Dormand–Prince だけ。RK4 と二分法の中は縮めず止まる（限界として ADR に）。
- 格子点の上限は 1000 万点（行 64 B + 8 B × 出力で、出力 10 個で約 1.3 GB）。
- `DefaultExperiment.step_size` は文字列のまま（他の属性と同じ）。None なら属性を書かないので Export の XML は変わらない。

## やったこと

- `globe-mode-fmi`: `DefaultExperiment.step_size`、読み書き、往復の試験。`globe-mode-fmu` は `step_size: None`（`DefaultExperiment` を組み立てていたのはここだけ）。
- `globe-mode-fmi-import`: `ode.rs`（却下と `Discard`）、`lib.rs`（`Error::Discard`、`grid_points`、`default_step_size` を削除）、`me.rs`（`grid_points`、模擬 FMU の `Discard` の試験）、`abi3.rs`（`u8`）、`tests/`（格子の上限）。CLI の試験に格子の上限。
- 文書: ADR-0026 の (m)、licenses.md・features.md・architecture.md・roadmap.md、CHANGELOG、セッションのスライドと解説書、レビューの採否。

## 検証

- 再現: 指摘 1 は、直す前のロジック + 新しい試験で 2 本とも失敗（`Ok(NaN)`、`Discard` がそのまま `Err`）。直した後は通る。
- コードの検査: `cargo fmt`・`clippy -D warnings` は通った。`cargo test -p globe-mode-fmi -p globe-mode-fmi-import -p globe-mode-fmu -p globe-mode-cli` は全部通った（fmi-import の単体 10、model_exchange 7、reference 3、fmpy 11。fmpy は負荷 100 前後で 2786 s かかった）。`cargo test --workspace`（fmu 以外）755 本通過・失敗 0。`mise run check` の途中で `globe-mode-fmu` の fmpy が共有 target の古い `globe-mode-fmi` で E0560 になった（`touch` して回し直して通った）ので、残りの `cargo deny check`（ok）・`uv sync --locked`（ok）は別に回した。
- `mise run wasm:node`: 通った（EXIT 0）。
- `check_docs.py`: デッキ OK / リンク 4103 件中の切れ 0 件。`render_slides.py`: 問題 0 件。
- push と workflow（feature `cb098ce`）: `fmu-import.yml` 42317246915 success（ubuntu・macOS・windows の 3 OS）、`fmu-export.yml` 42317250797 success、`ci.yml -f full=true` 42317255030 success。

## 残したこと

- 陰的な積分、連続時間の入力、`HybridSystem` へのアダプタ、ワーカー、facade・UI、マスター、SE（ADR (k)）。
- ADR (m) の表で「足す」にした項目（(k) 6）。
- 未確認: 上がるときの state event の数え方（表示だけ）。
- 二分法の中の `Discard`（縮めずに止まる）。
