# ADR-0016 プリフライトは組み立ての lowering の結果で行い、実行系の中の段を知らせる

- 状態: Accepted（2026-10-20、S014）
- 関係: [ADR-0013](0013-exec-backend.md) の S10（実行の直前）の評価の仕方と、[ADR-0014](0014-session-route-wasm.md) の「残した問題: プリフライトの lowering は 2 回のまま」を片付ける。どちらも覆さない

## 背景

- `globe_mode_exec::run` は、S10 の数値の部分（t0 で式の値が有限か。警告 `GM-W10002`）のために lowering を 1 回し、実行の組み立てでもう 1 回していた。lowering の遅いモデルでは時間が倍近くになる（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の「GlobeMode で直すもの」の 4）。
- Rumoca の `build_simulation_with_stage_timing_and_lowered_model` は、組み立ての前に lowering の結果（`LoweredSolveModel`）を見せる口を持つ。S012 から、実行に使うパラメータの値をここで読んでいた。session の道は `lower_correlated_for_simulation_with_overrides` を自分で呼ぶ。
- `cargo xtask msl-sim` のワーカーが親に知らせる段は、実行系の外の 3 つ（コンパイル・実行系・比較）までだった。外から止めた 7 本が lowering・初期化・積分のどこで止まったかは分からなかった（同じ節の 5）。

## 決定

### (a) プリフライトは組み立ての lowering の結果で行う

- `run` は、bdf・auto（型付きの道）と session の道（自前の積分器、wasm32 のすべて）では、組み立ての lowering の結果でプリフライトの評価（`SolveRuntime::eval_at`）をする。auto が dopri45 に回し直すときは、BDF の lowering の結果を使う。
- lowering を自分で呼ばないネイティブの `dopri45`（`rk45::simulate`）だけは、実行の前に単独のプリフライトをする（lowering は 2 回のまま）。
- 単独の `preflight` も、組み立てと同じ lowering（`lower_correlated_for_simulation_with_overrides`）を使う。どの道でも同じモデルを評価する。
- lowering の前に失敗したとき（パラメータの上書きの誤りなど）は、プリフライトは何も出さない（S013 までと同じ。実行の側が同じ失敗を診断にする）。
- プリフライトは警告しか出さないので、実行の前に止める判断は要らない（S013 までも止めたことは無い）。

### (b) 実行系の中の段を知らせる

- `globe_mode_exec::run_with_progress(model, settings, on_stage)` を足す。`run` は何もしない `on_stage` で呼ぶ。
- 段は `ExecStage`（`Preflight`・`Lowering`・`Build`・`Initialization`・`Integration`）。bdf・auto・session の道は Lowering → Preflight → Build → Initialization → Integration の順。ネイティブの dopri45 は Preflight → Integration（`rk45::simulate` は lowering から積分までを 1 回で呼ぶので分けられない）。
- `msl-sim` のワーカーは、これを `WorkerStage` の段（`preflight`・`lowering`・`build`・`initialization`・`integration`）として親に送る。外から止めたものの報告に最後の段が出る。

### 選ばなかった案

- 方程式の多いモデルではプリフライトを飛ばす（同じ節の 4 の (b)）: lowering を 1 回にできたので要らない。
- `rumoca-ir-solve` を依存に足して、`lower_for_simulation_with_overrides` の `SolveModel` のまま評価する: 依存が増え、組み立てと違う lowering を評価することになる。
- Rumoca の `begin_stage`（`ir_solve` などの細かい段の名前）をそのまま知らせる: 名前は Rumoca の内部の都合で、版で変わる。GlobeMode の段に丸める。

## 結果

- `run` の時間（release）: `DoublePendulum` は単独のプリフライトの 0.39 s、lowering で止まる `Fluid.Examples.HeatingSystem` は 4.26 s が要らなくなった。`FullRobot` は `msl-sim` で 111.6 s → 57.9 s（空いた機械で続けて回した）。
- 外から止めた例題の段が分かるようになった。S013 で外から止めた 7 本は 7 本とも lowering の段で止まっていた（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の節「S014 で直したもの」）。
- `msl:sim`（[msl-sim.md](../msl-sim.md)）: 356 本のうち 307 本が実行でき、300 / 306 本（98.0%）が管に収まった（S013 は 308 本・301 / 307 本。P3 の完了条件は満たしたまま）。減った 1 本の `LightningSegmentedTransmissionLine`（S013 で 115 s）は 120 s の外からの停止に変わった。機械の負荷（15 分の平均 10〜12、16 コア）のためで、同じ負荷では S013 の実行ファイルでも外から止まった（空いた機械では S014 のほうが速い）。外から止めた 9 本の最後の段は、S013 の 7 本が 7 本とも lowering、残りは `FullRobot` がプリフライト（lowering が約 118 s で終わった直後）、`LightningSegmentedTransmissionLine` が積分。
- 残したこと: ネイティブの dopri45 のプリフライトは lowering が 2 回のまま。`SimStats::build_seconds` にプリフライトの評価の時間が入る。
