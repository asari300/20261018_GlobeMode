# S306 `vscode-params`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s306-vscode-params` |
| worktree | `.git-worktree/s306-vscode-params` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S306-vscode-params/` |
| セッション記録 | なし（このセッション専用の記録が `log/local/` に無いので省く） |

## 依頼

VS Code の側のパネルで、図の部品のパラメータを見て、ソースの修飾として書けるようにする（UI のパラメータダイアログの VS Code 版の最小）。部品のダブルクリックから `globe/parameters`・`globe/setModifier` を呼ぶ。

## 決めたこと

- 入力は QuickPick → InputBox（Boolean・列挙は QuickPick）。webview の中の表より小さい（CSP・入力部品・検証・列挙の選びを `webview.js` に足さずに済む）。多くの値を一度に直す表は持ち越し。
- webview が送るのは `{type: "params", part}` だけ。`parseParams` が部品の名前（識別子）を検査し、`file`・`class` は S264 と同じく受け取らない。
- 値の字句の検査は Rust の `set_modifier` に任せ、断られた理由（-32803）を `showErrorMessage` に出す。InputBox は数に絞らない。
- 空の入力は修飾を外す（InputBox が既定値を初期値に持つので、消せば外れる）。Escape は取り消し。
- e2e は `panel.ui`（既定は `vscode.window`）を差し替えて値を渡す。ダブルクリックは拡張ホストから起こせないので、webview のメッセージを `panel.handle` に渡す（S264・S293 と同じ流儀）。
- `apply` の失敗理由を、版が変わった・編集が空、の 2 つの日本語の文にした（通知に出すため。以前は JSON を出していた）。

## やったこと

- `src/VSCode/globe-mode/edit.js`: `parseParams`・`partParams`・`enumValue`。
- `panel.js`: `editParams`、`handle` に `params`、`ui`。`webview.js`: `dblclick` で `params` を送る。
- `test/diagram-edit.test.mjs`: 検査の試験。`e2e/suite.cjs`・`e2e/fixtures/Params.mo`: e2e 1 本。
- 文書: ADR-0037 付記、roadmap・features の VS Code の行、拡張の README、セッション文書、索引。

## 検証

- `pnpm test`（`GLOBE_BIN` はデバッグの `globe`）: 24 件 pass / 0 件 fail（うち今回の追加 1 件）。
- `vscode.yml`（3 OS の e2e、run 43664255217。origin/main（S302）を取り込んだ後の再実行 43664791298 も 3 OS とも success）: ubuntu・windows・macos とも success。途中の失敗 3 回の原因は、(1) 開いたままの Broken.mo で LSP の Workspace 全体の変換が通らず `globe/parameters` が -32000 になる（e2e で Broken.mo を保存せずに直した。closeAllEditors では didClose が届かない）、(2) e2e の列挙の選びの書き間違い（`find` の中で `step++`）。
- `check_docs.py`: 切れ 0 件。`render_slides.py`: デッキ 1 件 / 問題 0 件（他の WP と同時のときは Chromium のスクリーンショットが時間切れになり、数回の再実行で通った。S299 の既存のデッキでも同じ）。
- Rust は変えていない（`mise run check` は回していない）。

## 残したこと

- キーボードだけで開く口（コマンドパレット）。
- 複数の値を一度に直す表。
- 構文の壊れた別の文書が開いていると `globe/parameters` が失敗し、通知に診断の文が出る（LSP が Workspace 全体を変換するため。UI と同じ制約）。
- 入れ子・`each`・`final`・`redeclare`、配列の部品。
