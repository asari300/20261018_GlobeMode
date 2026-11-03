# S138 `ui-editor-lsp`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s138-ui-editor-lsp` |
| worktree | `.git-worktree/s138-ui-editor-lsp` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S138-ui-editor-lsp/` |
| セッション記録 | （親が写す） |

## 依頼

features.md の「Text ビュー」の「CodeMirror と LSP の直接の接続は未」と、「補完・定義ジャンプ」の「ブラウザ UI 側の接続は持ち越し」（S132）を埋める。ブラウザと Tauri の CodeMirror 6 から定義ジャンプ（S132 の `textDocument/definition`）を使えるようにする。汎用の LSP クライアントは足さない。最小で、`Backend` に `definition(file, line, col)` を足す。Ctrl/Cmd+クリックか F12、同じファイルなら範囲へ、ライブラリのファイルなら S127 の流れで開く。試験は `test_ui.py` に 1 本（Decay と ChuaCircuit の Resistor）。新しい依存は足さない。

## 決めたこと

既定のまま進めたこと（ユーザーと新たに決めたことは無い）:

- 接続の形: `Backend.definition(file, line, character)` が `{file, text, range}` を返す。`text`（定義のあるファイルの本文）を含めるのは、Tauri の UI がディスクを読めず、ライブラリのファイルを開くのに本文が要るから。
- wasm: `globe-mode-lsp` は wasm32 でそのままビルドできたので、定義の核を facade や fmt に動かさず、wasm から `definition::definition_at` を呼ぶ。`Resolver` に `read: &dyn Fn(&str) -> Option<String>`（ファイル名から本文）を足し、LSP はディスク、wasm は読み込んだ組を渡す。wasm が lsp に依存するのはこの 1 点だけ（architecture.md に書いた）。
- Tauri: sidecar に `globe/definition`（`{file, line, character}`）を足した。`textDocument/definition` は VS Code 用にそのまま。S136 が触る `run_stdio` と `src/Svelte/src/lib/tauri-backend.ts` の進捗・中断の部分は触っていない（`tauri-backend.ts` は末尾に 1 メソッドを足しただけ）。
- 文書の本文は `GlobeWorker`（`client.js`）の `#files` から渡す。UI は呼ぶ前に `setFile` して `Outline` を最新にする。
- ライブラリのファイルへ入るとエディタの文書が置き換わるので、自分の文書を失わないよう、入る前の文書と位置を積み、Alt+← で戻す（1 本の積みで、進むは無い）。
- カーソルは宣言の頭に置く（範囲を選び、head を頭へ）。クラスの範囲は `end X;` まで続くので、末尾だと宣言が画面の外へ出る。試験のために `highlightActiveLine()` を足した。
- ホバー: LSP には既にあるが、UI の接続は今回しない（ユーザーの「定義ジャンプ」が主。持ち越し）。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/definition.rs`: `Resolver.read` を足し、`definition_at`（行・UTF-16 の列から `{file, range, text}` の JSON）を足した。`Resolver` に `#[allow(missing_debug_implementations)]`（`dyn Fn` を持つ）。
- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `mod definition` を `pub mod` に、`textDocument/definition` の `Resolver` に `read` を渡す、`globe/definition` の dispatch、表への 1 行、試験 1 本。
- `src/Rust/mod/globe-mode-wasm/`: `Cargo.toml` に `globe-mode-lsp`、`src/lib.rs` に `Workspace.definition`、`web/worker.js` と `web/client.js` に `definition`、`tests/decay.rs` に試験 1 本。
- `src/Svelte/`: `backend.ts`（`Definition` と `definition`）、`worker-backend.ts`、`tauri-backend.ts`、`lib/editor.ts`（`modelicaDefinition`。F12、Ctrl/Cmd+クリック、Alt+←。`Prec.high`）、`App.svelte`（`definition`・`goBack`・`highlightActiveLine`）、`tests/test_ui.py`（Decay の F12 と ChuaCircuit の Resistor の F12 と Alt+←）。
- `doc/features.md`（2 行）・`doc/roadmap.md`（P4 の行と「S138 で済んだこと」）・`doc/architecture.md`（wasm の行）・`doc/sessions/README.md`、セッション文書。

## 検証

- `cargo test -p globe-mode-lsp`: 17 本通過（新しい `globe_definition_returns_the_target_file_text` を含む）。`cargo test -p globe-mode-wasm`: `decay` 12 本通過・1 本 ignored（新しい `definition_jumps_into_the_library_file` を含む）。
- `cargo clippy -p globe-mode-lsp -p globe-mode-wasm --all-targets -- -D warnings`: 通過。`cargo check --target wasm32-unknown-unknown -p globe-mode-wasm`: 通過。
- `mise run wasm:build`: 通過（wasm 15,449,110 バイト）。`mise run wasm:node`: ok（47.8 s）。`wasm:compare` は実行系を変えていないので回していない。
- `pnpm check`: 261 ファイル、エラー 0、警告 0。
- `test_ui.py`（`uv run python src/Svelte/tests/test_ui.py`、data/msl あり、約 92 s）: 通過。Decay の F12 は「  Real x(start = 1, fixed = true);」の行へ。ChuaCircuit の F12 は Resistor.mo を開き、Alt+← で ChuaCircuit に戻った。既存の項目（10 万点の描画 214 ms、止める 36 ms、MSL を読む 0.6 s、ChuaCircuit の検査 2.3 s・実行 0.8 s）に変わりは無い。途中、実行を打ち切った直後の再実行が 2 回、ブラウザが閉じたことで落ちた（残ったプロセスの影響と思われる。3 回目から通る）。F12 の Alt+← は最初 `defaultKeymap` の `cursorSyntaxLeft` に取られて戻らなかったので `Prec.high` にして直した。
- コードの検査: `cargo fmt --all --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`ci_plan` の試験 25 本は通過。`mise run check` は負荷（load average 20〜50、他の worktree のビルドとテスト）で途中の `cargo test` が OOM で落ちた（mise が殺された）ので、`cargo test --workspace --no-fail-fast` を回し直した: 64 件の test result がすべて ok、643 本通過・失敗 0（`msl_examples_match` などは負荷で 1 本約 15 分）。`cargo deny check`: advisories / bans / licenses / sources すべて ok。`uv sync --locked`: ok。途中、共有 target の古いビルドで wasm が `definition_at` を見つけられず落ちた（`touch` して回し直して通過）
- `check_docs.py`: S138 のデッキ OK / 文書 570 件・相対リンク 2864 件中の切れ 0 件。`render_slides.py`: S138 の 1 デッキ（6 枚 × 3）の問題 0 件（全デッキを回すと負荷で遅いので S138 だけを渡した）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- ホバー・参照・リネーム・inlay hint の UI。
- ライブラリのファイルへ入ったときのモデル名は変えない（定義のクラスの完全修飾名を返していない）。複数の文書（タブ）が無いので、ライブラリのファイルへ入ると自分の文書は積みに退く（Alt+← で戻る。ページを読み込み直すと失う）。
- 受け継いだ入れ子のクラス・コメント中の名前（S132 の shortcut のまま）。
- Desktop（Tauri）の実機では回していない（sidecar の `globe/definition` は lsp の単体試験だけ）。
- ダークモード。
