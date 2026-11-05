# S171 `ui-completion-backend`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s171-ui-completion-backend` |
| worktree | `.git-worktree/s171-ui-completion-backend` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S171-ui-completion-backend/` |
| セッション記録 | （親が写す） |

## 依頼

S164 の穴 9。エディタの補完を Rust 側の補完につなぎ、ブラウザ（wasm）と Desktop（sidecar）の両方で意味のある候補を出す。Rust 側は、型の位置・`extends` の後・点つきの名前の途中のライブラリのクラス、`.` の後ろの継承した部品、式の中の部品だけ広げる。件数は 200、応答は MSL を読んだ状態で 50 ms が目安。`App.svelte`・`src/Rust/apps`・`src/VSCode` は触らない。

## 決めたこと

- 口は `Backend.completion` / wasm `completion` / rpc `globe/completion`（`textDocument/completion` と同じ計算）。返すのは LSP の `CompletionItem`。
- 計算は S132・S166 の `Resolver` と S167 の `LibraryTree` を使う。新しい解析は入れない（`completion::complete` が `Resolver` を受ける形に変えた）。
- 打ちかけで文書が壊れている間は、カーソルの行を空白にした本文のアウトラインで引く。同じファイルの型も引けるよう、`Resolver` に `overlay`（パスとアウトライン 1 つ）を足した。最初に壊れた文書を `scratch` に入れる案は、掃除が要るので採らなかった。
- Rust の source は `App.svelte` を触れないので、`hover` の `{backend, file}` が渡る `modelicaDefinition` から足す（`modelicaLinter` の `file` はタブを作るときの値で、タブが違うと合わない）。`autocompletion({override})` をやめ、`languageData` で予約語と本文の単語を足す。
- 件数の切り詰めは、打ちかけの語での絞り込み（字が順に当たる）のあとに 200 件。CodeMirror の `validFor` で語を打ち足す間は引き直さない。
- wasm の `scratch`（読んだライブラリのアウトライン）を呼び出しごとに作らず `Workspace` が持つ。MSL の継承した部品の補完が毎回 260 ms から 2 回目以降 1 ms に。

## やったこと

- Rust: `globe-mode-lsp/src/completion.rs`（全面。`inert`・`repair`・`members`・`class_children`・`complete_at`・単体 4 本）、`definition.rs`（`scopes_at` の切り出し、`resolve`・`component` を `pub(crate)`、`overlay`）、`lib.rs`（`globe/completion`、`pub mod completion`、RPC 1 本、doc 表）、`globe-mode-wasm/src/lib.rs`（`completion`、`scratch` を持つ）。
- wasm の JS: `web/client.js`・`web/worker.js`（`completion`）、`node/smoke.mjs`（MSL の項目）。
- Svelte: `backend.ts`（`CompletionEntry`・`completion`）、`worker-backend.ts`・`tauri-backend.ts`、`editor.ts`（`rustCompletion`・`completionFrom`）、`tests/test_ui.py`（2 項目）。
- 文書: features.md・roadmap.md・セッションの索引・スライド・解説書。

## 検証

- `cargo test -p globe-mode-lsp`: 32 本通過（新規 5 本: 型の位置のライブラリの子と絞り込みと import の別名、継承した部品と壊れた文書、文字列・コメントの中は空、予約語と部品名、RPC 1 本）。`cargo test -p globe-mode-wasm`: decay 15 本通過（無視 1）。
- `mise run wasm:node`: ok。MSL の補完: `Sources.` 31 件 1.5〜3.9 ms、`s.` 7 件は初回 255〜340 ms・2 回目 1.1〜1.4 ms、最上位 72 件 1.2〜2.0 ms。
- Desktop 相当（`Server`、ネイティブ release、MSL 4.1.0 を `Server::new`）の `globe/completion`: `s.` 初回 142〜302 ms、2 回目 0.3〜0.4 ms、最上位 70 件 0.3〜0.8 ms（一時の試験で測り、コミットしていない）。
- `pnpm check`: 0 errors / 0 warnings（264 ファイル）。`test_ui.py`（main の `.venv/bin/python -I`、MSL あり）: 最後まで ok。補完の行は 2 本: `Decay の補完: 部品名 level（型 Real つき）と、pin. の後ろの型 Pin の部品名 v・i が候補に出て、選ぶと入った`、`MSL の補完: Modelica.Blocks.Sources.Si で Sine が候補に出るまで 0.12〜0.16 s、選ぶと入った`。`GM-SMOKE` の行は変えていない。
- `mise run check`: 1 回目は load 9.5 で開始し、fmt / clippy / test の本体は通ったが、wasm の doc-test が共有 target の古い `globe-mode-lsp` の rlib を見て E0603（`completion` が private）で落ちた。4 つの crate を touch して再実行し、`cargo clippy --workspace --all-targets -- -D warnings`・`cargo test -p globe-mode-lsp -p globe-mode-wasm`・`cargo deny check`（advisories / bans / licenses / sources ok）・`uv sync --locked` が通り、`cargo fmt --all --check` は最後に整形して ok。
- `check_docs.py`: S171 のデッキ OK / リンク 3,573 件中の切れ 0 件。`render_slides.py`（S171 のデッキだけ）: 7 枚 × 3 画面、問題 0 件（1 回目で通った）。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: コミット前に grep（メールアドレス・ホームのパス・トークン）。

## 残したこと

- 初回の約 0.3 s（ライブラリのファイルを Rumoca で読む分）の先読み。50 ms の目安を超えるのはこの 1 回だけ（wasm は 255〜340 ms、ネイティブは 142〜302 ms）。
- 本文の単語（`completeAnyWord`）と Rust の候補が同名で 2 行出る。CodeMirror の重複除去は type が違うと効かない。
- カーソルの行以外にも誤りがある、または文が行をまたぐときは、文書を読み直せず候補が予約語と本文の単語だけになる。
- 文字列・コメントの中では Rust 側は空だが、UI の予約語と本文の単語はそのまま出る（UI 側で消すかは未決）。
- 見つけたこと: 共有 target の古い rlib は doc-test だけに効くことがある（E0603）。`touch` で直る。`autocompletion({override})` だと `languageData` の source は使われない。`languageData` の source は関数の同一性で区別されるので、毎回作ると結果が捨てられる。
