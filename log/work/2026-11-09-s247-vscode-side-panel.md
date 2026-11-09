# S247 `vscode-side-panel`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s247-vscode-side-panel` |
| worktree | `.git-worktree/s247-vscode-side-panel` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S247-vscode-side-panel/` |
| セッション記録 | （親が写す） |

## 依頼

VS Code 拡張に、Svelte の UI と同じ「説明 / アイコン / 図」の側のパネルを足す（`globe/documentation`・`globe/icon`・`globe/diagram` を再利用。拡張に業務ロジックを置かない）。無害化と SVG の組み立ては写さず共有する。

## 決めたこと

- 共有の置き場は `src/Svelte/src/lib/` のまま、拡張の `webview.js` が相対 import して esbuild で束ねる（移す案は得るものが無い）。
- Svelte の `ShapeView.svelte` は Svelte 部品なので webview から使えない。描画を `shape-svg.ts` の純粋な関数（要素の木 `SvgNode` を返す）に切り出し、Svelte は `SvgTree.svelte`（`<svelte:element>`）、webview は `toDom` で描く。`DiagramView` の部品のクリックは svg での一括受け（`data-line`）に替えた。DOM の形は変えていない。
- 無害化は webview の中（DOMParser がある）。拡張ホストでは洗わない。
- e2e は webview の DOM が見えないので、webview が描き終えた数を postMessage で返し、`activate` の戻り値 `panel.state` を読む（`reveal` の確認も同じ道）。
- 図の地は UI と同じく常に白。テーマ追従は持ち越し。

## やったこと

`src/VSCode/globe-mode/{panel.js,webview.js,extension.js,package.json,pack_vsix.py,README.md,test/panel.test.mjs,test/save.test.mjs,e2e/suite.cjs,e2e/fixtures/ChuaCircuit.mo}`、`src/Svelte/src/lib/{shape-svg.ts,SvgTree.svelte,IconView.svelte,DiagramView.svelte}`（`ShapeView.svelte` は削除）、`.github/workflows/vscode.yml`（paths に 2 ファイル）、文書（ADR-0037 の付記、features、roadmap、CHANGELOG、セッション文書、索引）。Rust は触っていない。

## 検証

結果は下に書く。

## 残したこと

図の編集、画像、テーマ追従の地の色。VS Code 本体を手元で起動していない（e2e は Actions だけ）。
