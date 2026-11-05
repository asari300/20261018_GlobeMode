# S161 `editor-references-rename`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s161-editor-references-rename` |
| worktree | `.git-worktree/s161-editor-references-rename` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S161-editor-references-rename/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

features.md の「補完・定義ジャンプ・参照・リネーム・ホバー」の残り。参照の一覧とリネームを、Svelte のエディタから使えるようにする（Shift+F12 / F2）。範囲は今の実装（開いている文書の同じクラスの中）。計算は Rust、`Backend` に `references`・`rename`、ブラウザ（wasm）と Desktop（sidecar）の両方。

## 決めたこと

- 計算は `globe-mode-lsp::definition` の `references_at`（Range の配列）と `rename_at`（TextEdit の配列）。wasm・`globe/*`・`textDocument/rename` が同じ関数を呼ぶ。
- リネームは「いちばん内側のクラスで宣言した部品」だけ受ける。型・クラス・継承した部品・`a.b` の `b` の上は「部品でない」と断る（依頼の「持ち越し」を、黙って部分的に直さず断る形にした）。
- 断る理由は 4 つ（部品でない、識別子でない、予約語＝`globe-mode-fmt::KEYWORDS`、同じクラスに既にある）。LSP は -32803（RequestFailed）、wasm は例外、UI は `failure` の `role="alert"`。
- UI は `window.prompt` を使わず、入力欄（Enter で確定、Escape でやめる）。編集は 1 回の `view.dispatch`。一覧は診断の表と同じ作りで、クラス名は `refs`（`table.diagnostics tr` の数を変えない）。
- 追加の指示（取り込み前）: 一致を字面からトークンに替えた。`globe-mode-fmt` の `lex`・`Token`・`Kind` を `pub use` し、`references` は識別子のトークンだけを見る（点つきの名前はトークン列、前が `.` は除く）。文字列・コメントの中・字句に分けられない本文は一致 0、リネームはカーソルが識別子の上にないと断る。
- `GM-SMOKE` の行は変えない（`bench_ui.py`・`desktop.yml`・`android.yml` の grep を壊さない）。`wasm:node` の `smoke.mjs` には足さず、wasm の単体試験（decay.rs）で受けた。
- Desktop の Rust（`rpc` は method を通すだけ）、`.github/workflows/` は触らない。

## やったこと

- Rust: `globe-mode-lsp/src/definition.rs`（`references_at`・`rename`・`rename_at`・単体試験）、`lib.rs`（`textDocument/rename`・`globe/references`・`globe/rename`・`renameProvider`・RPC の試験・doc 表）、`globe-mode-wasm/src/lib.rs`（`references`・`rename`）、`tests/decay.rs`、`web/client.js`・`web/worker.js`。
- Svelte: `backend.ts`・`worker-backend.ts`・`tauri-backend.ts`、`editor.ts`（`modelicaRefactor`）、`App.svelte`（一覧・入力欄）、`tests/test_ui.py`。
- 文書: features.md・roadmap.md・architecture.md・セッションの索引・スライド・解説書。
- 見つけたこと: wasm の `Workspace` には `impl` が 2 つあり（素の `impl` と `#[wasm_bindgen] impl`）、最初は素の側に足してしまい、ネイティブの試験は通るが JS から `ws.references is not a function` になった。`test_ui.py` で気づいた。検査のボタンを押したあとはフォーカスがボタンにあるので、Ctrl+Z の前にエディタへ戻す必要がある。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-wasm`: lsp 25 本・wasm の decay 15 本（無視 1）通過。
- `test_ui.py`（ブラウザ、MSL あり）: 最後まで ok。Decay の Shift+F12 は 3 行、F2 は `model` を断り `pos` にして検査が通り、Ctrl+Z 1 回で戻る。
- `pnpm check`: 0 errors / 0 warnings。
- `mise run check`（fmt / clippy / test / deny / `uv sync --locked`）: 通過（load 約 4 で 1 回。初回は `cargo fmt` の整形差分で止まり、直して回し直した）。
- `mise run wasm:node`: ok（Finished in 40.64 s）。
- `check_docs.py`: デッキ OK（S161 は slides.html 1 件）/ リンク 3,253 件中の切れ 0 件。`render_slides.py`（S161 のデッキだけ）: 3 画面 7 枚、問題 0 件（1 回目で通った）。
- 追加（識別子のトークンだけを一致の対象にした後）: `cargo test -p globe-mode-lsp -p globe-mode-wasm -p globe-mode-fmt` 通過（lsp 25・wasm の decay 15・fmt 9）、`mise run check` 通過（load 約 3）、`pnpm check` 0 errors、`test_ui.py` ok（足したコメント `// x と "x" は変えない` が F2 のあとも変わらず、Ctrl+Z 2 回で元の本文）、`check_docs.py` リンク 3,253 件中の切れ 0 件。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 下のコミット前に grep（メールアドレス・ホームのパス・トークン 0 件）。

## 残したこと

- 別ファイル・ライブラリ全体の参照、継承した部品・型・クラスのリネーム、`prepareRename`。
- Desktop（sidecar）の実機は確かめていない（`globe/references`・`globe/rename` は LSP の RPC 試験まで。Desktop は `rpc` が method を通すだけ）。
