# S354 `lsp-call-hierarchy`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s354-lsp-call-hierarchy`（base は S352 の `feature/s352-lsp-inlay-hints`。main に S352 が未着だったため） |
| worktree | `.git-worktree/s354-lsp-call-hierarchy` |
| Phase | Track B（自前コア）の製品へのつなぎ |
| セッション文書 | `doc/sessions/S354-lsp-call-hierarchy/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

LSP に call hierarchy の最小版を足す。`textDocument/prepareCallHierarchy`（カーソルの関数）・`callHierarchy/incomingCalls`・`callHierarchy/outgoingCalls`。対象は `function` だけ。参照の集め方は既存の references（S230）か resolve を再利用し、新しい規則を作らない。引けない呼び先は出さない。capability の宣言、試験、features の行と ADR の付記。flat と VS Code は触らない（S335・S353 が並行）。

## 決めたこと

- **base は S352 のブランチ。** S352 の `CallExpr` を resolve で引く道（inlay hint）が呼び先にそのまま使え、main への取り込みも S352 の後ろに並ぶ。
- **呼び元は references の道 + 構文の木での絞り込み。** `class_hits`・`disk_class_hits` は識別子の一致を返すので、宣言・`end f`・`extends f(…)` が混じる。自前の構文の `CallExpr` の呼び先の名前の終わりと一致するものだけを呼び出しとした（字面の規則を増やさない）。
- **呼び先の関数判定は resolve に `Class.is_function` を足した。** `Class` に種類が無かった。`kind_token()` が `function` か。レコードの構築子（`R(x)`）を除くのに要る。呼び元の探索の関数判定は、`class_name_at` が返す定義の場所をアウトラインの `ClassKind::Function` で確かめる（既存の道）。
- **状態を持たない。** 項目の `uri` と `selectionRange` の頭から毎回 `function_at` / `with_source` で引き直す。`data` は使わない。
- **呼び元の項目はクラスなら何でもよい**（関数 = SymbolKind 12、ほか 5）。モデルの項目から先は `null`（関数だけ）。

## やったこと

- `globe-mode-resolve`: `Class.is_function`（`lib.rs`・`build.rs`）。
- `globe-mode-lsp`: `src/call_hierarchy.rs`（新規）、`lib.rs`（モジュール・capability `callHierarchyProvider`・3 つの要求の口・冒頭の説明）、`definition.rs`（`enclosing` を `pub(crate)`）、`tests/call_hierarchy.rs`（新規。1 本）。
- 文書: features・ADR-0039（付記 S354 と製品へのつなぎの表）・architecture・roadmap・CHANGELOG・`doc/sessions/S354-lsp-call-hierarchy/`・`doc/sessions/README.md`・この記録。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-resolve`: lsp の lib 75 本 pass（1 本 ignored）・`call_hierarchy` 1・cross_file 5・folding 2・tolerant_outline 2、resolve の `tests/resolve.rs` 9 本 pass（2 本 ignored）。`cargo fmt` 済み、`cargo clippy -p globe-mode-lsp -p globe-mode-resolve --all-targets -- -D warnings` 0 件。
- 試験の効きの確認: `is_function` の検査を外すと outgoing(g) にレコード `R` が出て落ちる（戻した）。
- 回していないもの: `mise run check` 全体（依存を足していない）、release・wasm、`resolve_compare`（resolve の解決の規則は変えていない）、本物の VS Code（【未確認】）、`msl:*`。セッション記録は無いので `--check` は省く。

## 残したこと

- 構文の誤りでアウトラインが引けないファイルの関数は呼び元の探索の対象外。呼び元の探索は references と同じ費用（索引なし）。別の開いた文書の関数は呼び先に引けない。レコードの構築子・`operator function`。本物の VS Code での表示（【未確認】）。
