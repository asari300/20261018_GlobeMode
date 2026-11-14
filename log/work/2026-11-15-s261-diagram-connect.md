# S261 `diagram-connect`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s261-diagram-connect` |
| worktree | `.git-worktree/s261-diagram-connect` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S261-diagram-connect/` |
| セッション記録 | なし（サブエージェントの作業で、このセッション専用の記録が無い） |

## 依頼

Diagram ビューの編集の 2 つ目 = 結線の追加。部品のコネクタからドラッグして別のコネクタで離すと、クラスの equation 節に `connect(...) annotation(Line(...))` を足す編集を Rust が返し、UI がエディタに適用する（undo 可、保存しない）。

## 前提と進め方

- S250（部品の移動）がこの時点で main に入っていなかった（待つ仕組みのバックグラウンド監視が 144 で落ちた）。**S250 の feature ブランチ（`origin/feature/s250-diagram-move`、f103b0b）から切った**。S250 が main に入れば、`git merge main` でそのまま重なる（S261 は S250 の差分を含む）。

## 決めたこと

- `add_connect`（`globe-mode-fmt`）: 最後の `equation` 節の末尾（クラスの `annotation` の前）へ、その文の字下げで。節が無ければ `annotation`（無ければ `end`）の行の前に節ごと。`initial equation`・`algorithm` は数えない。端は英数字・`_`・`.`・`[]` だけ。
- S245 の図は部品の中のコネクタを出していなかったので `Part.connectors` を足した。部品のクラスと `extends` の 2 段、`iconTransformation`（無ければ `transformation`）の extent の中心。`icon.rs` の `extends` をたどる所を `find_base` に出して共有した（挙動は同じ）。
- `LimPID` の中の `Gain` など非コネクタを除くため、`iconTransformation` の無い宣言は型を引いて connector でないと分かれば除く（字面を探すだけ。引けなければ残す）。MSL の Capacitor・LimPID・Inertia で手元で確かめた（一時の試験。コミットしていない）。
- 型の互換は見ない。足したあとの検査が誤りを出す。
- 口: LSP `globe/addConnect`（`RPC_METHODS` 28）、wasm `add_connect`、`Backend.addConnect`。
- ブラウザの操作の試験（`test_ui.py`）は足していない（Playwright とブラウザ版 wasm のビルドが重い。S250 と同じ理由）。UI の動作は `pnpm check` と、ノードの生成の試験（`test_columns.mjs`）だけ。

## 検証

- `cargo test -p globe-mode-fmt`: unit 27 本・format 21 本 pass（`add_connect`・コネクタの位置・コネクタの見分けの 3 本を足した）。
- `cargo test -p globe-mode-lsp`: 54 本 pass・1 本 ignored（既存）。`add_connect_returns_a_text_edit_…` を含む。
- `cargo test -p globe-mode-wasm --test decay diagram`: 1 本 pass（`add_connect` を足した）。
- `cargo clippy -p globe-mode-fmt -p globe-mode-lsp -p globe-mode-wasm --all-targets -- -D warnings` 警告 0、`cargo fmt --all --check` 差分 0。
- Svelte: `pnpm check` 0 エラー・0 警告、`test_tauri.mjs`（`addConnect` の呼び出し）・`test_columns.mjs`（コネクタの円）pass。VS Code の `test/panel.test.mjs` 3 本 pass（`pnpm test` の LSP 起動の試験は `globe` が PATH に無く手元では回せない）。
- 回していないもの: `mise run wasm:node`（release の wasm。`node/smoke.mjs` に `add_connect` を 1 件足した。CI の `wasm-node` に任せる）、Desktop の RPC 許可の試験（別 workspace。`tauri-backend.ts` と `RPC_METHODS` に `globe/addConnect` を足した）、`mise run docs:check` の Playwright。
