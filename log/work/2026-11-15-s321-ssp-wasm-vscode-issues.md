# S321 `ssp-wasm-vscode-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s321-ssp-wasm-vscode-issues` |
| worktree | `.git-worktree/s321-ssp-wasm-vscode-issues` |
| Phase | 監査（3 回目のレビューの issue） |
| セッション文書 | `doc/sessions/S321-ssp-wasm-vscode-issues/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

3 回目のレビューの issue #112（SSP の階層名の長さと要素数）、#110（分割配信の塊が壊れていると固まる）、#117（VS Code のパネルでパラメータ入力中に対象が変わる）を 1 件ずつ監査して直す。issue は閉じない。

## 決めたこと

- 3 件とも妥当と判断して直した。判断の表は解説書の 2 章。
- #112: `MAX_PATH` 4096 と `MAX_ITEMS` 10000。コネクタ・接続・束縛も数える（階層名の複製を持つため）。書き出し側（S318 の担当）は触らない。
- #110: `fetchChunks` は読めてから `got`。索引と合わない塊は `got` に入れず誤り。`ensure` に回数の上限。
- #117: `apply` に `(t, v)` を足した。`edit` は従来どおり。

## 検証

- `cargo test -p globe-mode-fmi ssp`: 6 件（新規 1）通る。
- `node broken-chunks.mjs`: 直す前の worker.js では「固まった: 塊に無い 1 回目」で落ち、直すと通る。
- `node --test test/panel-params-target.test.mjs`: 直す前は 2 件失敗、直すと通る。panel・diagram-edit の既存の試験も通る。`pnpm test` の `lsp.test.mjs` は `globe` が PATH に無く落ちる（この変更とは無関係。CI の `vscode.yml` が `globe` を置いて回す）。

## 残したこと

`MAX_UNPACKED`、#117 の e2e（`vscode.yml` の手動実行）。
