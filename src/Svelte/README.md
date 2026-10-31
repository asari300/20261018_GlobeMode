# src/Svelte

Svelte 5 と TypeScript の UI（P4）。Tauri（デスクトップ、Android）とブラウザで共用する。

- 計算は `Backend` インターフェースを通して呼ぶ。実装は `TauriBackend`（ネイティブ）と `WorkerBackend`（Web Worker 上の wasm）
- 業務ロジックを置かない
- 使う部品:
  - エディタ: CodeMirror 6 と `@codemirror/lsp-client`
  - connect グラフ: sigma.js、graphology、ForceAtlas2。巨大なグラフは cosmos.gl
  - 回路図: Svelte Flow

## 使い方（S041）

ブラウザ UI の骨格。`Backend`（`src/lib/backend.ts`）、`WorkerBackend`（`src/lib/worker-backend.ts`。`globe-mode-wasm/web/client.js` の `GlobeWorker` を包む）、CodeMirror 6 のエディタ、診断パネル（段階 S0〜S10、重さの順。行を押すとエディタの選択が動く）だけ。Tauri、グラフ、プロットは未実装。

```bash
mise run wasm:build          # wasm と糊を target/wasm-pkg/web に作る
cd src/Svelte
pnpm install
pnpm dev                     # 糊を public/wasm/ にコピーして vite（http://localhost:5173）
mise run ui:check            # svelte-check と、Playwright の Chromium での試験（src/Svelte/tests/test_ui.py）
mise run ui:build            # dist/ に静的サイトを作る
```

## 実行とプロット（S042）

「実行」ボタンが `Backend.simulate({ model })` を呼ぶ（ソルバは既定のまま）。`refused` と診断は診断パネルとその上の赤字に出る。結果の CSV（`SimulateResponse.csv`）は `src/lib/plot/csv.ts` が列に分け、`src/lib/plot/Plot.svelte` が uPlot（MIT）で時系列に描く。信号はチェックボックスで選ぶ（既定は最初の 3 本）。ズームは uPlot の既定（ドラッグで拡大、ダブルクリックで戻す）。2 軸、凡例の凝った UI、MAT v4 の読み込みは未実装。

- CSP は `index.html` の meta で `connect-src 'self'` のまま。`'wasm-unsafe-eval'` は wasm の初期化に、`style-src 'unsafe-inline'` は Svelte と CodeMirror が注入する style に要る
- TypeScript は 6 系（svelte-check 4.7 が 7 を単独では受けない）
- 診断の `range` は UTF-8 のバイト位置。UI で UTF-16 の位置に直して選択する
