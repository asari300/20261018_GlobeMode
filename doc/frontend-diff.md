# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 17 本。

- 改善 `Modelica.Clocked.Examples.CascadeControlledDrive.AbsoluteClocks`: regular
- 改善 `Modelica.Clocked.Examples.CascadeControlledDrive.Continuous`: regular
- 改善 `Modelica.Clocked.Examples.CascadeControlledDrive.SubClocked`: regular
- 改善 `Modelica.Clocked.Examples.CascadeControlledDrive.SuperSampled`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClock`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.AssignClockVectorized`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.SampleWithADeffects`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoise`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.UpSample1`: regular
- 改善 `Modelica.Clocked.Examples.Elementary.RealSignals.UpSample2`: regular
- 改善 `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController`: regular
- 改善 `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController`: regular
- 改善 `Modelica.Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects`: regular
- 改善 `Modelica.Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController`: regular
- 改善 `Modelica.Clocked.Examples.Systems.EngineThrottleControl`: regular
- 改善 `Modelica.Media.Examples.WaterIF97`: regular

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 145 s（1 本ずつの検査 124 s） |
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
| flat | 方程式の数が Rumoca の flat と一致 | 273 | 273 |
| dae | 自前で DAE にできた | 279 | 279 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 201 | 279 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 201 | 201 |
| dae | 方程式と未知数の数が一致 | 201 | 201 |
| dae | 状態の数が一致 | 201 | 201 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 201 | 201 |

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

