# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 7 本。

- 改善 `Modelica.ComplexBlocks.Examples.ShowTransferFunction`: flat_eqs
- 改善 `Modelica.Electrical.QuasiStatic.Machines.Examples.TransformerTestbench`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Electrical.QuasiStatic.Polyphase.Examples.BalancingDelta`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Electrical.QuasiStatic.Polyphase.Examples.BalancingStar`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Electrical.QuasiStatic.Polyphase.Examples.TestSensors`: flat・dae
- 改善 `Modelica.Electrical.QuasiStatic.Polyphase.Examples.UnsymmetricalLoad`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_CommonLeakage`: flat・rumoca_flat・flat_names・flat_eqs・dae

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 1348 s（1 本ずつの検査 1187 s） |
| 最大の常駐メモリ（VmHWM） | 5.4 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 467 | 566 |
| flat | 自前で平らにできた | 477 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 468 | 477 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 468 | 468 |
| flat | 方程式の数が Rumoca の flat と一致 | 457 | 468 |
| dae | 自前で DAE にできた | 477 | 477 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 334 | 477 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 334 | 334 |
| dae | 方程式と未知数の数が一致 | 334 | 334 |
| dae | 状態の数が一致 | 332 | 334 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 334 | 334 |

未解決の参照の総数: 1330

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 99 | `Modelica.Blocks.Examples.Noise.ActuatorWithNoise` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `unsized-array` | 47 | `Modelica.Blocks.Examples.BooleanNetwork1` |
| `stream` | 21 | `Modelica.Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` |
| `connect` | 9 | `Modelica.Clocked.Examples.Elementary.ClockSignals.LogicalSample` |
| `non-constant-dim` | 6 | `Modelica.Blocks.Examples.Rectifier12pulseFFT` |
| `expandable` | 4 | `Modelica.Blocks.Examples.BusUsage` |
| `connect-array` | 1 | `Modelica.Fluid.Examples.HeatExchanger.HeatExchangerSimulation` |
| `non-constant-for` | 1 | `Modelica.Electrical.Polyphase.Examples.PolyphaseRectifier` |

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

- `Modelica.Blocks.Examples.Noise.ActuatorWithNoise`: flat: eq 728 vs 736 (rumoca); vars 913 vs 913; only mine []; only rumoca []
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DCBraking`: dae: eq 392 vs 392, unk 392 vs 392 (rumoca); states 18 vs 23, discrete 0 vs 0
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Initialize`: dae: eq 466 vs 466, unk 466 vs 466 (rumoca); states 18 vs 22, discrete 0 vs 0
- `Modelica.Mechanics.Rotational.Examples.CoupledClutches`: flat: eq 110 vs 113 (rumoca); vars 187 vs 187; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.Friction`: flat: eq 94 vs 96 (rumoca); vars 158 vs 158; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.HeatLosses`: flat: eq 239 vs 260 (rumoca); vars 364 vs 364; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo1`: flat: eq 69 vs 77 (rumoca); vars 106 vs 106; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo2`: flat: eq 91 vs 99 (rumoca); vars 143 vs 143; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo3`: flat: eq 67 vs 75 (rumoca); vars 92 vs 92; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.OneWayClutch`: flat: eq 40 vs 41 (rumoca); vars 73 vs 73; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.OneWayClutchDisengaged`: flat: eq 46 vs 47 (rumoca); vars 73 vs 73; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.SimpleGearShift`: flat: eq 85 vs 87 (rumoca); vars 136 vs 136; only mine []; only rumoca []
- `Modelica.Mechanics.Translational.Examples.Brake`: flat: eq 71 vs 73 (rumoca); vars 113 vs 113; only mine []; only rumoca []

