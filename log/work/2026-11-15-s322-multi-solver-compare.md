# S322 `multi-solver-compare`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s322-multi-solver-compare` |
| worktree | `.git-worktree/s322-multi-solver-compare` |
| Phase | P4 / P8 |
| セッション文書 | `doc/sessions/S322-multi-solver-compare/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

ユーザーの要望（2026-11-15、原文）:

「シミュレーション実行時に複数のソルバーを選択して結果を比較する手段が欲しい。
- 3以上選択可能とする（default=single, multiの場合はチェックボックス式）
- シミュレーションは一つずつ実行か並列実行か選択（default=single）
- 全シミュレーション終了後に自動で結果を比較するオプション（default=false）
  - 最大積分次数と許容値、比較閾値（10E-n）を指定してシミュレーションし比較」

この WP は計算の側。UI は次の WP S323。口の形は `doc/sessions/S322-multi-solver-compare/解説書.md` の「次回」に書いた。

## 決めたこと

- **最大次数**: 調べた結果、渡せるのは cvode・ida（SUNDIALS の `CVodeSetMaxOrd`・`IDASetMaxOrd`、1〜5）だけ。diffsol の BDF（`bdf`・`auto`）は上限が定数 5（`bdf_state.rs` の `MAX_ORDER`）で口が無く、Rumoca も出していない。ほかは手法で次数が固定。→ `SimSettings.max_order`（`deny_unknown_fields` のまま）。持たないソルバでは無視して情報 `GM-I10005`（黙って無視しない）、`runs[i].notes` に写す。cvode・ida は 1〜5 の範囲外を `GM-E10004`。
- **並列は別プロセスでなくスレッド**（依頼の文面と違う）: 同じ `CompiledModel` を共有するスレッドで最大 `min(CPU, 4, ソルバ数)`。理由は ADR-0040 の決定 3（RSS が GiB の単位、facade は wasm・Python からも呼ばれる、中断は既存の `CancelToken`）。欠点は、硬い実行を外から kill できないこと。Desktop の kill の予算は「ソルバの数 × 180 s」にした。wasm は逐次に落として `mode_note` に書く。
- **比較は補間と相対差でなく、既存の矩形の管**（依頼の文面と違う）: `globe-mode-sim::compare` を使う（MSL の参照結果の比較と同じ数え方を 1 か所に保つ。格子の違い・イベントに強い）。閾値 10^-n は管の幅の比率。最大の相対差・絶対差の数字は出さず、信号ごとの合否・外れた点の数・最大のはみ出し量・最初の時刻を出す。
- **上限**: 1〜16 個（重複は誤り）、`compare.threshold_exp` は 1〜15。
- ユーザーと決めたことではなく、依頼と brief の範囲で既定のまま進めた。ADR は [ADR-0040](../../doc/adr/0040-multi-solver-compare.md)。

## やったこと

- IR・core・sim: `SimSettings.max_order`・`ResolvedSettings.max_order`・`SolverId::has_max_order`、`GM-I10005`、`settings::resolve` の検査（試験 1 本）。
- exec: cvode・ida の作成時に `CVodeSetMaxOrd`・`IDASetMaxOrd`（feature `sundials`。`cargo check -p globe-mode-exec --features sundials` と clippy で通った。実際に回す試験はしていない）。
- protocol: `SimulateManyRequest`・`CompareRequest`・`SimulateManyResponse`・`ManyRun`・`ManyComparison` ほか。
- facade: `Workspace::simulate_many`（`prepare`・`finish_run`・`mark_response` に `simulate` から共通部分を取り出した）、`ManyHandle`・`ManyResult`・`ManyData`。
- CLI: `globe simulate --solvers a,b,c [--parallel] [--compare N] [--reference NAME] [--max-order N]`、`-o DIR`。LSP: `globe/simulateMany`（進捗の report に solver・index・count）。wasm: `simulate_many_columns`（`ManyColumns`）と `GlobeWorker.simulateMany`。Python: `Workspace.simulate_many`（`_native.simulate_many`、`Settings.max_order`）。Desktop: `RPC_METHODS` に `globe/simulateMany`、kill の予算。Svelte: `protocol.ts` の型、`Backend.simulateMany`（`WorkerBackend`・`TauriBackend`。UI は作っていない）。
- 試験: facade 3 本（逐次と並列が同じ列と最終値、比較が閾値で合否、注記、依頼の誤り、中断）、CLI 1 本、LSP（`custom_requests_check_simulate_and_graph` に逐次・並列と -32602）、sim 1 本、protocol 1 本、pytest 1 本、`test_tauri.mjs`、wasm の smoke と worker-node。
- 文書: ADR-0040、features・roadmap・ADR 索引・セッション索引、スライドと解説書。

## 検証

- `cargo fmt --all -- --check`: 差分 0。`cargo clippy --all-targets -- -D warnings`（ir・core・sim・exec・protocol・globe-mode・cli・lsp・wasm・py）: 警告 0。exec を `--features sundials` でも clippy 警告 0。
- `cargo test`（ir・core・sim・protocol・globe-mode・cli・lsp）: 351 件通過・0 失敗・6 無視。（exec・wasm・py）: 99 件通過・0 失敗・6 無視。
- `uv run pytest src/Python/tests`: 53 通過・5 skip・0 失敗（`test_strict` の `Settings` と `settings_sample` の一致を含む）。`node src/Svelte/tests/test_tauri.mjs`: ok。
- `wasm32-unknown-unknown` の `cargo check -p globe-mode-wasm`: 通過。
- Desktop（別の workspace、webkit が要るのでビルドしていない）: `cargo fmt` だけ。`RPC_METHODS` の件数 39 と配列の中身の数が一致（スクリプトで数えた）。
- `mise run wasm:node`: 通った（EXIT 0、82 s。`simulate_many_columns` の smoke を足した。data/msl が無いので MSL の例題は飛ばした）。
- `mise run wasm:compare`: **480 件中 466 件が一致、既知の差 13 件、一致しない 1 件**。一致しない 1 件は `Pendulum` / `dopri853`（相対差 2.0e-9、閾値 1e-9）。`Pendulum` は同じ日の S254 で足したモデルで、この WP の変更（`max_order` は None のとき実行系の数値に触れない）とは関係なく、main でも出る見込み（main では回していない【未確認】）。直していない（既知の差の表に足すか許容を決めるのは別の判断）。
- `check_docs.py`: 全デッキ OK、リンク 5024 件中の切れ 0 件。
- `render_slides.py`（S322 のデッキだけ）: 問題 0 件（Chromium が 30 s で時間切れになるのを 3 回待ち直した）。markdownlint は手元に無く回していない（表・見出し・コードの前後は空行にした）。

## 残したこと

- 別プロセスでの並列（硬い実行を確実に止める）。今は放置スレッド。
- 最大の相対差・絶対差の表示。管の合否だけ。
- `bdf` の最大次数（diffsol に口が無い。上流への依頼の材料）。
- cvode・ida に `max_order` を渡して回した試験（sundials を立てた build）は未実施。`msl-solvers.yml` の再実行も未実施（実行系の挙動は `max_order` が None のとき変わらない）。
- S323 の UI。
