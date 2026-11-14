# S294 `param-modifiers`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s294-param-modifiers` |
| worktree | `.git-worktree/s294-param-modifiers` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S294-param-modifiers/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

パラメータダイアログ（S248）の値を、部品の宣言の修飾 `(k = 2)` としてソースに書けるようにする（Rust・LSP・wasm・Backend・UI）。

## 決めたこと

- 式の検査は字句と括弧の対応だけ（fmt は依存 0 で、`globe-mode-syntax` の式のパーサを使わない）。
- LSP は `modifiers` の配列を受け、順に当てて 1 つの TextEdit にする（同じ部品への複数で範囲が重ならないように）。
- 断る理由は LSP -32803・wasm 例外（`globe/rename` と同じ）。`set_modifier` は `Result`。
- UI は `c.k` の形の行だけ書く。列挙（型の名前が無い）・モデル自身・入れ子・配列は対象外。修飾を外す UI は持ち越し。
- ADR は新設せず ADR-0037 に S294 の項を足した（S248 の ADR は無い）。

## やったこと

`globe-mode-fmt/src/diagram.rs`（`set_modifier`・`site`・`expr_ok`）、`globe-mode-lsp`（`diagram::set_modifiers`・`globe/setModifier`）、`globe-mode-wasm`（`set_modifier`・`client.js`・`worker.js`・`smoke.mjs`）、`src/Svelte`（`backend.ts`・`worker-backend.ts`・`tauri-backend.ts`・`ParameterDialog.svelte`・`SimSettings.svelte`・`App.svelte`・`test_tauri.mjs`）、`shell.rs` の `RPC_METHODS`（36）。文書は ADR-0037・roadmap・features・セッション文書。

## 検証

- fmt 32・LSP lib 65・wasm decay 28 が通る。変えた 3 crate の `clippy --all-targets -D warnings` は 0 件、`cargo fmt --check` は差分なし。
- `pnpm check` 0 エラー・0 警告、`test_tauri.mjs` ok。`tauri-backend.ts` の method と `RPC_METHODS` の差は 0（スクリプトで確認。desktop の cargo 試験は回していない）。
- 回していない: `test_ui.py`（ブラウザ）、`smoke.mjs`（wasm のビルド）、`mise run check` 全体。

## 残したこと

修飾を外す UI、入れ子の名前・`each`・`final`・`redeclare`、列挙、モデル自身の束縛の書き換え、書いたあとの既定値の列の更新、ブラウザでの操作の試験。
