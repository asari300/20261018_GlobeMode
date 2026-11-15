# S352 `lsp-inlay-hints`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s352-lsp-inlay-hints`（base は main） |
| worktree | `.git-worktree/s352-lsp-inlay-hints` |
| Phase | Track B（自前コア）の製品へのつなぎ |
| セッション文書 | `doc/sessions/S352-lsp-inlay-hints/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

LSP に `textDocument/inlayHint` の最小版を足す。出すのは関数の呼び出し・部品の修飾の位置引数の前の引数の名前だけ。名前は resolve で引いた関数の `input` の並びから取り、引けなければ出さない。書きかけの文書でも構文の木から出す。capability の宣言、`range` の内側だけ返す。試験は LSP の単体で 2 本（出る・引けない関数には出ない）。VS Code の拡張に変更が要らないことを確かめる。features の行を直し、ADR に付記。flat は触らない（S336 が並行）。

## 決めたこと

- **「部品の修飾の位置引数」は対象外にした。** Modelica の部品の修飾 `(k = v)` は名前つきで、位置引数が無い。位置引数を取るのは関数の呼び出しとレコードの構築子で、構築子の欄は `input` でなく宣言の順（別の規則）なので、この WP では関数の呼び出しだけにした（依頼の「`input` の並び」とも合う）。
- **`function_inputs` を resolve の公開関数にした。** `extends` の基底の `input` を先に並べるには `base` の解決（private）が要る。LSP 側に規則を複製しない（S345 の `redeclared_types` と同じ考え）。基底が引けなければ None。
- **`Component.input` は resolve の `build.rs` で `ComponentClause` の直下の字句から取った。** syntax の API を増やさない。
- **引数名と同じ名前の引数（`f(x)` の `x:`）も出す。** 省くのは任意の機能で、依頼の範囲外。
- **VS Code の拡張は変更なし。** `vscode-languageclient` 9.0.1 が `inlayHintProvider` を見て登録する。`vscode.yml` の起動試験には足していない（手元で VS Code を起動できず、確かめられない試験を CI に入れない）。

## やったこと

- `globe-mode-resolve`: `Component.input`（`lib.rs`・`build.rs`）、`Library::function_inputs`（`lookup.rs`）。
- `globe-mode-lsp`: `src/inlay.rs`（新規）、`lib.rs`（モジュール・capability `inlayHintProvider`・`textDocument/inlayHint`・試験 2 本・冒頭の説明）、`tolerant.rs`（`span` を `pub(crate)`）。
- 文書: features・ADR-0039（付記 S352 と製品へのつなぎの表）・architecture・roadmap・CHANGELOG・`doc/sessions/S352-lsp-inlay-hints/`・`doc/sessions/README.md`・この記録。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-resolve`: lsp の lib 75 本 pass（S345 の 73 本 + 新規 2 本・1 本 ignored）・cross_file 5・folding 2・tolerant_outline 2、resolve の `tests/resolve.rs` 8 本 pass（2 本 ignored）。`cargo fmt`（lsp・resolve）済み、`cargo clippy -p globe-mode-lsp -p globe-mode-resolve --all-targets -- -D warnings` 0 件。`Component` に欄を足したので、使う側の `cargo check -p globe-mode-flat -p globe-mode-frontend --all-targets` も通した。
- `check_docs.py`: スライド全部 OK・リンク 文書 1263 件 / 相対リンク 5286 件 / 切れ 0 件。`render_slides.py`（自分のデッキだけ）: 1 件 / 問題 0 件（desktop-light・desktop-dark・mobile 各 5 枚）。
- 初回の試験は列の数え間違い（11 行目の内包の `f` の中の `0` は 22 列、`f(1, 2)` の 1 は 51 列）で落ち、期待値を直した。実装の出力は正しかった。
- 回していないもの: `mise run check` 全体（deny・uv。依存を足していない）、release・wasm のビルド（`globe-mode-wasm` は `inlay` を呼ばない）、`resolve_compare`（resolve の規則は変えていない）、本物の VS Code（【未確認】）、`msl:*`。セッション記録は無いので `--check` は省く。

## 残したこと

- レコードの構築子の欄名、別の開いた文書の関数、本物の VS Code での表示（【未確認】）。call hierarchy は未。
