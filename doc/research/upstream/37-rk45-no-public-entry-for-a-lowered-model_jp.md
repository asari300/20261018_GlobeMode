# 下書き（未提出）37: `rk45` に、lowering 済みのモデルを渡す公開の口が無い（`simulate_artifact` が `pub(crate)`）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [37-rk45-no-public-entry-for-a-lowered-model.md](37-rk45-no-public-entry-for-a-lowered-model.md)、モデルは [.mo](37-rk45-no-public-entry-for-a-lowered-model.mo)、1 枚の解説は [_jp.html](37-rk45-no-public-entry-for-a-lowered-model_jp.html)。

判定（GlobeMode、S359）: **API の要望**（バグではない）。結果は誤っていない。DAE を Solve IR にする lowering は、モデルによっては準備でいちばん長い段だが、呼び手に要る回数より 1 回多く走り、公開の API では避けられない。v0.10.2・v0.10.4・main のソースを読んで比べた。実行して時間は測っていない。S344（2026-11-15）の同じ読みが GlobeMode の issue #46 の出どころ。

## 概要

`rumoca_sim::rk45` の公開の実行の入口は 3 つ: `simulate`（`simulate_dae`）、`simulate_with_diagnostics`、`SimulationSession`。どれも `&Dae` を受け取り、中で lowering する（`lower_runtime_artifact` → `lower_runtime_fmi_artifact` → `lower_correlated_for_simulation_with_overrides`）。lowering 済みのモデルで積分器を回す関数 `rk45::simulate_artifact(artifact, opts, execution_backend)` は `pub(crate)`。その引数 `(MeModelArtifact, Option<MeExecutionBackend>)` を `LoweredSolveModel` から作る `finish_runtime_fmi_artifact` と、実行の backend を選ぶ `admitted_native_execution_backend` も `pub(crate)`。

BDF の側には、呼び手が要る形が既にある。`build_simulation_with_stage_timing_and_lowered_model(dae, opts, begin_stage, observe_lowered_model)` は、lowering と実行時の部品の組み立ての間に `observe_lowered_model(&LoweredSolveModel)` を 1 回呼び、`PreparedSimulation` を返す。それを `run_prepared_simulation` が回す。lowering 済みのモデルを自分の用途にも使いたい呼び手（GlobeMode は `SolveRuntime` を作り、開始値でモデルを評価して、実行の前に有限でない開始値を警告する）は、BDF なら lowering が 1 回で済む。

rk45 では同じ呼び手が 2 回 lowering する: 自分の用途のために公開の `lower_correlated_for_simulation_with_overrides` で 1 回、`rk45::simulate` の中で 1 回。rk45 には `PreparedSimulation` も observer も無い。

## 再現手順

口が無いので、「再現」は書けない呼び出し。【ソースから読んだ。コンパイルはしていない】

```rust
let opts = rumoca_sim::SimOptions { solver_mode: rumoca_sim::SimSolverMode::RkLike, ..Default::default() };

// いまできること: 自分の用途で lowering し ...
let lowered = rumoca_sim::lower_correlated_for_simulation_with_overrides(&dae, &opts)?;
let _start_values = lowered.model().initial_y.clone();      // 呼び手の用途
// ... 呼び出しの中でもう一度 lowering する:
let result = rumoca_sim::rk45::simulate(&dae, &opts)?;

// できないこと: `lowered` で rk45 を回す。
// `rumoca_sim::rk45::simulate_artifact(artifact, &opts, backend)` は `pub(crate)`。
```

可視性の場所:

| Rumoca | `rk45::simulate_artifact` | `build_simulation_with_stage_timing_and_lowered_model`（BDF） |
| --- | --- | --- |
| v0.10.2（`276e82bd`） | `crates/rumoca-sim/src/rk45.rs:44`、`pub(crate)` | `diffsol.rs:184`、`pub` |
| v0.10.4（`9c8c56fc`） | `rk45.rs:71`、`pub(crate)` | `diffsol.rs:186`、`pub` |
| main（`b55eafce`、2026-11-13） | `rk45.rs:115`、`pub(crate)`（`simulate_artifact_with_observation` も `:123`） | `diffsol.rs:203`、`pub` |

【行番号は v0.10.2・v0.10.4 の git checkout から、main は GitHub の contents API から 2026-11-16 に読んだ】

ほかの呼び出しを試すモデルは [.mo](37-rk45-no-public-entry-for-a-lowered-model.mo)（`Rep37.A`。シミュレーションできるモデルなら何でも同じ呼び方）。`rumoca sim 37-rk45-no-public-entry-for-a-lowered-model.mo -m Rep37.A --solver rk-like` は v0.10.2・v0.10.4 で回る（501 点）。CLI では 2 回目の lowering は見えない。

## 期待

lowering 済みのモデルで rk45 を回す公開の口。形は Rumoca の設計に合うほうで。

1. `rk45::simulate_artifact` を公開し、`LoweredSolveModel` から引数を作る公開の口を足す（いまは `finish_runtime_fmi_artifact`・`admitted_native_execution_backend` が `pub(crate)`。`rumoca_phase_solve::fmi::finish_fmi_component` と `MeModelArtifact::new` は公開だが、手で作った artifact には v0.10.4 で `finish_runtime_fmi_artifact` が足すもの〔実行の backend と実行の受領証〕が無い）。または
2. rk45 にも BDF の道と同じ形、つまり `build_simulation_with_stage_timing_and_lowered_model` のような、`run()` を持つ準備済みの物を返す関数。

## 実際

そのような口は無い。呼び手は 2 回 lowering する。

## v0.10.4・main での再確認（S359、2026-11-16）

最新のリリースは v0.10.4 のまま（タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、2026-11-02 公開。`git ls-remote` と releases の API、2026-11-16）。`rk45::simulate_artifact` は v0.10.4 でも main（`b55eafce2475be1721324d56e1c51ddecfe7dbdb`、2026-11-13）でも `pub(crate)`。lowering の道は rk45 も BDF も同じ関数（`lower_correlated_for_simulation_with_overrides`）。`crates/rumoca-sim/src/solve_lowering/` に lowering の cache は見つからなかった。`rumoca-phase-solve` の中は探していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`、GlobeMode の固定）、v0.10.4（<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）、main（`b55eafce`）。ソースを読んで比べた。
- Linux x86_64（WSL2、Ubuntu）。この下書きのために何もビルドしていない。

## GlobeMode 側の回避

なし。GlobeMode のネイティブの `dopri45`（rk45）は 2 回 lowering する（自前のプリフライトの `lower_correlated_for_simulation_with_overrides` と `rk45::simulate`）。`auto` は回し直しのたびにもう一度 lowering する（BDF・esdirk34・dopri45、最大 3 回）。S344 で、回し直しは時間の予算が尽きたら始めないようにした（ADR-0041）が、予算が残っている実行の lowering の重複は残る。GlobeMode の issue #46 の案（dopri45 を GlobeMode の session の道〔`rumoca_solver::fmi_me`〕で回して lowering を 1 回にする）は、S218（#43）で BDF について測って取り消した。MSL の `EngineV6` で、型付きの BDF の道は 202 s で終わり、session の道は予算 170 s で時間切れになった（GlobeMode の ADR-0013、S218 の付記）。rk45 では測っていない【未確認】。

## 備考

- 費用: lowering はモデルによっては最も長い段で、Rumoca の時間の予算の外にある（予算は host を作るときから測る）。GlobeMode の MSL の実行（v0.10.2、S015、2026-10-20、空いた機械）で lowering の段は `EngineV6` 64 s、`FullRobot` 44 s、`Engine1b_analytic` 14 s。`FullRobot` は機械の負荷の下で約 118 s だった。これは 1 回の lowering の費用。dopri45 の 2 回目の lowering の費用は別には測っていない【未確認】。
- 初期化の双子が 38（準備済みの BDF の部品が 3 回初期化される）。14 は `admitted_native_execution_backend` を公開にする要望を「関連する可視性の依頼」として書いている。それも要るなら、上の 1 で足りる。
- 出どころ: GlobeMode の issue #46 とそのコメント（S226、S344）、[S344 の解説書](../../sessions/S344-solver-budget-issues/解説書.md)、[ADR-0041](../../adr/0041-auto-remaining-budget.md)。
