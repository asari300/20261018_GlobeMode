# S143 `desktop-config-root`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s143-desktop-config-root` |
| worktree | `.git-worktree/s143-desktop-config-root` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S143-desktop-config-root/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

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
- `desktop.yml`（`gh workflow run desktop.yml --ref feature/s143-desktop-config-root`、run 41696623389、headSha f50d469、全ジョブ成功）: Linux の UI の測定 3 回とも `GM-SMOKE check=ok simulate=ok x1=0.3678797 msl=ok cfg=ok`（ready 154〜159 ms、rss 150 MiB 前後）。Windows は toml を置かないので `cfg=none`（合格条件に入れていない）。cargo test の `root_config_drives_format_and_reloads` も CI で通過（`GLOBE_BIN` あり）。他に動いていた S141 の run が終わるのを待ってから回した。
- `check_docs.py`: 全デッキ OK / 相対リンク 582 文書 2916 件中の切れ 0 件。`render_slides.py`: 手元では 2 回とも Page.screenshot が時間切れ。手元では通らず、CI の docs で確認。
- ステージした差分の漏洩検査（メール・ホームのパス・トークン）は 0 件。セッション記録は親が写す。

## 残したこと

- 設定の値の編集 UI、「フォルダを開く」で根を替えること（今は起動時だけ。S141 の後）、起動時に設定を読んだ通知（S130 は誤りのときだけ `showMessage`）、VS Code 拡張。
