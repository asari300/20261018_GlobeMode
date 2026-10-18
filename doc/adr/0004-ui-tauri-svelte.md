# ADR-0004 UI は Tauri 2 + Svelte 5

- 状態: Accepted（2026-10-18、S001）

## 背景

- 対象はデスクトップ（Debian 系 / Windows 11）、Android、ブラウザ。
- エディタ（CodeMirror 6）、グラフ（sigma.js、cosmos.gl）、回路図（xyflow、maxGraph）の良い部品は JS にそろっている。

## 決定

- 本線は **Tauri 2 + Svelte 5（TypeScript）**。UI は `Backend` インターフェースだけに依存する。
  - デスクトップと Android では TauriBackend がネイティブのコアを呼ぶ。
  - ブラウザでは WorkerBackend が Web Worker 上の wasm を呼ぶ。
- 計算はすべて Rust で行う。UI に業務ロジックを置かない。
- **Leptos 版と Dioxus 版**は試験 feature として、機能を絞って作る（P7）。測定項目は起動時間、RSS / PSS、CPU、フレーム時間、バンドルサイズ。結果を `doc/bench/ui.md` に書く。

## 結果

- Node と pnpm が要る（mise で入れる）。npm の依存もライセンスを監査する。
- Slint は GPL の問題があるので不採用。egui はエディタが弱いので不採用。
