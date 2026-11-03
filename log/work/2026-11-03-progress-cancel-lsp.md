# S136 `progress-cancel-lsp`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s136-progress-cancel-lsp` |
| worktree | `.git-worktree/s136-progress-cancel-lsp` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S136-progress-cancel-lsp/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

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

- `cargo test -p globe-mode-lsp`: 17 本通過。新しい試験は 20 回続けて通った（load average 22〜27）。`cargo test -p globe-mode-lsp -p globe-mode-cli`: 42 本通過、失敗 0。
- 試験の直し: 最初の版（1000 s、最初の t の report で止める）は負荷の下で 2 通りに落ちた。(1) 実行が 0.14 s で終わり止める前に `stop_time` になる、(2) t < 1 で止めると列は出力の点 t = 0 の 1 点だけ。1e5 s にし、t > 1（出力の間隔）の report を待つようにした。
- `worker.rs`（使い捨ての crate に `#[path]` で読み、`GLOBE_BIN` は debug の `globe`）: 4 本 × 3 回通過、clippy 0。
- `pnpm check`: 0 errors / 0 warnings。
- `mise run check`: fmt・clippy（`--workspace --all-targets -D warnings`）は通った。`cargo test --workspace` は 2 回とも別の理由で最後まで回らなかった: 1 回目は S131 の `globe-mode-c` の `a_nonzero_callback_cancels_with_the_partial_trajectory` が負荷の下で t = 0 で止まって落ちた（単独で回し直すと 3 本通る。S136 は globe-mode-c を触っていない。同じ型の不安定さ）、2 回目は並行するセッション（S138）の `cargo test --workspace` と重なって SIGTERM で止まった。`cargo deny` は依存を変えていないので省いた。`uv sync --locked`: ok。
- `desktop.yml`（feature で dispatch）: run 41654000916（1 回目のコミット）と run 41666485823（試験を直した後）とも success（ui・Linux の deb・Windows の msi）。Windows で `canceller_stops_a_long_simulate` と `talks_to_globe_lsp_and_recovers_from_a_kill` が ok、smoke の `GM-SMOKE check=ok simulate=ok`。
- `check_docs.py`: デッキ OK / 文書 564 件・相対リンク 2834 件中の切れ 0 件。
- `render_slides.py`: 手元では 2 回とも `Page.screenshot` の 30 s の時間切れで通らず、CI の docs で確認。
- セッション記録の `--check`: 取りこぼし 0 件（親が写した）。
- 漏洩検査（`git diff main...HEAD` のメールアドレス・ホームのパス・トークン）: 0 件。

## 残したこと

- Desktop の既定（`auto`、ネイティブの型付きの道）では積分の中で t も中断も出ない。
- CodeMirror と LSP の直接の接続、wasm で止めたときの途中の列。
- S131 の `globe-mode-c` の試験（最初の t の進捗で止めて `0 < time`）は負荷の下で t = 0 になって落ちることがある（S136 の試験と同じ型。直すなら t > 出力の間隔を待つ）。
