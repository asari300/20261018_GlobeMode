# S293 `vscode-diagram-delete`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s293-vscode-diagram-delete` |
| worktree | `.git-worktree/s293-vscode-diagram-delete` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S293-vscode-diagram-delete/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

VS Code の側のパネルでも、部品と結線を選んで Delete / Backspace で消せるようにする（Svelte の DiagramView と同じ操作）。

## 決めたこと

- 選びの処理は `diagram-select.ts` に出して共有した（無理なく純粋な関数にできた）。
- 結線の端の検査は `名前[添字]` を許す（`deleteConnect` は書かずに比べるだけ）。
- `reveal` を `preserveFocus: true` にした（さもないと Delete がエディタの文字を消す）。

## やったこと

`src/Svelte/src/lib/{diagram-select.ts,DiagramView.svelte}`、`src/VSCode/globe-mode/{edit.js,panel.js,webview.js,test/diagram-edit.test.mjs,e2e/suite.cjs,README.md}`、ADR-0037 付記、roadmap・features・セッション索引。

## 検証

- `pnpm test`（globe の debug を PATH に置いて）22 本が通る。`pnpm check`（svelte-check）0 件。`pnpm build` 通る。
- e2e: `vscode.yml` を feature で回した。1 回目は ubuntu で落ち（結線だけを消したあとは空の `equation` の見出しが残るのに、元の本文と等しいと確かめていた。S281 の仕様どおり、確かめを直した）、2 回目 run 43655786357 は Linux・Windows・macOS の 3 OS とも success。実機の VS Code での目視はしていない。

## 残したこと

複数選び、削除の確認の問い、実機での目視。
