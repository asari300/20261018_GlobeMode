# S328 compare-max-error

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s328-compare-max-error`（`feature/s327-review4-issues` から切った。S327 は main に未取り込み） |
| worktree | `.git-worktree/s328-compare-max-error` |
| Phase | P8 |
| セッション文書 | `doc/sessions/S328-compare-max-error/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

複数ソルバの比較の結果に、変数ごとの最大の相対差と最大の絶対差、全体の要約（最大の相対差とその変数・時刻）を足す。管の判定は変えない。

## 決めたこと

- `compare::deviations`（別の関数。`compare` の型は変えない）。基準の出力点に試験側を線形補間、相対差は `|a-b| / max(|b|, 範囲 × 1e-12, MIN_POSITIVE)`。
- 出力は `pairs[i]`・`failures[]`・`comparison` に足すだけ。NaN は無限大（JSON は null）、比べた点が無ければキーなし。
- UI は比較の表に「最大の相対差」の列と全体の 1 行。閾値との大小は文字。

## やったこと

- `globe-mode-sim/src/compare.rs`: `deviations`・`Deviation`、試験 2 本（格子が違う・イベントの段差・0 を通る・NaN）。
- `globe-mode-protocol`・`globe-mode`（facade）・CLI・Python の docstring・Svelte（`protocol.ts`・`many.ts`・`ManyResult.svelte`）。
- 試験: facade `simulate_many.rs`、CLI `simulate.rs`、LSP、Python `test_columns.py`、`test_ui.py` の `multi_solver`。
- 文書: ADR-0040 に付記、features、README、CHANGELOG、Svelte の README、解説書・スライド、索引。

## 検証

- `cargo test`: globe-mode-sim の compare 21 本、`simulate_many` 3 本、CLI の `solvers_run_many_and_compare_with_an_exit_code`、LSP の `custom_requests_check_simulate_and_graph`。`pnpm check`: 0 件。Python の `test_columns.py` 4 本。
- 計測（debug、1 信号 100 万点）: `deviations` 0.24 s、`compare` 0.90 s。
- `test_ui.py` は wasm の release ビルドが要るので回していない（CI に任せる）。

## 残したこと

- release での計測、信号の多いモデルでの計測。
