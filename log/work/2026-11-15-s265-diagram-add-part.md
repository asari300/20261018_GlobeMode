# S265 `diagram-add-part`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s265-diagram-add-part` |
| worktree | `.git-worktree/s265-diagram-add-part` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S265-diagram-add-part/` |
| セッション記録 | なし（サブエージェントの作業で、このセッション専用の記録が無い） |

## 依頼

Diagram ビューの編集の 3 つ目 = 部品の追加。ライブラリの木の行を図へドロップすると、カーソルのクラスに `型 名前 annotation(Placement(...))` の宣言を足す編集を Rust が返し、UI がエディタに適用する（undo 可、保存しない）。

## 前提と進め方

- S261（結線の追加。S250 を含む）の先頭 `feature/s261-diagram-connect` から切った。両方が main に入れば `git merge main` でそのまま重なる。S260・S262・S264 と LSP の RPC 表・Svelte・VS Code で衝突しうる（`RPC_METHODS` の数と一覧）。

## 決めたこと

- `add_part`（`globe-mode-fmt`）: 最後の public の宣言の次の行へ、その字下げで。宣言が無ければ最初の節かクラスの `annotation`（無ければ `end`）の行の前。`add_connect` と共有できる要素の切り出しを `own_elements` に出した（挙動は同じ）。
- 名前は型の最後の部分を小文字始めにし、クラスの中の字句・キーワード・型の最後の部分と重なれば 1、2…。継いだクラスの名前は見ない。
- 種類は `is_connector` を `kind_of` に一般化して共有。package・function・record・type は断り、引けない・読めなければ受ける。
- 口: LSP `globe/addPart`（`RPC_METHODS` 29）、wasm `add_part`、`Backend.addPart`。
- UI: 木の行を `draggable`、図の svg が `drop`。キーボード用に「図へ」ボタン（図の `extent` の中心へ）。App の図の編集 3 本は `editDiagram` に共通化。
- ブラウザの操作の試験（`test_ui.py`）は足していない（Playwright とブラウザ版 wasm のビルドが重い。S250・S261 と同じ理由）。

## 検証

- `cargo test -p globe-mode-fmt`: `add_part_goes_after_the_last_public_declaration` を足し、diagram の 7 本 pass。
- `cargo test -p globe-mode-lsp`: 55 本 pass・1 本 ignored（`add_part_returns_a_text_edit_…` を足した）。
- `cargo test -p globe-mode-wasm --test decay`: 25 本 pass（diagram の試験に `add_part` を足した）。
- `cargo clippy -p globe-mode-fmt -p globe-mode-lsp -p globe-mode-wasm --all-targets -- -D warnings` 警告 0、`cargo fmt --check` 差分 0。
- Svelte: `pnpm check` 0 エラー・0 警告、`test_tauri.mjs`（`addPart` の呼び出し）pass。
- `git merge origin/main` は衝突なし（S251 などを取り込み）のあと `mise run check`（fmt・clippy・test・deny・uv sync）が exit 0。cargo test は 887 本 pass・失敗 0。`check_docs.py` はリンク切れ 0。
- 回していないもの: `mise run wasm:node`（`node/smoke.mjs` に `add_part` を 1 件足した。CI の `wasm-node` に任せる）、Desktop の RPC 許可の試験（別 workspace。`tauri-backend.ts` と `RPC_METHODS` に `globe/addPart` を足した）、Playwright。
