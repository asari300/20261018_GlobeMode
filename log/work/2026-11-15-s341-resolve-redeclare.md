# S341 `resolve-redeclare`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s341-resolve-redeclare`（main から切った） |
| worktree | `.git-worktree/s341-resolve-redeclare` |
| Phase | Track B（B2 の続き） |
| セッション文書 | `doc/sessions/S341-resolve-redeclare/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

Track B の自前の resolve（`globe-mode-resolve`）で、MSL の例題 566 本のうち 99 本が「未解決の参照が 0」にならない（理由はすべて `Redeclare`。未解決の参照の総数 1330）。差し替えた後の型を引いて解決できるようにする。回帰 0、`msl:diff-frontend` を 1 回回して resolve 467/566 の変化と未解決の参照の総数を書く。

## 決めたこと

- まず、99 本のクラスから `diff-frontend` と同じ手順で未解決の参照を全部出して形ごとに数えた（一時の `#[ignore]` 試験。コミットしない）。1330 件はすべて成分参照の続きで、2 つの形だった: (1) Media の `state.T`・`state_a.p`（型 `ThermodynamicState` が基底では空の `replaceable record` で、受け継いだ側の外側のパッケージが `class extends` で差し替える）、(2) `extends B(redeclare T x)` の部品の再宣言（`internalThermalPort`・`thermalPort`・`cellData`）。
- 直しは `step`（部品の型の中で続きを探す所）に、既定の型で引けず理由が `Redeclare` のときだけ差し替えた型を試す。(1) は参照のあるクラスから型の名前を引き直す（部品の持ち主を受け継ぐときだけ）、(2) は参照のあるクラスの受け継ぎをたどって `redeclare T x` を探す。flat の S297 の `cenv` は再利用しなかった（flat は実体ごとの環境を持つが、resolve は参照の場所だけで決まるので、受け継ぎの木をたどれば済む）。
- 既定の型で引けたものは変えない（回帰 0 と、LSP の定義ジャンプの突き合わせを保つため）。flat は `resolve_head` と型名だけを使うので影響しない。

## やったこと

- `globe-mode-syntax/src/ast.rs`: `RedeclaredComponent`・`ExtendsClause`/`ShortBody::redeclared_components`。経路を読む関数（`path_at`・`constraint_from`）をクラスの再宣言と共用に切り出した。
- `globe-mode-resolve`: `PathRef::comp_redecls`（`build.rs`）、`step`・`comp_redecl`・`reaches`（`lookup.rs`）。試験 `redeclared_component_types`。
- `xtask diff-frontend --dump`: 未解決の参照の行（`unresolved …`）を出す。
- 文書: ADR-0039 付記（S341）・roadmap・architecture・`doc/frontend-diff.md`（生成）・セッション文書・索引。

## 検証

- 99 本＋1 本のクラスの未解決の参照: 1343 → 0（一時の試験）。MSL の全 `.mo`（`msl_stats`）の成分参照の続き 45463 件の未解決: 639 → 0。
- `mise run msl:diff-frontend`（release の xtask を worktree でビルドして scratchpad にコピーし、`flock` つきで同じ引数で回した。他の WP が先に走っていたので待ち、走っている間は swap していた。全体 1880 s、最大 RSS 3.9 GiB）: **resolve 467/566 → 566/566**、未解決の参照の総数 **1330 → 0**。`GlobeModeValidation` 74/75 → **75/75**、13 → **0**。flat 471/566・Rumoca も通った 463/471・名前の一致 463/463・方程式の数 451/463・DAE 471/471・釣り合い 334/334・状態 332/334・正則 334/334 は変わらない。**回帰 0 本、改善 100 本**（すべて resolve の旗。MSL 99 本＋`GlobeModeValidation.Connect.MediumFlow`）。
- `cargo test -p globe-mode-resolve -p globe-mode-syntax -p globe-mode-flat -p globe-mode-lsp`（全 pass）、`-p globe-mode-frontend --features native`（全 pass）、`-p globe-mode-lsp --test resolve_compare -- --ignored`（定義ジャンプの Rumoca との突き合わせ: 型名 110 箇所は一致 110・不一致 0、成分参照の続き 93 箇所は一致 86・LSP が null 7・不一致 0。S275 と同じ）、clippy（syntax・resolve・flat・xtask、`-D warnings`）0 件、`cargo fmt --check` 差分なし。
- `check_docs.py`: 全デッキ OK、相対リンク 5172 件中の切れ 0 件。`render_slides.py`（S341 のデッキ）: 問題 0 件。

## 残したこと

- 部品の修飾の中の `A a(redeclare T x)`（MSL の例題には出ていない）。
- LSP の補完 `state.` は既定の型の要素のまま。
