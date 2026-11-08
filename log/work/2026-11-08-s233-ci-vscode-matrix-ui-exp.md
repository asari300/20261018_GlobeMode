# S233 `ci-vscode-matrix-ui-exp`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s233-ci-vscode-matrix-ui-exp` |
| worktree | `.git-worktree/s233-ci-vscode-matrix-ui-exp` |
| Phase | P4・P7 |
| セッション文書 | `doc/sessions/S233-ci-vscode-matrix-ui-exp/` |
| セッション記録 | （親が写す） |

## 依頼

roadmap の CI の穴 2 つ。`vscode.yml` を 3 OS に、試験版 UI（Leptos・Dioxus）の push の検査 `ui_exp`・`ui-exp` を足す。

## 決めたこと

- Rust は全 OS で rustup を直接使う（Windows の mise の shim が失敗した前例）。mise は node・pnpm だけ。
- macOS の失敗は試験の書き方（`e2e/run.mjs` の user-data-dir が長く IPC ソケットが 103 文字を超えた）。拡張の実装は直していない。
- `ui-exp` の `cargo test` は試験版にホストの試験が無く、ビルドの確認にとどまる。trunk・dx は回さない。
- CHANGELOG は書かない（CI の変更で利用者に見えない）。

## やったこと

- `.github/workflows/vscode.yml`（matrix、bash、`$GITHUB_PATH`、Linux だけ apt・xvfb）、`.github/workflows/ci.yml`（`ui-exp`、plan の出力）、`ci_plan.py`・`test_ci_plan.py`（`ui_exp`、1 本）。
- `src/VSCode/globe-mode/e2e/run.mjs`（短い `--user-data-dir`）。
- `src/Rust/apps/experimental/{leptos,dioxus}/Cargo.lock`（`cargo update -w`。`--locked` が見つけた古さ）。
- 文書: CLAUDE.md、architecture 2.5、ADR-0006 の付記、roadmap の 2 行、セッション文書。

## 検証

- `test_ci_plan.py`: 29 件 OK。
- vscode.yml run 42461195163（ubuntu 成功・windows 成功・macos 失敗）、直して run 42462400728（3 OS 成功）。
- ci.yml full run 42461749541 成功（`ui-exp` 2 本を含む）。最初の run 42461197860 は古い lock で `ui-exp` が失敗し、取り消された。

## 残したこと

- Windows・macOS の fmusim など、roadmap の別の CI の穴。
