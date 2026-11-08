# S227 `doc-stale-notes`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s227-doc-stale-notes` |
| worktree | `.git-worktree/s227-doc-stale-notes` |
| Phase | 文書 |
| セッション文書 | `doc/sessions/S227-doc-stale-notes/` |
| セッション記録 | （親が写す） |

## 依頼

S217 が「直せなかった・直さなかった食い違い」として残したものと、その後の WP が見つけた古い記述を直す。内容の直しで、推敲ではない。7 項目（README の英語の要約、ADR-0026、ADR-0023、`globe-mode-lsp` のコメント、mise-setup.html、workflow の一覧の突き合わせ、roadmap の末尾の表）。

## 決めたこと

- ADR は本文を書き換えず、付記（ADR-0026 の (o)、ADR-0023 の rpc の項の下）で現状を書いた。
- rpc の数は `shell.rs` の `RPC_METHODS` を数えて 20（`globe/*` 19、`textDocument/foldingRange` 1）。ADR-0023 の 2 GiB の作り直しは S223（#69）の付記がすでにあった。
- Rust は `globe-mode-lsp/src/lib.rs` のコメントだけ。コードは触らない。
- CLAUDE.md の workflow の一覧は 12 本すべて載っていて直しなし。architecture.md は `pull_request`（`ci.yml` にある。使わない決め）と `python` ジョブの FastAPI の試験を足した。
- roadmap の末尾の表は、MAT v4・結果を開く・折りたたみ・Enumeration と String がすでに打ち消されていて、Astro の CI の行だけ直した。
- CHANGELOG は利用者から見て変わる点が無いので足していない。
- 突き合わせで新たに見つけた食い違い: mise-setup.html の uv の 0.12 の記述が 3-4 章と 8 章にもあった、`uv run 20261018-globemode` と `src/main.rs`（どちらも無い）。同じコミットで直した。

## やったこと

README.md、doc/adr/0023・0026、doc/architecture.md、doc/roadmap.md、doc/mise-setup.html、`src/Rust/mod/globe-mode-lsp/src/lib.rs`（コメント）、doc/sessions/S227-doc-stale-notes/、doc/sessions/README.md、この記録。

## 検証

- `cargo fmt --check -p globe-mode-lsp`: ok。
- `check_docs.py`: 文書 907 件 / 相対リンク 4330 件 / 切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件。
- `ci.yml`（feature ブランチ、full）run 42446798160: success。

## 残したこと

- mise-setup.html の S001 の手順は実行して確かめていない。
- README の英語に、日本語の側のキーボードの補足（スキップリンクなど）は足していない。
