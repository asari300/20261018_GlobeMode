# S279 `lsp-completion-resolve`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s279-lsp-completion-resolve` |
| worktree | `.git-worktree/s279-lsp-completion-resolve` |
| Phase | Track B（IDE の機能から自前に） |
| セッション文書 | `doc/sessions/S279-lsp-completion-resolve/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

LSP の補完（`textDocument/completion`）で、`a.` の後ろに部品の型のメンバー（部品・クラス、受け継ぎを含む）を出す。構文の誤りのある書きかけの文書でも出るように、`globe-mode-syntax` の寛容な CST と `globe-mode-resolve` を使う。

## 決めたこと

- 今の補完は `a.` を、カーソルの行を空白にして読み直した `Outline` で引く（S171）。`a.b.` の続きも引ける。**出ないのは**、ほかの行にも誤りがある・`end` の前でファイルが切れている、のとき（試験で空になることを確かめた）。
- そこで「今の道で空のときだけ」自前の道を使う。読める文書の出力と既存の試験は変わらない。
- 部品の型の引きに S275 の `Component::tys`（`a.b.c` の続き）が要るので、S275 の feature ブランチを worktree に取り込んで作った（コミットは同一なので、main に S275 が入れば衝突しない）。S275 が先に main に入らなければ、この分岐の取り込みで S275 も一緒に入る。
- `Library` は `Server` が 1 回だけ作って持つ（`tree` と同じく更新は見ない）。書きかけの文書は補完のたびに `with_source` で足して取り除く。補完は `Ref` を使わないので作ったあと `refs` を捨てる。
- wasm は `lib` を持たない（今の道だけ）。今回は LSP のみ。

## やったこと

- `globe-mode-resolve`: `Class::range`、`Library::with_source`（一時的に文書を足して戻す）、`class_at`、`members_of`、`Member`。試験 `members_of_a_broken_document_and_the_library_is_restored`、計測用の `#[ignore]` `msl_members_and_memory`。
- `globe-mode-lsp`: `Resolver::lib`、`Server::lib`、`completion::tolerant_members`、依存を `[dev-dependencies]` から `[dependencies]` へ。試験 4 本（補完 3 本、RPC 1 本）。wasm の `Resolver` の組み立てに `lib: None`。
- doc: ADR-0039 の付記、roadmap（Track B の 2 か所）、features、architecture、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-resolve`: lsp 59 pass・1 ignored（lib）、統合 5 + 1 pass、resolve 6 pass・2 ignored。`cargo clippy -p globe-mode-resolve -p globe-mode-lsp -p globe-mode-wasm --all-targets -- -D warnings` 0 件、`cargo fmt --check` ok。
- MSL 4.1.0（debug）: `Library` を作るのに 8.8 秒、RSS 3.3 MB → 83 MB、`refs` を捨てて 75 MB。書きかけの文書を足して `Resistor r;` の `r.` を引くのに約 0.5 ms で 13 件（R・n・p・v・i・T・T_heatPort・heatPort・useHeatPort ほか）。release は測っていない。
- `wasm:node`・`wasm:compare` は回していない（wasm 側は `lib: None` を足しただけで、clippy は通した）。

## 残したこと

- 別の開いている文書にある型、`redeclare` が 2 段以上先で効く型、組み込みの属性（`x.start`）。
- ライブラリの更新で `Library` を作り直す仕組み、wasm への展開、定義ジャンプ・参照・ホバーの自前化。
