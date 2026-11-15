# S334 `multibody-regularity`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s334-multibody-regularity`（`feature/s333-connections-isroot` から切った） |
| worktree | `.git-worktree/s334-multibody-regularity` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S334-multibody-regularity/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S333 の後に残った、MultiBody の構造の正則 33 本と `PrismaticConstraint`・`Fourbar1` の方程式の数の原因を確かめて直す。

## 決めたこと

- `diff-frontend --dump` の行の incidence を、自前と Rumoca で最大マッチングにかけて比べた（`FreeBody` が最小）。未マッチは `Parts.Body` の `w_a = angularVelocity2(frame_a.R)` の行で、自前の incidence が空だった。
- 直しは、変数でないレコードの参照を欄すべてに触れるとみなす（`globe-mode-dae`）。Rumoca の v0.10.4 の確認は不要（自前の誤り）。
- `PrismaticConstraint` は左辺の関数の呼び出しの大きさ、`Fourbar1` は木に選ばれなかった辺の `zeros(3) = equalityConstraint`。

## やったこと

- `globe-mode-dae/src/lib.rs`: `leaves_of`。`globe-mode-flat/src/flatten.rs`: `fn_out_dims`・`class_out_dims`、`oc_nodes` を関数名と大きさつきに。`flatten/graph.rs`: 選ばれなかった辺の等式。
- 試験 3 本。文書: ADR-0039 付記（S334）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

- `mise run msl:diff-frontend`（554 s、5.8 GiB）: 構造の正則 301/334 → 334/334、DAE の判定・数 332/334 → 334/334、flat の方程式の数 449/463 → 451/463、回帰 0。`GlobeModeValidation` 変わらず。
- `cargo test -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend -p globe-mode-lsp` 通過、clippy 0、fmt 済み。

## 残したこと

- 状態の数 2 本（Electrical.Machines）、`ShowTransferFunction` の方程式の数。
