# S323 `multi-solver-ui`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s323-multi-solver-ui` |
| worktree | `.git-worktree/s323-multi-solver-ui` |
| Phase | P4 / P8 |
| セッション文書 | `doc/sessions/S323-multi-solver-ui/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S322 の計算の側（`Backend.simulateMany`）を使う UI。実行の設定に、ソルバの選び方（1 つ／複数のチェックボックス）、1 つずつ／並列、全部終わったら比べる（n・基準）、最大積分次数の欄を足し、結果をソルバごとの重ね描きと表で出す。設定はモデルごとの保存（S228）に乗せる。

## 決めたこと

- 欄は `SimSettings.svelte` に足し、依頼の分は `many`（`lib/many.ts`）として App に渡す。許容誤差・最大次数は 1 つの欄を全部に渡す（ソルバごとの値は持たない）。最大次数は 1〜5 の整数だけ。
- 重ね描きは `Plot` を変えず、列を時刻の和集合に折れ線で補って `x [bdf]` の 1 つの `Columns` にする。実行の順は一覧の順（チェックした順ではない）。
- 複数のときの強行（`can_force`）は応答に無いので出さない（持ち越し）。
- ブラウザの並列は選べるが注記を出す（`inDesktop()` でない間）。

## やったこと

- 新規: `src/Svelte/src/lib/many.ts`（`ManyConfig`・`mergeRuns`・`pairSummary`）、`ManyResult.svelte`（実行の表・比較の表・`mode_note`）。
- 変更: `SimSettings.svelte`（欄・検査・保存）、`App.svelte`（`simulateMany`・進捗はソルバごと・表の表示・`reveal`）、`tests/test_ui.py`（`multi_solver`）。
- 文書: ADR-0040 の付記、features・roadmap、`src/Svelte/README.md`、セッション索引、スライドと解説書。

## 検証

- `pnpm check`: 287 ファイル 0 エラー 0 警告。
- `test_tauri.mjs`: ok。
- `test_ui.py`: `multi_solver`（新規）通過。ほかの節も通過したが、最後の `GM-SMOKE` の節だけ `data/msl` の MSL.json が無い（404）ため落ちる（この worktree に MSL の取得物が無いための既存の状態。今回の変更とは無関係）。wasm は S322 でビルド済みの共有 target のものを使った（Rust は変えていない）。
- `check_docs.py`・リンク・漏洩検査: 下の報告のとおり。

## 残したこと

- 複数のときの「それでも実行する」、ソルバごとの許容誤差、Desktop の実機（sidecar の並列）での画面確認はしていない。
