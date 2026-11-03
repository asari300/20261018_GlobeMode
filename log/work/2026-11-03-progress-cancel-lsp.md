# S136 `progress-cancel-lsp`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s136-progress-cancel-lsp` |
| worktree | `.git-worktree/s136-progress-cancel-lsp` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S136-progress-cancel-lsp/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0033 の順の 5: LSP の `$/progress` と `$/cancelRequest`（読みと計算を別スレッドに）、Desktop の `TauriBackend` で進捗と「止める」。試験は長いモデルを `$/cancelRequest` で止めて `GM-I11004` と途中の列。ADR を Accepted に、機能表と features.md。

## 決めたこと

- 読みのスレッドがキューに積み、計算と書き出しは元のスレッド（書き手 1 つ）。`CancelToken` は読んだ時点で id の表に置く（待っている要求にも効く）。
- 進捗は params の `workDoneToken` があるときだけ（ADR の決定 (3)。`window/workDoneProgress/create` は使わない。依頼の文面とは違うが ADR に従った）。report に `stage`・`time`・`start_time`・`stop_time` を添えた。
- **依頼では `src/Rust/apps/desktop` は触らない見込みだったが、触った。** 実行中は `Worker` の `Mutex` が握られていて UI から `$/cancelRequest` を送る道が無いため。`worker.rs` に `Canceller`（Mutex の外で stdin と実行中の id を共有）、`main.rs` に command `cancel_run` を足した。応えなければ 2 s で kill して作り直す（ADR の表の Desktop の行）。S134・S135 との merge で衝突しうる（`main.rs` の `generate_handler!` と `Worker::new` の行）。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `serve`・`handle_with`・`Server::simulate`、試験 1 本。
- `src/Rust/apps/desktop/src/worker.rs`・`main.rs`: `Canceller`・`cancel_run`、試験 1 本（`GLOBE_BIN` があるとき）。
- `src/Svelte/src/lib/tauri-backend.ts`: `workDoneToken` を付け、`globe-notification` の `$/progress` を `onProgress` へ。`cancel()`。`backend.ts` のコメント。
- 文書: ADR-0033（5 段目の結果、Accepted）、`doc/adr/README.md`、`doc/architecture.md`、`doc/research/lang-api-matrix-2026-11-03.md`（LSP の進捗・中断、穴 3 → 2）、`doc/features.md`、`doc/roadmap.md`。

## 検証

- `cargo test -p globe-mode-lsp`: 17 本通過（新しい試験は 0.18 s で止まる）。`cargo clippy -p globe-mode-lsp --all-targets -D warnings`: 0。
- `worker.rs`（使い捨ての crate に `#[path]` で読み、`GLOBE_BIN` は debug の `globe`）: 4 本通過、clippy 0。
- `pnpm check`: 0 errors / 0 warnings。
- `mise run check`: （下で更新）
- `desktop.yml`: （下で更新）
- `check_docs.py`・`render_slides.py`: （下で更新）

## 残したこと

- Desktop の既定（`auto`、ネイティブの型付きの道）では積分の中で t も中断も出ない。
- CodeMirror と LSP の直接の接続、wasm で止めたときの途中の列。
