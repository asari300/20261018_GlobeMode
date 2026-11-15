# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果が無かったので、差は書かない。

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 317 s（1 本ずつの検査 296 s） |
| 最大の常駐メモリ（VmHWM） | 1.2 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 467 | 566 |
| flat | 自前で平らにできた | 279 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 273 | 279 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 273 | 273 |
| flat | 方程式の数が Rumoca の flat と一致 | 254 | 273 |
| dae | 自前で DAE にできた | 279 | 279 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 201 | 279 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 191 | 201 |
| dae | 方程式と未知数の数が一致 | 191 | 201 |
| dae | 状態の数が一致 | 185 | 201 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 179 | 201 |

未解決の参照の総数: 1330

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 99 | `Modelica.Blocks.Examples.Noise.ActuatorWithNoise` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `composite-binding` | 155 | `Modelica.Blocks.Examples.Interaction1` |
| `connect-array` | 72 | `Modelica.Blocks.Examples.Rectifier12pulseFFT` |
| `unsized-array` | 20 | `Modelica.Blocks.Examples.BooleanNetwork1` |
| `conditional-component` | 19 | `Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks` |
| `array-modification` | 8 | `Modelica.Electrical.QuasiStatic.Machines.Examples.TransformerTestbench` |
| `connect` | 8 | `Modelica.Fluid.Examples.ControlledTankSystem.ControlledTanks` |
| `expandable` | 2 | `Modelica.Blocks.Examples.BusUsage` |
| `non-constant-dim` | 2 | `Modelica.Math.FastFourierTransform.Examples.RealFFT1` |
| `stream` | 1 | `Modelica.Fluid.Examples.DrumBoiler.DrumBoiler` |

## GlobeModeValidation（75 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 75 | 75 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 74 | 75 |
| flat | 自前で平らにできた | 74 | 75 |
| flat | Rumoca も flat まで通った（比べられた本数） | 73 | 74 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 73 | 73 |
| flat | 方程式の数が Rumoca の flat と一致 | 73 | 73 |
| dae | 自前で DAE にできた | 74 | 74 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 73 | 74 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 73 | 73 |
| dae | 方程式と未知数の数が一致 | 73 | 73 |
| dae | 状態の数が一致 | 73 | 73 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 72 | 72 |

未解決の参照の総数: 13

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 1 | `GlobeModeValidation.Connect.MediumFlow` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `stream` | 1 | `GlobeModeValidation.Connect.MediumFlow` |

## 一致しなかったもの（先頭 100 件。全部は JSON）

- `Modelica.Clocked.Examples.CascadeControlledDrive.AbsoluteClocks`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.Continuous`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.SubClocked`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.SuperSampled`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClock`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClockVectorized`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.SampleWithADeffects`: flat: eq 21 vs 23 (rumoca); vars 50 vs 50; only mine []; only rumoca []
- `Modelica.Clocked.Examples.Elementary.RealSignals.SampleWithADeffects`: dae: eq 19 vs 21, unk 21 vs 21 (rumoca); states 0 vs 0, discrete 4 vs 18
- `Modelica.Clocked.Examples.Elementary.RealSignals.SampleWithADeffects`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`: flat: eq 10 vs 12 (rumoca); vars 20 vs 20; only mine []; only rumoca []
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`: dae: eq 7 vs 9, unk 9 vs 9 (rumoca); states 0 vs 0, discrete 3 vs 7
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star`: flat: eq 10 vs 11 (rumoca); vars 21 vs 21; only mine []; only rumoca []
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star`: dae: eq 7 vs 8, unk 8 vs 8 (rumoca); states 0 vs 0, discrete 2 vs 6
- `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.UpSample1`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.UpSample2`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects`: flat: eq 64 vs 66 (rumoca); vars 120 vs 120; only mine []; only rumoca []
- `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects`: dae: eq 66 vs 68, unk 68 vs 68 (rumoca); states 3 vs 3, discrete 6 vs 47
- `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Systems.EngineThrottleControl`: dae: 正則 自前 false / rumoca true
- `Modelica.ComplexBlocks.Examples.TestConversionBlock`: dae: eq 9 vs 10, unk 10 vs 10 (rumoca); states 0 vs 0, discrete 0 vs 0
- `Modelica.ComplexBlocks.Examples.TestConversionBlock`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Analog.Examples.GenerationOfFMUs`: dae: eq 306 vs 306, unk 306 vs 306 (rumoca); states 32 vs 10, discrete 0 vs 0
- `Modelica.Electrical.Analog.Examples.ResonanceCircuits`: dae: eq 252 vs 252, unk 252 vs 252 (rumoca); states 28 vs 10, discrete 0 vs 0
- `Modelica.Electrical.Batteries.Examples.SuperCapDischargeCharge`: flat: eq 71 vs 72 (rumoca); vars 118 vs 118; only mine []; only rumoca []
- `Modelica.Electrical.Batteries.Examples.SuperCapDischargeCharge`: dae: eq 76 vs 77, unk 77 vs 77 (rumoca); states 2 vs 2, discrete 3 vs 3
- `Modelica.Electrical.Batteries.Examples.SuperCapDischargeCharge`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive`: flat: eq 597 vs 601 (rumoca); vars 997 vs 997; only mine []; only rumoca []
- `Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive`: dae: eq 667 vs 671, unk 671 vs 671 (rumoca); states 34 vs 37, discrete 5 vs 5
- `Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics`: flat: eq 544 vs 548 (rumoca); vars 1095 vs 1095; only mine []; only rumoca []
- `Modelica.Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics`: dae: eq 662 vs 666, unk 666 vs 666 (rumoca); states 38 vs 33, discrete 0 vs 0
- `Modelica.Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.ParallelResonance`: flat: eq 80 vs 75 (rumoca); vars 148 vs 148; only mine []; only rumoca []
- `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.SeriesResonance`: flat: eq 77 vs 75 (rumoca); vars 148 vs 148; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap`: flat: eq 203 vs 197 (rumoca); vars 369 vs 369; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap`: flat: eq 137 vs 132 (rumoca); vars 240 vs 240; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection`: flat: eq 137 vs 132 (rumoca); vars 243 vs 243; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CuboidSections`: flat: eq 112 vs 110 (rumoca); vars 212 vs 212; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CylinderSections`: flat: eq 105 vs 104 (rumoca); vars 198 vs 198; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.CylinderLeakage`: flat: eq 154 vs 150 (rumoca); vars 275 vs 275; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.GeneralLeakage`: flat: eq 52 vs 51 (rumoca); vars 96 vs 96; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.LinearInductor`: flat: eq 204 vs 200 (rumoca); vars 348 vs 348; only mine []; only rumoca []
- `Modelica.Magnetic.QuasiStatic.FluxTubes.Examples.NonLinearInductor`: flat: eq 288 vs 284 (rumoca); vars 529 vs 529; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.GenerationOfFMUs`: flat: eq 210 vs 214 (rumoca); vars 266 vs 266; only mine []; only rumoca []
- `Modelica.Mechanics.Translational.Examples.GenerationOfFMUs`: dae: eq 201 vs 201, unk 201 vs 201 (rumoca); states 33 vs 17, discrete 0 vs 0
- `Modelica.Media.Examples.SimpleLiquidWater`: dae: eq 33 vs 34, unk 36 vs 34 (rumoca); states 4 vs 4, discrete 0 vs 0
- `Modelica.Media.Examples.SimpleLiquidWater`: dae: 正則 自前 false / rumoca true
- `Modelica.Media.Examples.WaterIF97`: dae: eq 26 vs 34, unk 35 vs 34 (rumoca); states 5 vs 5, discrete 4 vs 4
- `Modelica.Media.Examples.WaterIF97`: dae: 正則 自前 false / rumoca true
- `Modelica.Thermal.FluidHeatFlow.Examples.IndirectCooling`: dae: eq 169 vs 169, unk 169 vs 169 (rumoca); states 6 vs 4, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.OneMass`: dae: eq 79 vs 79, unk 79 vs 79 (rumoca); states 3 vs 2, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.ParallelCooling`: dae: eq 144 vs 144, unk 144 vs 144 (rumoca); states 6 vs 5, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut`: dae: eq 149 vs 149, unk 149 vs 149 (rumoca); states 6 vs 5, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.PumpAndValve`: dae: eq 111 vs 111, unk 111 vs 111 (rumoca); states 5 vs 4, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.PumpDropOut`: dae: eq 85 vs 85, unk 85 vs 85 (rumoca); states 3 vs 2, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.SimpleCooling`: dae: eq 80 vs 80, unk 80 vs 80 (rumoca); states 3 vs 2, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.TwoMass`: dae: eq 137 vs 137, unk 137 vs 137 (rumoca); states 6 vs 5, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.TwoTanks`: dae: eq 50 vs 50, unk 50 vs 50 (rumoca); states 5 vs 4, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.WaterPump`: dae: eq 127 vs 127, unk 127 vs 127 (rumoca); states 6 vs 3, discrete 3 vs 3
- `Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs`: dae: eq 102 vs 102, unk 102 vs 102 (rumoca); states 16 vs 5, discrete 0 vs 0

