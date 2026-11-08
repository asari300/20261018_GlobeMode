# S225 `editor-folding`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s225-editor-folding` |
| worktree | `.git-worktree/s225-editor-folding` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S225-editor-folding/` |
| セッション記録 | （親が写す） |

## 依頼

S220 が持ち越した UI（CodeMirror 6）の折りたたみ。LSP の `folding.rs` を再利用し、wasm と Desktop の sidecar から範囲を取る。

## 決めたこと

- wasm は `folding_ranges(file, text)`。クラスと節の範囲に文書のアウトラインが要るので、`set_file` 済みの file を引く（依頼の `folding_ranges(text)` から file を足した）。
- Desktop は新しい method を作らず、sidecar の `textDocument/foldingRange` を `RPC_METHODS` に足した（20 本）。
- 取り直しは検査（lint の 500 ms）に相乗りせず、折りたたみ用の 300 ms の debounce にした。lint は `file` を知らず、結合が増えるため。`setFile` と `foldingRanges` の 2 往復で、補完・ホバーと同じ。
- `foldGutter` は範囲の到着で自動では更新されないので `foldingChanged` を渡した（最初に気づかなかった所。印が出なかった）。

## やったこと

`globe-mode-wasm/src/lib.rs` と `web/{worker,client}.js`、`apps/desktop/src/shell.rs`、`src/Svelte/src/lib/{backend,worker-backend,tauri-backend,editor}.ts`、`App.svelte`（拡張を 1 行）、試験（`node/smoke.mjs`、`tests/test_ui.py`）、文書。

## 検証

- `node smoke.mjs`（リリースの wasm）: 折りたたみ `[[0,5],[2,5],[4,5]]`。
- `pnpm check` 0 エラー、`test_tauri.mjs` ok、`test_ui.py` ok（MSL あり。折りたたみ: model Decay を畳むと `end Decay;` が隣に来て、開くと戻る）。
- `shell.rs` の突き合わせ試験は Desktop の workspace を回せないので、同じ照合を手元で確かめた（UI が送る method と許可の一覧が一致、20 本）。
- `mise run check`、`mise run wasm:node`、`docs:check`、CI（full）: 下の「結果」。

## 残したこと

- 構文が壊れている間はクラスと節の印が出ない（LSP と同じ限界）。
- 畳んだ状態は再読み込みで戻らない。
- `test_ui.py` の「エディタの Tab」の補完の確定（`mod` + Enter）が 1 回だけ落ちた（再実行で通る。折りたたみの 300 ms の往復と重なる可能性があるが再現せず）。
