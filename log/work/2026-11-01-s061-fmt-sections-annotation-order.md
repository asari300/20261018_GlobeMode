# S061 `fmt-sections-annotation-order`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s061-fmt-sections-annotation-order` |
| worktree | `.git-worktree/s061-fmt-sections-annotation-order` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S061-fmt-sections-annotation-order/` |
| セッション記録 | `log/sessions/2026-10-31-ad17d3be-95c2-47b5-8ad8-6421e92ef889.md`（S053 と同じセッション） |

## 依頼

- 「VSCode で確認できました」（スクリーンショットは `GM-I10001` の hover）。
- 「equation や algorithm の前後に空行を追加するルールを追加、default=true だがオフにもできるように」
- 「annotation 内でも `experiment` -> `Diagram` -> `Documantation/{header,info,rev}` の順にリント(notification)&フォーマット」。例として Diagram・Icon・Documentation（`__OpenModelica_infoHeader`・info・revisions）を持つ Objects_Before を貼った。

## 決めたこと

- ユーザーが決めたこと: 節の前後の空行（既定オン、切れる）。annotation の順は experiment → Diagram → Documentation、Documentation の中は header → info → revisions。lint は通知の重さ。
- 既定のまま進めたこと（ユーザーに未確認）:
  - `Icon` は `Diagram` の直後に置いた。表に無い引数（`uses` など）は後ろに元の順で。
  - header は `__OpenModelica_infoHeader`（貼られた例の名前）。
  - 空行の対象は `equation` と `algorithm`（`initial` 付きも）。`public` / `protected` は対象外。
  - 並べ替えはランクを付けず（強制改行なし）、順だけを変える。
  - LSP のコードは `GM-L0004`、重さは Information。
  - VS Code の整形は独自の要求 `globe/format` で頼む。vscode-languageclient 9 の `asFormattingOptions` が `tabSize`・`insertSpaces` と 3 つの標準のキーしか通さないことを、node_modules のソースで確かめた。
  - wasm の `format` とウェブアプリには空行を切る口を足していない（既定のオンで動く）。

## やったこと

- `globe-mode-fmt`: `Options.blank_around_sections`、`structure::split` の節の空行、`annotation.rs` の `ANNOTATION_ORDER`・`DOCUMENTATION_ORDER`・並べ替え・`misordered`、`lint.rs` の `annotation-order`。試験: `spacing_and_blank_lines` を新しい既定とオフの両方に、`annotation_and_documentation_order`（`tests/data/annotation_order_{before,after}.mo`）。
- `globe-mode-lsp`: `textDocument/formatting` と `globe/format` が `insertOrigin`・`blankAroundSections` を読む。lint の診断の対応表に `GM-L0004`（Information）。単体試験 1 本。
- `globe-mode-cli`: `globe fmt --no-blank-around-sections`。
- `src/VSCode/globe-mode` 0.0.3: 設定 `globeMode.blankAroundSections`、整形を `globe/format` に振り替える middleware、試験の追加と期待値の更新、README。
- 文書: `doc/format-style.md`（規則と lint の表）、`doc/roadmap.md`、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-fmt`: 単体 4・統合 8 が通過。ユーザーの例を元にした入力は、experiment が先頭、Icon が Diagram の後、Documentation が header → info → revisions、uses が最後に並んだ。lint は整形前 2 件・整形後 0 件。
- `cargo test -p globe-mode-lsp -p globe-mode-cli`: 通過（新しい単体試験を含む）。
- 拡張の `pnpm test`: 2/2。1 回目は `globe lsp` の往復の期待値が古い既定のままで落ちたので直した。
- `mise run check`（fmt / clippy / test / deny / uv）: 通過。1 回目は clippy の `collapsible_if`（`misordered` の入れ子の `if let`）で落ち、let-chain に直した。
- `render_slides.py`: S061 のデッキ 1 件 / 問題 0 件（1・2 回目はスクリーンショットで時間切れ。間に S060 のデッキは通り、3 回目で通った。手元の Chromium の不安定さ）。
- `check_docs.py`: S061 のデッキ OK、文書 265 件 / リンク 1321 件中の切れ 0 件。
- セッション記録の `--check`: 取りこぼし 0 件。
- ステージした差分の漏洩検査: 0 件。

## 残したこと

- `Icon` の位置をユーザーに確かめる。
- wasm・ウェブアプリから空行を切る口。
- `src/Modelica` の 69 本の書き換え。
