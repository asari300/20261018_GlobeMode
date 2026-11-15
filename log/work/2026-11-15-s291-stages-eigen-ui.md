# S291 `stages-eigen-ui`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s291-stages-eigen-ui` |
| worktree | `.git-worktree/s291-stages-eigen-ui` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S291-stages-eigen-ui/` |
| セッション記録 | なし（`log/local/` にこのセッションのものが無い） |

## 依頼

UI に、線形化の固有値の表と安定性の 1 行、変換の段のパネル（ツールバーの「段」）を足す。

## 決めたこと

- Rust は変えない。`protocol.ts` の `Linearization` に `eigenvalues`・`stability` を足し、`LinearizeView` に表、`StagesPanel.svelte` を新設（BltPanel の流儀）。
- 安定性は文字（安定 / 不安定 / 境界 / 判定しない）。
- worktree は S282 が main に入る前だったので `origin/feature/integrate-1116` から切った。

## 検証

- `pnpm check` 0 errors / 0 warnings。
- wasm を `mise run wasm:build` で作り直し（main の `public/wasm` は stages を持たない古いもの）、`pnpm build` のあと `test_ui.py` 全体が ok（新しい 1 本 `eigen_and_stages`: LinearSystem で -1・-3 と「安定」、Pendulum の段でインデックス 3 と 5 段の順）。

## 残したこと

- 変換前後の式の差分、固有ベクトル、解析ヤコビ行列。
