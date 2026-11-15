# S325 `dae-regularity`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s325-dae-regularity`（`feature/s312-dae-mismatches` から切った） |
| worktree | `.git-worktree/s325-dae-regularity` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S325-dae-regularity/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S312 で残った構造の正則の食い違い 17 本（Clocked の例題 16 本と `Modelica.Media.Examples.WaterIF97`。自前が非正則、Rumoca が正則）の原因を確かめて直す。`globe-mode-dae` の中だけを変える。

## 決めたこと

- S312 の見立て（Clock の信号を Rumoca が離散側に入れている）は当たり。Rumoca の `rumoca-phase-dae`（`analysis/clocks.rs`）は MLS 16.5.1 のクロックの区画を求める。自前は素集合で区画を作り、`sample` などの結果・`previous`・`when Clock()` を含む区画を離散にする簡易版にした（持ち主・変換の整合・`der` は見ない）。
- Clocked の 4 本（`CascadeControlledDrive`）は区画でなく、`if` の枝が離散と連続の代入を混ぜていた。`WaterIF97` はレコードの離散の欄。原因は 3 つに分かれた。
- 計測は worktree 専用の `CARGO_TARGET_DIR`（`target-s325`）で回した。共有 `target-shared` の `target/release/xtask` が別の worktree に上書きされ、`--dump` の新しい出力が出ない・直した結果が出ないことがあったため。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: 区画（`Ctx::partition`・`is_clocked_when`・`Partitions`）、`split_if`、レコードの離散の欄。試験 3 本。
- `src/Rust/xtask/src/diff_frontend.rs`: `--dump` に when・algorithm と、自前・Rumoca の `DaeStructure`（行の incidence と、参照されない未知数）を足した。
- 文書: ADR-0039 に付記（S325）、roadmap・architecture・features、`doc/frontend-diff.md`（`msl:diff-frontend` が書いた）。

## 検証

- 構造の正則の一致（MSL の例題、Rumoca も通った 201 本）: 184/201 → 201/201。回帰 0・改善 17。方程式の数 273/273・釣り合い 201/201・状態の数 201/201・`GlobeModeValidation`（正則 72/72）は変わらず。
- `cargo test -p globe-mode-dae`: 10 本通過。

## 残したこと

- クロックの持ち主・変換の整合・区画の `der`、入れ子の `if`・`for` の離散と連続の混在。
- 計測用の `target-s325` は締めで消す。
