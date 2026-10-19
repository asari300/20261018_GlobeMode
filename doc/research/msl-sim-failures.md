# S011 の MSL の一括実行で止まった 54 本と管から外れた 10 本の原因

確認日はすべて 2026-10-19。Rumoca のソースは rev `276e82bd`（`~/.cargo/git/checkouts/rumoca-90ebcedec99354b7/276e82b/`、読むだけ）の行番号。GlobeMode のソースはブランチ `feature/s012-wasm-solvers` の時点の行番号。

## 目的

S011 の `cargo xtask msl-sim`（報告は [msl-sim.md](../msl-sim.md)）で、対象 357 本のうち 54 本が終了時刻まで進まず、参照結果と比べた 302 本のうち 10 本が管（幅 0.002）から外れた。1 本ずつ原因を確かめ、GlobeMode の側で直せるもの（失敗の分類、予算、設定、ソルバの選び方、比べ方）と、Rumoca の側の問題（上流に報告する候補）に分ける。コードは直さず、直し方の案だけを書く。

## 方法

- S011 の結果 `target/msl-sim.json` と `target/msl-funnel.json`（S6〜S10 の数）を突き合わせた。
- `msl-sim.json` には Rumoca の文言（診断の補足 `rumoca: …`）が残らない。そこで止まった 47 本（外から停止の 7 本を除く）を `globe simulate` で 1 本ずつ回し直し、補足を読んだ。

  ```bash
  cargo build --release -p globe-mode-cli -p xtask
  ./target/release/globe simulate -L data/msl -m <モデル> --json [-o 結果.csv] [--solver dopri45] [--tolerance 1e-7] [--timeout 800]
  ```

- `globe simulate` は `--timeout` を省くと壁時計の予算が無い（`globe-mode-exec/src/lib.rs` の既定 `max_wall_seconds: None`）。最初の回し直しは予算なし・外から 170 s で kill、その後に必要なものを `--solver dopri45`、`--tolerance`、`--timeout 300〜800` で回した。
- 管の判定は、`globe-mode-sim/src/compare.rs` の冒頭の記述（δt = tol·max(t_N − t_0, |t_0|, 1e-12)、δy = tol·max(y_max − y_min, |y_min|, 0.001)、窓の中の折れ線の最大・最小 ± δy）を Python で書き直した使い捨てのスクリプトで行った。S011 の CSV（`target/msl-sim-results/`）に当てて、`DoublePendulumInitTip` の外れ（`revolute1.phi`・3 点・t = 4.83・はみ出し 4.857e-3）が Rust の結果と一致することを確かめてから使った。
- Rumoca の CLI（`data/rumoca/rumoca-linux-x86_64`、v0.10.2）では次のように回した。

  ```bash
  data/rumoca/rumoca-linux-x86_64 sim "data/msl/Modelica 4.1.0/package.mo" -m <モデル> --source-root data/msl -o out.html
  ```

- Rumoca の基準値 `crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json` の `runtime_ratio_cohort_models`（Rumoca と OMC の両方が実行に成功した 216 本 ＝ `sim_ok`）と、`msl_trace_compare_exclusions.json`（比較から外した理由の一覧）と照らした。
- 注意: 調査の間、同じ機械で親が並行してビルドしていた（load average 15〜21）。秒数は S011 より大きく出ている。秒数の比較は目安にとどめる。

## 結論の要約

| 分類 | 本数 | GlobeMode で通せる見込み |
| --- | ---: | ---: |
| Rumoca の構造解析（インデックスの低減が足りない、EL005） | 18 | 0 |
| Rumoca の構造解析（その他）・Solve IR への変換・組み立て | 6 | 0（うち 1 本は対象から外すべき） |
| Rumoca の方向微分（ヤコビアン）が作れない | 2 | 0 |
| 初期化の失敗 | 6 | 0 |
| 積分の失敗（BDF） | 6 | 1（dopri45 に切り替えると通る） |
| 時間切れ（予算 30 s） | 5 | 1（予算を延ばすと通る） |
| 実行系の内部エラー（サンプラの端点の契約） | 4 | 3（dopri45 で 2、許容誤差の下限で 1） |
| 外から停止（Solve IR への変換などが終わらない） | 7 | 0 |
| **止まった（計）** | **54** | **5** |
| 管: イベントの行の扱い（同じ瞬間の中間の行・終了時刻のイベント） | 2 | 2（比べ方を直す） |
| 管: 許容誤差（参照結果は annotation の 1/10） | 2 | 2（実験の扱い。他への影響は【未確認】） |
| 管: モデル・参照結果・比較の限界、Rumoca の意味の違い | 6 | 0 |
| **管から外れた（計）** | **10** | **4** |

- **Rumoca の CLI やハーネスでは通るのに GlobeMode では止まるものは、見つからなかった。** 54 本はどれも Rumoca の基準値の `sim_ok`（216 本）に入っていない。CLI で回した 3 本（`FilterWithDifferentiation`、`DrumBoiler`、`MomentumBalanceFittings`）も、同じ EL005 か、240 s で終わらなかった。
- 止まった 54 本のうち 46 本は、Rumoca の側の問題（構造解析・変換・初期化・積分・Solve IR の変換の遅さ）。残る 8 本のうち 5 本は GlobeMode の設定（ソルバの切り替え、許容誤差の下限、予算）で終了時刻まで進み、管にも収まった。1 本（`SpacePhasorLimiter`）は例題ではないので、対象から外すのが正しい。

## 止まった 54 本

S7 の欄は `msl-funnel.json` の「構造インデックス / 状態のうち動的なもの」（Pantelides 法の見込み）。「再現」は 2026-10-19 に回し直した結果。

### 実行系が未対応（GM-U0004、26 本）

#### 構造解析で止まった: インデックスの低減が足りない（18 本）

Rumoca の文言はどれも `[StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: <m> matched out of <n> equations and <n> unknowns`（`crates/rumoca-phase-structural/src/types.rs:145-147`、`crates/rumoca-phase-solve/src/error.rs:102`）。

| # | モデル | 一致 / 方程式 | S7（インデックス・状態） |
| ---: | --- | --- | --- |
| 1 | `Blocks.Examples.FilterWithDifferentiation` | 14 / 15 | 4・6→3 |
| 2 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 377 / 392 | 3・23→6 |
| 3 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 899 / 913 | 3・25→9 |
| 4 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 644 / 659 | 3・19→2 |
| 5 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 469 / 474 | 2・6→1 |
| 6 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 899 / 913 | 3・25→9 |
| 7 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 1027 / 1034 | 2・14→7 |
| 8 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 569 / 573 | 2・8→4 |
| 9 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 768 / 772 | 2・6→2 |
| 10 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 1760 / 1767 | 2・41→34 |
| 11 | `Fluid.Examples.PumpingSystem` | 277 / 278 | 2・6→5 |
| 12 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 126 / 128 | 2・2→0 |
| 13 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 233 / 242 | 3・11→0 |
| 14 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 147 / 151 | 3・5→0 |
| 15 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 287 / 290 | 2・8→5 |
| 16 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 3771 / 3844 | 3・94→2 |
| 17 | `Media.Examples.SimpleLiquidWater` | 31 / 34 | 2・4→1 |
| 18 | `Media.Examples.WaterIF97` | 28 / 30 | 2・5→3 |

- **原因**: 18 本すべてで、GlobeMode の S6 が「インデックスの低減が要る」（`index_reduction_needed`）、S7 がインデックス 2 以上と見込んでいる。Rumoca の構造の変換は「状態が微分可能な拘束で直接決まるとき、その状態を降格して微分を代入する」形だけを扱う（`crates/rumoca-phase-structural/src/dae_transform.rs:1-7`）。微分の連鎖（`FilterWithDifferentiation` は 14 本を最大 3 回微分）や、磁束・電流の拘束が複数の状態を同時に縛る形（変圧器、FluxTubes）は扱えず、一致の数が足りないまま構造の証明に失敗する。
- **根拠**: `rumoca sim … -m Modelica.Blocks.Examples.FilterWithDifferentiation --source-root data/msl` も同じ EL005（14 / 15）で止まる（CLI で確認）。Rumoca の基準値でも 18 本はどれも `sim_ok` に無い。
- **GlobeMode で直せるか**: 直せない（Phase A は Rumoca の構造の変換をそのまま使う。ADR-0013）。S7 の見込みと Rumoca の失敗がよく対応しているので、S7 の GM-I7001 / GM-I7002 で「実行系が未対応の見込み」を前もって知らせることはできる（ただし S011 で終了時刻まで進んだ 303 本のうち 118 本もインデックスの低減が要ったので、S7 だけで止める判断はできない）。
- **上流**: 報告の候補（下の「上流に報告する候補」の 1）。

#### 構造解析・Solve IR への変換・組み立てで止まった（6 本）

| # | モデル | Rumoca の文言（要旨） | 原因 |
| ---: | --- | --- | --- |
| 19 | `Fluid.Examples.HeatingSystem` | `[StructuralAnalysis] … independent state selection: linear solve requires the Solve tensor execution kernel` | 状態選択で線形の解法が要るが、Rumoca の Solve の実行の核がそれを持たない（Rumoca の未対応） |
| 20 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | `[SolveLowering] … [EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid` | 関数呼び出しの引数・スロットの形を Solve IR が受け付けない（`crates/rumoca-ir-solve/src/typed_program/program/construction_error.rs:77`）。乱数の関数（配列の状態を返す） |
| 21 | `Utilities.Examples.WriteRealMatrixToFile` | 同上 | 外部関数（ファイルへの書き出し）の呼び出しの形。状態も方程式もほぼ無い例題 |
| 22 | `Electrical.Polyphase.Examples.TransformerYD` | `[SolveLowering] … DAE system is not computable: affine derivative coefficient is not compile-time numeric` | 微分の係数がコンパイル時の数でない（パラメータの式）と Solve IR に落とせない（Rumoca の未対応） |
| 23 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | `[BackendBuild] … [EX002] input u1 has neither a checked default nor a runtime value` | **例題ではない**。`ExampleUtilities` の中の block で、最上位の入力 `u1`・`u2` に値が無く、experiment annotation も無い。S8 も GM-W8003 を出している |
| 24 | `Media.Examples.TwoPhaseWater.BaseProperties` | S011 では「実行系の組み立てで止まった」（40 s）。回し直しは 170 s で kill され、文言は取れなかった【未確認】 | 実行系の組み立て（Rumoca）。詳細は【未確認】 |

- **GlobeMode で直せるか**: #23 だけ直せる。`msl-sim` の対象の選び方で、experiment annotation が無く、値の無い最上位の入力がある（S8 の GM-W8003 か、DAE の入力の数）クラスを「例題でない」として実行しない（partial と同じ扱い）。他は Rumoca の側。

#### 方向微分（ヤコビアン）が作れない（2 本）

| # | モデル | Rumoca の文言 | 再現 |
| ---: | --- | --- | --- |
| 25 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | `[Integration] directional derivative is unavailable: algebraic projection produced a non-finite sensitivity for y[432]` | `--solver auto` でも同じ（積分の途中の失敗なので Auto は切り替えない）。`--solver dopri45 --timeout 300` は 300 s の予算を守らず、外から 400 s で kill |
| 26 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | `[Integration] directional derivative is unavailable: algebraic projection sensitivity matrix is singular` | `--solver dopri45` と `--solver auto` は終了時刻まで進むが、`directCapacity.heatCapacitor.T` が 293.15 K のまま動かず、管から外れる（922 点、はみ出し 47.6 K。参照は t = 0.1 で 309.8 K）。**黙って誤った結果になる** |

- 文言の出どころは `crates/rumoca-solver/src/runtime/projection.rs:592`、`…/projection/seed_linearization.rs:264`。
- **GlobeMode で直せるか**: 直せない。分類の文言「方向微分（ヤコビアン）の準備で止まった」は、積分の途中（`[Integration]`）の失敗にも出るので、「積分の途中で方向微分が作れなくなった」と分けるとよい（`globe-mode-exec/src/rumoca.rs:642-668` の `unsupported_where` で、`Phase::Integration` のときの文言を変える）。

### 初期化に失敗した（GM-E11001、6 本）

| # | モデル | Rumoca の文言（要旨） | 原因 |
| ---: | --- | --- | --- |
| 27 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | `initial variable projection did not satisfy the complete residual system: … owner=none(row reads a coordinate outside the planned initialization unknown space …) value=-1` | 初期化の縮約した射影が、ある行の未知数を持たない（計画の外）。dopri45 でも同じ |
| 28 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 同上（value = -2.52e8） | 同上。dopri45 でも同じ |
| 29 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | S011 では「初期化系が解けない」（93.6 s）。回し直しは 170 s で kill（負荷のため）【未確認】 | OneAxis と同じ型の見込み【未確認】 |
| 30 | `Blocks.Examples.TotalHarmonicDistortion` | `initial homotopy continuation failed at lambda=0 …: non-finite (NaN) value computed for thd1.y` | homotopy の λ = 0 の系で `thd1.y`（`Division` の出力）が NaN。dopri45 でも同じ |
| 31 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | `non-finite (NaN) value computed for openTank.h` | `level(fixed=true, start=0.5)` で、状態は `m` と `H`（start 0）。`h = H/m` が 0/0。fixed の付いた代数変数から状態の初期値を解いていない。S10 のプリフライトも GM-W10002 を出している |
| 32 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | `non-finite (NaN) value computed for cylinder1.h` | TestOpenTank と同じ型（GM-W10002 あり） |

- 文言の出どころは `crates/rumoca-solver/src/runtime/projection/initial_diagnostics.rs:62`。
- **GlobeMode で直せるか**: 直せない（Rumoca の初期化）。#31・#32 は S10 の GM-W10002 が前もって当たっていた（警告の価値の確認になる）。

### 積分に失敗した（GM-E11002、6 本）

| # | モデル | Rumoca の文言（要旨） | `--solver dopri45` |
| ---: | --- | --- | --- |
| 33 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | `algebraic projection did not establish coordinate convergence: … target=T2.C.i value=-3e-8 … scaled_tolerance=1e-10` | 同じ（残差 6e-5） |
| 34 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 同上（`T2.C.i`、3e-8） | 同じ |
| 35 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 同上（`rectifier.diode_p.plug_p.pin[3].i`、4.4e-3） | 同じ（7.6） |
| 36 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | `[ES016] the algebraic loop over opAmp.v_out, … reached a fold of V0*v_in<vns: the relation switches its own loop under noEvent … (t=0.027)` | 同じ |
| 37 | `Fluid.Examples.Tanks.TanksWithOverflow` | `[EX004] algebraic block over [overflow.flowModel.m_flows[1], …] is singular in its active branch combination …` | t = 1468.68 でイベントの反復が収束しない（GM-E11003） |
| 38 | `Thermal.FluidHeatFlow.Examples.TwoMass` | `[EX004] algebraic block over [pipe2.flowPort_a.H_flow, pump.flowPort_b.H_flow, …] is singular in its active branch combination` | **終了時刻まで進み、管 5/5** |

- **原因**: #33〜#35 は、代数変数の射影の収束の判定（`crates/rumoca-solver/src/runtime/projection.rs:726`）が、スケールした許容誤差 1e-10 に対して残差 3e-8 で通らない（理想ダイオード・トランジスタの電流）。判定の許容誤差がどこから 1e-10 になるかは【未確認】（`scaling.rs:57` の `scaled_tolerance(tol, scale)` に渡る `tol`）。#36 は `noEvent` の中の比較が代数ループの分岐を切り替える折り返し（`crates/rumoca-ir-solve/src/model.rs:77`）で、Rumoca は意図して止める。#37・#38 は、分岐の組み合わせによって代数ブロックが特異になる。#38 は dopri45 では通るので、BDF の刻みの取り方で特異な組み合わせに入る【未確認】。
- **GlobeMode で直せるか**: #38 は直せる（下の「GlobeMode で直すもの」の 1）。他は Rumoca の側。

### 時間切れ（GM-E11005、5 本）

| # | モデル | 予算 800 s（bdf） | `--solver dopri45 --timeout 300` |
| ---: | --- | --- | --- |
| 39 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 時間切れ | 予算を守らず、外から 400 s で kill |
| 40 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 時間切れ | 時間切れ |
| 41 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 時間切れ | 予算を守らず、外から 400 s で kill |
| 42 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 161 s で GM-U0005（下の #47 と同じサンプラの契約違反: `state 1 is 31.2994, but the checked point carries 31.2995`） | 時間切れ |
| 43 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | **終了時刻まで（実行 102〜152 s）。管 25/25** | - |

- **原因**: サイリスタ・ダイオードの切り替えのイベントが多く（50 Hz で数秒〜24 s）、Rumoca のイベントの処理が遅い。#43 だけは予算を延ばせば通る。#39・#41 の dopri45 では、協調型の時間切れが積分器の 1 歩の内側やイベントの処理の中で見られないため、予算を超えても止まらない（`crates/rumoca-solver/src/fmi_me/session.rs:486,1414` でしか見ない）。
- **GlobeMode で直せるか**: #43 は直せる（下の「GlobeMode で直すもの」の 3）。他は Rumoca の側（速さ）。

### 実行系の内部エラー（GM-U0005、4 本）

文言はどれも `accepted-interval left endpoint sampler state <i> is <a>, but the checked point carries <b>; the disagreement exceeds <bound>`。Rumoca は受け入れた 1 歩ごとに、積分器の連続な補間（サンプラ）が歩の端の点と一致するかを確かめ（`crates/rumoca-solver/src/fmi_me/root.rs:340-358` の `accept_step`）、差が `max(atol, rtol·scale)`（`root.rs:156-160`）を超えると契約違反にする（`root.rs:177-207`）。

| # | モデル | 差 / 上限 | 再現 |
| ---: | --- | --- | --- |
| 44 | `Electrical.Analog.Examples.SimpleTriacCircuit` | 1.0e-12 / 1e-12（experiment の Tolerance = 1e-12） | **`--tolerance 1e-9` と `1e-10` で終了時刻まで進み、管 3/3** |
| 45 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 1.8e-5 / 1e-6 | dopri45 は 300 s で時間切れ |
| 46 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | 4.9e-6 / 1e-6 | **dopri45 で終了時刻まで進み（27 s）、管 19/19** |
| 47 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | 1.8e-4 / 1.2e-5 | **dopri45 で終了時刻まで進み（27 s）、管 4/4** |

- **原因**: #44 は丸めの誤差（相対 1e-12）が上限ちょうどを超えたもの。上限に丸めの床が無い。#45〜#47 は差が上限の 5〜15 倍あり、イベントの後の BDF の補間（履歴）が端の点と食い違う【未確認: どのイベントの後かは確かめていない】。同じ違反は管の外れの調査でも出た（`DemonstrateSignalExtrema` を `--tolerance 1e-8` で回すと GM-U0005）。
- **GlobeMode で直せるか**: #44・#46・#47 は直せる（下の 1・2）。根本は Rumoca の側（上流の候補 2）。

### 外から停止（120 s で応答が無い、7 本）

| # | モデル | 再現 |
| ---: | --- | --- |
| 48 | `Fluid.Examples.DrumBoiler.DrumBoiler` | `--timeout 60` でも 400 s で kill。Rumoca の CLI も 240 s で終わらない（"Simulating … to t=5400..." の後で止まる） |
| 49 | `Fluid.Examples.Explanatory.MeasuringTemperature` | `--timeout 60` でも 400 s で kill |
| 50 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | `--timeout 60` でも 400 s で kill。Rumoca の CLI も 240 s で終わらない |
| 51 | `Fluid.Examples.InverseParameterization` | `--timeout 60` でも 400 s で kill（S8 は GM-W8003 を出している） |
| 52 | `Fluid.Examples.NonCircularPipes` | `--timeout 60` でも 400 s で kill |
| 53 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | `--timeout 60` でも 400 s で kill（方程式 11107 本、S7 は 4669 本を微分） |
| 54 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 400 s で kill（方程式 9001 本） |

- **原因**: 予算（協調型）が見られない所で止まっている。`DrumBoiler` を 60 s 回したところで `perf record` を 8 s 取ると、上位の関数は `rumoca_phase_solve::lower::scalar::ScalarCompiler::program`・`materialize_activation_guards`、`rumoca_ir_solve::linear_op::LinearOp` と `FoldTensorNode` の複製だった（`perf report` のアドレスを `addr2line -f -C -e target/release/globe` で引いた）。つまり **Solve IR への変換（lowering）が終わらない**。lowering は予算の外にある（Rumoca の予算は `advance_to` と根探索でだけ見る）。Rumoca の MSL のハーネスは lowering（`ir_solve` の段）に別の監視 10 s を掛けて止める（`crates/rumoca-test-msl/src/bin/rumoca-sim-worker.rs:63-66,700-712`）ので、Rumoca の基準値では「変換が遅すぎる」として数えられている見込み【未確認】。DrumBoiler 以外の 6 本でどの段かは確かめていない【未確認】。
- **GlobeMode の側の事情**: GlobeMode は実行の前に S10 のプリフライトで `rumoca_sim::eval_dae_at` を呼ぶ（`globe-mode-exec/src/lib.rs:96`）。`eval_dae_at` は中で `lower_dae_for_simulation` を呼ぶ（`crates/rumoca-sim/src/solve_lowering/probe.rs:47`）ので、**lowering を 2 回している**（プリフライトと `build_simulation`）。lowering が遅いモデルでは時間が 2 倍になる。止まっていたのがプリフライトの側か組み立ての側かは、`perf` の呼び出しの連鎖が取れず（フレームポインタが無い）確かめられなかった【未確認】。
- **GlobeMode で直せるか**: 通るようにはならない（Rumoca の CLI でも終わらない）。分類と時間は直せる（下の 4・5）。

## 管から外れた 10 本

| # | モデル | 外れた信号と時刻 | 原因 | `--tolerance 1e-7` | GlobeMode で直せるか |
| ---: | --- | --- | --- | --- | --- |
| 1 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | 電流・電圧 6 個、各 1 点、t = 0.04525 | 同じ瞬間の 2 つの時間イベントの間の行。Rumoca は PWM の切り替えを t = 0.04524999999999976 と 0.04525000000000025（浮動小数点の和の誤差）で別々に処理し、その間の出力の格子 t = 0.04525 に中間の値（電流 −0.667。参照の跳びは +0.333 → −0.333）を書く | - | **直せる**（比べ方 6）。中間の行を除くと 24/24 |
| 2 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | `voltageSensor.v[1]`・`v[2]`、各 1 点、t = 0.1（終了時刻） | 終了時刻のイベント。Rumoca は t = 0.09999999999999999 でイベントを処理し、t = 0.1 の行に後の値（16.67）を書く。参照（Dymola）は終了時刻のイベントを処理しない（t = 0.1 で 1.5e-6） | - | **直せる**（比べ方 6）。終了時刻の行を最初の 1 行にすると 47/47 |
| 3 | `Blocks.Examples.DemonstrateSignalExtrema` | `signalExtrema1.y_min`・`y_max`、t = 0.643〜 | 100 Hz の標本化で、周波数が 10〜100 Hz で変わる正弦波の極値を拾う（MSL の説明に「正確には見つからない」とある）。位相のわずかな違いで拾う値が変わる（y_max 4.649 と 4.633） | **管 8/8** | 許容誤差を参照に合わせれば収まる（7）。1e-8 では GM-U0005 |
| 4 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | `dcPowerSupply.i`・`bufferCapacitor.i`、t = 0.5215〜 | 短い電流のパルスの高さ（−1.25 と −0.93）。ダイオードの曲がり目（Rumoca の除外の一覧にも `diode.off` が model_issue として載る。`msl_trace_compare_exclusions.json:111`） | **管 4/4**（1e-8 も 4/4） | 許容誤差を参照に合わせれば収まる（7） |
| 5 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | `revolute1.phi`、3 点、t = 4.83 | 二重振り子（カオス的）で、ずれが積み重なり終わり際に 0.0049 rad はみ出す | 2/4（悪くなる） | 直せない（解が許容誤差に敏感。参照の側も同じ） |
| 6 | `Blocks.Examples.DemoSignalCharacteristic` | `harmonic.mean1.y`、t = 0.0215〜 | 解析解が 0 の信号（第 1 高調波の cos の係数）。参照は −1.5e-4、GlobeMode は −9.6e-5（積分の残差）。δy は床 0.001 で決まり 2e-6 しか無い。Rumoca の除外の一覧も comparator_limitation とする（`msl_trace_compare_exclusions.json:5`） | 4/5 | 直せない（参照の側・比べ方の限界）。既知の外れの一覧に理由付きで載せるのがよい（8） |
| 7 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 3 個すべて、t = 0.032〜 | `noEvent` で包んだ飽和の弛張発振。周期がソルバに依存して位相がずれる。Rumoca の除外の一覧も model_issue（`msl_trace_compare_exclusions.json:136`。Rumoca の半周期 0.0472 s、OMC 0.0495 s） | 0/3（1e-8 も 0/3） | 直せない（モデルの性質）。一覧に載せる（8） |
| 8 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | `sample_angle.y` t = 1.0、`rotationalClock.direction` t = 1.3976 | t = 1 で、状態イベント（角度の変化が 1 rad に達する。sin が 0 を横切る点）と時間イベント（`trigger_interval` が 1 → 3）が数学的に同時に起こる。GlobeMode（Rumoca）は t = 1 で刻みを出し、参照は出さない。その後の向きの切り替えも 1.3976 と 1.4167 にずれる | - | 直せない（同時のイベントの順序。Rumoca の意味） |
| 9 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | `rotational_clock_1.direction` など 4 個、t = 0.056〜 | #8 と同じ t = 1 の件に加え、`ConjunctiveClock`（「すべての入力の clock が少なくとも 1 回刻んだら刻む」）が参照では t = 0.0485 で刻み、GlobeMode では刻まない（最初の刻みの数え方の違い）。Rumoca の基準値の `unexcepted_non_high_models` にこのモデルだけが載っている | - | 直せない（Rumoca の clock の意味） |
| 10 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | `vH01.y_arg`・`iH01.y_arg`、t = 0.26〜 | 第 1 高調波の位相（`Harmonic` の y_arg）が、窓の更新の時刻 t = 0.26 で 2.796（参照）と 2.772 に分かれる。サイリスタの点弧の時刻のずれが窓の平均に入る見込み【未確認】 | 9/11（1e-8 は 4/11） | 直せない見込み。原因は【未確認】 |

## GlobeMode で直すもの

見込みの効果は、この調査で回し直して確かめた本数。全 357 本での副作用は【未確認】（全体の回し直しはしていない）。

1. **BDF の積分の失敗と GM-U0005 で dopri45 に切り替える**（`src/Rust/mod/globe-mode-exec/src/rumoca.rs:288-296` の `run_auto`）。
   - 今の `run_auto` は、組み立て・初期化で方向微分が作れないときだけ RK に切り替える（Rumoca の `select_auto_integrator` と同じ）。これを「BDF が `Phase::Integration` で `Kind::Integration`（GM-E11002）か、サンプラの契約違反（`MeSessionError::Contract`、GM-U0005）で失敗したら、`run_rk` でもう一度回す」に広げる。切り替えたことは情報の診断（例: GM-I10003 に倣った新しい GM-I）で知らせる。
   - `msl-sim` の既定のソルバを `auto` にする（`src/Rust/xtask/src/main.rs` の `MslSim` の `--solver`、または `globe-mode-sim/src/settings.rs` の既定）。
   - 見込み: **+3 本**（`TwoMass` 5/5、`ThyristorCenterTap2mPulse_RLV_Characteristic` 19/19、`ChopperBuckBoost_DutyCycle` 4/4。どれも管に収まる）。
   - 注意: 方向微分が作れない失敗（GM-U0004）では切り替えない。`GenerationOfFMUs` は RK で終了時刻まで進むが、温度が動かない誤った結果になる（管 2/3）。RK に切り替えた結果は、参照結果と比べるまで信用しない。
2. **許容誤差に下限を設ける**（`src/Rust/mod/globe-mode-sim/src/settings.rs:65-70` の Tolerance の決め方）。
   - 例: 1e-10 より小さい Tolerance は 1e-10 にし、情報の診断で知らせる（Dymola の参照は 1e-13 で作っているが、Rumoca のサンプラの契約は丸めの床を持たないので、1e-12 では丸めで違反する）。
   - 見込み: **+1 本**（`SimpleTriacCircuit` は 1e-9・1e-10 のどちらでも 3/3）。MSL で Tolerance < 1e-10 の例題の本数は数えていない【未確認】。
3. **壁時計の予算を延ばす**（`src/Rust/xtask/src/main.rs:175-180` の `--wall-seconds` の既定 30 と `--kill-seconds` 120）。
   - 例: 予算 180 s、kill は予算 + 60 s。または「予算を超えたものだけ、予算を延ばして 1 回だけ回し直す」。
   - 見込み: **+1 本**（`ThyristorBridge2mPulse_RLV_Characteristic` は 102〜152 s で終了時刻まで進み、25/25）。残る 4 本は 800 s でも終わらないので、全体の時間は 4 × 150 s ほど延びる。
4. **プリフライトの lowering を 1 回にする**（`src/Rust/mod/globe-mode-exec/src/lib.rs:96`、`rumoca.rs:68-75`）。
   - `eval_dae_at` が lowering をやり直す（`probe.rs:47`）ので、lowering の遅いモデルで時間が 2 倍になる。案: (a) 組み立て（`build_simulation`）を先に行い、その lowering を使ってプリフライトの評価をする道が Rumoca の公開の API にあるか確かめる【未確認】、(b) 無ければ、プリフライトに時間の上限を設けるか、方程式の数の大きいモデル（例: 2000 本以上）では飛ばして GM-I で知らせる。
   - 見込み: 通る本数は増えない（7 本は Rumoca の CLI でも終わらない）。S011 の「実行系の準備」139.75 s の一部が減る【未確認】。
5. **ワーカーに段の名前を報告させ、外から停止を分類する**（`src/Rust/xtask/src/sim.rs` のワーカーの通信）。
   - ワーカーが段（コンパイル・プリフライト・lowering と組み立て・初期化・積分）に入るたびに 1 行送り、親が kill したときに最後の段を記録する。lowering で止まったものは「実行系の変換が終わらない（GM-U）」、積分で止まったものは「時間切れ（予算の外）」に分けられる。Rumoca のハーネスの段ごとの監視（`rumoca-sim-worker.rs:700-712`）と同じ考え方。
   - あわせて、`msl-sim.json` と報告に診断の補足（`rumoca: …` の文言）を残す（`sim.rs` の `message` だけでなく `notes` も）。今回の調査は 47 本を回し直さないと原因が読めなかった。
6. **比べるときにイベントの行を整える**（`src/Rust/mod/globe-mode-sim/src/compare.rs`、または `globe-mode-exec/src/rumoca.rs:312` の `trajectory`）。
   - (a) 時刻の差が `1e-12·max(1, |t|)` 以内の行を同じ瞬間とみなし、3 行以上あれば最初（左極限）と最後（右極限）だけを残す。(b) 終了時刻の瞬間の行は最初の 1 行だけを残す（終了時刻のイベントの後の値を比べない）。
   - 結果の CSV そのものは変えず、比べる前の正規化にするのがよい（中間の行は Rumoca の出力として意味がある）。
   - 見込み: **管 +2 本**（`PolyphaseTwoLevel_R` 24/24、`PolyphaseTwoLevel_RL` 47/47。試験側の点を減らすだけなので、他のモデルの外れを増やすことは無い。ただし「区間を覆わない」の判定は確かめていない【未確認】）。
7. **参照結果と同じ許容誤差で回す選択肢**（`src/Rust/xtask/src/main.rs` の `MslSim` に `--tolerance-scale` を足す）。
   - MAP-LIB の参照は experiment annotation の 1/10 で作られている（[openmodelica-dymola.md](openmodelica-dymola.md) の節「S011 で調べた参照結果と比較の方法」）。1/10 で回すと `DemonstrateSignalExtrema` 8/8、`DemoPowerSupplyWithBuffer` 4/4 になった（**管 +2 本**）。一方で `DoublePendulumInitTip` は 3/4 → 2/4、`DemonstrateSignalExtrema` は 1/100 で GM-U0005 になる。今通っている 292 本への影響と時間の増え方は【未確認】なので、既定は変えずに実験の選択肢にとどめる。
8. **既知の外れの一覧**（`src/Modelica/` に、`msl-known-unsupported.txt` と同じ形の一覧を足す案）。
   - `DemoSignalCharacteristic`（解析解 0 の信号）、`Multivibrator`（noEvent の弛張発振）を、理由と出典（Rumoca の `msl_trace_compare_exclusions.json`）付きで載せ、報告で「既知の外れ」と分ける。合否の数は変えない。
9. **対象から例題でないものを外す**（`src/Rust/xtask/src/sim.rs` の対象の選び方）。
   - experiment annotation が無く、値の無い最上位の入力があるクラス（`SpacePhasorLimiter`）を partial と同じく「実行しない」にする。見込み: 対象 357 → 356、止まった 54 → 53。
10. **分類の文言**（`src/Rust/mod/globe-mode-exec/src/rumoca.rs:642-668`）。
    - 積分の途中の方向微分の失敗（`[Integration] directional derivative is unavailable`）を「準備で止まった」と書かない。
    - サンプラの契約違反（GM-U0005）は「内部エラー」だが、許容誤差やソルバで変わるので、補足に「`--solver dopri45` や許容誤差を変えると進むことがある」と書く。

1〜3・9 を合わせると、止まった 54 本は 48 本（対象 356 本のうち実行できた 308 本）、6・7 を合わせると管の外れは 10 本から 6 本になる見込み（全体の回し直しで確かめる必要がある）。

## 上流に報告する候補

どれも MSL 4.1.0、Rumoca v0.10.2（rev `276e82bd`）、`data/rumoca/rumoca-linux-x86_64 sim "data/msl/Modelica 4.1.0/package.mo" -m <モデル> --source-root data/msl` で再現する（CLI で確かめたのは 1・3 だけ。他は GlobeMode から Rumoca のライブラリを呼んで確かめた）。

1. **インデックス 2 以上の DAE で EL005（structurally singular）**: `Modelica.Blocks.Examples.FilterWithDifferentiation`（15 本で一番小さい。14 / 15）。他に変圧器・誘導機・FluxTubes・Media の 17 本。
2. **受け入れた歩の端の点とサンプラの食い違い（`accepted-interval left endpoint sampler …`）**: (a) `Modelica.Electrical.Analog.Examples.SimpleTriacCircuit`（Tolerance = 1e-12。差 1.0e-12 が上限 1e-12 を超える。上限に丸めの床が無い）、(b) `Modelica.Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle`（BDF、差 1.8e-4 / 上限 1.2e-5。RK では通る）。
3. **Solve IR への変換が終わらない、かつ予算の外**: `Modelica.Fluid.Examples.DrumBoiler.DrumBoiler`、`Modelica.Fluid.Examples.Explanatory.MomentumBalanceFittings`（CLI で 240 s 以上。`ScalarCompiler` と `LinearOp` の複製が上位）。
4. **理想スイッチの電流で代数の射影の収束の判定が通らない**: `Modelica.Electrical.Analog.Examples.HeatingNPN_NORGate`（`T2.C.i` の残差 3e-8 / スケールした許容誤差 1e-10）。
5. **fixed = true の代数変数から状態の初期値を解かない**: `Modelica.Thermal.FluidHeatFlow.Examples.TestOpenTank`（`level(fixed=true, start=0.5)`、状態 `m`・`H` が 0 のまま `h = H/m` が NaN）。
6. **RK で黙って誤った結果になる**: `Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs` を `--solver rk-like` で回すと `directCapacity.heatCapacitor.T` が 293.15 K のまま（BDF では方向微分が作れずに止まる）。
7. **RK の道で壁時計の予算が守られない**: `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_YDarc` を RK、`max_wall_seconds = 300` で回すと 400 s を過ぎても返らない。
8. **関数呼び出しの Solve IR の契約（`pure-call argument or slot interface is invalid`）**: `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star`、`Modelica.Utilities.Examples.WriteRealMatrixToFile`。
9. **同時のイベントと clock の意味**: `Modelica.Clocked.Examples.Elementary.ClockSignals.RotationalSample`（t = 1 の状態イベントと時間イベントの同時）、`LogicalSample`（`ConjunctiveClock` の最初の刻み）。Rumoca 自身の基準値も `LogicalSample` を `unexcepted_non_high_models` に載せている。

## 確かめられなかったこと

- `FullRobot`、`TwoPhaseWater.BaseProperties` の文言（回し直しは負荷のため 170 s で kill。S011 の分類のまま）。
- 外から停止の 7 本のうち `DrumBoiler` 以外がどの段で止まるか、`DrumBoiler` でもプリフライトの側か組み立ての側か。
- 射影の判定の許容誤差 1e-10 の出どころ、GM-U0005 の食い違いがどのイベントの後に起こるか、`Dimmer_R` の位相のずれの原因、`TwoMass` が BDF でだけ特異な組み合わせに入る理由。
- 1・2・3・6・7 の変更を全 357 本に当てたときの副作用と時間（全体の回し直しはしていない）。
- OpenModelica（omc 1.27.1）での回し直しはしていない。

## 出典

- GlobeMode の S011 の報告: [msl-sim.md](../msl-sim.md)、[msl-funnel.md](../msl-funnel.md)、[ADR-0013](../adr/0013-exec-backend.md)、[rumoca.md](rumoca.md) の節「S011 で調べた実行系（v0.10.2）」、[openmodelica-dymola.md](openmodelica-dymola.md) の節「S011 で調べた参照結果と比較の方法」（2026-10-19）。
- Rumoca rev `276e82bd` のソース（2026-10-19）: `crates/rumoca-phase-structural/src/dae_transform.rs:1-21`、`crates/rumoca-phase-structural/src/types.rs:145-147`、`crates/rumoca-phase-solve/src/error.rs:102`、`crates/rumoca-ir-solve/src/model.rs:77`、`crates/rumoca-ir-solve/src/typed_program/program/construction_error.rs:77`、`crates/rumoca-solver/src/runtime/projection.rs:592,726`、`crates/rumoca-solver/src/runtime/projection/scaling.rs:57-64`、`crates/rumoca-solver/src/runtime/projection/seed_linearization.rs:264`、`crates/rumoca-solver/src/runtime/projection/initial_diagnostics.rs:62`、`crates/rumoca-solver/src/fmi_me/root.rs:156-160,177-207,340-358`、`crates/rumoca-solver/src/fmi_me/session.rs:486,1414`、`crates/rumoca-sim/src/solve_lowering/probe.rs:47`、`crates/rumoca-test-msl/src/bin/rumoca-sim-worker.rs:63-66,700-712`。
- Rumoca の基準値と除外の一覧: [msl_quality_baseline.json](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json)、[msl_trace_compare_exclusions.json](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_trace_compare_exclusions.json)（2026-10-19）。
- MSL 4.1.0 のソース（`data/msl/Modelica 4.1.0/`）: `Blocks/package.mo:1468-1508`（DemonstrateSignalExtrema）、`Blocks/Math.mo` の `TotalHarmonicDistortion`、`Thermal/FluidHeatFlow/Components/OpenTank.mo`、`Thermal/FluidHeatFlow/Examples/TestOpenTank.mo:7-15`、`Clocked/ClockSignals/Clocks/Rotational/RotationalClock.mo`、`Clocked/Examples/Elementary/ClockSignals/LogicalSample.mo`、`Magnetic/QuasiStatic/FundamentalWave/Examples/ExampleUtilities/SpacePhasorLimiter.mo`（2026-10-19）。
- 参照結果: MAP-LIB_ReferenceResults のブランチ `v4.1.0`（`data/msl-ref/`、commit `dac67829004d`）。
- 実行したコマンド: 「方法」の節の `globe simulate`（`--solver dopri45`・`auto`、`--tolerance 1e-7`・`1e-8`・`1e-9`・`1e-10`、`--timeout 60`・`300`・`800`）、Rumoca の CLI の `sim`、`perf record -F 199 -g -p <pid>` と `addr2line`（2026-10-19）。

## S012 で直したもの

上の「GlobeMode で直すもの」のうち、1〜3・5・6・8 と 9・10 を S012 で直した（確認日 2026-10-19。設計は [ADR-0014](../adr/0014-session-route-wasm.md)）。全 357 本での効果は `mise run msl:sim` の回し直し（[msl-sim.md](../msl-sim.md)）で確かめた: 対象 356 本（partial の 12 本と例題でない `SpacePhasorLimiter` を除く）のうち **309 本が終了時刻まで進み**（S011 は 303 本、+6）、参照結果と比べた 308 本のうち **299 本（97.1%）が管に収まった**（S011 は 292 / 302 本、+7。信号の数で 1753 / 1772）。止まった 47 本（S011 は 54 本）は、実行系が未対応（GM-U0004）24・初期化 6・時間切れ 5・積分 4・イベント 1・外から停止 7（どれも実行系の段）。GM-U0005（サンプラの食い違い）は 4 → 0。時間切れの 6 本を予算 180 s で回し直して 1 本が通った。`auto` が dopri45 に切り替えたもの（GM-I11002）は 11 本で、うち 4 本が最後まで進んだ。管から外れた 9 本のうち 2 本は既知の外れ（`src/Modelica/msl-sim-known-deviations.txt`）。新しく実行できた 6 本のうち `GenerationOfFMUs` は、Rumoca と同じ規則（初期化で方向微分が作れない）で RK に切り替わって最後まで進むが、温度が動かず管から外れる（調査の上流への報告の候補 6「RK で黙って誤った結果」）。全体 2945.8 s（49 分。回し直しを含む。S011 は 31 分 52 秒）、ワーカーの最大の常駐メモリ 3.4 GiB（[msl-sim.md](../msl-sim.md)）。 調査の見込み（止まる 54 → 48 本、管の外れ 10 → 6 本）に対し、実際は止まる 47 本、管の外れ 9 本（既知の 2 本を除くと 7 本）。管の外れが見込みより多いのは、`GenerationOfFMUs`（RK で誤った結果）が実行できるようになって外れに加わったのと、`DemonstrateSignalExtrema`・`DemoPowerSupplyWithBuffer`（許容誤差を 1/10 にすると収まるもの。`--tolerance-scale` は入れていない）が残ったため。

| # | 直したこと | 場所 | 1 本ずつ確かめた結果 |
| ---: | --- | --- | --- |
| 1 | `auto` は、BDF が積分に失敗したとき（`GM-E11002`）とサンプラが食い違ったとき（`MeSessionError::Contract` の文言）も dopri45 で回し直し、情報 `GM-I11002` を出す。組み立て・初期化で方向微分が作れないときの切り替え（S011 の規則）も `GM-I11002` で知らせる。積分の途中で方向微分が作れない失敗では切り替えない。ネイティブと wasm32 で同じ判断（`fallback()`）。`GenerationOfFMUs` の失敗は GlobeMode の初期化の段（`check_prepared_initialization`）で起きるので、S011 の規則で RK に切り替わり、終了時刻まで進むが温度が動かず管から外れた（`GM-I11002` は出る。規則の見直しは S013 の候補）。`msl-sim` の既定のソルバを `auto` にした | `globe-mode-exec/src/rumoca.rs`、xtask の `--solver` | `TwoMass` は bdf で `GM-E11002`、auto で dopri45 に切り替わり管 5/5 |
| 2 | 許容誤差が 1e-10 未満なら（指定でも experiment でも）1e-10 に上げ、警告 `GM-W10003` を出す | `globe-mode-sim/src/settings.rs` | `SimpleTriacCircuit`（experiment が 1e-12）が通る見込み（調べもので 1e-9・1e-10 のどちらでも 3/3） |
| 3 | 時間切れ（`GM-E11005`）だけ、予算 180 s で 1 回回し直す（`--retry-wall-seconds 180`、kill は予算 + 60 s） | xtask の `sim.rs`・`main.rs` | `ThyristorBridge2mPulse_RLV_Characteristic` は回し直し 133 s で管 25/25 |
| 5 | ワーカーが段（コンパイル・実行系・比較）を親に知らせ、外から止めたものに `worker_stage` を残す。最初の Error の補足（Rumoca の文言）を最大 4 つ・600 字まで JSON と報告に残す | xtask の `sim.rs` | `DrumBoiler` は実行系の段で外から停止、`FilterWithDifferentiation` は補足に EL005。段は exec の外の 3 つまで（exec の中の段は持ち越し） |
| 6 | 比べる前にイベントの行を整える（`normalize_events`）。`1e-12·max(1, \|t\|)` 以内に続く 3 行以上は最初と最後だけ、終了時刻の瞬間は最初の 1 行だけを残す。結果の CSV は変えない | `globe-mode-sim/src/compare.rs` | S011 の結果を比べ直すと管 292 → 294 本（`PolyphaseTwoLevel_R` 24/24、`_RL` 47/47） |
| 8 | 既知の外れの一覧 `src/Modelica/msl-sim-known-deviations.txt`（`DemoSignalCharacteristic`、`Multivibrator`。Rumoca の `msl_trace_compare_exclusions.json` の 5 行目と 136 行目）を `--known-deviations` で読み、報告で分ける。合否の数は変えない | xtask の `sim.rs`、`src/Modelica/` | `Multivibrator` は既知の外れと表示 |
| 9 | 例題でないもの（funnel.json で S10 に `GM-I10001` があり、S8 の w8003 が真）を partial と同じく実行しない | xtask の `sim.rs` | 今は `SpacePhasorLimiter` だけが当たり、対象外になる |
| 10 | 積分の途中で方向微分が作れなくなった失敗の文言を「積分の途中の方向微分（ヤコビアン）の計算で止まった」にした | `globe-mode-exec/src/rumoca.rs` | — |

- 前回との差は `--baseline-json`（`mise run msl:sim` は前回の `target/msl-sim.json` を渡す）で報告に書く。
- 4（プリフライトの lowering を 1 回にする）と 7（参照結果と同じ許容誤差で回す選択肢）は持ち越した。4 は S012 でプリフライトにパラメータの上書きを反映するため `lower_for_simulation_with_overrides` を使うようにしたが、lowering は 2 回のまま（`DoublePendulum` でプリフライト 0.94 s・組み立て 2.07 s、release）。
- 上流に報告する候補の 9 つは変わらない（報告はユーザーが出す。[roadmap.md](../roadmap.md#次のセッション)）。
