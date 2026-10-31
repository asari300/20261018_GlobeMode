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

下書きは [upstream/README.md](upstream/README.md)（S045）。

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
10. **実行時オブジェクトの組み立てで、巨大な根の条件の構造解析が終わらない**（S016）: `Modelica.Fluid.Examples.DrumBoiler.DrumBoiler`。根の条件 31 本が計 59.7 万演算（1 本あたり約 6.5 万演算）。`SolveRuntime::construct` の `root_condition_rows`（`crates/rumoca-solver/src/runtime/solve_runtime.rs:938`）→ `target_assignment_shapes_with_output_offsets`（`crates/rumoca-eval-solve/src/prepared/construction.rs:68`）→ `crates/rumoca-ir-solve/src/refresh/assignment_shape.rs:100-118` が store の位置ごとに prefix の依存集合を作り直し、先頭の 1 本が 900 s で終わらない。組み立ての Cranelift の JIT（`regalloc2`）も長い。lowering の遅さ（117〜440 s）は `Box<[RegisteredAssertion]>::clone` と `CallScopedActionCollector::insert` が支配的。
11. **初期化方程式を関係式の既定値で解き、関係式が反転しても解き直さない**（S016）: 上の R1（`CriticalDamping` の `initType=SteadyState` が `if v < 0 then 1 else 2` を読む）。t = 0 で `filter.x = 2`、正しくは 1（omc 1.27.1 は 1）。MSL では `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`（`filter.x` が 0.066、参照 563.19）。Rumoca の CLI v0.10.2 でも同じ値。

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

## S013 で直したもの

持ち越した 7（参照結果と同じ許容誤差で回す選択肢）と、S012 で残した `auto` の初期化での切り替えを S013 で扱った（確認日 2026-10-20。設計は [ADR-0015](../adr/0015-auto-and-reference-tolerance.md)）。

| # | 直したこと | 場所 | 全例題での結果（[msl-sim.md](../msl-sim.md)） |
| ---: | --- | --- | --- |
| 1 | `auto` は方向微分が作れない失敗（`GM-U0004`）では、組み立て・初期化でも dopri45 に切り替えない | `globe-mode-exec/src/rumoca.rs` | `GenerationOfFMUs` は誤った結果（管 2/3）を返さず `GM-U0004` で止まる。実行できた 309 → 308 |
| 7 | `cargo xtask msl-sim --tolerance-scale <倍率>`。注釈に Tolerance があるときだけ倍率を掛ける（無ければ 1e-6 のまま）。`mise run msl:sim` は 0.1 | xtask の `sim.rs`・`main.rs`、`mise.toml` | `DemonstrateSignalExtrema`・`DemoPowerSupplyWithBuffer` が管に収まる。管に収まった 299 → 301。収まらなくなったものは無い |
| — | 既知の外れの一覧に `DoublePendulumInitTip`（上の表の 5）を足した | `src/Modelica/msl-sim-known-deviations.txt` | 外れ 6 本のうち既知 3 本 |

- 既知の外れでない残りは 3 本（上の表の 8〜10）。いずれも許容誤差では変わらない。

## S014 で直したもの

上の「GlobeMode で直すもの」の 4 と 5 を S014 で直した（確認日 2026-10-20。設計は [ADR-0016](../adr/0016-preflight-on-build-lowering.md)）。

| # | 直したこと | 場所 | 結果 |
| ---: | --- | --- | --- |
| 4 | プリフライトを組み立ての lowering の結果で行う（bdf・auto と session の道。ネイティブの dopri45 だけは単独のまま） | `globe-mode-exec/src/rumoca.rs`・`rumoca/session.rs`・`lib.rs` | `run` の時間（release）: `DoublePendulum` 1.07 s（単独のプリフライト 0.39 s が要らなくなった）、`Fluid.Examples.HeatingSystem` 4.00 s（同じく 4.26 s）。`FullRobot` は `msl-sim` で 111.6 s → 57.9 s（同じ機械で続けて回した） |
| 5 | 実行系の中の段（プリフライト・lowering・組み立て・初期化・積分）を `run_with_progress` で知らせ、ワーカーが親に送る | 同上と xtask の `sim.rs` | 外から止めた 7 本（節「外から停止」の 48〜54）は、**7 本とも lowering の段で止まっていた**（段に入ってから 84〜118 s で 120 s の kill）。上の「原因」の見立て（Solve IR への変換が終わらない）を全部で確かめた |

- 節「外から停止」の【未確認】のうち「DrumBoiler 以外の 6 本でどの段か」と「プリフライトの側か組み立ての側か」は片付いた: lowering は 1 回になり、7 本ともその 1 回が終わらない。
- 120 s 近くかかる例題（`LightningSegmentedTransmissionLine` 116 s、`FullRobot` 112 s、時間切れの回し直しの `ThyristorBridge2Pulse_DC_Drive` 197 s）は、機械が混んでいると外からの停止に変わる。S014 の 1 回目の全例題の実行（15 分の負荷の平均 26.7、16 コア）では 3 本がそうなった。同じ 3 本を空いた機械で S013 と S014 の実行ファイルで続けて回すと、どれも S014 のほうが同じか速かった。2 回目（負荷の平均 10〜12）では `LightningSegmentedTransmissionLine`（積分の段）と `FullRobot`（プリフライトの段。lowering が約 118 s で終わった直後）の 2 本が外からの停止に変わり、同じ負荷で 2 本を S013 と S014 の実行ファイルで続けて回すと、4 回とも 120 s で外から止まった。

## S015 で調べたこと

節「外から停止」の 7 本を、上限を 600 s にして 1 本ずつ回した（確認日 2026-10-20。xtask は S014 のもの、`--wall-seconds 600 --kill-seconds 600 --tolerance-scale 0.1`、空いた機械で 15 分の負荷の平均 0.1〜4。設計は [ADR-0017](../adr/0017-lowering-limit-and-hard-timeout-retry.md)）。

| # | モデル | 600 s での結果 | 壁時計 / CPU 時間（`/usr/bin/time`、MSL の読み込みを含む） |
| ---: | --- | --- | --- |
| 48 | `DrumBoiler` | **lowering は終わり**、S10 のプリフライト（t0 の評価 `SolveRuntime::eval_at`）が段に入ってから 481 s で終わらない（外から停止）。S015 の全例題の実行の回し直しでは lowering 117 s の後、プリフライトで 121 s | 604 s / 600 s |
| 49 | `MeasuringTemperature` | lowering が終わらない（段に入ってから 593 s） | 604 s / 606 s |
| 50 | `MomentumBalanceFittings` | lowering が終わらない（段に入ってから 595 s） | 603 s / 605 s |
| 51 | `InverseParameterization` | 530 s で `GM-U0004`（`[SolveLowering] … [EL005] DAE system is not computable: … linear solve requires an aggregate function owner`） | 537 s / 541 s |
| 52 | `NonCircularPipes` | lowering が終わらない（段に入ってから 594 s） | 603 s / 606 s |
| 53 | `EngineV6` | **終了時刻まで 169 s**（コンパイル 14 s、準備 95 s のほとんどが lowering、積分 58 s）。管は 2 / 4（`filter.x[1]` など 2 つが外れる） | 2 本で 696 s / 739 s |
| 54 | `EngineV6_analytic` | 523 s で `GM-U0004`（`[StructuralAnalysis] … [EL005] DAE structural proof failed: structurally singular system: 8318 matched out of 8443 equations`） | （同上）|

- 3 本は 600 s でも lowering が終わらない。CPU 時間が壁時計とほぼ同じなので、待っているのではなく計算し続けている。
- `DrumBoiler` は lowering が約 117 s で終わり、GlobeMode のプリフライトの評価が終わらない。S014 までの「7 本とも lowering の段」は、120 s の kill が lowering の終わる前に来ていたため。プリフライトは警告（`GM-W10002`）しか出さないので、ここで止まるのは GlobeMode の側で直せる【原因は未確認】。→ **S016 で誤りと分かった**。止まっているのはプリフライトの評価ではなく、Rumoca の実行時オブジェクトの組み立て（`SolveRuntime::new`）で、組み立ての段も同じ所を通る（節「S016 で調べたこと」）。
- 2 本は約 520 s で lowering（構造解析を含む）が終わり、EL005 の類の未対応になる。節「実行系が未対応（GM-U0004）」の EL005 と同じ上流の問題。
- `EngineV6` は S014 で lowering が 1 回になって、終わるようになった（S012 の調べでは 400 s で kill）。ただしコンパイル 11 s・lowering 64 s・組み立て 12 s・積分 49 s（S015 の全例題の実行で測った段ごとの秒）で、120 s では終わらない。
- 終わるもので lowering が長いのは、ほかに `FullRobot` 52 s、`Engine1b_analytic` 17 s（S015 の xtask で段ごとの秒を測った。負荷の平均 5〜9）。
- ワーカーの CPU を使うのはほぼ 1 スレッド（17 スレッドのうち 1 本が 100%）。負荷で遅くなるのは CPU 時間も含めてなので、CPU 時間で止めても負荷の揺れは消えない（16 論理コアは 8 コア × 2 のハイパースレッド）。

## S016 で調べたこと

### DrumBoiler: プリフライトの段で何に時間を使っているか

`DrumBoiler` だけを 1 本ずつ 5 回回した（確認日 2026-10-21。`cargo xtask msl-sim --only DrumBoiler.DrumBoiler --wall-seconds 600 --kill-seconds 840〜900 --lowering-kill-seconds 同じ値 --retry-wall-seconds 0 --tolerance-scale 0.1`）。設計の判断は [ADR-0018](../adr/0018-no-preflight-limit.md)。

- 機械は WSL のメモリ 3 GB（スワップ 16 GB）。同時に動いていたほかのプロセスとメモリを取り合い、ワーカーは何度も D 状態になった。壁時計の秒は参考にとどめ、CPU 時間を併記する。
- `perf` は libelf と DWARF の巻き戻しの無い版で、gdb も無い。そこで `perf record -F 199` の IP を `nm` の表で関数名に直した。呼び出し元は `--call-graph dwarf` のスタックの生データから拾い、足りない所はコードを読んでつないだ。

| 回 | 内容 | 結果 | user CPU | 最大 RSS |
| --- | --- | --- | --- | --- |
| 1 | そのまま | lowering 約 278 s の後、プリフライトの段に 562 s いて外から停止 | 745 s | 1.68 GB |
| 2 | 計測を入れる | `eval_at_start` は `SolveRuntime::new` から戻らず、`eval_at` に一度も入らない | 279 s | 1.24 GB |
| 3 | プリフライトを飛ばす（一時的な環境変数） | lowering 261 s の後、組み立ての段に 227 s いて外から停止 | 377 s | 1.73 GB |
| 4 | 同上 | lowering 440 s の後、組み立ての段に 295 s（CPU 約 120 s）いて外から停止 | 379 s | 1.08 GB |
| 5 | 同上、ブロックごとの解析の時間を測る | 根の条件の先頭の 1 本の解析が終わらないまま 900 s で停止 | — | — |

- **時間は Newton 法ではなく、実行時オブジェクトの組み立て（`SolveRuntime::new` → `construct`、`crates/rumoca-solver/src/runtime/solve_runtime.rs:499,527`）で使っている。**
  - 1 回目の 562 s の標本の先端は、`BTreeSet<usize>` の解放約 30%、`rumoca_ir_solve::structural_pattern::DependencyWalk::pure_call` 12〜39%、`BTreeMap::clone_subtree` 約 8%。
  - 99.5% の標本に `PreparedScalarProgramBlock::with_facts` → `prepare_row` → `derive_target_assignment_shapes` があった。
- 重いのは根の条件の program。5 回目にブロックごとに測った。

  | ブロック | program 数 | 演算の数 | 解析の時間 |
  | --- | ---: | ---: | --- |
  | implicit_rhs | 129 | 715 | 0.09 s |
  | implicit_jacobian_v | 129 | 1,382 | 0.11 s |
  | visible_value_rows | 152 | 304 | 0.003 s |
  | **events.root_conditions** | **31** | **597,270**（1 本あたり最大 64,828） | **先頭の 1 本が 900 s で終わらない** |

  - 道筋: `construct` の `root_condition_rows: prepare(…)`（`solve_runtime.rs:938`）→ `RowFacts::Full` の `target_assignment_shapes_with_output_offsets`（`crates/rumoca-eval-solve/src/prepared/construction.rs:68`）→ `crates/rumoca-ir-solve/src/refresh/assignment_shape.rs:100-118`。store の位置ごとに、prefix について `UniqueProgram::new` と `ScalarProgramYDependency::new`（全レジスタの依存集合）を作り直す。
  - 計算量はおよそ「store の数 × prefix の長さ × 依存集合の大きさ」。上限の無いループは見つからなかった。理屈の上では終わるが、上限の中では終わらない。
  - 根の条件に Media（水と蒸気）の関数の中身が展開されている見込み（pure call が約 24 個）【未確認】。
- モデルの本体は小さい: 状態 3、solver_y 135、パラメータ 218、代数の射影のブロック 122（最大 11 変数、ほかは 1 変数）。
- **組み立ての段も同じ `construct` を通る。** FMI ME の部品を作る `SolveRuntime::new_with_execution_backend`（`crates/rumoca-solver/src/fmi_me/kernel/component.rs:547`）が、bdf・auto の型付きの道と session の道の両方から呼ばれる。3・4 回目の組み立ての段の標本は、Cranelift の JIT（`ProgramLayout::new` 26〜45%、3 回目の終わりは `regalloc2` の `split_into_minimal_bundles` 92〜95%）、同じ `DependencyWalk::pure_call`、`TypedProgram::clone` の順だった。
- lowering の段の標本は、`Box<[RegisteredAssertion]>::clone` 25〜43%、`CallScopedActionCollector::insert` 27〜54%、`RegisteredCall` の解放 8〜14%（どれも `rumoca_phase_solve::lower`）。
- Rumoca には評価を外から打ち切る手段が無い。スレッドごとの締め切りを確かめる `panic_on_expired_solver_deadline`（`crates/rumoca-solver/src/runtime/timeout.rs:97`）を呼ぶ所が無い。`max_wall_seconds` の予算は初期化の後に作られる（`crates/rumoca-solver/src/fmi_me/session.rs:164,184`）。見るのは刻みの頭と根の探索だけ（`:486`）。
- ほかの 355 本のプリフライトの段は最大 1.9 s（S015 の `stage_times`）。

**結論**: DrumBoiler は GlobeMode の側では実行できるようにならない。プリフライトに上限を掛けても、組み立ての段で同じ所に詰まる。上流に報告する候補の 10 に足した。

### EngineV6: `filter.x` が t = 0 から外れる

S015 で初めて実行できた `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6` は、管が 2 / 4 で、`filter.x[1]`・`filter.x[2]` が t = 0 から外れていた（はみ出し 562）。確認日 2026-10-21。

- `filter` は `Blocks.Continuous.CriticalDamping(n=2, f=5, initType=SteadyState)` で、入力 u はエンジンの軸トルク `torqueSensor.tau`。初期化方程式は `der(x) = zeros(n)` なので、正しくは `x[1] = x[2] = u(0)`。

  | t = 0 | 参照（MAP-LIB v4.1.0） | GlobeMode | Rumoca の CLI v0.10.2 |
  | --- | --- | --- | --- |
  | `filter.u` | 563.19 | 563.186235502294 | 563.186235502294 |
  | `filter.x[1]`・`filter.x[2]` | 563.18623550229211 | 0.06599180619489453 | 0.06599180619489453 |
  | `der(filter.x[1])` | 0 | 27487.7 | 27487.7 |

- GlobeMode の u(0) は参照の x(0) と下の桁まで一致する。積分中の式と比べ方（変数の対応・添字・列名）は正しく、外れているのは `filter.x` の初期値だけ。`filter.x_start` を変えても同じ値になる。
- **原因は Rumoca の初期化**: 初期化方程式を解く途中で、関係式 `v_rel < 0`（MSL の `Mechanics/MultiBody/Examples/Loops/Utilities/GasForce2.mo:30`、ガスの圧力の枝）の値（relation memory）が既定の false のまま使われる。そのため `filter.x` は膨張の枝で計算した軸トルク 0.066 に落ち着く。初期化の後に関係式は true（圧縮の枝）に更新され、トルクは 563.19 になる。ところが `filter.x` は解き直されず、初期化方程式を満たさないまま積分が始まる。
  - cylinder1 の `gasForce.press` は、初期化の評価で 3.375e6、初期化の後で 1.053e6。GasForce2 の 2 つの枝を手で計算すると 3.375e6（膨張）と 1.055e6（圧縮）。
- **小さいモデルで再現した**（GlobeMode と Rumoca の CLI の両方。omc 1.27.1 は `filter.x = 1`・`der(filter.x) = 0` で正しい）。

  ```modelica
  model R1
    Real s(start=0, fixed=true); Real v; Real u;
    Modelica.Blocks.Continuous.CriticalDamping filter(n=2, initType=Modelica.Blocks.Types.Init.SteadyState, f=5);
  equation
    der(s) = v; v = -0.5*cos(s); u = if v < 0 then 1 else 2; filter.u = u;
  end R1;
  ```

  - GlobeMode では t = 0 で `u = 1` なのに `filter.x = 2`（else の枝）。v を -1e-16 にしても、v を非線形の代数ループの中に置いても同じ。関係式の無いモデル（インデックスの低減、非線形の代数ループ、歯車）では正しい。
  - 起きる条件: 初期化方程式（SteadyState など）が、t = 0 で true になる関係式を読むとき。
- Rumoca のコードの見立て【計測では未確認】: `crates/rumoca-solver/src/runtime/solve_runtime/initial_projection.rs:784` の写像の後で `update_root_relation_memory_from_values` の戻り値（変わったか）を捨てている。`settle_initialization_system` の繰り返し（`:831-838`）は delay と更新の行の変化しか見ないので、relation memory が反転しても写像をやり直さない。
- **既知の外れの一覧には載せない**。一覧はモデル・参照・比べ方の限界で外れるものに限っていて、実行系（Rumoca）の不具合は外れとして数え続ける。上流に報告する候補の 11 に足した。
- GlobeMode は初期化の後の初期化方程式の残差を見ていないので、この誤りを黙って通す。

### 出典（S016）

- Rumoca rev `276e82bd` のソース（2026-10-21）: `crates/rumoca-solver/src/runtime/solve_runtime.rs:499-530,938`、`crates/rumoca-solver/src/runtime/eval_at.rs:79-157`、`crates/rumoca-solver/src/runtime/timeout.rs:75-108`、`crates/rumoca-solver/src/fmi_me/kernel/component.rs:547`、`crates/rumoca-solver/src/fmi_me/session.rs:164,184,486`、`crates/rumoca-eval-solve/src/prepared/construction.rs:68`、`crates/rumoca-ir-solve/src/refresh/assignment_shape.rs:100-118`。
- Rumoca rev `276e82bd` の `crates/rumoca-solver/src/runtime/solve_runtime/initial_projection.rs:784,831-838`、MSL 4.1.0 の `Mechanics/MultiBody/Examples/Loops/Utilities/GasForce2.mo:30`、参照結果 MAP-LIB v4.1.0 の EngineV6（2026-10-21）。
- EngineV6 の再現: `globe simulate -L data/msl -m Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6 --stop-time 0.01 --tolerance 1e-7 --timeout 600`（約 5 分 40 秒、最大 RSS 1.37 GB）、Rumoca の CLI の `sim … --inspect eval` と `--t-end 0.001`、R1 を `globe simulate` と omc で（2026-10-21）。
- 測った値: S016 の `cargo xtask msl-sim --only DrumBoiler.DrumBoiler` の 5 回、`perf record -F 199`（`--call-graph dwarf` を含む）、S015 の `target/msl-sim.json` の `stage_times`（2026-10-21）。

## S018 で足したもの

### 初期化の解き直しの検査（GM-W10004）

S016 の EngineV6 の原因（初期化方程式が読む関係式の反転で Rumoca が解き直さない）を、GlobeMode の側で検出できるようにした。設計の判断は [ADR-0020](../adr/0020-init-resettle-check.md)。確認日 2026-10-22。

- Rumoca rev `276e82bd` で、初期化方程式の残差を評価する `eval_initial_residual` は crate の中の trait `AlgebraicProjectionModel`（`crates/rumoca-solver/src/runtime/projection.rs`。`runtime.rs` で `pub(crate) mod projection`）の実装で、公開されていない。公開は方向微分 `eval_initial_residual_jacobian_v`（`initial_projection.rs:493`）だけ。
- 公開されている `settle_initialization_system`・`update_relation_memory_from_state`・`refresh_algebraic_and_output_slots` を、Rumoca の FMI ME の kernel（`fmi_me/kernel/component.rs:1184`、`exit_initialization_mode_inner`）と同じ順に呼ぶと、実行が始まる点を再現できる。関係式の記憶はパラメータの枠（`events.root_relation_memory_targets` の `ScalarSlot::P`）にあり、`p` に残る。
- そこからもう一度 `settle_initialization_system` を呼ぶと、EngineV6 の仕組みでは反転した後の枝で解けて値が動く。初期化の結果が初期化方程式を満たしていれば動かない。これを S10 のプリフライト（`globe-mode-exec/src/rumoca.rs` の `resettle`）にし、動いた変数のうち、**実行が実際に始まった t0 の値（結果の最初の行）も 2 回目と違うもの**を GM-W10004 で知らせる（`run` だけ）。
- 1 回目との差だけで警告したときの MSL の結果（RPi、2026-10-22、356 本）: 2 本で出た。
  - `Electrical.Machines.Examples.InductionMachines.IMC_Initialize`: 169 変数が動く（`aimc.idq_ss[2] = 0 → -23.976`、相電流 `aimc.plug_sp.pin[2].i = 20.29 → -0.47` など）。ところが結果の t = 0 の行は 2 回目の値（`idq_ss[2] = -23.976`、`pin[2].i = -0.471`）で、管も 8 / 8 で収まる。Rumoca の FMI ME の host は初期化のあとに初期イベントの反復（`host_state.rs` の `update_discrete_states_to_completion`）を行い、このモデル（摩擦などの離散変数がある）ではそこで解き直される。`InitRelationBranch`（離散変数が無い）では解き直されない。1 回目との差で警告すると誤りになるので、実行の t0 の値と比べる形にした。
  - `Fluid.Examples.Tanks.TanksWithOverflow`: 2 回目の `settle_initialization_system` が `initial homotopy continuation failed at lambda=0 … discrete iteration did not converge` で失敗する。1 回目（start 値からの homotopy の掃引）は通る。解いた点から λ = 0 の掃引をやり直すと収束しない。失敗は実行が始まる点の誤りを示さないので、2 回目の失敗は黙る。このモデルは S015 と同じく積分で GM-E11003（GM-E11002）になる。
- 最小の再現 `GlobeModeValidation.Simulation.InitRelationBranch`（S016 の R1 と同じ仕組み。MSL に依らない）: 1 次のフィルタ `der(x) = (u - x) / T` の定常の初期化 `der(x) = 0` が `u = if v < 0 then 1 else 2`（t = 0 で v = -0.5）を読む。Rumoca は x(0) = 2、正しくは 1。プリフライトは `x = 2 → 1` を警告する。
- MSL の全例題での本数と費用は [msl-sim.md](../msl-sim.md) と S018 の作業記録。

### 出典（S018）

- Rumoca rev `276e82bd` のソース（2026-10-22）: `crates/rumoca-solver/src/runtime.rs:23`（`pub(crate) mod projection`）、`crates/rumoca-solver/src/runtime/solve_runtime/initial_projection.rs:11-16,203,493,746,799`、`crates/rumoca-solver/src/runtime/solve_runtime/relation_memory.rs:4,755-794`、`crates/rumoca-solver/src/fmi_me/kernel/component.rs:1176-1205`、`crates/rumoca-sim/src/prepared_vectors.rs:15-16,88-110`。

## S021 で直したもの

自前の積分器（S012）が MSL で落ちる原因を調べ、GlobeMode の側で直せるものを直した。確認日 2026-10-25。数字は S021 の作業記録。

### esdirk34: Newton 法の失敗を積分全体で累積して数える（直した）

- `Electrical.Analog.Examples` 54 本を `--solver esdirk34` で回すと、bdf で通る 7 本（`AmplifierWithOpAmpDetailed`・`CauerLowPassSC`・`DifferenceAmplifier`・`HeatingRectifier`・`OpAmps.Differentiator`・`OvervoltageProtection`・`ThyristorBehaviourTest`）が GM-E11002 で止まる。文言は 6 本が `Exceeded maximum number of nonlinear solver failures (50)`、`CauerLowPassSC` が `Step size is too small at time = 1.05`、`HeatingRectifier` が射影の収束（下）。
- 原因は diffsol（Rumoca の vendor、0.13.2）の SDIRK。`vendor/diffsol/src/ode_solver/runge_kutta.rs:849-872` の `solve_fail` が `statistics.number_of_nonlinear_solver_fails`（積分全体の累計）を上限 `max_nonlinear_solver_failures`（既定 50）と比べる。BDF（`bdf.rs:1269,1356-1359`）は歩の始めの値との差（その歩の中の回数）で比べる。切り替え（ダイオード・サイリスタ・飽和する OpAmp）のたびに Newton 法が数回失敗するので、`Differentiator` は 55 歩目（t = 0.0092）で累計が 50 を超えた。
- 直し方: SDIRK の問題を組むときに `ode_options.max_nonlinear_solver_failures = usize::MAX`（`globe-mode-exec/src/rumoca/plugins/rk.rs`）。1 歩の中の暴走は、失敗ごとに刻みを 0.3 倍にする diffsol の規則と刻みの下限（`min_timestep`）が止める。最小の再現 `GlobeModeValidation.Simulation.DiodeSwitching`（1 ms ごとに向きが変わる電流を、漏れ抵抗つきの膝の鋭いダイオード（熱電圧 2.6 mV）に流す。98 回の切り替え。直す前は esdirk34 が t = 0.0027、trbdf2 が t = 5e-6 で止まる）を足し、`globe-mode-exec/tests/integrators.rs` で esdirk34・trbdf2 が終了時刻まで進んで bdf と一致する（v(0.0985) の差 1.3e-5・2e-6）ことを確かめる。終点は平らな所に置いた（斜面の途中だと wasm とネイティブの bdf が 2.3e-4 ずれて `wasm:compare` が一致しないと数える）。
- 上流への報告の候補 12（下）。

### tsit45: 硬い回路で最初の刻みから状態が発散する（直さない）

- `SimpleTriacCircuit`・`ThyristorBehaviourTest`・`HeatingNPN_NORGate` などで `[Integration] algebraic projection did not establish coordinate convergence`（`vAK` が 5e4〜5e12）。許容誤差を 1e-8 にしても、終了時刻を 1e-4 にしても同じ値で止まる。
- `rhs` の呼び出しを印字すると、最初の刻み（h = 2e-5。出力の間隔で区切られた）の段の点で状態が 0 → 2.6e4 → 2.9e5 → 1.0e6 → -1.5e8 と発散し、刻みを半分にした試しの点で射影が発散して導関数が NaN になる。Rumoca の host は、chart を切り替える部品以外の評価の失敗を復帰不能として記憶する（`crates/rumoca-solver/src/fmi_me/integrator/derivative.rs:176-196` の `fail`。`discards_failed_trials()` が true の部品だけが「試しの点の棄却」になる）ので、プラグインが刻みを縮めても失敗のまま終わる。
- Rumoca 自身の `dopri45` も最初の出力の間隔（`--stop-time 2e-5`）で失敗し、既定の終了時刻では 60 s の予算で時間切れになる。陽的 RK がこの硬さ（サイリスタの時定数）に向かないのが原因で、GlobeMode のプラグインの不具合ではない。直さない。ユーザーには `bdf`・`esdirk34` を使ってもらう。
- 上流への報告の候補 13（下）。

### esdirk34・trbdf2: OvervoltageProtection で誤った軌道（直していない）

- 上限を外した esdirk34 は `OvervoltageProtection` を終了時刻まで進めるが、`CL.v` がツェナーの降伏（t = 0.0184、`ZDiode` の `smooth(1, if …)` の 3 つの領域の切り替え）から参照とずれ、終了時刻で -1.64 V（参照と bdf は -0.0006 V）。管 0/1。直す前も同じ軌道で t = 0.3088 に「nonlinear solver failures (50)」で止まっていたので、上限を外したことが誤りを作ったのではなく、黙って終わるようになっただけ。trbdf2 は同じ所で射影の発散（`zDiode.v` = 1.8e25）で止まる。SDIRK の段の Newton 法が強い非線形の代数ループ（ツェナーの 3 領域）で別の枝に落ちる見込み【未確認】。

### heun: 既定の刻み（出力の間隔）での精度（変えない）

- `Rotational.Examples` 7 本を `--solver heun` で回すと 7 本とも終了時刻まで進み、管に収まるのは 3 本（S012 と同じ）。`Backlash`・`First`・`FirstGrounded` は 1 信号ずつ外れ、`ElasticBearing` は `shaft.phi` が 1e139 まで発散する（bdf・rk4 は 445.56。陽的 2 次の固定刻みが硬いモデルで不安定になる）。値が有限なので失敗にならず「終了時刻まで」になる。既定の刻みは変えない（`--step` で選べる。精度は刻み幅で決まると [architecture](../architecture.md) に書いてある）。

### 上流に報告する候補（S021 で足す）

12. **diffsol の SDIRK が Newton 法の失敗を積分全体で累積して上限と比べる**: `vendor/diffsol/src/ode_solver/runge_kutta.rs:849-872`（`solve_fail`）。BDF は 1 歩の中で数える（`bdf.rs:1356-1359`）。切り替えの多いモデルは 50 回で止まる。再現は `GlobeModeValidation.Simulation.DiodeSwitching` を esdirk34 で。diffsol の本家（https://github.com/martinjrobins/diffsol ）で同じかは【未確認】。
13. **陽的 RK の試しの点で代数の射影が発散すると、刻みを縮めて再試行できず失敗で終わる**: `crates/rumoca-solver/src/fmi_me/integrator/derivative.rs:176-196`。`SimpleTriacCircuit` を `dopri45` で `--stop-time 2e-5`。BDF では Newton 法の予測子が近いので当たらない。

## S022 で直したもの

S021 が残した esdirk34 の件を調べた。確認日 2026-10-27。数字は S022 の作業記録。

### esdirk34・trbdf2: diffsol の SDIRK がヤコビ行列を間違った点で取る（直した）

- `OvervoltageProtection` を esdirk34 で回すと終了時刻まで進むが、ツェナーの降伏（t ≈ 0.018）から参照とずれ、CL.v(0.4) = -1.64 V（参照と bdf は -0.0006 V）。出力の `zDiode.i` = -7.4 A は、その `CL.v` に対しては正しい解（ZDiode の既定 Ibv = 0.7 A の指数の枝）で、代数ループは与えられた状態に対して正しく解けている。誤っているのは状態の軌道の方。
- 原因は diffsol（0.13.2、Rumoca の vendor）の SDIRK。段の方程式は `F(x) = M x − h f(phi + c x)`（`op/sdirk.rs`）で、ヤコビ行列を取り直すとき `ode_solver/runge_kutta.rs:637` と `ode_solver/sdirk.rs:261,277`（`jacobian_updates`）が段の未知数 `x` の位置に状態 `y` をそのまま渡す（コードのコメントは「x は何でもよい」）。`jacobian_inplace` は `phi + c·y`（phi ≈ y_n なので y_n の約 1.4 倍の点）で部品の `f'` を呼ぶ。ツェナーの指数ではその点の `J` が 1e15（正しい点では 5e5）になり、Newton 法の更新量 `r / (1 − hγJ)` も、同じ行列で解く埋め込みの誤差の見積もりも 1e12 分の 1 に潰れる。解けていない段と大きな刻みをそのまま受理する。計測では、受理した点 `t = 0.0184, CL.v = 5.41` の導関数が -2.8e5 で、段の評価の後に Newton 法が `CL.v = -3839` まで飛んでから「収束」していた。
- 直し方: `globe-mode-exec/src/rumoca/plugins/rk.rs` のヤコビ行列の閉包で、diffsol が渡す点を使わず、刻みの始めの受理した状態（`anchor`）で `directional_derivative_into` を呼ぶ。刻みの中では一歩分だけ古いヤコビ行列になるが、SDIRK はもともと刻みの中で行列を使い回す。
- 直した後: esdirk34 の CL.v(0.4) = -0.00061562（bdf -0.00061563）、t = 0.02 の `CL.v`・`zDiode.i`・`Rv.i` が bdf と 5 桁で一致。trbdf2 も通る（直す前は射影の発散で止まっていた）。esdirk34 の `--tolerance 1e-7`（直す前は「accepted step … made no progress beyond roundoff」の契約違反 GM-U0005）も通る。`DiodeSwitching` の esdirk34・trbdf2 は bdf と 1.3e-6・1.9e-6 で一致（直す前は 1.3e-5）。回帰の試験の許容を 1e-4 から 1e-5 に締めた。
- 直す前に `--tolerance 1e-8` にすると正しい値になるが 32 s かかる（直した後は 0.15 s）。許容誤差を締めても、ずれた点のヤコビ行列を力ずくで埋め合わせているだけ。
- 残り: `--tolerance 1e-5` では esdirk34・trbdf2 とも t ≈ 0.225 で射影の発散（Newton 法が `CL.v = 7.6, f = -2e15` から -4e9 へ飛び、host がその点の代数ループを解けず、失敗を復帰不能として記憶する。候補 13 と同じ機構）。試しの点が受理点から 1e3 倍以上離れたら host を呼ばずに NaN を返す案は効いた（1e-5 で -0.00062）が、閾値が恣意的なので入れていない。
- 上流への報告の候補 14（下）。

### esdirk34 の残り 3 本の分類

- **`HeatingRectifier`**（直った）: 直す前は t ∈ (3.12, 3.13] で射影の発散（`HeatingDiode1.auxp` = -6.3e4）。そこは bdf でダイオードが点き、t = 3.12 で v = 0.396、i = 2.0 A、Maxexp = 15 の線形の続きに入る所。残差の値が許容誤差に対して単調でなく（1e-4 で -1.2e4、1e-8 で i = 104）、軌道から遠い試しの点だった。上のヤコビ行列の直しで esdirk34（既定・1e-5・1e-7・1e-8）と trbdf2 が通る。tsit45・dopri45 は候補 13 の機構で落ちたまま。
- **`ThyristorBehaviourTest`**（esdirk34 だけ残る）: t ∈ (5.3e-5, 5.5e-5] で射影の発散（`thyristor_v4_1.vAK`）。bdf ではサイリスタが 5.017e-5 で消弧し、逆回復で Anode.i が -0.40、vAK が -1.4 → -7.0 に動く所。vControl が Voff = 1.5 と Von = 5 の間にあり、導通の混合が約 5 桁（Ron 0.04、Roff 9.8e5）に広がるので残差が状態の小さな誤差に敏感。直す前の残差は許容誤差 1e-5〜1e-8 で -0.36 とほぼ一定（軌道の近く）、直した後は +8.3 に変わり失敗のまま。trbdf2 は直した後に通る。host の射影が硬い混合で失敗する（候補 13 の機構）と見るが、ESDIRK34 の段の点が関わる可能性もある【未確認、確信は低い】。
- **`CauerLowPassSC`**（S023 で直した。下の「S023 で直したもの」）: esdirk34・trbdf2 とも「Step size is too small at time = 1.05」。1.05 はスイッチトキャパシタの BooleanPulse（周期 0.1）の立ち下がりで、V.v が t = 1 で立ち上がった後の最初の電荷の移動。S022 では試しの点の NaN かイベントの時刻の分解能を疑ったが、S023 の計測で否定した（再開の後に 1 歩も試していない）。
- 副次の発見: `globe simulate --param V.startTime=0.5` は `--list-parameters` に載っているのに効かない（`CauerLowPassSC` を bdf で `--stop-time 0.7` にしても V.v = 0 のまま。`V.V` は効く）。時間イベントに使うパラメータの上書きが積分の予定に届いていない見込み【未確認】。S022 では直していない。S023 で原因を確かめた（下）。

### 上流に報告する候補（S022 で足す）

14. **diffsol の SDIRK がヤコビ行列を段の未知数ではなく状態の点で取る**: `vendor/diffsol/src/ode_solver/runge_kutta.rs:637`・`ode_solver/sdirk.rs:261,277` が `reset_jacobian(op, &state.y, t)` を呼び、`op/sdirk.rs` の `jacobian_inplace` が `phi + c·y` で `f'` を評価する。指数で立ち上がる式では Newton 法と誤差の見積もりが盲目になり、誤った軌道を黙って受理する。再現は `Modelica.Electrical.Analog.Examples.OvervoltageProtection` を esdirk34 で（CL.v(0.4) が -1.64、参照 -0.0006）。diffsol の本家で同じかは【未確認】。

## S023 で直したもの

調査はユーザーの指示でサブエージェント（Sonnet、effort high）2 本に分けた。

### RK: 再開の最初の刻みの見積もりが刻みの下限を割る（直した）

- `CauerLowPassSC` の esdirk34 は、t = 1.05 の再開の後に右辺もヤコビ行列も 1 度も呼ばずに `step()` が「Step size is too small」を返していた。試しの点の NaN でも host の失敗の記憶でもない。
- 再開は `rk.rs` の `initial_state()` で diffsol の `RkState::set_step_size`（`vendor/diffsol/src/ode_solver/state.rs:1039-1100`、Hairer の見積もり）を呼ぶ。1.05 の状態はほぼ 0（|y| ≈ 1e-22）で、導関数は -3.8e4・4.3e3・2.3e3。重み付きのノルム（atol = rtol = 1e-6）で d0 = 2.5e-5、d1 = 1.25e11 になり、`h0 = 0.01·d0/d1 = 2e-18`、`h = min(100·h0, h1) = 1.994e-16`。刻みの下限 `min_timestep` は t ≈ 1 で 1e-13 なので、最初の `step()` が `runge_kutta.rs:841,866` の判定ですぐ失敗する。t = 1 の再開では d0 = 0（`h0 = 1e-6` の枝）で h = 1e-4 になり通る。S022 で見た許容誤差と V.V への依存もこれで説明がつく（d0 は y / atol）。
- 直し: `initial_state()` の最後で、見積もった刻みが `min_timestep` の 10 倍より小さければ 10 倍に持ち上げる（符号は保つ）。刻みは受理ごとに伸びるので、計測では 2e-13 から十数歩で 2e-7 に届いた。esdirk34・trbdf2・tsit45 が同じ関数を通る。auto（Rumoca の BDF と dopri45）は通らない。
- 結果: `CauerLowPassSC` は esdirk34（t = 60 まで 75 s）と trbdf2 で通り、bdf との差は全信号で |x−y|/(1+|y|) ≤ 5.4e-9・1.2e-8。`OvervoltageProtection`・`HeatingRectifier` の esdirk34 の出力は直す前と同じバイト列。最小の再現 `GlobeModeValidation.Simulation.TinyStateRestart`（x = 3e-11 から t = 1 で der(x) が 0 → 1e5）は、直す前は esdirk34・trbdf2・tsit45 とも再開で落ち、直した後は x(2) = 1e5。回帰の試験 `rk_restarts_from_a_tiny_state_with_a_large_derivative`。全体 356 本の esdirk34 は 実行 282 → 286、管 273 → 277（新たに通ったのはサイリスタの整流器 4 本: `HalfControlledBridge2mPulse`・`ThyristorBridge2mPulse_R`・`ThyristorCenterTap2mPulse_R`・`ThyristorCenterTapmPulse_R`。同じ再開の刻みで GM-E11002 だった）。
- 上流への報告の候補 15（下）。

### `--param` で時間イベントの時刻が変わらない（直していない）

- `--param` の値は Rumoca に届き、パラメータの値は上書きされる（`globe-mode-exec/src/rumoca.rs:75-80` の `param_overrides`、`session.rs:83` の `lower_correlated_for_simulation_with_overrides`、rumoca-sim の `verify_parameter_overrides`）。抵抗・`V.offset`・`V.signalSource.height` の上書きは効く。
- 時刻は DAE を組む段で既定値から決まっている。rumoca-phase-dae の `time_event_instant`（`construction/analysis/expression_events.rs:621`）が `time < startTime` を `constant_context()`（`construction/analysis.rs:1410`。調整できるパラメータも束縛の既定値で畳む）で評価し、静的な時間イベント（`TimeEventOperation::Static`）にする。solve の段はそれを `scheduled_time_events` に写し（`rumoca-phase-solve/src/lower/events.rs:1950-1958`）、実行時はその時刻で止まる。solve の段自身は調整できるパラメータを畳まない（`constants.rs:114`）ので、DAE の段と食い違っている。
- `StepVoltage` を R と接地に繋いだ小さな回路で `--param V.startTime=0.5` としても、V.v は既定（1）と同じ時刻で 0 → 1 に変わる。関係式の値もその時刻まで固定される。時刻が式として残る単純なモデルでは上書きが効くが、既定の時刻にも静的なイベントが出るので、イベントの位置がずれる。どの形で固定されるかの境目は【未確認】。
- 直す道: (1) 上流で `time_event_instant` が調整できるパラメータを畳まない（候補 16）。(2) 手元で `rumoca_compile::Session::set_structural_overrides` で上書きを組み込んでからコンパイルし直す（frontend の `Checker`・キャッシュ・wasm まで届く設計の変更なので、別のセッションで ADR を書いてから）。(3) 効かない上書きを警告する（DAE の関係式から時刻とパラメータの依存を辿る。新しい GM-W が要る）。S023 ではどれも入れていない。

### 上流に報告する候補（S023 で足す）

15. **diffsol の最初の刻みの見積もりが `min_timestep` を下回る**: `RkState::set_step_size`（`ode_solver/state.rs:1039-1100`）は `h0 = 0.01·‖y‖/‖f‖` を下限と比べない。状態がほぼ 0 で導関数が大きい点から始めると、最初の `step()` が 1 歩も試さずに「Step size is too small」で終わる。再現は `x(start = 3e-11)`、`der(x) = if time < 1 then 0 else 1e5`。diffsol の本家で同じかは【未確認】。
16. **Rumoca の DAE の段が時間イベントの時刻を調整できるパラメータの既定値で畳む**（S024 で改めた: 根の原因ではない。候補 18）: `time_event_instant` が `constant_context()` で評価するため、実行時のパラメータの上書き（`param_overrides`）が時刻に届かない。`Modelica.Electrical.Analog.Basic` の回路に `StepVoltage` を置き、`startTime` を上書きして確かめられる。Rumoca の LSP と wasm でも同じはず【未確認】。

## S024 で直したもの

調査はユーザーの指示でサブエージェント（Sonnet、effort high）に分けた。1 本が `ThyristorBehaviourTest` の計測、1 本が `--param` の直し方の見積もり（読むだけ）、もう 1 本が決めた直しの実装。

### `--param` が同じ名前の修飾で写された値に届かない（直した）

- S023 の見立て（時間イベントの時刻の畳み込み）は根の原因ではなかった。実行時の上書きは、内側の部品が同じ名前の修飾で外側の値を受け取る形（`SineVoltage V(f = 1)` の中の `signalSource(final f = f)`、`StepVoltage` の `startTime` など）では、外側の `V.f` だけを変えて内側には届かない。instantiate が外側の修飾の値を内側の束縛にそのまま書き込むため（rumoca-phase-instantiate の `mod_env.rs:694` の同じ名前の参照の解決、`:722` の `resolve_single_part_ref_expr` と読んだ。行までは追い切っていない【未確認】）。DAE の時点で内側のパラメータは、`final` なら評価済み（調整できない）、そうでなければ値の書かれた調整できるパラメータで、外側との依存は消えている。
- 時間イベントの畳み込みを調整できるパラメータで止める 15 行の patch（rumoca-phase-dae の `analysis.rs`・`expression_events.rs`）を試すと、静的な時間イベントは `Dynamic(deadline)` になったが、変わったのは実行時の上書きで既に効いていた形だけで、`StepVoltage` の形は `static [1.0]` のまま。
- 再現（2026-10-28 確認）: `SineVoltage V(V = 1, f = 1)` を R と接地に繋ぎ、`globe simulate --param V.f=2` で V.v(0.05) = 0.309（f = 1 のまま。正しくは sin(2π·2·0.05) = 0.588）。警告も出ない。
- 直し: 上書きがあれば、ライブラリだけの session を複製して `Session::set_structural_overrides` で組み込んでからコンパイルする（[ADR-0022](../adr/0022-structural-parameter-overrides.md)）。同じ session で一度コンパイルした後に呼ぶと ED020（`DAE provenance references an unknown source`。解決済みの木の `source_map` に合成のソースが無い。`session_impl.rs:1322-1328`）で落ちるので、複製してから使う。
- 結果（2026-10-28、WSL、release）: 上の `SineVoltage` は `--param V.f=2` で V.v(0.05) = 0.5878・V.v(0.1) = 0.9511（上書きなしは 0.309・0.5878 で変わらない）。`--param R.R=2` は前と同じ R.i(0.05) = 0.1545。`CauerLowPassSC` に `--param V.startTime=0.5 --stop-time 0.7` で V.v は t = 0.45 まで 0、0.5 から 1（前は全区間 0）。費用は `CauerLowPassSC` で複製と文書の追加 0.4 s・コンパイル約 2 s、最大 RSS 1.58 → 2.0〜2.1 GB（3 GB の WSL では swap に入り、壁時計は 7 s → 16〜26 s）。構造パラメータ・`final`・`Evaluate = true`・定数・無い名前は前と同じく GM-E10006 で断る。最小の再現 `GlobeModeValidation.Simulation.InlinedParameters` と試験 `structural_overrides_reach_bindings_inlined_by_same_name_modifications`。

### esdirk34: `ThyristorBehaviourTest`（直していない。host の限界）

- msl-sim の許容誤差は 1e-7（注釈に Tolerance が無いので 1e-6 の 0.1 倍）。1e-6・1e-8 でも落ちる。
- サイリスタが t = 5.016723e-5 で消弧し（状態 y1 が IH = 0.006）、host が積分器を作り直す（`initial_state`）。diffsol の最初の刻みの見積もりは esdirk34（4 次）で 4.05e-6、trbdf2（2 次）で 5.37e-7（イベントの前の刻みは 2.3e-7）。イベントで vConmain が 5 → 0 になり、d(vControl)/dt が -3.6e3 → -5.26e5 に跳ぶので、1 歩で vControl が Von = 5 から Voff = 1.5 の帯（抵抗の混合が 0.04〜1e6 Ω の 5 桁）を横切る。
- ESDIRK の第 2 段（c = 0.8717、t = 5.3698e-5）の Newton 法の初期値（Euler の点、vControl 3.177、y1 -0.657）は host で評価できるが、次の反復が y1 を 0.006 → 8.32 に飛ばし、その点で host の射影が失敗する。`derivative.rs:176-196` の `fail()` は `discards_failed_trials()`（`switches_reduced_charts()`、`chart_switch.rs:12`）が偽のとき失敗を復帰不能として記憶するので、diffsol が刻みを縮めて回し直しても GM-E11002 で止まる（候補 13 と同じ機構で、陰的 RK でも起きる）。
- ヤコビ行列の点（S022 の直し）は効かない（Euler の予測点で取っても落ちる）。受理点から遠い試しの点を host に渡さず NaN を返す番人も効かない（host の射影は前の解から始めるので、遠い点で失敗すると受理点の近くでも失敗が続く。1 回の計測からの推測【未確認】）。
- 再開の刻みを小さくすると通る: 見積もりの 0.9 倍（3.6e-6）で通り、1 倍（4.05e-6）で落ちる。イベントの前の刻みの 4 倍で抑えても通る（`OvervoltageProtection`・`HeatingRectifier`・`CauerLowPassSC` の出力は変わらない）。崖が見積もりのすぐ下にある調整の値で、host の限界を避けるだけなので入れていない。
- 通しても管には収まらない見込み: esdirk34 は最初の 2e-5 を 1 歩で進み、その間の出力（diffsol の Hermite の補間、`runge_kutta.rs:984-1005`）が硬い準静的な変数（Roff ≈ 1e6）で外れる（`Cathode.v` が 5.98 V、bdf は約 0）。補間は刻みの始めの導関数 f(y_n) ≈ 0 を使うが、真の傾きは 1.9。刻みの端の値は 3 つのソルバで一致する。

### 上流に報告する候補（S024 で足す）

17. **`set_structural_overrides` を一度コンパイルした session で呼ぶと ED020**: 平坦化と DAE のキャッシュだけを消し、解決済みの木の `source_map` に合成の上書きのソースが入らない（`session_impl.rs:1322-1328`）。上流の単体試験は最初のコンパイルの前に上書きを置くので通っている。名前の誤りも黙って無視する。
18. **同じ名前の修飾の値を内側の束縛に書き込むので、実行時のパラメータの上書きが内側に届かない**: `SineVoltage` の `f`・`phase`・`startTime`、`StepVoltage` の `startTime` など。黙って誤った結果になる。候補 16 の時間イベントの時刻もこれの現れ。
19. **host の射影の失敗を、陰的 RK の試しの点でも復帰不能として記憶する**: 候補 13 を陰的 RK に広げたもの。再現は `ThyristorBehaviourTest` の esdirk34。
20. **diffsol の Hermite の補間が硬い準静的な変数で外れる**: 刻みの始めの導関数だけで補間するため。刻みの端は正しい。

## S025 で調べたこと

調査と直しはサブエージェント（Sonnet、effort high）に任せ、断りの範囲は手元で広げた。再現は `GlobeModeValidation.Simulation.ArrayParameters`（2026-10-28 確認、debug の `globe simulate`、bdf）。

| 上書き | 以前 | 今 |
| --- | --- | --- |
| `a[2]=5`（直接使う配列） | 正しい（y_a 2 → 5。実行時の上書きだけで届く） | 同じ |
| `t[1]=0.2`（`if time < t[1]`） | 正しい（t = 0.5 の y_t が 0 → 1） | 同じ |
| `a=5`（配列全体） | `GM-U0001`（ED020 の「未対応」）。Rumoca が配列の束縛をスカラのリテラルで置き換える | 構造的な上書きに渡さず、`GM-E10006` |
| `src.k[1]=0.2`（`Inner i(final k = k)` で写される配列の要素） | 黙って誤る（`src.i.s` は 0.8 で切り替わったまま、`src.k[2]=7` でも `src.y` は 20 のまま） | 変換の前に `GM-E10006` |
| `src.i.k[1]=0.2`（内側の要素。final） | `GM-E10006` | 同じ |

- 原因: Rumoca の構造的な上書きはスカラだけ（`StructuralOverride` は `Bool`・`Int`・`Real`、`root_modifications` は session の非公開の欄で、`from_dotted` は添字を目的の経路に解かない。rumoca-compile の `session/structural_overrides.rs:36-77`、読んだだけで要素の名前を渡す試しはしていない）。実行時の上書きはスカラの名前で一致するので要素を受けるが、インスタンス化で同名の修飾が展開済みの束縛には届かない（候補 18 と同じ根）。
- 断りの範囲は名前の形で決めている（`frontend/src/rumoca.rs` の `compile_retaining_with_overrides`）。違う名前の修飾（`final kk = k`）は見逃す。要素の上書きが届いたかを DAE から調べる安い方法は無い。
- 上流への報告の候補 21: **構造的な上書きが配列と要素を受けない**（候補 17・18 と同じ報告にまとめてよい）。

