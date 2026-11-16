# S300 `diff-frontend`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s300-diff-frontend` |
| worktree | `.git-worktree/s300-diff-frontend` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S300-diff-frontend/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

`cargo xtask diff-frontend` を作り、自前の段（parse → resolve → flat → dae）と Rumoca を MSL の例題と GlobeModeValidation で一度に突き合わせ、段ごとの表を `doc/frontend-diff.md`、JSON を `target/frontend-diff.json` に書く。前回の JSON との回帰を報告の先頭に出す。

## 決めたこと

- 数え方は xtask の 1 か所に一本化し、`globe-mode-flat`・`globe-mode-dae` の `tests/compare.rs`（`#[ignore]`、`one`・`show_model` などのデバッグ用も含む）と `globe-mode-frontend` の dev 依存を消した。`test_ci_plan.py` の期待値から flat・dae（frontend の dev 依存元だったため）を外した。
- 旗 12 個（parse・resolve・flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular）。前回の JSON で通っていた旗が通らなくなったら回帰、終了コード 1。
- resolve は初版（例題のクラスの中だけ）で 566/566 になり判定にならなかったため、`extends`・部品の型をたどった到達先まで数えるようにした（467/566、未解決は全部 `Redeclare`）。
- Rumoca は自前で平坦化できたものにだけ呼ぶ（Rumoca 側の全体の本数は `msl-funnel` の報告）。
- mise のタスク `msl:diff-frontend` を足した。CI には入れない。

## やったこと

- `src/Rust/xtask/src/diff_frontend.rs`（新規）、`main.rs`（サブコマンド）、`Cargo.toml`（syntax・resolve・flat・dae の path 依存。外部依存は無し）、`Cargo.lock`、`mise.toml`。
- 文書: `doc/frontend-diff.md`（生成物）、`doc/roadmap.md` の Track B の 2 か所、`doc/architecture.md` の xtask の 2 行、ADR-0039 の付記（S300）、セッション文書と索引。

## 検証

- 全体の実行（release、MSL 566 本 + GlobeModeValidation 75 本）: 77〜99 秒、最大 RSS 1.1 GiB（`/usr/bin/time -v` の 1,166,496 kB）。load は他のセッションと同時で 3〜9。
- MSL: parse 566/566、resolve 467/566、flat 192（Rumoca も通った 189、名前 189/189、方程式の数 186/189）、dae 192（Rumoca の釣り合いまで通った 146、判定 144/146、方程式と未知数の数 144/146、状態 141/146、正則 132/146）。
- GlobeModeValidation: parse 75/75、resolve 74/75、flat 72（Rumoca も 71、名前 71/71、方程式 71/71）、dae 72（釣り合い 71 で判定・数・状態 71/71、正則 70/70）。
- 回帰の仕組み: resolve の定義を変えた 2 回目の実行で、回帰 100 本・終了コード 1。同じ定義の 3 回目で回帰 0・改善 0・終了コード 0。
- `cargo test -p xtask diff_frontend`（1 本）、`cargo clippy -p xtask --all-targets -- -D warnings`、`test_ci_plan.py`（29 本）が通る。

## 残したこと

- 回帰の閾値（`native` の既定を切り替える基準）は決めていない。
- CLAUDE.md に足す文（親が反映する）: 「Track B の自前の段（`globe-mode-syntax`・`-resolve`・`-flat`・`-dae`）か Rumoca の rev を変えたら、`mise run msl:diff-frontend`（自前の parse → resolve → flat → dae を Rumoca と MSL の例題 566 本・GlobeModeValidation 75 本で突き合わせ、前回の `target/frontend-diff.json` との回帰を報告 `doc/frontend-diff.md` の先頭に書く。release で約 1.5 分、最大 RSS 1.1 GiB。回帰があれば終了コード 1）を通し、回帰の本数を作業記録に書く。CI には入れていない。」と、表の `doc/msl-funnel.md …` の行に `frontend-diff.md` を足す。
