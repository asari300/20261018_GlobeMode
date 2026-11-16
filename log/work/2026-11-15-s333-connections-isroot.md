# S333 `connections-isroot`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s333-connections-isroot`（`feature/s332-trackb-after-329` から切った） |
| worktree | `.git-worktree/s333-connections-isroot` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S333-connections-isroot/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S332 が特定した MultiBody の DAE の不一致を、`Connections.isRoot`・`rooted` の根の決定で直す。

## 決めたこと

- `if` を枝ごとに平坦化して置き、根を決めたあとに選ぶ（`OcIf`）。S312 の文字による判定は統合した。
- 根・深さ・木の辺の順は Rumoca の `build_vcg` と同じ段取り（コードは写さず、読んで自前で書いた）。
- 1 回目の計測で `UniversalConstraint` が回帰に出た。原因は枝の和集合と打ち消し合っていた `a * b`（ベクトルの積）の数え間違い。`dims_of` を直して回し直した。

## やったこと

- `globe-mode-flat/src/flatten.rs`: `defer_root_if`・`if_branches`、`dims_of` の `*`。
- `globe-mode-flat/src/flatten/graph.rs`: 木の辺の順、根・深さ、`OcIf` の解決。
- 試験 4 本。文書: ADR-0039 付記（S333）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

- `mise run msl:diff-frontend`（519 s、6.0 GiB）: flat の方程式の数 435/463 → 449/463、DAE の判定・数 297/334 → 332/334、状態 294/334 → 332/334、正則 296/334 → 301/334、回帰 0、改善 39 本。`GlobeModeValidation` 変わらず。
- `cargo test -p globe-mode-flat` 33 本、clippy 0。

## 残したこと

- 構造の正則 33 本（MultiBody、原因未確認）、`PrismaticConstraint`・`Fourbar1` の方程式の数。
