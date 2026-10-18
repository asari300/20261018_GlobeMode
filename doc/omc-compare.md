# MSL 4.1.0 の全例題の omc との照合（globe check ↔ OpenModelica）

`cargo xtask omc-compare` が書く。`msl-funnel --json` の結果（globe check の S0〜S8 の判定）を、omc（OpenModelica のコンパイラ、別プロセス）の `checkModel`（方程式数と変数数）と `translateModel`（バックエンドまで。構造特異と初期条件の過剰）と突き合わせる。規則は [ADR-0010](adr/0010-omc-comparison.md)。

| 項目 | 値 |
| --- | --- |
| omc | `OpenModelica 1.27.1~2-g6db4671` |
| MSL | `data/msl`（4.1.0。omc 同梱の `+maint.om` ではなく、globe check と同じ素の MSL） |
| globe check の結果 | `target/msl-funnel.json` |
| 例題 | 566 本 |
| omc の実行 | 12 プロセス（50 本ごと、制限 900 s）、390.8 s、時間切れ 0 回、異常終了 0 回、1 本ずつの再実行 0 本、結果の無いモデル 0 本 |
| translateModel | 呼んだ |

## 集計

| 項目 | 本数 |
| --- | --- |
| omc の checkModel が通った（数が出た） | 566 |
| omc の checkModel が失敗 | 0 |
| omc の結果が無い（落ちた・時間切れ） | 0 |

### S5（方程式数と未知数が合うか）の一致率 — P2 の完了条件

globe check に S5 の判定があるモデル（OK、または S5 以降で止まったもの。GM-U と S4 以前で止まったものは判定が無い）と、omc の `checkModel` の数が出たモデルを比べる。「合う / 合わない」の判定が同じなら一致。

| 項目 | 本数 |
| --- | --- |
| 比べたモデル | 382 |
| 判定が一致 | 382 |
| 判定が不一致 | 0 |
| **一致率** | **100.0%**（完了条件 95% 以上: 達成） |
| 方程式数と未知数が omc と同じ数 | 307 / 382（80.4%） |

判定は一致したが数が違うモデル（globe check の方程式数 / 未知数 ↔ omc の方程式数 / 変数数）:

| モデル | globe check | omc |
| --- | --- | --- |
| `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | 67 / 67 | 73 / 73 ○ |
| `Clocked.Examples.CascadeControlledDrive.SubClocked` | 67 / 67 | 72 / 72 ○ |
| `Clocked.Examples.CascadeControlledDrive.SuperSampled` | 67 / 67 | 69 / 69 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | 8 / 8 | 11 / 11 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | 16 / 16 | 19 / 19 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.BackSample` | 13 / 13 | 16 / 16 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.Hold` | 13 / 13 | 16 / 16 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.Sample1` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.Sample2` | 14 / 14 | 17 / 17 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | 3 / 3 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | 6 / 6 | 9 / 9 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.SubSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | 5 / 5 | 7 / 7 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | 6 / 6 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.BooleanSignals.UpSample` | 24 / 24 | 27 / 27 ○ |
| `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 212 / 212 | 242 / 242 ○ |
| `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 34 / 34 | 39 / 39 ○ |
| `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | 3 / 3 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.ClockSignals.SubSample` | 3 / 3 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.ClockSignals.SuperSample` | 3 / 3 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | 8 / 8 | 11 / 11 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | 16 / 16 | 19 / 19 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.BackSample` | 13 / 13 | 16 / 16 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.Hold` | 13 / 13 | 16 / 16 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.Sample1` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.Sample2` | 14 / 14 | 17 / 17 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | 3 / 3 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | 6 / 6 | 9 / 9 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.SubSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.IntegerSignals.UpSample` | 24 / 24 | 27 / 27 ○ |
| `Clocked.Examples.Elementary.RealSignals.AssignClock` | 8 / 8 | 11 / 11 ○ |
| `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | 7 / 7 | 10 / 10 ○ |
| `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | 11 / 11 | 14 / 14 ○ |
| `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | 16 / 16 | 19 / 19 ○ |
| `Clocked.Examples.Elementary.RealSignals.BackSample` | 7 / 7 | 10 / 10 ○ |
| `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | 8 / 8 | 11 / 11 ○ |
| `Clocked.Examples.Elementary.RealSignals.Hold` | 7 / 7 | 10 / 10 ○ |
| `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | 19 / 19 | 22 / 22 ○ |
| `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | 19 / 19 | 22 / 22 ○ |
| `Clocked.Examples.Elementary.RealSignals.Sample1` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.RealSignals.Sample2` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.RealSignals.Sample3` | 12 / 12 | 15 / 15 ○ |
| `Clocked.Examples.Elementary.RealSignals.SampleClocked` | 3 / 3 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | 6 / 6 | 9 / 9 ○ |
| `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | 21 / 21 | 24 / 24 ○ |
| `Clocked.Examples.Elementary.RealSignals.ShiftSample` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.RealSignals.SubSample` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.RealSignals.SuperSample` | 5 / 5 | 8 / 8 ○ |
| `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | 12 / 12 | 15 / 15 ○ |
| `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | 7 / 7 | 9 / 9 ○ |
| `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | 4 / 4 | 6 / 6 ○ |
| `Clocked.Examples.Elementary.RealSignals.UniformNoise` | 9 / 9 | 12 / 12 ○ |
| `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 8 / 8 | 11 / 11 ○ |
| `Clocked.Examples.Elementary.RealSignals.UpSample1` | 18 / 18 | 21 / 21 ○ |
| `Clocked.Examples.Elementary.RealSignals.UpSample2` | 40 / 40 | 43 / 43 ○ |
| `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | 28 / 28 | 30 / 30 ○ |
| `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | 27 / 27 | 29 / 29 ○ |
| `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | 68 / 68 | 70 / 70 ○ |
| `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | 28 / 28 | 31 / 31 ○ |
| `Clocked.Examples.Systems.EngineThrottleControl` | 117 / 117 | 128 / 128 ○ |
| `Fluid.Examples.InverseParameterization` | 279 / 279 | 287 / 287 ○ |
| `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | 1 / 1 | 3 / 3 ○ |
| `Media.Examples.TwoPhaseWater.BaseProperties` | 16 / 16 | 18 / 18 ○ |

### GM-U（コンパイラが未対応）のモデルは omc で通るか

globe check が GM-U にしたモデルが omc で通れば、それは Rumoca の制約であって、モデルの誤りではない（[ADR-0007](adr/0007-unsupported-classification.md) の C）。

| 項目 | 本数 |
| --- | --- |
| GM-U のモデル | 184 |
| omc の checkModel で数が合う | 184（100.0%） |
| omc の translateModel も通る | 180（97.8%） |

既知の制約の一覧（`GM-U0003`、`src/Modelica/msl-known-unsupported.txt`）に当たった 50 本:

| モデル | Rumoca のコード | omc の checkModel | omc の translateModel |
| --- | --- | --- | --- |
| `Blocks.Examples.Rectifier12pulseFFT` | ET004 | 5853 / 5853 ○ | 通る |
| `Blocks.Examples.Rectifier6pulseFFT` | ET004 | 5386 / 5386 ○ | 通る |
| `Electrical.Batteries.Examples.CCCV_CellRC` | ED001 | 154 / 154 ○ | 通る |
| `Electrical.Batteries.Examples.CCCVcharging` | ED001 | 123 / 123 ○ | 通る |
| `Electrical.Digital.Examples.Counter` | ED010 | 322 / 322 ○ | 通る |
| `Electrical.Digital.Examples.Counter3` | ED010 | 237 / 237 ○ | 通る |
| `Electrical.Machines.Examples.SynchronousMachines.SMEE_LoadDump` | ED010 | 731 / 731 ○ | 通る |
| `Electrical.PowerConverters.Examples.ACAC.SoftStarter` | ED001 | 1192 / 1192 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_CommonLeakage` | ED001 | 3573 / 3573 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_Polyphase` | ED001 | 2135 / 2135 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMS_Start_Polyphase` | ED001 | 2681 / 2681 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Conveyor` | ED001 | 843 / 843 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_DOL` | ED001 | 1348 / 1348 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Initialize` | ED001 | 819 / 819 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Inverter` | ED001 | 822 / 822 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Steinmetz` | ED001 | 844 / 844 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Transformer` | ED001 | 1287 / 1287 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_YD` | ED001 | 1011 / 1011 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_withLosses` | ED001 | 877 / 877 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Start` | ED001 | 1778 / 1778 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMEE_Generator_Polyphase` | ED001 | 2293 / 2293 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMPM_Inverter_Polyphase` | ED001 | 1932 / 1932 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMR_Inverter_Polyphase` | ED001 | 1854 / 1854 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_DOL` | ED001 | 1051 / 1051 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Generator` | ED001 | 1496 / 1496 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_LoadDump` | ED001 | 1076 / 1076 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Rectifier` | ED001 | 958 / 958 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Braking` | ED001 | 818 / 818 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_CurrentSource` | ED001 | 826 / 826 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Inverter` | ED001 | 1413 / 1413 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_VoltageSource` | ED001 | 911 / 911 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMR_Inverter` | ED001 | 1347 / 1347 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.Components.EddyCurrentLosses` | ED001 | 689 / 689 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.Components.PolyphaseInductance` | ED001 | 588 / 588 ○ | 通る |
| `Magnetic.FundamentalWave.Examples.Components.SinglePhaseInductance` | ED001 | 84 / 84 ○ | 通る |
| `Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CuboidSections` | ED001 | 188 / 188 ○ | 通る |
| `Math.FastFourierTransform.Examples.RealFFT1` | ET004 | 245 / 245 ○ | 通る |
| `Math.FastFourierTransform.Examples.RealFFT2` | ET004 | 224 / 224 ○ | 通る |
| `Mechanics.Rotational.Examples.CoupledClutches` | ED001 | 112 / 112 ○ | 通る |
| `Mechanics.Rotational.Examples.Friction` | ED001 | 102 / 102 ○ | 通る |
| `Mechanics.Rotational.Examples.HeatLosses` | ED001 | 248 / 248 ○ | 通る |
| `Mechanics.Rotational.Examples.LossyGearDemo1` | ED001 | 76 / 76 ○ | 通る |
| `Mechanics.Rotational.Examples.LossyGearDemo2` | ED001 | 100 / 100 ○ | 通る |
| `Mechanics.Rotational.Examples.LossyGearDemo3` | ED001 | 67 / 67 ○ | 通る |
| `Mechanics.Rotational.Examples.OneWayClutch` | ED001 | 43 / 43 ○ | 通る |
| `Mechanics.Rotational.Examples.OneWayClutchDisengaged` | ED001 | 46 / 46 ○ | 通る |
| `Mechanics.Rotational.Examples.SimpleGearShift` | ED001 | 88 / 88 ○ | 通る |
| `Mechanics.Translational.Examples.Brake` | ED001 | 73 / 73 ○ | 通る |
| `Mechanics.Translational.Examples.Friction` | ER130 | 90 / 90 ○ | 通る |
| `Mechanics.Translational.Examples.HeatLosses` | ER130 | 174 / 174 ○ | 通る |

一覧の全部が omc で通る。一覧は Rumoca の制約として正しい。

### S6（構造特異）の照合

globe check の S6（完全マッチングの有無）と、omc の `translateModel` の結果（「structurally singular」「pantelidesIndexReduction failed」「imbalanced number of equations」を特異とみなす）を比べる。omc が別の理由（未対応の関数など）で失敗したものは比べない。

| 項目 | 本数 |
| --- | --- |
| 比べたモデル | 363 |
| 一致 | 362（99.7%） |
| 不一致 | 1 |

globe check が S6 で止めた 12 本:

| モデル | omc の checkModel | omc の translateModel | omc の文言 |
| --- | --- | --- | --- |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Add` | 58 / 58 ○ | 特異 | Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equatio… |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Buffer` | 45 / 45 ○ | 特異 | Found equation without time-dependent variables: opAmp.in_p.i = 0.0 |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Der` | 43 / 43 ○ | 失敗 | Parameter f has neither value nor start value, and is fixed during initialization (fixed=true). |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Derivative` | 52 / 52 ○ | 失敗 | Parameter T has neither value nor start value, and is fixed during initialization (fixed=true). |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Feedback` | 67 / 67 ○ | 特異 | Found equation without time-dependent variables: r4.i = 0.0 |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.FirstOrder` | 52 / 52 ○ | 失敗 | Parameter T has neither value nor start value, and is fixed during initialization (fixed=true). |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Gain` | 45 / 45 ○ | 特異 | Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equatio… |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Integrator` | 43 / 43 ○ | 失敗 | Parameter f has neither value nor start value, and is fixed during initialization (fixed=true). |
| `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.PI` | 51 / 51 ○ | 失敗 | Parameter T has neither value nor start value, and is fixed during initialization (fixed=true). |
| `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.ConstantActuator` | 84 / 84 ○ | 特異 | Found equation without time-dependent variables: n.i = 0.0 |
| `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.PermeanceActuator` | 133 / 133 ○ | 特異 | Found equation without time-dependent variables: n.i = 0.0 |
| `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.SimpleSolenoid` | 202 / 202 ○ | 特異 | Found equation without time-dependent variables: n.i = 0.0 |

不一致のモデル:

| モデル | globe check | omc の translateModel | omc の文言 |
| --- | --- | --- | --- |
| `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | OK | 特異 | Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equatio… |

### S8（初期条件の過剰）の照合

globe check の S8 の判定（要る数と与えられた数）と、omc の `translateModel` の警告「The initial conditions are over specified」を比べる。omc は不足を黙って start 値で埋める（`-d=initialization` でも警告が出ない）ので、**過剰だけ**を比べる。omc の translateModel が失敗したものは比べない。

| 項目 | 本数 |
| --- | --- |
| 比べたモデル | 355 |
| 一致（両方が過剰、または両方が過剰でない） | 354（99.7%） |
| 不一致 | 1 |

どちらかが過剰としたモデル:

| モデル | globe check の S8（与えられた / 要る） | omc |
| --- | --- | --- |
| `Electrical.Analog.Examples.GenerationOfFMUs` | 過剰 8 / 6（確定できない） | 過剰 2 |
| `Electrical.Analog.Examples.Lines.CompareLineTrunks` | 過剰 60 / 57（確定できない） | 過剰 3 |
| `Fluid.Examples.NonCircularPipes` | 不足 4 / 40 | 過剰 2 |
| `Magnetic.FluxTubes.Examples.Hysteresis.ThreePhaseTransformerWithRectifier` | - | 過剰 2 |
| `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 過剰 4 / 3（確定できない） | 過剰 1 |

### globe check が OK なのに omc の translateModel が失敗したモデル

- 失敗: 13 本
  - `Electrical.PowerConverters.Examples.ACAC.ExampleTemplates.Dimmer`: Illegal to instantiate partial class Dimmer.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.Thyristor1Pulse`: Illegal to instantiate partial class Thyristor1Pulse.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2Pulse`: Illegal to instantiate partial class ThyristorBridge2Pulse.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2mPulse`: Illegal to instantiate partial class ThyristorBridge2mPulse.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2Pulse`: Illegal to instantiate partial class ThyristorCenterTap2Pulse.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2mPulse`: Illegal to instantiate partial class ThyristorCenterTap2mPulse.
  - `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTapmPulse`: Illegal to instantiate partial class ThyristorCenterTapmPulse.
  - `Electrical.PowerConverters.Examples.DCAC.ExampleTemplates.SinglePhaseTwoLevel`: Illegal to instantiate partial class SinglePhaseTwoLevel.
  - `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperBuckBoost`: Illegal to instantiate partial class ChopperBuckBoost.
  - `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepDown`: Illegal to instantiate partial class ChopperStepDown.
  - `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepUp`: Illegal to instantiate partial class ChopperStepUp.
  - `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.HBridge`: Illegal to instantiate partial class HBridge.
  - `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter`: Parameter yMax has neither value nor start value, and is fixed during initialization (fixed=true).
- 特異: 1 本
  - `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid`: Internal error IndexReduction.pantelidesIndexReduction failed! Found empty set of continuous equatio…

## モデルごとの結果

globe check の列は msl-funnel の判定（OK、NG と止まった段階、GM-U と Rumoca のコード）と方程式数 / 未知数。omc の列は checkModel の方程式数 / 変数数（○ は合う、× は合わない）と translateModel の結果。S5 は判定の一致（○ / ×、- は比べられない）。S8 は globe check の判定 ↔ omc の過剰の警告。

| # | モデル | globe check | 数 | omc checkModel | S5 | omc translateModel | S8 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `Blocks.Examples.BooleanNetwork1` | GM-U0001 ED019 | - | 88 / 88 ○ | - | 通る | - |
| 2 | `Blocks.Examples.BusUsage` | OK | 14 / 14 | 14 / 14 ○ | ○ | 通る | 不足 0 / 7 ↔ 過剰でない |
| 3 | `Blocks.Examples.BusUsage_Utilities.Part` | OK | 4 / 4 | 4 / 4 ○ | ○ | 通る | 不足 0 / 2 ↔ 過剰でない |
| 4 | `Blocks.Examples.CompareSincExpSine` | OK | 4 / 4 | 4 / 4 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 5 | `Blocks.Examples.DemoSignalCharacteristic` | OK | 53 / 53 | 53 / 53 ○ | ○ | 通る | 合う 17 / 17 ↔ 過剰でない |
| 6 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | OK | 43 / 43 | 43 / 43 ○ | ○ | 通る | 合う 21 / 21 ↔ 過剰でない |
| 7 | `Blocks.Examples.DemonstrateSignalExtrema` | OK | 18 / 18 | 18 / 18 ○ | ○ | 通る | 合う 11 / 11 ↔ 過剰でない |
| 8 | `Blocks.Examples.Filter` | GM-U0001 ED019 | - | 34 / 34 ○ | - | 通る | - |
| 9 | `Blocks.Examples.FilterWithDifferentiation` | OK | 15 / 15 | 15 / 15 ○ | ○ | 通る | 合う 10 / 10 ↔ 過剰でない |
| 10 | `Blocks.Examples.FilterWithRiseTime` | OK | 22 / 22 | 22 / 22 ○ | ○ | 通る | 合う 18 / 18 ↔ 過剰でない |
| 11 | `Blocks.Examples.IntegerNetwork1` | GM-U0001 ED019 | - | 38 / 38 ○ | - | 通る | - |
| 12 | `Blocks.Examples.Interaction1` | OK | 50 / 50 | 50 / 50 ○ | ○ | 通る | 不足 18 / 33 ↔ 過剰でない |
| 13 | `Blocks.Examples.InverseModel` | OK | 15 / 15 | 15 / 15 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 14 | `Blocks.Examples.LogicalNetwork1` | OK | 24 / 24 | 24 / 24 ○ | ○ | 通る | 不足 5 / 18 ↔ 過剰でない |
| 15 | `Blocks.Examples.Modulation` | OK | 32 / 32 | 32 / 32 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 16 | `Blocks.Examples.Noise.ActuatorWithNoise` | GM-U0001 ED019 | - | 814 / 814 ○ | - | 通る | - |
| 17 | `Blocks.Examples.Noise.AutomaticSeed` | GM-U0001 ED019 | - | 45 / 45 ○ | - | 通る | - |
| 18 | `Blocks.Examples.Noise.Densities` | OK | 14 / 14 | 14 / 14 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 19 | `Blocks.Examples.Noise.Distributions` | GM-U0001 ED019 | - | 17 / 17 ○ | - | 通る | - |
| 20 | `Blocks.Examples.Noise.DrydenContinuousTurbulence` | GM-U0001 ED019 | - | 15 / 15 ○ | - | 通る | - |
| 21 | `Blocks.Examples.Noise.ImpureGenerator` | GM-U0001 ED019 | - | 2 / 2 ○ | - | 通る | - |
| 22 | `Blocks.Examples.Noise.NormalNoiseProperties` | GM-U0001 ED019 | - | 38 / 38 ○ | - | 通る | - |
| 23 | `Blocks.Examples.Noise.UniformNoise` | GM-U0001 ED019 | - | 15 / 15 ○ | - | 通る | - |
| 24 | `Blocks.Examples.Noise.UniformNoiseProperties` | GM-U0001 ED019 | - | 38 / 38 ○ | - | 通る | - |
| 25 | `Blocks.Examples.PID_Controller` | OK | 89 / 89 | 89 / 89 ○ | ○ | 通る | 不足 6 / 9 ↔ 過剰でない |
| 26 | `Blocks.Examples.RealNetwork1` | GM-U0001 ED019 | - | 34 / 34 ○ | - | 通る | - |
| 27 | `Blocks.Examples.Rectifier12pulseFFT` | GM-U0003 ET004 | - | 5853 / 5853 ○ | - | 通る | - |
| 28 | `Blocks.Examples.Rectifier6pulseFFT` | GM-U0003 ET004 | - | 5386 / 5386 ○ | - | 通る | - |
| 29 | `Blocks.Examples.ShowLogicalSources` | OK | 13 / 13 | 13 / 13 ○ | ○ | 通る | 不足 3 / 10 ↔ 過剰でない |
| 30 | `Blocks.Examples.SinCosEncoder` | OK | 53 / 53 | 53 / 53 ○ | ○ | 通る | 不足 5 / 6 ↔ 過剰でない |
| 31 | `Blocks.Examples.SlewRateLimiter` | OK | 19 / 19 | 19 / 19 ○ | ○ | 通る | 不足 3 / 4 ↔ 過剰でない |
| 32 | `Blocks.Examples.TotalHarmonicDistortion` | OK | 141 / 141 | 141 / 141 ○ | ○ | 通る | 不足 18 / 34 ↔ 過剰でない |
| 33 | `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | OK | 67 / 67 | 73 / 73 ○ | ○ | 通る | 不足 3 / 23 ↔ 過剰でない |
| 34 | `Clocked.Examples.CascadeControlledDrive.Continuous` | OK | 56 / 56 | 56 / 56 ○ | ○ | 通る | 不足 3 / 6 ↔ 過剰でない |
| 35 | `Clocked.Examples.CascadeControlledDrive.SubClocked` | OK | 67 / 67 | 72 / 72 ○ | ○ | 通る | 不足 3 / 23 ↔ 過剰でない |
| 36 | `Clocked.Examples.CascadeControlledDrive.SuperSampled` | OK | 67 / 67 | 69 / 69 ○ | ○ | 通る | 不足 3 / 23 ↔ 過剰でない |
| 37 | `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | OK | 8 / 8 | 11 / 11 ○ | ○ | 通る | 不足 0 / 8 ↔ 過剰でない |
| 38 | `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | OK | 16 / 16 | 19 / 19 ○ | ○ | 通る | 不足 0 / 16 ↔ 過剰でない |
| 39 | `Clocked.Examples.Elementary.BooleanSignals.BackSample` | OK | 13 / 13 | 16 / 16 ○ | ○ | 通る | 不足 2 / 10 ↔ 過剰でない |
| 40 | `Clocked.Examples.Elementary.BooleanSignals.Hold` | OK | 13 / 13 | 16 / 16 ○ | ○ | 通る | 不足 2 / 10 ↔ 過剰でない |
| 41 | `Clocked.Examples.Elementary.BooleanSignals.Sample1` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 5 ↔ 過剰でない |
| 42 | `Clocked.Examples.Elementary.BooleanSignals.Sample2` | OK | 14 / 14 | 17 / 17 ○ | ○ | 通る | 不足 0 / 14 ↔ 過剰でない |
| 43 | `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | OK | 3 / 3 | 6 / 6 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 44 | `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | OK | 6 / 6 | 9 / 9 ○ | ○ | 通る | 不足 0 / 6 ↔ 過剰でない |
| 45 | `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 46 | `Clocked.Examples.Elementary.BooleanSignals.SubSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 47 | `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 48 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | OK | 5 / 5 | 7 / 7 ○ | ○ | 通る | 不足 0 / 5 ↔ 過剰でない |
| 49 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 50 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | OK | 6 / 6 | 8 / 8 ○ | ○ | 通る | 不足 1 / 6 ↔ 過剰でない |
| 51 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 52 | `Clocked.Examples.Elementary.BooleanSignals.UpSample` | OK | 24 / 24 | 27 / 27 ○ | ○ | 通る | 不足 2 / 21 ↔ 過剰でない |
| 53 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | OK | 212 / 212 | 242 / 242 ○ | ○ | 通る | 不足 14 / 180 ↔ 過剰でない |
| 54 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | OK | 34 / 34 | 39 / 39 ○ | ○ | 通る | 不足 3 / 17 ↔ 過剰でない |
| 55 | `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | OK | 3 / 3 | 8 / 8 ○ | ○ | 通る | 不足 0 / 1 ↔ 過剰でない |
| 56 | `Clocked.Examples.Elementary.ClockSignals.SubSample` | OK | 3 / 3 | 8 / 8 ○ | ○ | 通る | 不足 0 / 1 ↔ 過剰でない |
| 57 | `Clocked.Examples.Elementary.ClockSignals.SuperSample` | OK | 3 / 3 | 8 / 8 ○ | ○ | 通る | 不足 0 / 1 ↔ 過剰でない |
| 58 | `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | OK | 8 / 8 | 11 / 11 ○ | ○ | 通る | 不足 0 / 8 ↔ 過剰でない |
| 59 | `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | OK | 16 / 16 | 19 / 19 ○ | ○ | 通る | 不足 0 / 16 ↔ 過剰でない |
| 60 | `Clocked.Examples.Elementary.IntegerSignals.BackSample` | OK | 13 / 13 | 16 / 16 ○ | ○ | 通る | 不足 2 / 10 ↔ 過剰でない |
| 61 | `Clocked.Examples.Elementary.IntegerSignals.Hold` | OK | 13 / 13 | 16 / 16 ○ | ○ | 通る | 不足 2 / 10 ↔ 過剰でない |
| 62 | `Clocked.Examples.Elementary.IntegerSignals.Sample1` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 5 ↔ 過剰でない |
| 63 | `Clocked.Examples.Elementary.IntegerSignals.Sample2` | OK | 14 / 14 | 17 / 17 ○ | ○ | 通る | 不足 0 / 14 ↔ 過剰でない |
| 64 | `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | OK | 3 / 3 | 6 / 6 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 65 | `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | OK | 6 / 6 | 9 / 9 ○ | ○ | 通る | 不足 0 / 6 ↔ 過剰でない |
| 66 | `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 67 | `Clocked.Examples.Elementary.IntegerSignals.SubSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 68 | `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 69 | `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 70 | `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 71 | `Clocked.Examples.Elementary.IntegerSignals.UpSample` | OK | 24 / 24 | 27 / 27 ○ | ○ | 通る | 不足 2 / 21 ↔ 過剰でない |
| 72 | `Clocked.Examples.Elementary.RealSignals.AssignClock` | OK | 8 / 8 | 11 / 11 ○ | ○ | 通る | 不足 0 / 8 ↔ 過剰でない |
| 73 | `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | OK | 7 / 7 | 10 / 10 ○ | ○ | 通る | 不足 0 / 5 ↔ 過剰でない |
| 74 | `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | OK | 11 / 11 | 14 / 14 ○ | ○ | 通る | 不足 2 / 8 ↔ 過剰でない |
| 75 | `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | OK | 16 / 16 | 19 / 19 ○ | ○ | 通る | 不足 0 / 16 ↔ 過剰でない |
| 76 | `Clocked.Examples.Elementary.RealSignals.BackSample` | OK | 7 / 7 | 10 / 10 ○ | ○ | 通る | 不足 0 / 5 ↔ 過剰でない |
| 77 | `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | OK | 8 / 8 | 11 / 11 ○ | ○ | 通る | 不足 0 / 6 ↔ 過剰でない |
| 78 | `Clocked.Examples.Elementary.RealSignals.Hold` | OK | 7 / 7 | 10 / 10 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 79 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | OK | 19 / 19 | 22 / 22 ○ | ○ | 通る | 不足 0 / 15 ↔ 過剰でない |
| 80 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | OK | 19 / 19 | 22 / 22 ○ | ○ | 通る | 不足 0 / 15 ↔ 過剰でない |
| 81 | `Clocked.Examples.Elementary.RealSignals.Sample1` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 82 | `Clocked.Examples.Elementary.RealSignals.Sample2` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 83 | `Clocked.Examples.Elementary.RealSignals.Sample3` | OK | 12 / 12 | 15 / 15 ○ | ○ | 通る | 不足 0 / 8 ↔ 過剰でない |
| 84 | `Clocked.Examples.Elementary.RealSignals.SampleClocked` | OK | 3 / 3 | 6 / 6 ○ | ○ | 通る | 不足 0 / 1 ↔ 過剰でない |
| 85 | `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | OK | 6 / 6 | 9 / 9 ○ | ○ | 通る | 不足 0 / 2 ↔ 過剰でない |
| 86 | `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | OK | 21 / 21 | 24 / 24 ○ | ○ | 通る | 不足 3 / 18 ↔ 過剰でない |
| 87 | `Clocked.Examples.Elementary.RealSignals.ShiftSample` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 88 | `Clocked.Examples.Elementary.RealSignals.SubSample` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 89 | `Clocked.Examples.Elementary.RealSignals.SuperSample` | OK | 5 / 5 | 8 / 8 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 90 | `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | OK | 12 / 12 | 15 / 15 ○ | ○ | 通る | 不足 0 / 10 ↔ 過剰でない |
| 91 | `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 92 | `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | OK | 7 / 7 | 9 / 9 ○ | ○ | 通る | 不足 0 / 6 ↔ 過剰でない |
| 93 | `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 94 | `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 95 | `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 96 | `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | OK | 4 / 4 | 6 / 6 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 97 | `Clocked.Examples.Elementary.RealSignals.UniformNoise` | OK | 9 / 9 | 12 / 12 ○ | ○ | 通る | 不足 3 / 7 ↔ 過剰でない |
| 98 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | OK | 8 / 8 | 11 / 11 ○ | ○ | 通る | 不足 0 / 6 ↔ 過剰でない |
| 99 | `Clocked.Examples.Elementary.RealSignals.UpSample1` | OK | 18 / 18 | 21 / 21 ○ | ○ | 通る | 不足 0 / 16 ↔ 過剰でない |
| 100 | `Clocked.Examples.Elementary.RealSignals.UpSample2` | OK | 40 / 40 | 43 / 43 ○ | ○ | 通る | 不足 0 / 38 ↔ 過剰でない |
| 101 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | OK | 28 / 28 | 30 / 30 ○ | ○ | 通る | 不足 3 / 12 ↔ 過剰でない |
| 102 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | OK | 27 / 27 | 29 / 29 ○ | ○ | 通る | 不足 3 / 11 ↔ 過剰でない |
| 103 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscretizedContinuousController` | GM-U0001 ED018 | - | 29 / 29 ○ | - | 通る | - |
| 104 | `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | OK | 68 / 68 | 70 / 70 ○ | ○ | 通る | 不足 5 / 49 ↔ 過剰でない |
| 105 | `Clocked.Examples.SimpleControlledDrive.Continuous` | OK | 21 / 21 | 21 / 21 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 106 | `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | OK | 28 / 28 | 31 / 31 ○ | ○ | 通る | 不足 3 / 12 ↔ 過剰でない |
| 107 | `Clocked.Examples.Systems.ControlledMixingUnit` | GM-U0001 ED018 | - | 34 / 34 ○ | - | 通る | - |
| 108 | `Clocked.Examples.Systems.EngineThrottleControl` | OK | 117 / 117 | 128 / 128 ○ | ○ | 通る | 不足 9 / 40 ↔ 過剰でない |
| 109 | `ComplexBlocks.Examples.ShowTransferFunction` | GM-U0001 ED019 | - | 29 / 29 ○ | - | 通る | - |
| 110 | `ComplexBlocks.Examples.TestConversionBlock` | OK | 10 / 10 | 10 / 10 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 111 | `Electrical.Analog.Examples.AD_DA_conversion` | GM-U0001 ED013 | - | 50 / 50 ○ | - | 通る | - |
| 112 | `Electrical.Analog.Examples.AmplifierWithOpAmpDetailed` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 113 | `Electrical.Analog.Examples.CauerLowPassAnalog` | OK | 69 / 69 | 69 / 69 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 114 | `Electrical.Analog.Examples.CauerLowPassOPV` | OK | 240 / 240 | 240 / 240 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 115 | `Electrical.Analog.Examples.CauerLowPassSC` | OK | 658 / 658 | 658 / 658 ○ | ○ | 通る | 不足 27 / 137 ↔ 過剰でない |
| 116 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | OK | 80 / 80 | 80 / 80 ○ | ○ | 通る | 不足 0 / 3 ↔ 過剰でない |
| 117 | `Electrical.Analog.Examples.CharacteristicThyristors` | OK | 98 / 98 | 98 / 98 ○ | ○ | 通る | 不足 7 / 14 ↔ 過剰でない |
| 118 | `Electrical.Analog.Examples.ChuaCircuit` | OK | 44 / 44 | 44 / 44 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 119 | `Electrical.Analog.Examples.CompareTransformers` | OK | 123 / 123 | 123 / 123 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 120 | `Electrical.Analog.Examples.ControlledSwitchWithArc` | OK | 80 / 80 | 80 / 80 ○ | ○ | 通る | 不足 5 / 6 ↔ 過剰でない |
| 121 | `Electrical.Analog.Examples.DemoPowerSupply` | OK | 24 / 24 | 24 / 24 ○ | ○ | 通る | 不足 0 / 2 ↔ 過剰でない |
| 122 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | OK | 50 / 50 | 50 / 50 ○ | ○ | 通る | 不足 3 / 6 ↔ 過剰でない |
| 123 | `Electrical.Analog.Examples.DemonstrateLightning` | OK | 58 / 58 | 58 / 58 ○ | ○ | 通る | 不足 12 / 14 ↔ 過剰でない |
| 124 | `Electrical.Analog.Examples.DifferenceAmplifier` | OK | 221 / 221 | 221 / 221 ○ | ○ | 通る | 合う 9 / 9 ↔ 過剰でない |
| 125 | `Electrical.Analog.Examples.GenerationOfFMUs` | OK | 306 / 306 | 306 / 306 ○ | ○ | 通る | 過剰 8 / 6（確定できない） ↔ 過剰 2 |
| 126 | `Electrical.Analog.Examples.HeatingMOSInverter` | OK | 92 / 92 | 92 / 92 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 127 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | OK | 158 / 158 | 158 / 158 ○ | ○ | 通る | 不足 5 / 8 ↔ 過剰でない |
| 128 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | OK | 158 / 158 | 158 / 158 ○ | ○ | 通る | 不足 5 / 8 ↔ 過剰でない |
| 129 | `Electrical.Analog.Examples.HeatingRectifier` | OK | 49 / 49 | 49 / 49 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 130 | `Electrical.Analog.Examples.IdealTriacCircuit` | OK | 62 / 62 | 62 / 62 ○ | ○ | 通る | 不足 4 / 8 ↔ 過剰でない |
| 131 | `Electrical.Analog.Examples.InvertingAmp` | OK | 60 / 60 | 60 / 60 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 132 | `Electrical.Analog.Examples.Lines.CompareLineTrunks` | OK | 1091 / 1091 | 1091 / 1091 ○ | ○ | 通る | 過剰 60 / 57（確定できない） ↔ 過剰 3 |
| 133 | `Electrical.Analog.Examples.Lines.CompareLosslessLines` | OK | 118 / 118 | 118 / 118 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 134 | `Electrical.Analog.Examples.Lines.LightningLosslessTransmissionLine` | OK | 53 / 53 | 53 / 53 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 135 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | OK | 6477 / 6477 | 6477 / 6477 ○ | ○ | 通る | 合う 406 / 406 ↔ 過剰でない |
| 136 | `Electrical.Analog.Examples.Lines.SmoothStep` | OK | 1935 / 1935 | 1935 / 1935 ○ | ○ | 通る | 合う 116 / 116 ↔ 過剰でない |
| 137 | `Electrical.Analog.Examples.NandGate` | OK | 147 / 147 | 147 / 147 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 138 | `Electrical.Analog.Examples.OpAmps.Adder` | OK | 78 / 78 | 78 / 78 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 139 | `Electrical.Analog.Examples.OpAmps.Comparator` | OK | 61 / 61 | 61 / 61 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 140 | `Electrical.Analog.Examples.OpAmps.ControlCircuit` | OK | 311 / 311 | 311 / 311 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 141 | `Electrical.Analog.Examples.OpAmps.DifferentialAmplifier` | OK | 137 / 137 | 137 / 137 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 142 | `Electrical.Analog.Examples.OpAmps.Differentiator` | OK | 59 / 59 | 59 / 59 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 143 | `Electrical.Analog.Examples.OpAmps.HighPass` | OK | 68 / 68 | 68 / 68 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 144 | `Electrical.Analog.Examples.OpAmps.Integrator` | OK | 59 / 59 | 59 / 59 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 145 | `Electrical.Analog.Examples.OpAmps.InvertingAmplifier` | OK | 61 / 61 | 61 / 61 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 146 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | OK | 49 / 49 | 49 / 49 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 147 | `Electrical.Analog.Examples.OpAmps.LCOscillator` | OK | 61 / 61 | 61 / 61 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 148 | `Electrical.Analog.Examples.OpAmps.LowPass` | OK | 68 / 68 | 68 / 68 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 149 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | OK | 55 / 55 | 55 / 55 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 150 | `Electrical.Analog.Examples.OpAmps.NonInvertingAmplifier` | OK | 61 / 61 | 61 / 61 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 151 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Add` | NG S6 GM-E6001 | 58 / 58 | 58 / 58 ○ | ○ | 特異 | - |
| 152 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Buffer` | NG S6 GM-E6001 | 45 / 45 | 45 / 45 ○ | ○ | 特異 | - |
| 153 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Der` | NG S6 GM-E6001 | 43 / 43 | 43 / 43 ○ | ○ | 失敗 | - |
| 154 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Derivative` | NG S6 GM-E6001 | 52 / 52 | 52 / 52 ○ | ○ | 失敗 | - |
| 155 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Feedback` | NG S6 GM-E6001 | 67 / 67 | 67 / 67 ○ | ○ | 特異 | - |
| 156 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.FirstOrder` | NG S6 GM-E6001 | 52 / 52 | 52 / 52 ○ | ○ | 失敗 | - |
| 157 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Gain` | NG S6 GM-E6001 | 45 / 45 | 45 / 45 ○ | ○ | 特異 | - |
| 158 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Integrator` | NG S6 GM-E6001 | 43 / 43 | 43 / 43 ○ | ○ | 失敗 | - |
| 159 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.PI` | NG S6 GM-E6001 | 51 / 51 | 51 / 51 ○ | ○ | 失敗 | - |
| 160 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.PartialOpAmp`（partial） | NG S5 GM-E5002 | 23 / 27 | 23 / 27 × | ○ | 失敗 | - |
| 161 | `Electrical.Analog.Examples.OpAmps.SchmittTrigger` | OK | 49 / 49 | 49 / 49 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 162 | `Electrical.Analog.Examples.OpAmps.SignalGenerator` | OK | 75 / 75 | 75 / 75 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 163 | `Electrical.Analog.Examples.OpAmps.Subtracter` | OK | 87 / 87 | 87 / 87 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 164 | `Electrical.Analog.Examples.OpAmps.VoltageFollower` | OK | 49 / 49 | 49 / 49 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 165 | `Electrical.Analog.Examples.OvervoltageProtection` | OK | 49 / 49 | 49 / 49 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 166 | `Electrical.Analog.Examples.ParallelResonance` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 167 | `Electrical.Analog.Examples.Rectifier` | OK | 128 / 128 | 128 / 128 ○ | ○ | 通る | 不足 4 / 10 ↔ 過剰でない |
| 168 | `Electrical.Analog.Examples.Resistor` | OK | 28 / 28 | 28 / 28 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 169 | `Electrical.Analog.Examples.ResonanceCircuits` | OK | 252 / 252 | 252 / 252 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 170 | `Electrical.Analog.Examples.SeriesResonance` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 171 | `Electrical.Analog.Examples.ShowSaturatingInductor` | OK | 23 / 23 | 23 / 23 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 172 | `Electrical.Analog.Examples.ShowVariableResistor` | OK | 67 / 67 | 67 / 67 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 173 | `Electrical.Analog.Examples.SimpleTriacCircuit` | OK | 89 / 89 | 89 / 89 ○ | ○ | 通る | 不足 5 / 7 ↔ 過剰でない |
| 174 | `Electrical.Analog.Examples.SwitchWithArc` | OK | 71 / 71 | 71 / 71 ○ | ○ | 通る | 不足 6 / 10 ↔ 過剰でない |
| 175 | `Electrical.Analog.Examples.ThyristorBehaviourTest` | OK | 48 / 48 | 48 / 48 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 176 | `Electrical.Batteries.Examples.BatteryDischargeCharge` | GM-U0001 ED020 | - | 215 / 215 ○ | - | 通る | - |
| 177 | `Electrical.Batteries.Examples.CCCV_Cell` | OK | 120 / 120 | 120 / 120 ○ | ○ | 通る | 不足 4 / 7 ↔ 過剰でない |
| 178 | `Electrical.Batteries.Examples.CCCV_CellRC` | GM-U0003 ED001 | - | 154 / 154 ○ | - | 通る | - |
| 179 | `Electrical.Batteries.Examples.CCCV_Stack` | GM-U0001 EF020 | - | 837 / 837 ○ | - | 通る | - |
| 180 | `Electrical.Batteries.Examples.CCCV_StackRC` | GM-U0001 EF020 | - | 1013 / 1013 ○ | - | 通る | - |
| 181 | `Electrical.Batteries.Examples.CCCVcharging` | GM-U0003 ED001 | - | 123 / 123 ○ | - | 通る | - |
| 182 | `Electrical.Batteries.Examples.ShowImpedance` | GM-U0001 ED019 | - | 4 / 4 ○ | - | 通る | - |
| 183 | `Electrical.Batteries.Examples.SuperCapDischargeCharge` | OK | 77 / 77 | 77 / 77 ○ | ○ | 通る | 不足 2 / 5 ↔ 過剰でない |
| 184 | `Electrical.Digital.Examples.Adder4` | GM-U0001 ED013 | - | 341 / 341 ○ | - | 通る | - |
| 185 | `Electrical.Digital.Examples.BUF3S` | GM-U0001 ED013 | - | 14 / 14 ○ | - | 通る | - |
| 186 | `Electrical.Digital.Examples.Counter` | GM-U0003 ED010 | - | 322 / 322 ○ | - | 通る | - |
| 187 | `Electrical.Digital.Examples.Counter3` | GM-U0003 ED010 | - | 237 / 237 ○ | - | 通る | - |
| 188 | `Electrical.Digital.Examples.DFFREG` | GM-U0001 ED013 | - | 40 / 40 ○ | - | 通る | - |
| 189 | `Electrical.Digital.Examples.DFFREGL` | GM-U0001 ED013 | - | 40 / 40 ○ | - | 通る | - |
| 190 | `Electrical.Digital.Examples.DFFREGSRH` | GM-U0001 ED013 | - | 43 / 43 ○ | - | 通る | - |
| 191 | `Electrical.Digital.Examples.DFFREGSRL` | GM-U0001 ED013 | - | 43 / 43 ○ | - | 通る | - |
| 192 | `Electrical.Digital.Examples.DLATREG` | GM-U0001 ED013 | - | 40 / 40 ○ | - | 通る | - |
| 193 | `Electrical.Digital.Examples.DLATREGL` | GM-U0001 ED013 | - | 40 / 40 ○ | - | 通る | - |
| 194 | `Electrical.Digital.Examples.DLATREGSRH` | GM-U0001 ED013 | - | 43 / 43 ○ | - | 通る | - |
| 195 | `Electrical.Digital.Examples.DLATREGSRL` | GM-U0001 ED013 | - | 43 / 43 ○ | - | 通る | - |
| 196 | `Electrical.Digital.Examples.FlipFlop` | GM-U0001 ED013 | - | 81 / 81 ○ | - | 通る | - |
| 197 | `Electrical.Digital.Examples.FullAdder` | GM-U0001 ED013 | - | 324 / 324 ○ | - | 通る | - |
| 198 | `Electrical.Digital.Examples.HalfAdder` | GM-U0001 ED013 | - | 42 / 42 ○ | - | 通る | - |
| 199 | `Electrical.Digital.Examples.INV3S` | GM-U0001 ED013 | - | 14 / 14 ○ | - | 通る | - |
| 200 | `Electrical.Digital.Examples.MUX2x1` | GM-U0001 ED013 | - | 16 / 16 ○ | - | 通る | - |
| 201 | `Electrical.Digital.Examples.Multiplexer` | GM-U0001 ED013 | - | 139 / 139 ○ | - | 通る | - |
| 202 | `Electrical.Digital.Examples.NRXFER` | GM-U0001 ED013 | - | 14 / 14 ○ | - | 通る | - |
| 203 | `Electrical.Digital.Examples.NXFER` | GM-U0001 ED013 | - | 14 / 14 ○ | - | 通る | - |
| 204 | `Electrical.Digital.Examples.RAM` | GM-U0002 EF025 | - | 43 / 43 ○ | - | 通る | - |
| 205 | `Electrical.Digital.Examples.VectorDelay` | GM-U0001 ED013 | - | 30 / 30 ○ | - | 通る | - |
| 206 | `Electrical.Digital.Examples.WiredX` | GM-U0001 ED013 | - | 33 / 33 ○ | - | 通る | - |
| 207 | `Electrical.Machines.Examples.ControlledDCDrives.CurrentControlledDCPM` | GM-U0002 EF024 | - | 375 / 375 ○ | - | 通る | - |
| 208 | `Electrical.Machines.Examples.ControlledDCDrives.PositionControlledDCPM` | GM-U0002 EF024 | - | 469 / 469 ○ | - | 通る | - |
| 209 | `Electrical.Machines.Examples.ControlledDCDrives.SpeedControlledDCPM` | GM-U0002 EF024 | - | 413 / 413 ○ | - | 通る | - |
| 210 | `Electrical.Machines.Examples.DCMachines.DCEE_Start` | OK | 246 / 246 | 246 / 246 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 211 | `Electrical.Machines.Examples.DCMachines.DCPM_Cooling` | OK | 272 / 272 | 272 / 272 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 212 | `Electrical.Machines.Examples.DCMachines.DCPM_CurrentControlled` | OK | 247 / 247 | 247 / 247 ○ | ○ | 通る | 不足 5 / 6 ↔ 過剰でない |
| 213 | `Electrical.Machines.Examples.DCMachines.DCPM_Drive` | OK | 671 / 671 | 671 / 671 ○ | ○ | 通る | 不足 16 / 21 ↔ 過剰でない |
| 214 | `Electrical.Machines.Examples.DCMachines.DCPM_QuasiStatic` | OK | 392 / 392 | 392 / 392 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 215 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | OK | 200 / 200 | 200 / 200 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 216 | `Electrical.Machines.Examples.DCMachines.DCPM_Temperature` | OK | 214 / 214 | 214 / 214 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 217 | `Electrical.Machines.Examples.DCMachines.DCPM_withLosses` | OK | 393 / 393 | 393 / 393 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 218 | `Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase` | OK | 250 / 250 | 250 / 250 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 219 | `Electrical.Machines.Examples.DCMachines.DCSE_Start` | OK | 249 / 249 | 249 / 249 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 220 | `Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics` | OK | 666 / 666 | 666 / 666 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 221 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | OK | 490 / 490 | 490 / 490 ○ | ○ | 通る | 合う 9 / 9 ↔ 過剰でない |
| 222 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | OK | 392 / 392 | 392 / 392 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 223 | `Electrical.Machines.Examples.InductionMachines.IMC_DOL` | OK | 524 / 524 | 524 / 524 ○ | ○ | 通る | 不足 7 / 19 ↔ 過剰でない |
| 224 | `Electrical.Machines.Examples.InductionMachines.IMC_Initialize` | OK | 466 / 466 | 466 / 466 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 225 | `Electrical.Machines.Examples.InductionMachines.IMC_Inverter` | OK | 468 / 468 | 468 / 468 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 226 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | OK | 1064 / 1064 | 1064 / 1064 ○ | ○ | 通る | 不足 22 / 98 ↔ 過剰でない |
| 227 | `Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz` | OK | 491 / 491 | 491 / 491 ○ | ○ | 通る | 不足 9 / 15 ↔ 過剰でない |
| 228 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | OK | 934 / 934 | 934 / 934 ○ | ○ | 通る | 不足 9 / 30 ↔ 過剰でない |
| 229 | `Electrical.Machines.Examples.InductionMachines.IMC_YD` | OK | 658 / 658 | 658 / 658 ○ | ○ | 通る | 不足 10 / 55 ↔ 過剰でない |
| 230 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | OK | 745 / 745 | 745 / 745 ○ | ○ | 通る | 不足 30 / 69 ↔ 過剰でない |
| 231 | `Electrical.Machines.Examples.InductionMachines.IMC_withLosses` | OK | 524 / 524 | 524 / 524 ○ | ○ | 通る | 合う 8 / 8 ↔ 過剰でない |
| 232 | `Electrical.Machines.Examples.InductionMachines.IMS_Start` | OK | 798 / 798 | 798 / 798 ○ | ○ | 通る | 不足 8 / 29 ↔ 過剰でない |
| 233 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL` | OK | 701 / 701 | 701 / 701 ○ | ○ | 通る | 不足 7 / 21 ↔ 過剰でない |
| 234 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator` | OK | 656 / 656 | 656 / 656 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 235 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_LoadDump` | GM-U0003 ED010 | - | 731 / 731 ○ | - | 通る | - |
| 236 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier` | OK | 617 / 617 | 617 / 617 ○ | ○ | 通る | 不足 16 / 23 ↔ 過剰でない |
| 237 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking` | OK | 653 / 653 | 653 / 653 ○ | ○ | 通る | 不足 5 / 11 ↔ 過剰でない |
| 238 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | OK | 659 / 659 | 659 / 659 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 239 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter` | OK | 581 / 581 | 581 / 581 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 240 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad` | OK | 404 / 404 | 404 / 404 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 241 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking` | OK | 739 / 739 | 739 / 739 ○ | ○ | 通る | 不足 6 / 34 ↔ 過剰でない |
| 242 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource` | OK | 775 / 775 | 775 / 775 ○ | ○ | 通る | 不足 7 / 8 ↔ 過剰でない |
| 243 | `Electrical.Machines.Examples.SynchronousMachines.SMR_DOL` | OK | 606 / 606 | 606 / 606 ○ | ○ | 通る | 不足 6 / 18 ↔ 過剰でない |
| 244 | `Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter` | OK | 553 / 553 | 553 / 553 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 245 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | OK | 474 / 474 | 474 / 474 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 246 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | OK | 934 / 934 | 934 / 934 ○ | ○ | 通る | 不足 9 / 30 ↔ 過剰でない |
| 247 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | OK | 1046 / 1046 | 1046 / 1046 ○ | ○ | 通る | 不足 7 / 19 ↔ 過剰でない |
| 248 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | OK | 579 / 579 | 579 / 579 ○ | ○ | 通る | 不足 4 / 10 ↔ 過剰でない |
| 249 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | OK | 772 / 772 | 772 / 772 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 250 | `Electrical.Polyphase.Examples.PolyphaseRectifier` | GM-U0001 ED019 | - | 1254 / 1254 ○ | - | 通る | - |
| 251 | `Electrical.Polyphase.Examples.Rectifier` | OK | 531 / 531 | 531 / 531 ○ | ○ | 通る | 不足 4 / 10 ↔ 過剰でない |
| 252 | `Electrical.Polyphase.Examples.TestSensors` | OK | 578 / 578 | 578 / 578 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 253 | `Electrical.Polyphase.Examples.TransformerYD` | OK | 285 / 285 | 285 / 285 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 254 | `Electrical.Polyphase.Examples.TransformerYY` | OK | 281 / 281 | 281 / 281 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 255 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | OK | 197 / 197 | 197 / 197 ○ | ○ | 通る | 不足 30 / 42 ↔ 過剰でない |
| 256 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | OK | 203 / 203 | 203 / 203 ○ | ○ | 通る | 不足 31 / 43 ↔ 過剰でない |
| 257 | `Electrical.PowerConverters.Examples.ACAC.ExampleTemplates.Dimmer`（partial） | OK | 188 / 188 | 188 / 188 ○ | ○ | 失敗 | 不足 30 / 42 ↔ （失敗） |
| 258 | `Electrical.PowerConverters.Examples.ACAC.SoftStarter` | GM-U0003 ED001 | - | 1192 / 1192 ○ | - | 通る | - |
| 259 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.Thyristor1Pulse`（partial） | OK | 93 / 93 | 93 / 93 ○ | ○ | 失敗 | 不足 12 / 23 ↔ （失敗） |
| 260 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2Pulse`（partial） | OK | 154 / 154 | 154 / 154 ○ | ○ | 失敗 | 不足 15 / 41 ↔ （失敗） |
| 261 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2mPulse`（partial） | OK | 443 / 443 | 443 / 443 ○ | ○ | 失敗 | 不足 27 / 113 ↔ （失敗） |
| 262 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2Pulse`（partial） | OK | 137 / 137 | 137 / 137 ○ | ○ | 失敗 | 不足 13 / 37 ↔ （失敗） |
| 263 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2mPulse`（partial） | OK | 441 / 441 | 441 / 441 ○ | ○ | 失敗 | 不足 21 / 101 ↔ （失敗） |
| 264 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTapmPulse`（partial） | OK | 325 / 325 | 325 / 325 ○ | ○ | 失敗 | 不足 18 / 77 ↔ （失敗） |
| 265 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R` | OK | 102 / 102 | 102 / 102 ○ | ○ | 通る | 不足 12 / 23 ↔ 過剰でない |
| 266 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R_Characteristic` | OK | 104 / 104 | 104 / 104 ○ | ○ | 通る | 不足 12 / 23 ↔ 過剰でない |
| 267 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.DiodeBridge2Pulse` | OK | 103 / 103 | 103 / 103 ○ | ○ | 通る | 不足 9 / 13 ↔ 過剰でない |
| 268 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.HalfControlledBridge2Pulse` | OK | 161 / 161 | 161 / 161 ○ | ○ | 通る | 不足 13 / 39 ↔ 過剰でない |
| 269 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | OK | 373 / 373 | 373 / 373 ○ | ○ | 通る | 不足 22 / 48 ↔ 過剰でない |
| 270 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_R` | OK | 163 / 163 | 163 / 163 ○ | ○ | 通る | 不足 15 / 41 ↔ 過剰でない |
| 271 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RL` | OK | 169 / 169 | 169 / 169 ○ | ○ | 通る | 不足 16 / 42 ↔ 過剰でない |
| 272 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV` | OK | 175 / 175 | 175 / 175 ○ | ○ | 通る | 不足 16 / 42 ↔ 過剰でない |
| 273 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic` | OK | 177 / 177 | 177 / 177 ○ | ○ | 通る | 不足 16 / 42 ↔ 過剰でない |
| 274 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse` | OK | 251 / 251 | 251 / 251 ○ | ○ | 通る | 不足 9 / 15 ↔ 過剰でない |
| 275 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse` | OK | 425 / 425 | 425 / 425 ○ | ○ | 通る | 不足 21 / 86 ↔ 過剰でない |
| 276 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | OK | 748 / 748 | 748 / 748 ○ | ○ | 通る | 不足 41 / 127 ↔ 過剰でない |
| 277 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R` | OK | 452 / 452 | 452 / 452 ○ | ○ | 通る | 不足 27 / 113 ↔ 過剰でない |
| 278 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL` | OK | 458 / 458 | 458 / 458 ○ | ○ | 通る | 不足 28 / 114 ↔ 過剰でない |
| 279 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV` | OK | 464 / 464 | 464 / 464 ○ | ○ | 通る | 不足 28 / 114 ↔ 過剰でない |
| 280 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | OK | 466 / 466 | 466 / 466 ○ | ○ | 通る | 不足 28 / 114 ↔ 過剰でない |
| 281 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.DiodeCenterTap2Pulse` | OK | 88 / 88 | 88 / 88 ○ | ○ | 通る | 不足 9 / 11 ↔ 過剰でない |
| 282 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_R` | OK | 146 / 146 | 146 / 146 ○ | ○ | 通る | 不足 13 / 37 ↔ 過剰でない |
| 283 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RL` | OK | 152 / 152 | 152 / 152 ○ | ○ | 通る | 不足 14 / 38 ↔ 過剰でない |
| 284 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV` | OK | 158 / 158 | 158 / 158 ○ | ○ | 通る | 不足 14 / 38 ↔ 過剰でない |
| 285 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV_Characteristic` | OK | 160 / 160 | 160 / 160 ○ | ○ | 通る | 不足 14 / 38 ↔ 過剰でない |
| 286 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.DiodeCenterTap2mPulse` | OK | 267 / 267 | 267 / 267 ○ | ○ | 通る | 不足 9 / 15 ↔ 過剰でない |
| 287 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R` | OK | 450 / 450 | 450 / 450 ○ | ○ | 通る | 不足 21 / 101 ↔ 過剰でない |
| 288 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL` | OK | 456 / 456 | 456 / 456 ○ | ○ | 通る | 不足 22 / 102 ↔ 過剰でない |
| 289 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV` | OK | 462 / 462 | 462 / 462 ○ | ○ | 通る | 不足 22 / 102 ↔ 過剰でない |
| 290 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | OK | 464 / 464 | 464 / 464 ○ | ○ | 通る | 不足 22 / 102 ↔ 過剰でない |
| 291 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.DiodeCenterTapmPulse` | OK | 166 / 166 | 166 / 166 ○ | ○ | 通る | 不足 9 / 12 ↔ 過剰でない |
| 292 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R` | OK | 334 / 334 | 334 / 334 ○ | ○ | 通る | 不足 18 / 77 ↔ 過剰でない |
| 293 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL` | OK | 340 / 340 | 340 / 340 ○ | ○ | 通る | 不足 19 / 78 ↔ 過剰でない |
| 294 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV` | OK | 346 / 346 | 346 / 346 ○ | ○ | 通る | 不足 19 / 78 ↔ 過剰でない |
| 295 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic` | OK | 348 / 348 | 348 / 348 ○ | ○ | 通る | 不足 19 / 78 ↔ 過剰でない |
| 296 | `Electrical.PowerConverters.Examples.DCAC.ExampleTemplates.SinglePhaseTwoLevel`（partial） | OK | 162 / 162 | 162 / 162 ○ | ○ | 失敗 | 不足 16 / 40 ↔ （失敗） |
| 297 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | OK | 614 / 614 | 614 / 614 ○ | ○ | 通る | 不足 48 / 122 ↔ 過剰でない |
| 298 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | OK | 1268 / 1268 | 1268 / 1268 ○ | ○ | 通る | 不足 101 / 247 ↔ 過剰でない |
| 299 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM` | OK | 402 / 402 | 402 / 402 ○ | ○ | 通る | 不足 30 / 100 ↔ 過剰でない |
| 300 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_R` | OK | 171 / 171 | 171 / 171 ○ | ○ | 通る | 不足 16 / 40 ↔ 過剰でない |
| 301 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_RL` | OK | 177 / 177 | 177 / 177 ○ | ○ | 通る | 不足 17 / 41 ↔ 過剰でない |
| 302 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | OK | 174 / 174 | 174 / 174 ○ | ○ | 通る | 不足 19 / 43 ↔ 過剰でない |
| 303 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_R` | OK | 116 / 116 | 116 / 116 ○ | ○ | 通る | 不足 12 / 29 ↔ 過剰でない |
| 304 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_RL` | OK | 122 / 122 | 122 / 122 ○ | ○ | 通る | 不足 13 / 30 ↔ 過剰でない |
| 305 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepUp.ChopperStepUp_R` | OK | 116 / 116 | 116 / 116 ○ | ○ | 通る | 不足 12 / 29 ↔ 過剰でない |
| 306 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperBuckBoost`（partial） | OK | 173 / 173 | 173 / 173 ○ | ○ | 失敗 | 不足 19 / 43 ↔ （失敗） |
| 307 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepDown`（partial） | OK | 102 / 102 | 102 / 102 ○ | ○ | 失敗 | 不足 12 / 29 ↔ （失敗） |
| 308 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepUp`（partial） | OK | 102 / 102 | 102 / 102 ○ | ○ | 失敗 | 不足 12 / 29 ↔ （失敗） |
| 309 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.HBridge`（partial） | OK | 213 / 213 | 213 / 213 ○ | ○ | 失敗 | 不足 10 / 68 ↔ （失敗） |
| 310 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | OK | 421 / 421 | 421 / 421 ○ | ○ | 通る | 不足 17 / 80 ↔ 過剰でない |
| 311 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_R` | OK | 222 / 222 | 222 / 222 ○ | ○ | 通る | 不足 10 / 68 ↔ 過剰でない |
| 312 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_RL` | OK | 228 / 228 | 228 / 228 ○ | ○ | 通る | 不足 11 / 69 ↔ 過剰でない |
| 313 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_TrianglePWM_RL` | OK | 233 / 233 | 233 / 233 ○ | ○ | 通る | 不足 13 / 72 ↔ 過剰でない |
| 314 | `Electrical.QuasiStatic.Machines.Examples.TransformerTestbench` | GM-U0002 ED008 | - | 4885 / 4885 ○ | - | 通る | - |
| 315 | `Electrical.QuasiStatic.Polyphase.Examples.BalancingDelta` | GM-U0002 ED008 | - | 1104 / 1104 ○ | - | 通る | - |
| 316 | `Electrical.QuasiStatic.Polyphase.Examples.BalancingStar` | GM-U0002 ED008 | - | 1224 / 1224 ○ | - | 通る | - |
| 317 | `Electrical.QuasiStatic.Polyphase.Examples.TestSensors` | GM-U0002 EF023 | - | 2457 / 2457 ○ | - | 通る | - |
| 318 | `Electrical.QuasiStatic.Polyphase.Examples.UnsymmetricalLoad` | GM-U0002 ED008 | - | 1730 / 1730 ○ | - | 通る | - |
| 319 | `Electrical.QuasiStatic.SinglePhase.Examples.MultipleResonance` | GM-U0002 ED008 | - | 199 / 199 ○ | - | 通る | - |
| 320 | `Electrical.QuasiStatic.SinglePhase.Examples.ParallelResonance` | GM-U0002 ED008 | - | 134 / 134 ○ | - | 通る | - |
| 321 | `Electrical.QuasiStatic.SinglePhase.Examples.Rectifier` | GM-U0002 ED008 | - | 251 / 251 ○ | - | 通る | - |
| 322 | `Electrical.QuasiStatic.SinglePhase.Examples.SeriesBode` | GM-U0002 ED008 | - | 151 / 151 ○ | - | 通る | - |
| 323 | `Electrical.QuasiStatic.SinglePhase.Examples.SeriesResonance` | GM-U0002 ED008 | - | 134 / 134 ○ | - | 通る | - |
| 324 | `Electrical.QuasiStatic.SinglePhase.Examples.Transformer` | GM-U0002 ED008 | - | 246 / 246 ○ | - | 通る | - |
| 325 | `Electrical.Spice3.Examples.CascodeCircuit` | GM-U0001 ED019 | - | 68 / 68 ○ | - | 通る | - |
| 326 | `Electrical.Spice3.Examples.CoupledInductors` | OK | 103 / 103 | 103 / 103 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 327 | `Electrical.Spice3.Examples.FourInverters` | GM-U0001 ED019 | - | 288 / 288 ○ | - | 通る | - |
| 328 | `Electrical.Spice3.Examples.Graetz` | GM-U0001 ED019 | - | 63 / 63 ○ | - | 通る | - |
| 329 | `Electrical.Spice3.Examples.Inverter` | GM-U0001 ED019 | - | 80 / 80 ○ | - | 通る | - |
| 330 | `Electrical.Spice3.Examples.InvertersApartRecord` | GM-U0001 ED019 | - | 154 / 154 ○ | - | 通る | - |
| 331 | `Electrical.Spice3.Examples.InvertersExtendedModel` | GM-U0001 ED019 | - | 154 / 154 ○ | - | 通る | - |
| 332 | `Electrical.Spice3.Examples.Nand` | GM-U0001 ED019 | - | 146 / 146 ○ | - | 通る | - |
| 333 | `Electrical.Spice3.Examples.Nor` | GM-U0001 ED019 | - | 149 / 149 ○ | - | 通る | - |
| 334 | `Electrical.Spice3.Examples.Oscillator` | GM-U0001 ED019 | - | 131 / 131 ○ | - | 通る | - |
| 335 | `Electrical.Spice3.Examples.Spice3BenchmarkDifferentialPair` | GM-U0001 ED019 | - | 157 / 157 ○ | - | 通る | - |
| 336 | `Electrical.Spice3.Examples.Spice3BenchmarkFourBitBinaryAdder` | GM-U0001 ED019 | - | 10307 / 10307 ○ | - | 通る | - |
| 337 | `Electrical.Spice3.Examples.Spice3BenchmarkMosfetCharacterization` | GM-U0001 ED019 | - | 64 / 64 ○ | - | 通る | - |
| 338 | `Electrical.Spice3.Examples.Spice3BenchmarkRtlInverter` | GM-U0001 ED019 | - | 76 / 76 ○ | - | 通る | - |
| 339 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | OK | 2389 / 2389 | 2389 / 2389 ○ | ○ | 通る | 不足 237 / 672 ↔ 過剰でない |
| 340 | `Fluid.Examples.AST_BatchPlant.Test.OneTank` | OK | 126 / 126 | 126 / 126 ○ | ○ | 通る | 不足 6 / 11 ↔ 過剰でない |
| 341 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe1` | OK | 145 / 145 | 145 / 145 ○ | ○ | 通る | 不足 6 / 10 ↔ 過剰でない |
| 342 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe2` | OK | 164 / 164 | 164 / 164 ○ | ○ | 通る | 不足 8 / 10 ↔ 過剰でない |
| 343 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe1` | OK | 256 / 256 | 256 / 256 ○ | ○ | 通る | 不足 14 / 18 ↔ 過剰でない |
| 344 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe2` | OK | 315 / 315 | 315 / 315 ○ | ○ | 通る | 不足 18 / 26 ↔ 過剰でない |
| 345 | `Fluid.Examples.AST_BatchPlant.Test.TwoTanks` | OK | 120 / 120 | 120 / 120 ○ | ○ | 通る | 不足 6 / 10 ↔ 過剰でない |
| 346 | `Fluid.Examples.BranchingDynamicPipes` | GM-U0002 ED008 | - | 2025 / 2025 ○ | - | 通る | - |
| 347 | `Fluid.Examples.ControlledTankSystem.ControlledTanks` | OK | 527 / 527 | 527 / 527 ○ | ○ | 通る | 不足 111 / 256 ↔ 過剰でない |
| 348 | `Fluid.Examples.DrumBoiler.DrumBoiler` | OK | 152 / 152 | 152 / 152 ○ | ○ | 通る | 不足 7 / 20 ↔ 過剰でない |
| 349 | `Fluid.Examples.Explanatory.MeasuringTemperature` | OK | 462 / 462 | 462 / 462 ○ | ○ | 通る | 不足 0 / 48 ↔ 過剰でない |
| 350 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | OK | 256 / 256 | 256 / 256 ○ | ○ | 通る | 不足 0 / 24 ↔ 過剰でない |
| 351 | `Fluid.Examples.HeatExchanger.HeatExchangerSimulation` | GM-U0001 ED019 | - | 3731 / 3731 ○ | - | 通る | - |
| 352 | `Fluid.Examples.HeatingSystem` | OK | 538 / 538 | 538 / 538 ○ | ○ | 通る | 不足 8 / 21 ↔ 過剰でない |
| 353 | `Fluid.Examples.IncompressibleFluidNetwork` | GM-U0001 ED019 | - | 1492 / 1492 ○ | - | 通る | - |
| 354 | `Fluid.Examples.InverseParameterization` | OK | 279 / 279 | 287 / 287 ○ | ○ | 通る | 不足 1 / 20 ↔ 過剰でない |
| 355 | `Fluid.Examples.NonCircularPipes` | OK | 391 / 391 | 391 / 391 ○ | ○ | 通る | 不足 4 / 40 ↔ 過剰 2 |
| 356 | `Fluid.Examples.PumpingSystem` | OK | 307 / 307 | 307 / 307 ○ | ○ | 通る | 不足 6 / 36 ↔ 過剰でない |
| 357 | `Fluid.Examples.Tanks.EmptyTanks` | OK | 138 / 138 | 138 / 138 ○ | ○ | 通る | 不足 2 / 10 ↔ 過剰でない |
| 358 | `Fluid.Examples.Tanks.TanksWithOverflow` | OK | 271 / 271 | 271 / 271 ○ | ○ | 通る | 不足 5 / 20 ↔ 過剰でない |
| 359 | `Fluid.Examples.Tanks.ThreeTanks` | OK | 261 / 261 | 261 / 261 ○ | ○ | 通る | 不足 6 / 18 ↔ 過剰でない |
| 360 | `Fluid.Examples.TraceSubstances.RoomCO2` | GM-U0002 ED008 | - | 228 / 228 ○ | - | 通る | - |
| 361 | `Fluid.Examples.TraceSubstances.RoomCO2WithControls` | GM-U0001 ED019 | - | 829 / 829 ○ | - | 通る | - |
| 362 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | OK | 128 / 128 | 128 / 128 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 363 | `Magnetic.FluxTubes.Examples.BasicExamples.SaturatedInductor` | OK | 67 / 67 | 67 / 67 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 364 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 365 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 0 / 0 ↔ 過剰でない |
| 366 | `Magnetic.FluxTubes.Examples.Hysteresis.Components.Transformer1PhaseWithHysteresis` | GM-U0001 ED019 | - | 109 / 109 ○ | - | 特異 | - |
| 367 | `Magnetic.FluxTubes.Examples.Hysteresis.Components.Transformer3PhaseYyWithHysteresis` | GM-U0001 EI035 | - | 315 / 315 ○ | - | 特異 | - |
| 368 | `Magnetic.FluxTubes.Examples.Hysteresis.HysteresisModelComparison` | GM-U0001 ED019 | - | 404 / 404 ○ | - | 通る | - |
| 369 | `Magnetic.FluxTubes.Examples.Hysteresis.InductorWithHysteresis` | GM-U0001 ED019 | - | 63 / 63 ○ | - | 通る | - |
| 370 | `Magnetic.FluxTubes.Examples.Hysteresis.SinglePhaseTransformerWithHysteresis1` | GM-U0001 ED019 | - | 89 / 89 ○ | - | 通る | - |
| 371 | `Magnetic.FluxTubes.Examples.Hysteresis.SinglePhaseTransformerWithHysteresis2` | GM-U0001 ED019 | - | 138 / 138 ○ | - | 通る | - |
| 372 | `Magnetic.FluxTubes.Examples.Hysteresis.ThreePhaseTransformerWithRectifier` | GM-U0001 ED019 | - | 490 / 490 ○ | - | 通る | - ↔ 過剰 2 |
| 373 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ArmatureStroke` | OK | 249 / 249 | 249 / 249 ○ | ○ | 通る | 不足 6 / 10 ↔ 過剰でない |
| 374 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.ConstantActuator` | NG S6 GM-E6001 | 84 / 84 | 84 / 84 ○ | ○ | 特異 | - |
| 375 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.PermeanceActuator` | NG S6 GM-E6001 | 133 / 133 | 133 / 133 ○ | ○ | 特異 | - |
| 376 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | OK | 246 / 246 | 246 / 246 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 377 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | OK | 153 / 153 | 153 / 153 ○ | ○ | 通る | 不足 0 / 2 ↔ 過剰でない |
| 378 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonPullInStroke` | OK | 532 / 532 | 532 / 532 ○ | ○ | 通る | 不足 11 / 15 ↔ 過剰でない |
| 379 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonQuasiStatic` | OK | 531 / 531 | 531 / 531 ○ | ○ | 通る | 不足 9 / 13 ↔ 過剰でない |
| 380 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | OK | 292 / 292 | 292 / 292 ○ | ○ | 特異 | 不足 5 / 7 ↔ （失敗） |
| 381 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.SimpleSolenoid` | NG S6 GM-E6001 | 202 / 202 | 202 / 202 ○ | ○ | 特異 | - |
| 382 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_CommonLeakage` | GM-U0003 ED001 | - | 3573 / 3573 ○ | - | 通る | - |
| 383 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_Polyphase` | GM-U0003 ED001 | - | 2135 / 2135 ○ | - | 通る | - |
| 384 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMS_Start_Polyphase` | GM-U0003 ED001 | - | 2681 / 2681 ○ | - | 通る | - |
| 385 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Conveyor` | GM-U0003 ED001 | - | 843 / 843 ○ | - | 通る | - |
| 386 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_DOL` | GM-U0003 ED001 | - | 1348 / 1348 ○ | - | 通る | - |
| 387 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Initialize` | GM-U0003 ED001 | - | 819 / 819 ○ | - | 通る | - |
| 388 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Inverter` | GM-U0003 ED001 | - | 822 / 822 ○ | - | 通る | - |
| 389 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Steinmetz` | GM-U0003 ED001 | - | 844 / 844 ○ | - | 通る | - |
| 390 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Transformer` | GM-U0003 ED001 | - | 1287 / 1287 ○ | - | 通る | - |
| 391 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_YD` | GM-U0003 ED001 | - | 1011 / 1011 ○ | - | 通る | - |
| 392 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_withLosses` | GM-U0003 ED001 | - | 877 / 877 ○ | - | 通る | - |
| 393 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Start` | GM-U0003 ED001 | - | 1778 / 1778 ○ | - | 通る | - |
| 394 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMEE_Generator_Polyphase` | GM-U0003 ED001 | - | 2293 / 2293 ○ | - | 通る | - |
| 395 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMPM_Inverter_Polyphase` | GM-U0003 ED001 | - | 1932 / 1932 ○ | - | 通る | - |
| 396 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMR_Inverter_Polyphase` | GM-U0003 ED001 | - | 1854 / 1854 ○ | - | 通る | - |
| 397 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_DOL` | GM-U0003 ED001 | - | 1051 / 1051 ○ | - | 通る | - |
| 398 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Generator` | GM-U0003 ED001 | - | 1496 / 1496 ○ | - | 通る | - |
| 399 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_LoadDump` | GM-U0003 ED001 | - | 1076 / 1076 ○ | - | 通る | - |
| 400 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Rectifier` | GM-U0003 ED001 | - | 958 / 958 ○ | - | 通る | - |
| 401 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Braking` | GM-U0003 ED001 | - | 818 / 818 ○ | - | 通る | - |
| 402 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_CurrentSource` | GM-U0003 ED001 | - | 826 / 826 ○ | - | 通る | - |
| 403 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Inverter` | GM-U0003 ED001 | - | 1413 / 1413 ○ | - | 通る | - |
| 404 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_VoltageSource` | GM-U0003 ED001 | - | 911 / 911 ○ | - | 通る | - |
| 405 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMR_Inverter` | GM-U0003 ED001 | - | 1347 / 1347 ○ | - | 通る | - |
| 406 | `Magnetic.FundamentalWave.Examples.Components.EddyCurrentLosses` | GM-U0003 ED001 | - | 689 / 689 ○ | - | 通る | - |
| 407 | `Magnetic.FundamentalWave.Examples.Components.PolyphaseInductance` | GM-U0003 ED001 | - | 588 / 588 ○ | - | 通る | - |
| 408 | `Magnetic.FundamentalWave.Examples.Components.SinglePhaseInductance` | GM-U0003 ED001 | - | 84 / 84 ○ | - | 通る | - |
| 409 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | GM-U0002 ED008 | - | 328 / 328 ○ | - | 通る | - |
| 410 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | GM-U0002 ED008 | - | 212 / 212 ○ | - | 通る | - |
| 411 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | GM-U0002 ED008 | - | 212 / 212 ○ | - | 通る | - |
| 412 | `Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CuboidSections` | GM-U0003 ED001 | - | 188 / 188 ○ | - | 通る | - |
| 413 | `Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CylinderSections` | GM-U0001 ED019 | - | 170 / 170 ○ | - | 通る | - |
| 414 | `Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.CylinderLeakage` | GM-U0001 ED019 | - | 249 / 249 ○ | - | 通る | - |
| 415 | `Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.GeneralLeakage` | GM-U0002 ED008 | - | 81 / 81 ○ | - | 通る | - |
| 416 | `Magnetic.QuasiStatic.FluxTubes.Examples.LinearInductor` | GM-U0002 ED008 | - | 282 / 282 ○ | - | 通る | - |
| 417 | `Magnetic.QuasiStatic.FluxTubes.Examples.NonLinearInductor` | GM-U0002 ED008 | - | 371 / 371 ○ | - | 通る | - |
| 418 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Characteristics` | GM-U0002 EF023 | - | 2620 / 2620 ○ | - | 通る | - |
| 419 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Conveyor` | GM-U0002 EF023 | - | 3019 / 3019 ○ | - | 通る | - |
| 420 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_DOL` | GM-U0002 EF023 | - | 3921 / 3921 ○ | - | 通る | - |
| 421 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Initialize` | GM-U0002 EF023 | - | 2952 / 2952 ○ | - | 通る | - |
| 422 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Inverter` | GM-U0002 EF023 | - | 2107 / 2107 ○ | - | 通る | - |
| 423 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Transformer` | GM-U0002 EF023 | - | 5818 / 5818 ○ | - | 通る | - |
| 424 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_YD` | GM-U0002 EF023 | - | 4252 / 4252 ○ | - | 通る | - |
| 425 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_withLosses` | GM-U0001 EF004 | - | 2685 / 2685 ○ | - | 通る | - |
| 426 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Characteristics` | GM-U0002 EF023 | - | 3035 / 3035 ○ | - | 通る | - |
| 427 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Start` | GM-U0002 EF023 | - | 5030 / 5030 ○ | - | 通る | - |
| 428 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Generator` | GM-U0002 EF023 | - | 3043 / 3043 ○ | - | 通る | - |
| 429 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_CurrentSource` | GM-U0002 EF023 | - | 3414 / 3414 ○ | - | 通る | - |
| 430 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_FieldWeakening` | GM-U0002 EF023 | - | 1903 / 1903 ○ | - | 通る | - |
| 431 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_MTPA` | GM-U0002 EF023 | - | 2599 / 2599 ○ | - | 通る | - |
| 432 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Mains` | GM-U0002 EF023 | - | 3330 / 3330 ○ | - | 通る | - |
| 433 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_OpenCircuit` | GM-U0002 EF023 | - | 1994 / 1994 ○ | - | 通る | - |
| 434 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMR_CurrentSource` | GM-U0002 EF023 | - | 3329 / 3329 ○ | - | 通る | - |
| 435 | `Magnetic.QuasiStatic.FundamentalWave.Examples.Components.EddyCurrentLosses` | GM-U0002 ED008 | - | 2497 / 2497 ○ | - | 通る | - |
| 436 | `Magnetic.QuasiStatic.FundamentalWave.Examples.Components.PolyphaseInductance` | GM-U0002 ED008 | - | 2549 / 2549 ○ | - | 通る | - |
| 437 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.FieldWeakeningController` | GM-U0001 ED020 | - | 49 / 49 ○ | - | 失敗 | - |
| 438 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | OK | 1 / 1 | 3 / 3 ○ | ○ | 失敗 | 合う 0 / 0 ↔ （失敗） |
| 439 | `Math.FastFourierTransform.Examples.RealFFT1` | GM-U0003 ET004 | - | 245 / 245 ○ | - | 通る | - |
| 440 | `Math.FastFourierTransform.Examples.RealFFT2` | GM-U0003 ET004 | - | 224 / 224 ○ | - | 通る | - |
| 441 | `Math.Nonlinear.Examples.QuadratureLobatto3` | GM-U0001 ED020 | - | 3 / 3 ○ | - | 通る | - |
| 442 | `Math.Random.Examples.GenerateRandomNumbers` | GM-U0001 ED019 | - | 44 / 44 ○ | - | 通る | - |
| 443 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | OK | 2429 / 2429 | 2429 / 2429 ○ | ○ | 通る | 不足 8 / 124 ↔ 過剰でない |
| 444 | `Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint` | OK | 2312 / 2312 | 2312 / 2312 ○ | ○ | 通る | 不足 4 / 108 ↔ 過剰でない |
| 445 | `Mechanics.MultiBody.Examples.Constraints.SphericalConstraint` | OK | 2323 / 2323 | 2323 / 2323 ○ | ○ | 通る | 不足 12 / 116 ↔ 過剰でない |
| 446 | `Mechanics.MultiBody.Examples.Constraints.UniversalConstraint` | OK | 2598 / 2598 | 2598 / 2598 ○ | ○ | 通る | 不足 8 / 124 ↔ 過剰でない |
| 447 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulum` | OK | 1302 / 1302 | 1302 / 1302 ○ | ○ | 通る | 不足 4 / 61 ↔ 過剰でない |
| 448 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | OK | 1479 / 1479 | 1479 / 1479 ○ | ○ | 通る | 不足 4 / 73 ↔ 過剰でない |
| 449 | `Mechanics.MultiBody.Examples.Elementary.ForceAndTorque` | OK | 1541 / 1541 | 1541 / 1541 ○ | ○ | 通る | 不足 4 / 77 ↔ 過剰でない |
| 450 | `Mechanics.MultiBody.Examples.Elementary.FreeBody` | OK | 1330 / 1330 | 1330 / 1330 ○ | ○ | 通る | 不足 12 / 81 ↔ 過剰でない |
| 451 | `Mechanics.MultiBody.Examples.Elementary.HeatLosses` | OK | 2151 / 2151 | 2151 / 2151 ○ | ○ | 通る | 不足 37 / 139 ↔ 過剰でない |
| 452 | `Mechanics.MultiBody.Examples.Elementary.InitSpringConstant` | OK | 1275 / 1275 | 1275 / 1275 ○ | ○ | 通る | 不足 3 / 63 ↔ 過剰でない |
| 453 | `Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses` | OK | 2303 / 2303 | 2303 / 2303 ○ | ○ | 通る | 不足 4 / 103 ↔ 過剰でない |
| 454 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | OK | 934 / 934 | 934 / 934 ○ | ○ | 通る | 不足 2 / 38 ↔ 過剰でない |
| 455 | `Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper` | OK | 1349 / 1349 | 1349 / 1349 ○ | ○ | 通る | 不足 4 / 64 ↔ 過剰でない |
| 456 | `Mechanics.MultiBody.Examples.Elementary.PointGravity` | OK | 911 / 911 | 911 / 911 ○ | ○ | 通る | 不足 24 / 63 ↔ 過剰でない |
| 457 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses` | OK | 1113 / 1113 | 1113 / 1113 ○ | ○ | 通る | 不足 24 / 75 ↔ 過剰でない |
| 458 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2` | OK | 2860 / 2860 | 2860 / 2860 ○ | ○ | 通る | 不足 24 / 149 ↔ 過剰でない |
| 459 | `Mechanics.MultiBody.Examples.Elementary.RollingWheel` | OK | 947 / 947 | 947 / 947 ○ | ○ | 通る | 不足 8 / 44 ↔ 過剰でない |
| 460 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving` | OK | 1959 / 1959 | 1959 / 1959 ○ | ○ | 通る | 不足 14 / 88 ↔ 過剰でない |
| 461 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling` | OK | 2047 / 2047 | 2047 / 2047 ○ | ○ | 通る | 不足 16 / 93 ↔ 過剰でない |
| 462 | `Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem` | OK | 1686 / 1686 | 1686 / 1686 ○ | ○ | 通る | 不足 14 / 92 ↔ 過剰でない |
| 463 | `Mechanics.MultiBody.Examples.Elementary.SpringMassSystem` | OK | 1469 / 1469 | 1469 / 1469 ○ | ○ | 通る | 不足 4 / 67 ↔ 過剰でない |
| 464 | `Mechanics.MultiBody.Examples.Elementary.SpringWithMass` | OK | 968 / 968 | 968 / 968 ○ | ○ | 通る | 不足 12 / 57 ↔ 過剰でない |
| 465 | `Mechanics.MultiBody.Examples.Elementary.Surfaces` | OK | 1143 / 1143 | 1143 / 1143 ○ | ○ | 通る | 不足 2 / 41 ↔ 過剰でない |
| 466 | `Mechanics.MultiBody.Examples.Elementary.ThreeSprings` | OK | 1516 / 1516 | 1516 / 1516 ○ | ○ | 通る | 不足 12 / 87 ↔ 過剰でない |
| 467 | `Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField` | OK | 951 / 951 | 951 / 951 ○ | ○ | 通る | 不足 2 / 41 ↔ 過剰でない |
| 468 | `Mechanics.MultiBody.Examples.Loops.Engine1a` | OK | 2386 / 2386 | 2386 / 2386 ○ | ○ | 通る | 不足 2 / 119 ↔ 過剰でない |
| 469 | `Mechanics.MultiBody.Examples.Loops.Engine1b` | OK | 2488 / 2488 | 2488 / 2488 ○ | ○ | 通る | 不足 2 / 125 ↔ 過剰でない |
| 470 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | OK | 2752 / 2752 | 2752 / 2752 ○ | ○ | 通る | 不足 7 / 154 ↔ 過剰でない |
| 471 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | OK | 11107 / 11107 | 11107 / 11107 ○ | ○ | 通る | 不足 4 / 619 ↔ 過剰でない |
| 472 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | OK | 9001 / 9001 | 9001 / 9001 ○ | ○ | 通る | 不足 34 / 592 ↔ 過剰でない |
| 473 | `Mechanics.MultiBody.Examples.Loops.Fourbar1` | OK | 2058 / 2058 | 2058 / 2058 ○ | ○ | 通る | 不足 2 / 89 ↔ 過剰でない |
| 474 | `Mechanics.MultiBody.Examples.Loops.Fourbar2` | OK | 2242 / 2242 | 2242 / 2242 ○ | ○ | 通る | 不足 3 / 90 ↔ 過剰でない |
| 475 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | OK | 1743 / 1743 | 1743 / 1743 ○ | ○ | 通る | 不足 3 / 87 ↔ 過剰でない |
| 476 | `Mechanics.MultiBody.Examples.Loops.PlanarFourbar` | OK | 2348 / 2348 | 2348 / 2348 ○ | ○ | 通る | 不足 2 / 92 ↔ 過剰でない |
| 477 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | OK | 4012 / 4012 | 4012 / 4012 ○ | ○ | 通る | 不足 17 / 185 ↔ 過剰でない |
| 478 | `Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive` | OK | 1413 / 1413 | 1413 / 1413 ○ | ○ | 通る | 不足 4 / 69 ↔ 過剰でない |
| 479 | `Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D` | OK | 1750 / 1750 | 1750 / 1750 ○ | ○ | 通る | 不足 8 / 71 ↔ 過剰でない |
| 480 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint` | OK | 1731 / 1731 | 1731 / 1731 ○ | ○ | 通る | 不足 4 / 81 ↔ 過剰でない |
| 481 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects` | OK | 2276 / 2276 | 2276 / 2276 ○ | ○ | 通る | 不足 16 / 121 ↔ 過剰でない |
| 482 | `Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive` | OK | 1736 / 1736 | 1736 / 1736 ○ | ○ | 通る | 不足 8 / 79 ↔ 過剰でない |
| 483 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | OK | 4570 / 4570 | 4570 / 4570 ○ | ○ | 通る | 不足 54 / 315 ↔ 過剰でない |
| 484 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | OK | 391 / 391 | 391 / 391 ○ | ○ | 通る | 不足 10 / 25 ↔ 過剰でない |
| 485 | `Mechanics.Rotational.Examples.Backlash` | OK | 62 / 62 | 62 / 62 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 486 | `Mechanics.Rotational.Examples.CompareBrakingTorque` | OK | 52 / 52 | 52 / 52 ○ | ○ | 通る | 合う 8 / 8 ↔ 過剰でない |
| 487 | `Mechanics.Rotational.Examples.CoupledClutches` | GM-U0003 ED001 | - | 112 / 112 ○ | - | 通る | - |
| 488 | `Mechanics.Rotational.Examples.EddyCurrentBrake` | OK | 25 / 25 | 25 / 25 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 489 | `Mechanics.Rotational.Examples.ElasticBearing` | OK | 63 / 63 | 63 / 63 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 490 | `Mechanics.Rotational.Examples.First` | OK | 54 / 54 | 54 / 54 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 491 | `Mechanics.Rotational.Examples.FirstGrounded` | OK | 50 / 50 | 50 / 50 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 492 | `Mechanics.Rotational.Examples.Friction` | GM-U0003 ED001 | - | 102 / 102 ○ | - | 通る | - |
| 493 | `Mechanics.Rotational.Examples.GenerationOfFMUs` | GM-U0001 ED019 | - | 214 / 214 ○ | - | 通る | - |
| 494 | `Mechanics.Rotational.Examples.HeatLosses` | GM-U0003 ED001 | - | 248 / 248 ○ | - | 通る | - |
| 495 | `Mechanics.Rotational.Examples.LossyGearDemo1` | GM-U0003 ED001 | - | 76 / 76 ○ | - | 通る | - |
| 496 | `Mechanics.Rotational.Examples.LossyGearDemo2` | GM-U0003 ED001 | - | 100 / 100 ○ | - | 通る | - |
| 497 | `Mechanics.Rotational.Examples.LossyGearDemo3` | GM-U0003 ED001 | - | 67 / 67 ○ | - | 通る | - |
| 498 | `Mechanics.Rotational.Examples.OneWayClutch` | GM-U0003 ED001 | - | 43 / 43 ○ | - | 通る | - |
| 499 | `Mechanics.Rotational.Examples.OneWayClutchDisengaged` | GM-U0003 ED001 | - | 46 / 46 ○ | - | 通る | - |
| 500 | `Mechanics.Rotational.Examples.RollingWheel` | OK | 41 / 41 | 41 / 41 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 501 | `Mechanics.Rotational.Examples.SimpleGearShift` | GM-U0003 ED001 | - | 88 / 88 ○ | - | 通る | - |
| 502 | `Mechanics.Translational.Examples.Accelerate` | OK | 15 / 15 | 15 / 15 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 503 | `Mechanics.Translational.Examples.Brake` | GM-U0003 ED001 | - | 73 / 73 ○ | - | 通る | - |
| 504 | `Mechanics.Translational.Examples.CompareBrakingForce` | OK | 52 / 52 | 52 / 52 ○ | ○ | 通る | 合う 8 / 8 ↔ 過剰でない |
| 505 | `Mechanics.Translational.Examples.Damper` | OK | 59 / 59 | 59 / 59 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 506 | `Mechanics.Translational.Examples.EddyCurrentBrake` | OK | 25 / 25 | 25 / 25 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 507 | `Mechanics.Translational.Examples.ElastoGap` | OK | 72 / 72 | 72 / 72 ○ | ○ | 通る | 不足 4 / 6 ↔ 過剰でない |
| 508 | `Mechanics.Translational.Examples.Friction` | GM-U0003 ER130 | - | 90 / 90 ○ | - | 通る | - |
| 509 | `Mechanics.Translational.Examples.GenerationOfFMUs` | OK | 201 / 201 | 201 / 201 ○ | ○ | 通る | 合う 10 / 10 ↔ 過剰でない |
| 510 | `Mechanics.Translational.Examples.HeatLosses` | GM-U0003 ER130 | - | 174 / 174 ○ | - | 通る | - |
| 511 | `Mechanics.Translational.Examples.InitialConditions` | OK | 64 / 64 | 64 / 64 ○ | ○ | 通る | 合う 8 / 8 ↔ 過剰でない |
| 512 | `Mechanics.Translational.Examples.Oscillator` | OK | 50 / 50 | 50 / 50 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 513 | `Mechanics.Translational.Examples.PreLoad` | OK | 120 / 120 | 120 / 120 ○ | ○ | 通る | 不足 6 / 10 ↔ 過剰でない |
| 514 | `Mechanics.Translational.Examples.Sensors` | OK | 38 / 38 | 38 / 38 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 515 | `Mechanics.Translational.Examples.SignConvention` | OK | 41 / 41 | 41 / 41 ○ | ○ | 通る | 合う 6 / 6 ↔ 過剰でない |
| 516 | `Mechanics.Translational.Examples.Vehicle` | OK | 249 / 249 | 249 / 249 ○ | ○ | 通る | 不足 11 / 12 ↔ 過剰でない |
| 517 | `Mechanics.Translational.Examples.WhyArrows` | OK | 56 / 56 | 56 / 56 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 518 | `Media.Examples.IdealGasH2O` | GM-U0002 ED008 | - | 17 / 17 ○ | - | 通る | - |
| 519 | `Media.Examples.MixtureGases` | GM-U0002 EF025 | - | 75 / 75 ○ | - | 通る | - |
| 520 | `Media.Examples.MoistAir` | GM-U0001 ED019 | - | 40 / 40 ○ | - | 通る | - |
| 521 | `Media.Examples.PsychrometricData` | GM-U0001 ED019 | - | 621 / 621 ○ | - | 通る | - |
| 522 | `Media.Examples.R134a.R134a1` | GM-U0001 ED019 | - | 81 / 81 ○ | - | 通る | - |
| 523 | `Media.Examples.R134a.R134a2` | GM-U0001 ED019 | - | 116 / 116 ○ | - | 通る | - |
| 524 | `Media.Examples.ReferenceAir.DryAir1` | GM-U0002 ED008 | - | 66 / 66 ○ | - | 通る | - |
| 525 | `Media.Examples.ReferenceAir.DryAir2` | GM-U0002 ED008 | - | 96 / 96 ○ | - | 通る | - |
| 526 | `Media.Examples.ReferenceAir.Inverse_sh_T` | GM-U0002 ED008 | - | 6 / 6 ○ | - | 通る | - |
| 527 | `Media.Examples.ReferenceAir.Inverse_sh_TX` | GM-U0002 ED008 | - | 6 / 6 ○ | - | 通る | - |
| 528 | `Media.Examples.ReferenceAir.MoistAir` | GM-U0001 ED019 | - | 40 / 40 ○ | - | 通る | - |
| 529 | `Media.Examples.ReferenceAir.MoistAir1` | GM-U0002 ED008 | - | 107 / 107 ○ | - | 通る | - |
| 530 | `Media.Examples.ReferenceAir.MoistAir2` | GM-U0001 ED019 | - | 154 / 154 ○ | - | 通る | - |
| 531 | `Media.Examples.SimpleLiquidWater` | OK | 34 / 34 | 34 / 34 ○ | ○ | 通る | 合う 1 / 1 ↔ 過剰でない |
| 532 | `Media.Examples.SolveOneNonlinearEquation.InverseIncompressible_sh_T` | GM-U0001 ED019 | - | 6 / 6 ○ | - | 通る | - |
| 533 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sh_T` | GM-U0002 ED008 | - | 6 / 6 ○ | - | 通る | - |
| 534 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sh_TX` | GM-U0002 ED008 | - | 6 / 6 ○ | - | 通る | - |
| 535 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sine` | GM-U0001 ED019 | - | 1 / 1 ○ | - | 通る | - |
| 536 | `Media.Examples.TwoPhaseWater.BaseProperties` | OK | 16 / 16 | 18 / 18 ○ | ○ | 通る | 不足 0 / 2 ↔ 過剰でない |
| 537 | `Media.Examples.TwoPhaseWater.ExtendedProperties` | GM-U0001 ED019 | - | 53 / 53 ○ | - | 通る | - |
| 538 | `Media.Examples.TwoPhaseWater.TestTwoPhaseStates` | GM-U0001 ED019 | - | 53 / 53 ○ | - | 通る | - |
| 539 | `Media.Examples.WaterIF97` | OK | 34 / 34 | 34 / 34 ○ | ○ | 通る | 不足 3 / 7 ↔ 過剰でない |
| 540 | `Media.Incompressible.Examples.TestGlycol` | GM-U0001 ED019 | - | 20 / 20 ○ | - | 通る | - |
| 541 | `StateGraph.Examples.ControlledTanks` | OK | 319 / 319 | 319 / 319 ○ | ○ | 通る | 不足 113 / 252 ↔ 過剰でない |
| 542 | `StateGraph.Examples.ExecutionPaths` | OK | 243 / 243 | 243 / 243 ○ | ○ | 通る | 不足 84 / 203 ↔ 過剰でない |
| 543 | `StateGraph.Examples.FirstExample` | OK | 55 / 55 | 55 / 55 ○ | ○ | 通る | 不足 20 / 45 ↔ 過剰でない |
| 544 | `StateGraph.Examples.FirstExample_Variant2` | OK | 60 / 60 | 60 / 60 ○ | ○ | 通る | 不足 22 / 48 ↔ 過剰でない |
| 545 | `StateGraph.Examples.FirstExample_Variant3` | OK | 60 / 60 | 60 / 60 ○ | ○ | 通る | 不足 22 / 49 ↔ 過剰でない |
| 546 | `StateGraph.Examples.ShowCompositeStep` | OK | 261 / 261 | 261 / 261 ○ | ○ | 通る | 不足 87 / 221 ↔ 過剰でない |
| 547 | `StateGraph.Examples.ShowExceptions` | OK | 299 / 299 | 299 / 299 ○ | ○ | 通る | 不足 95 / 249 ↔ 過剰でない |
| 548 | `Thermal.FluidHeatFlow.Examples.IndirectCooling` | OK | 169 / 169 | 169 / 169 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 549 | `Thermal.FluidHeatFlow.Examples.OneMass` | OK | 79 / 79 | 79 / 79 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 550 | `Thermal.FluidHeatFlow.Examples.ParallelCooling` | OK | 144 / 144 | 144 / 144 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 551 | `Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut` | OK | 149 / 149 | 149 / 149 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 552 | `Thermal.FluidHeatFlow.Examples.PumpAndValve` | OK | 111 / 111 | 111 / 111 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 553 | `Thermal.FluidHeatFlow.Examples.PumpDropOut` | OK | 85 / 85 | 85 / 85 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 554 | `Thermal.FluidHeatFlow.Examples.SimpleCooling` | OK | 80 / 80 | 80 / 80 ○ | ○ | 通る | 合う 2 / 2 ↔ 過剰でない |
| 555 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | OK | 61 / 61 | 61 / 61 ○ | ○ | 通る | 合う 7 / 7 ↔ 過剰でない |
| 556 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | OK | 47 / 47 | 47 / 47 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 557 | `Thermal.FluidHeatFlow.Examples.TwoMass` | OK | 137 / 137 | 137 / 137 ○ | ○ | 通る | 合う 5 / 5 ↔ 過剰でない |
| 558 | `Thermal.FluidHeatFlow.Examples.TwoTanks` | OK | 50 / 50 | 50 / 50 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 559 | `Thermal.FluidHeatFlow.Examples.WaterPump` | OK | 127 / 127 | 127 / 127 ○ | ○ | 通る | 不足 3 / 4 ↔ 過剰でない |
| 560 | `Thermal.HeatTransfer.Examples.ControlledTemperature` | OK | 52 / 52 | 52 / 52 ○ | ○ | 通る | 不足 2 / 6 ↔ 過剰でない |
| 561 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | OK | 102 / 102 | 102 / 102 ○ | ○ | 通る | 過剰 4 / 3（確定できない） ↔ 過剰 1 |
| 562 | `Thermal.HeatTransfer.Examples.Motor` | OK | 41 / 41 | 41 / 41 ○ | ○ | 通る | 合う 4 / 4 ↔ 過剰でない |
| 563 | `Thermal.HeatTransfer.Examples.TwoMasses` | OK | 20 / 20 | 20 / 20 ○ | ○ | 通る | 合う 3 / 3 ↔ 過剰でない |
| 564 | `Utilities.Examples.ReadRealMatrixFromFile` | GM-U0002 EF025 | - | 1 / 1 ○ | - | 失敗 | - |
| 565 | `Utilities.Examples.WriteRealMatrixToFile` | OK | 4 / 4 | 4 / 4 ○ | ○ | 通る | 不足 0 / 4 ↔ 過剰でない |
| 566 | `Utilities.Examples.readRealParameterModel` | GM-U0002 EF025 | - | 0 / 0 ○ | - | 通る | - |
