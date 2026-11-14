# S275 `resolve-paths`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s275-resolve-paths` |
| worktree | `.git-worktree/s275-resolve-paths` |
| Phase | Track B（B2 の 2 つ目） |
| セッション文書 | `doc/sessions/S275-resolve-paths/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

`globe-mode-resolve` で、成分参照の続き（`a.b.c`）を部品の型の中へたどり、`replaceable` の既定の型・`constrainedby`・修飾の `redeclare package M = X` を 1 段だけ扱う。MSL の計測に続きまでの解決率を足し、LSP との突き合わせを続きにも広げる。

## 決めたこと

- `resolve` は続きまで、`resolve_head` は S272 と同じ先頭まで（前後の比較用）。
- 部品の型は `constrainedby` を先、宣言の型を後に試す（`Component::tys`）。型が組み込みなら続きは `Builtin`（属性）。
- `redeclare package M = X` は `extends`・部品・short class の修飾の直下だけ読む（`ast::RedeclaredClass`。パーサは変えない）。部品の再宣言と長い形は読まない。
- 継いだ側の `redeclare` を、継がれた側で宣言された部品の型に効かせる（使う側のクラスで型名を引き直す）のは、当て推量になるのでやらない。`Reason::Redeclare` のまま。

## やったこと

- `globe-mode-syntax/src/ast.rs`: `RedeclaredClass` と、`ExtendsClause`・`ShortBody`・`Declaration` の `redeclared_classes()`。
- `globe-mode-resolve`: `Component` に型の候補と修飾の置き換え、`PathRef` に修飾の置き換え、`St::Builtin`、`lookup.rs` に `step`（部品の続き）・`redecl_hit`・`target_state`。`tests/resolve.rs` に単体試験 2 本と、`msl_stats` に続きの解決率。
- `globe-mode-lsp/tests/resolve_compare.rs`: 突き合わせを共通の関数にして、型名に続いて成分参照の続きも比べる。
- doc: ADR-0039 の付記、roadmap（Track B の 2 か所）、architecture、features、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-resolve -p globe-mode-syntax`: resolve 5 pass・1 ignored、syntax 9 pass・1 ignored。clippy `-D warnings`（resolve・syntax・lsp の全 target）0、fmt ok。
- MSL 4.1.0（debug、2556 本・参照 161869 件）: 続きのある成分参照 45463 件のうち 44824 件が最後の名前まで解決、639 件が `Redeclare`。S272 の未解決 10 件は 0。続きの解決は S272 では 0 件。`NotFound`・`Cycle`・`Inherited` は 0。
- LSP との突き合わせ（`#[ignore]`、debug で約 8 分）: 型名 110 箇所すべて一致。続き 93 箇所で一致 86、不一致 0、LSP が null 7。

## 残したこと

- `Redeclare` の 639 件（大半は Media の `state.T` など 2 段以上先。`cellData.…` は部品の再宣言。`internalThermalPort.heatPort…` は原因を調べていない）。
- LSP・`Frontend` へのつなぎ、文の型付き AST、診断の番号（`GM-E`）、`protected` へのアクセス検査。
