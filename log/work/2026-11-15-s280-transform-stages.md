# S280 `transform-stages`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s280-transform-stages` |
| worktree | `.git-worktree/s280-transform-stages` |
| Phase | P8（変換デバッガの 2 つ目） |
| セッション文書 | `doc/sessions/S280-transform-stages/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

`globe stages -m <model> [-L lib] [--json]` で、変換の段ごとの要約と差分を出す。新しい解析は足さない。CLI と facade だけ。

## 決めたこと

- flat → DAE は数だけにした。Rumoca の DAE は別名を畳まず（`y = x` が方程式のまま残る）、`FlatSummary` に変数名が無いので、名前の差分は取れない。名前を出す差分はインデックスの低減と tearing。
- 型は `globe-mode-analysis::stages` に新設し、既存の `LoopReport`・`BltReport` は変えない（S9 の JSON と snapshot を動かさない）。
- 試験は 1 本。別名のあるモデルは `src/Modelica` に足さず（位置と本数に依存する試験があるため）、手元の一時ファイルで「flat と DAE の数が同じ」ことだけ確かめた。

## やったこと

- `globe-mode-analysis/src/stages.rs`（新規）、`loops.rs` の `column_name` を `pub(crate)` に。
- `GlobeDb::stages`、`Workspace::stages`、`globe stages`（text と `--json`）。
- 試験 `stages_summarizes_each_transform_stage`（NoLoop・NonlinearLoopWithStart・Index.Pendulum）。
- doc: features・roadmap（2 か所）・architecture、セッション文書と索引。

## 検証

- `cargo fmt --check`: 差分 0。`cargo clippy -p globe-mode-analysis -p globe-mode-db -p globe-mode -p globe-mode-cli --all-targets -- -D warnings`: 警告 0。
- `cargo test -p globe-mode-cli -p globe-mode-analysis`: 失敗 0（cli_flow 13 本中 12 本 + 1 ignore、新しい試験は pass）。
- 出力例: Pendulum はインデックス 3・微分した方程式 3 本（方程式 +4・未知数 +2: `der(der(x))`, `der(der(y))`）・BLT は大きさ 5 が 1 つ・tearing 変数 `lambda`。ParallelCapacitors はインデックス 2・微分 1 本・大きさ 4 のループ。

## 残したこと

- 段の UI・LSP・Python。変換前後の式の差分（別名の除去、微分した式）。flat の変数名を持てば flat → DAE の名前の差分が出せる。
