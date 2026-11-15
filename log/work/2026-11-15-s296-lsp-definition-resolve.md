# S296 `lsp-definition-resolve`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s296-lsp-definition-resolve` |
| worktree | `.git-worktree/s296-lsp-definition-resolve` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S296-lsp-definition-resolve/` |
| セッション記録 | 専用のものが無いので省く |

## 依頼

定義ジャンプとホバーを、今の道で答えが出ない書きかけの文書でも `globe-mode-syntax` と `globe-mode-resolve` で答える（S279 の補完と同じ形）。

## 決めたこと

- 自前の道は文書の `Outline` が `Err` のときだけ（読める文書は変えない。null のたびに木を作らない）。
- `with_source` で足した文書の `Ref` を段で切って `resolve`。ホバーは `hover_at` が定義の `Target` から作る。`textDocument/hover` は読めない文書で null のときだけ足す。

## やったこと

- `globe-mode-lsp/src/completion.rs`: `Library` を作る処理を `tolerant_lib` に切り出して共有。
- `globe-mode-lsp/src/definition.rs`: `tolerant_definition` を足し、`Resolver::definition` が `Outline` なしのとき使う。
- `globe-mode-lsp/src/lib.rs`: `textDocument/hover` の足しと試験 1 本。
- 文書: ADR-0039 の付記、roadmap・features の行、セッション文書。

## 検証

- `cargo test -p globe-mode-lsp` 全 pass、clippy 0、fmt 差分 0。
- 依存の追加なし。
- `check_docs.py`: デッキ OK / リンク切れ 0 件。`render_slides.py` は既存の S288 のデッキでも `Page.screenshot` が 30 秒で時間切れになり（この環境の問題）、通っていない。

## 残したこと

- 添字に `.` がある `a[x.y].b` の段のずれ、属性・2 段以上先の `redeclare` は null、別の開いている文書の型は読まない。
