# S062 `connect-type-lint`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s062-connect-type-lint` |
| worktree | `.git-worktree/s062-connect-type-lint` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S062-connect-type-lint/` |
| セッション記録 | `log/sessions/2026-10-31-ad17d3be-95c2-47b5-8ad8-6421e92ef889.md`（S053 と同じセッション） |

## 依頼

- 「型チェックの機能を追加し、異なる型で connect された時に lint で通知」
- 「今後必要になりそうな機能と現在のルールを README.md に一覧表として追記。処理の流れは Mermaid」
- 「VSCode 成功」（S061 の 0.0.3 を確かめた）。Objects_Before の例を貼った。
- 途中の追加: 「HTML も以下のように改行+インデントフォーマット（t/f スイッチング、default=true）、リントはしない」。info に h1 と ul/li を持つ例を貼った。

## 決めたこと

- ユーザーが決めたこと: connect の型の食い違いを lint で通知。README の表と Mermaid。HTML の整形は既定オンで切れ、lint はしない。
- 既定のまま進めたこと（ユーザーに未確認）:
  - 「同じ型」は名前が同じか、中身（変数の名前・flow / stream・型）が同じ（MLS 9.3 の構造の同値に倣う）。
  - 型を解決できない端（`--library` の MSL、LSP で開いていないファイル）は判定しない。
  - 重さは warning（`GM-L0005`）。S061 の「通知」（Information）より重くした。コンパイルで落ちる誤りなので。
  - HTML の対象は `Documentation(…)` の `name = "…"` のうちタグを含む文字列。`<pre>` `<textarea>` `<script>` `<style>` を含むものは触らない。インライン要素は行を分けない。
  - 整形の保証（空白以外のトークン列が不変）は、HTML の文字列だけ「空白を除いて同じ」に緩めた。
  - wasm とウェブアプリには HTML を切る口を足していない（既定のオンで動く）。

## やったこと

- `globe-mode-graph`: `connect_mismatches`・`ConnectMismatch`、単体試験 1 本。facade `Workspace::connect_mismatches`。
- `globe-mode-lsp`: 診断に `GM-L0005`（warning）、`globe/format` と `textDocument/formatting` が `formatHtml` を読む。単体試験 1 本と 1 か所追加。
- `globe-mode-cli`: `globe lint` が渡したファイルをまとめて型を突き合わせる（`--disable connect-type-mismatch`）。`globe fmt --no-format-html`。試験 1 本。
- `globe-mode-fmt`: `html.rs`（HTML の段付け）、`Kind::Html`、`Options.format_html`、出力の照合を HTML の文字列だけ空白を除いて比べる。単体 2・統合 1。`annotation_order_after.mo` の期待値を更新。
- VS Code 拡張 0.0.4: 設定 `globeMode.formatHtml`。
- 文書: README（エディタ機能の章: 処理の流れの Mermaid、今の規則の表、今後の機能の表）、`doc/format-style.md`、`doc/roadmap.md`、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-graph -p globe-mode-fmt -p globe-mode-lsp -p globe-mode-cli`: 通過。1 回目は CLI の試験の列の期待（connect の位置は引数の先頭で 11 列）と、LSP の試験の行の数え違い（17）で落ち、期待を直した。
- `globe lint src/Modelica`: 69 ファイル、型の通知は `Invalid/SignalTypeMismatch.mo` の 1 件だけ（わざと誤らせたもの）。connect のある 9 本で誤った通知は 0 件。
- 拡張の `pnpm test`: 2/2。
- `mise run check`: 通過（fmt / clippy / test / deny / uv）
- `render_slides.py` / `check_docs.py`: S062 のデッキ 1 件 / 問題 0 件（1 回目はスクリーンショットで時間切れ、2 回目で通過）。文書 269 件 / 相対リンク 1333 件中の切れ 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- 漏洩検査: 0 件

## 残したこと

- LSP が開いていないファイルと `--library` の型でも突き合わせる。
- connect の向き・配列の大きさ・単位の検査。
- wasm とウェブアプリから空行・HTML を切る口。
- `src/Modelica` の 69 本の書き換え。
