# S143 `desktop-config-root`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s143-desktop-config-root` |
| worktree | `.git-worktree/s143-desktop-config-root` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S143-desktop-config-root/` |
| セッション記録 | （親が写す） |

## 依頼

S140 の点検の順位 5。Desktop が `globe-mode.toml`（S130）を読めるようにする。`worker.rs` の `initialize` に根を渡し、`App.svelte` の `format()` から固定値を外し、設定の読み直しを command 1 つで送れるようにする。`desktop.yml` の smoke に `cfg=ok` を 1 項目。

## 決めたこと

- 根は `--root`、無ければ `GLOBE_ROOT`、無ければカレントディレクトリ。`rootUri` でなく `rootPath` で渡す（LSP は両方読む。URI の組み立てを足さない）。
- 依頼の前提は半分外れた。「LSP の設定は S130 で済」だが、UI が呼ぶ独自の要求 `globe/format`・`globe/lint` は設定を見ていなかった（標準の `textDocument/formatting` と診断だけ）。Desktop が設定に任せるには、この 2 つを設定を土台にする形に直す必要があった（params のキーがあれば優先なので、ブラウザ版と既存の試験は変わらない）。依頼の「`globe-mode-lsp` の中身は触らない」から外れた点で、親に伝える。
- 「設定ファイルがあれば `window/showMessage` で『読んだ』と出る」は S130 の実際の挙動と違い、`showMessage` は誤りのときだけ。UI は誤りを受けて「設定を読み直す」の行に出す。成功の通知は足さない。
- smoke は 3 回とも `indent = 4` の toml のある作業ディレクトリから起動し、Desktop だけ GM-SMOKE の行に `cfg=ok`（効かなければ `cfg=none`）を足す。`desktop.yml` の Linux の合格条件を `msl=ok cfg=ok` にした。ブラウザの行（`test_ui.py`・`bench_ui.py`・android.yml）は変わらない。

## やったこと

- `globe-mode-lsp`: `globe/format`・`globe/lint` が設定を使う。既存の試験 `config_file_at_the_root_drives_lint_and_formatting` に 3 つの確認（params なしで indent 4、params の indent 2 が勝つ、lint の disable）。
- `apps/desktop`: `Worker::with_root`、`initialize` の `rootPath`、`root_from_env`、command `reload_config`、試験 `root_config_drives_format_and_reloads`（`GLOBE_BIN` の実プロセス）。
- `src/Svelte`: `Backend.format` の引数を optional、`WorkerBackend` が 2・true、`TauriBackend.reloadConfig`・`onMessage`、`App.svelte` の `format()`・「設定を読み直す」・smoke の `cfg`。
- `.github/workflows/desktop.yml`: UI の測定を toml のある作業ディレクトリから、`cfg=ok` を数える。
- `doc/features.md`（整形・lint、Desktop）、`doc/roadmap.md`（P4 と「S143 で済んだこと」）、ADR-0023 と ADR-0024 に 1 段落、セッション文書、索引。

## 検証

- `cargo test -p globe-mode-lsp config_file`: 通過。
- worker の試験: Desktop の crate は手元で作れない（webkit2gtk なし）ので、`worker.rs` だけを入れた雛形の crate に `GLOBE_BIN` を付けて 1 本通過（根の toml で indent 4、書き換えて reload で 3）。
- `pnpm check`: 262 ファイル 0 error 0 warning。`test_ui.py`: ok（ブラウザの GM-SMOKE の行に cfg は付かない）。
- `desktop.yml`: （下に書く）
