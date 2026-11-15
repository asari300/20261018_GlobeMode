# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 0 本。

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 77 s（1 本ずつの検査 65 s） |
| 最大の常駐メモリ（VmHWM） | 1.1 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 467 | 566 |
| flat | 自前で平らにできた | 192 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 189 | 192 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 189 | 189 |
| flat | 方程式の数が Rumoca の flat と一致 | 186 | 189 |
| dae | 自前で DAE にできた | 192 | 192 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 146 | 192 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 144 | 146 |
| dae | 方程式と未知数の数が一致 | 144 | 146 |
| dae | 状態の数が一致 | 141 | 146 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 132 | 146 |

未解決の参照の総数: 1330

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 99 | `Modelica.Blocks.Examples.Noise.ActuatorWithNoise` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `redeclare` | 263 | `Modelica.Blocks.Examples.Noise.ActuatorWithNoise` |
| `composite-binding` | 41 | `Modelica.Blocks.Examples.Interaction1` |
| `array-type` | 13 | `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` |
| `unsized-array` | 13 | `Modelica.Blocks.Examples.BooleanNetwork1` |
| `class-extends` | 11 | `Modelica.Media.Examples.IdealGasH2O` |
| `replaceable` | 9 | `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` |
| `connect` | 7 | `Modelica.StateGraph.Examples.ControlledTanks` |
| `connect-array` | 7 | `Modelica.Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` |
| `conditional-component` | 6 | `Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks` |
| `expandable` | 2 | `Modelica.Blocks.Examples.BusUsage` |
| `non-constant-dim` | 2 | `Modelica.Math.FastFourierTransform.Examples.RealFFT1` |

## GlobeModeValidation（75 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 75 | 75 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 74 | 75 |
| flat | 自前で平らにできた | 72 | 75 |
| flat | Rumoca も flat まで通った（比べられた本数） | 71 | 72 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 71 | 71 |
| flat | 方程式の数が Rumoca の flat と一致 | 71 | 71 |
| dae | 自前で DAE にできた | 72 | 72 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 71 | 72 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 71 | 71 |
| dae | 方程式と未知数の数が一致 | 71 | 71 |
| dae | 状態の数が一致 | 71 | 71 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 70 | 70 |

未解決の参照の総数: 13

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 1 | `GlobeModeValidation.Connect.MediumFlow` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `redeclare` | 3 | `GlobeModeValidation.Components.FluidBoundary` |

## 一致しなかったもの（先頭 100 件。全部は JSON）

- `Modelica.Clocked.Examples.CascadeControlledDrive.AbsoluteClocks`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.Continuous`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.SubClocked`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.CascadeControlledDrive.SuperSampled`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClock`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClockVectorized`: dae: 正則 自前 false / rumoca true
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
- `Modelica.Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController`: dae: 正則 自前 false / rumoca true
- `Modelica.Clocked.Examples.Systems.EngineThrottleControl`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Analog.Examples.GenerationOfFMUs`: dae: eq 306 vs 306, unk 306 vs 306 (rumoca); states 32 vs 10, discrete 0 vs 0
- `Modelica.Mechanics.Rotational.Examples.GenerationOfFMUs`: flat: eq 210 vs 214 (rumoca); vars 266 vs 266; only mine []; only rumoca []
- `Modelica.Mechanics.Translational.Examples.GenerationOfFMUs`: dae: eq 201 vs 201, unk 201 vs 201 (rumoca); states 33 vs 17, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.PumpAndValve`: dae: eq 111 vs 111, unk 111 vs 111 (rumoca); states 3 vs 4, discrete 0 vs 0
- `Modelica.Thermal.FluidHeatFlow.Examples.WaterPump`: dae: eq 127 vs 127, unk 127 vs 127 (rumoca); states 2 vs 3, discrete 3 vs 3
- `Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs`: dae: eq 102 vs 102, unk 102 vs 102 (rumoca); states 16 vs 5, discrete 0 vs 0

