# S260 `linearize-ui`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s260-linearize-ui` |
| worktree | `.git-worktree/s260-linearize-ui` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S260-linearize-ui/` |
| セッション記録 | なし（並行のサブエージェントの担当で、このセッションだけの記録が `log/local/` に無い） |

## 依頼

S254 の線形化を UI から使えるようにする。LSP `globe/linearize`、Desktop の `RPC_METHODS`、wasm `linearize`、`Backend.linearize`、ツールバーの「線形化」と A・B・C・D の表（JSON の書き出し）。固有値は今回なし。

## 決めたこと

- JSON の組み立ては facade の `linearization_json` に移し、CLI・LSP・wasm が共有する（キーは `globe linearize --json` のまま）。
- 結果は「ファイルのタブ」ではなく、プロット・グラフと並ぶ結果の欄の `LinearizeView.svelte`。タブの型（`Tab`）は本文とプロットを前提にしているので、足すと保存・復元・閉じる処理まで触ることになる。
- 時刻は欄が空なら開始時刻。上書きは実行の設定のパラメータを流用（欄を増やさない）。

## やったこと

`globe-mode`（`linearization_json`）、`globe-mode-cli`（その関数を使う）、`globe-mode-lsp`（`globe/linearize`、試験 1 本）、`globe-mode-wasm`（`linearize`、`tests/decay.rs` 1 本、`node/smoke.mjs` 1 件、`web/worker.js`・`client.js`）、Desktop の `RPC_METHODS`（26 → 27）、Svelte（`protocol.ts`・`backend.ts`・`worker-backend.ts`・`tauri-backend.ts`・`LinearizeView.svelte`・`App.svelte` の「線形化」ボタンと時刻の欄、`test_tauri.mjs`・`test_ui.py`）、features・roadmap・CHANGELOG・Svelte README。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-cli -p globe-mode-wasm` 全 pass（lsp 54 本のうち新規 1 本、wasm の `decay` 27 本のうち新規 1 本）。`cargo clippy -p globe-mode -p globe-mode-cli -p globe-mode-lsp -p globe-mode-wasm --all-targets -- -D warnings` 警告 0、`cargo fmt --all --check` 差分なし。
- `mise run wasm:build` のあと `node …/smoke.mjs` が ok（線形化の件を含む。data/msl が無いので MSL の節は飛ぶ）。
- `pnpm check` 0 errors 0 warnings、`test_tauri.mjs` ok、`test_ui.py` の `linearize_view` だけを単独で回して ok（A = -2・B = C = 1・D = 0。console の error 0）。`test_ui.py` 全体は回していない。
- Desktop の crate は webkit のシステムライブラリが無く手元でビルドできないので、`relayed_methods_are_exactly_what_the_ui_sends` と同じ突き合わせ（`tauri-backend.ts` の `method:` と `RPC_METHODS`＋`NOTIFY_METHODS`）を Python で行い、27 本が一致。

## 残したこと

固有値、Python の口、平衡点、解析ヤコビ行列。結果を「タブ」にするなら `Tab` の型の拡張が要る。
