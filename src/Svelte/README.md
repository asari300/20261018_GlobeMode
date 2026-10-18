# src/Svelte

Svelte 5 と TypeScript の UI（P4）。Tauri（デスクトップ、Android）とブラウザで共用する。

- 計算は `Backend` インターフェースを通して呼ぶ。実装は `TauriBackend`（ネイティブ）と `WorkerBackend`（Web Worker 上の wasm）
- 業務ロジックを置かない
- 使う部品:
  - エディタ: CodeMirror 6 と `@codemirror/lsp-client`
  - connect グラフ: sigma.js、graphology、ForceAtlas2。巨大なグラフは cosmos.gl
  - 回路図: Svelte Flow
