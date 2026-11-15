# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 4 本。

- 改善 `Modelica.Blocks.Examples.Rectifier12pulseFFT`: flat・dae
- 改善 `Modelica.Blocks.Examples.Rectifier6pulseFFT`: flat・dae
- 改善 `Modelica.Math.FastFourierTransform.Examples.RealFFT1`: flat・dae
- 改善 `Modelica.Math.FastFourierTransform.Examples.RealFFT2`: flat・dae

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 853 s（1 本ずつの検査 792 s） |
| 最大の常駐メモリ（VmHWM） | 5.9 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 566 | 566 |
| flat | 自前で平らにできた | 565 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 532 | 565 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 529 | 532 |
| flat | 方程式の数が Rumoca の flat と一致 | 517 | 532 |
| dae | 自前で DAE にできた | 565 | 565 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 382 | 565 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 382 | 382 |
| dae | 方程式と未知数の数が一致 | 381 | 382 |
| dae | 状態の数が一致 | 381 | 382 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 382 | 382 |

未解決の参照の総数: 0

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `unsized-array` | 1 | `Modelica.Utilities.Examples.ReadRealMatrixFromFile` |

## GlobeModeValidation（75 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 75 | 75 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 75 | 75 |
| flat | 自前で平らにできた | 75 | 75 |
| flat | Rumoca も flat まで通った（比べられた本数） | 74 | 75 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 74 | 74 |
| flat | 方程式の数が Rumoca の flat と一致 | 74 | 74 |
| dae | 自前で DAE にできた | 75 | 75 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 74 | 75 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 74 | 74 |
| dae | 方程式と未知数の数が一致 | 74 | 74 |
| dae | 状態の数が一致 | 74 | 74 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 73 | 73 |

未解決の参照の総数: 0

## 一致しなかったもの（先頭 100 件。全部は JSON）

- `Modelica.Blocks.Examples.Noise.ActuatorWithNoise`: flat: eq 728 vs 736 (rumoca); vars 913 vs 913; only mine []; only rumoca []; element counts []
- `Modelica.Electrical.Batteries.Examples.CCCV_CellRC`: flat: eq 138 vs 128 (rumoca); vars 271 vs 271; only mine []; only rumoca []; element counts []
- `Modelica.Electrical.Batteries.Examples.CCCVcharging`: flat: eq 111 vs 101 (rumoca); vars 206 vs 206; only mine []; only rumoca []; element counts []
- `Modelica.Fluid.Examples.BranchingDynamicPipes`: flat: eq 1260 vs 1256 (rumoca); vars 830 vs 830; only mine []; only rumoca []; element counts []
- `Modelica.Fluid.Examples.HeatingSystem`: dae: eq 538 vs 538, unk 538 vs 538 (rumoca); states 20 vs 11, discrete 4 vs 4
- `Modelica.Fluid.Examples.IncompressibleFluidNetwork`: flat: eq 920 vs 919 (rumoca); vars 2078 vs 2078; only mine []; only rumoca []; element counts []
- `Modelica.Fluid.Examples.InverseParameterization`: flat: eq 177 vs 177 (rumoca); vars 513 vs 507; only mine ["pipe1.flowModel.Res", "pipe1.flowModel.dps_fg_turbulent", "pipe1.flowModel.m_flows_turbulent", "pipe2.flowModel.Res"]; only rumoca []; element counts []
- `Modelica.Fluid.Examples.InverseParameterization`: dae: eq 287 vs 279, unk 287 vs 279 (rumoca); states 0 vs 0, discrete 19 vs 19
- `Modelica.Fluid.Examples.TraceSubstances.RoomCO2`: flat: eq 180 vs 180 (rumoca); vars 293 vs 291; only mine ["pipe.flowModel.Res", "pipe.flowModel.m_flows_turbulent"]; only rumoca []; element counts []
- `Modelica.Fluid.Examples.TraceSubstances.RoomCO2WithControls`: flat: eq 585 vs 585 (rumoca); vars 748 vs 744; only mine ["ductIn.flowModel.Res", "ductIn.flowModel.m_flows_turbulent", "ductOut.flowModel.Res", "ductOut.flowModel.m_flows_turbulent"]; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.CoupledClutches`: flat: eq 110 vs 113 (rumoca); vars 187 vs 187; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.Friction`: flat: eq 94 vs 96 (rumoca); vars 158 vs 158; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.HeatLosses`: flat: eq 239 vs 260 (rumoca); vars 364 vs 364; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo1`: flat: eq 69 vs 77 (rumoca); vars 106 vs 106; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo2`: flat: eq 91 vs 99 (rumoca); vars 143 vs 143; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.LossyGearDemo3`: flat: eq 67 vs 75 (rumoca); vars 92 vs 92; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.OneWayClutch`: flat: eq 40 vs 41 (rumoca); vars 73 vs 73; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.OneWayClutchDisengaged`: flat: eq 46 vs 47 (rumoca); vars 73 vs 73; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Rotational.Examples.SimpleGearShift`: flat: eq 85 vs 87 (rumoca); vars 136 vs 136; only mine []; only rumoca []; element counts []
- `Modelica.Mechanics.Translational.Examples.Brake`: flat: eq 71 vs 73 (rumoca); vars 113 vs 113; only mine []; only rumoca []; element counts []

