# S212 `core-issues-2` のレビュー（ponytail-audit）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s212-core-issues-2` |
| 対象 | GitHub の issue #23・#29・#31・#34・#37・#38（ユーザーが 2026-11-07 に `/ponytail-audit` 5.3.2 で立てた 20 件のうちの 6 件） |
| レビュアー | ponytail-audit（コードを読んだだけで、ビルドも実行もしていない指摘） |
| 依頼の文面 | 1 件ずつ、いまの main で再現してから直す。再現しなければ理由を書いて直さない |
| 作業記録 | `log/work/2026-11-07-s212-core-issues-2.md` |

## 指摘（原文のまま）

issue の本文は `gh issue view <n>` にある（重さは audit の点数）。要点だけを写す。

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 23 | 2/20 | `globe-mode-fmi/src/export.rs:85-89` | `write_fmu` が `<out>/<名前>/` を確認なしに `remove_dir_all` で消す。守り（`prepare_out_dir`）は CLI にしか無く、Desktop・Python・C・Elixir は守りなしで届く。直し方: `modelDescription.xml` があるときだけ消し、無ければエラー | 採用 | 再現した（`mine.mo` の入ったフォルダが消えた）。`write_fmu` の中に `check_out_root` を置き、CLI の事前検査は同じ関数に寄せた |
| 34 | 13/20 | `globe-mode-sim/src/settings.rs`、`globe-mode/src/lib.rs`（CSV）、`globe-mode-fmi-import` `run_cs` | 出力の点数に上限が無く、小さい Interval・step で OOM になる。`resolve` で `span / interval`・`span / step` が 1e7 を越えたら GM-E10004 | 採用（sim の設定だけ） | 再現した（`--stop-time 1000 --interval 1e-9` と rk4 の `--step 1e-15` が設定を通った）。`run_cs` は S206 で済んでいるので触らない |
| 38 | 17/20 | `globe-mode-frontend/src/rumoca.rs:735-737` | Integer に `n=2.5` は `round` で 3、Boolean に `b=0.3` は true。実行時の `param_overrides` には素の値が渡る。GM-E10006 で断る | 採用 | 再現した（`expect_err` が `Ok`）。丸めを断りに変えた |
| 37 | 16/20 | `globe-mode-frontend/src/rumoca.rs:835`（`Checker::reset`）、`1322-1338` | 長く生きる Workspace が `reset` を呼ばず、メモリが増える一方。xtask と同じく、異なるモデルの数が閾値を越えたら `reset` | 採用 | facade の `Workspace` に置いた（LSP・Python・C・NIF・wasm が乗る）。測定は解説書 |
| 31 | 10/20 | `globe-mode/src/lib.rs:689`（`collect_mo`） | 既定のライブラリ読み込みに S068 の規則が無く、AixLib で型が二重に読まれる。`collect_mo_files` を公開して写しを消す | 採用 | 再現した（AixLib の `RadiatorType` が GM-E1001 の二重定義）。facade と db の写しを消した |
| 29 | 8/20 | `globe-mode-fmi-import/src/ode.rs:139-143`、`me.rs:275-280` | FMU が返した NaN が Dormand–Prince の誤差検査をすり抜け、成功として終わる。RK4 には検査自体が無い | 一部採用 | Dormand–Prince は S206 で直っていた（試験で確認）。RK4 は再現した（`Ok` の行に NaN）ので `drive` に有限性の検査を足した |

## 結論

- 採用 6 件（#29 は RK4 の半分）・保留 0 件・不採用 0 件。
- 指摘が誤った点: #29 の 1 点目は S206 で直っていた。#31 の `globe-mode-lsp`・`globe-mode-cli` の写しは範囲外（別セッション）。
