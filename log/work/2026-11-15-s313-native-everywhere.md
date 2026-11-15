# S313 `native-everywhere`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s313-native-everywhere`（`feature/s311-flat-ranges` から切った） |
| worktree | `.git-worktree/s313-native-everywhere` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S313-native-everywhere/` |
| セッション記録 | なし（専用のものが log/local/ に無い） |

## 依頼

CLI 以外（facade・LSP・Python）からも native の検査を選べるようにする。既定は Rumoca。wasm・UI は今回やらない。

## 決めたこと

- facade `CheckFrontend` と `Workspace::check_with`。`native_response` を CLI と共有。
- feature `native` を facade・LSP に足し、CLI・Python が立てる。wasm は立てない。立てない build で選ぶと `GM-U0006`。
- LSP は `[check] frontend`（`LintOptions::frontend`）。`publishDiagnostics` だけ。
- Python は `check(model, frontend="rumoca")`。

## 検証

- `cargo test -p globe-mode --test check_with`（feature あり・なし両方 pass）・`-p globe-mode-lsp --features native`（69 本 pass、1 本 ignore）・`-p globe-mode-cli --test check_native`（4 本 pass）・`-p globe-mode-fmt --features config config`（2 本 pass）・`uv run pytest src/Python/tests/test_api.py`（11 本 pass）。
- `cargo clippy -D warnings`（globe-mode・lsp・fmt・cli・py の全 target）・`cargo fmt`: 警告 0。`mise run check` 全体は回していない（変えた crate に絞った）。
- 共有 target の古いビルドで flat・ir などが別 worktree のものになり、lib.rs を touch して直した。

## 残したこと

wasm・UI から選ぶこと（wasm のビルドの大きさの測定が先）。LSP の `globe/check`・補完などを native にすること。
