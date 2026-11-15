# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 38 本。

- 改善 `Modelica.ComplexBlocks.Examples.ShowTransferFunction`: flat_names
- 改善 `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Constraints.SphericalConstraint`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Constraints.UniversalConstraint`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.ForceAndTorque`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.FreeBody`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.HeatLosses`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.InitSpringConstant`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravity`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses`: flat_names・dae・rumoca_balance・states
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheel`: flat_names・dae・rumoca_balance・balance・counts
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringMassSystem`: flat_names・dae・rumoca_balance・balance・counts・regular
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringWithMass`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.ThreeSprings`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1a`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b_analytic`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar1`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar2`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar_analytic`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarFourbar`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects`: flat_names・dae・rumoca_balance
- 改善 `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive`: flat_names・dae・rumoca_balance

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 551 s（1 本ずつの検査 510 s） |
| 最大の常駐メモリ（VmHWM） | 5.7 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 467 | 566 |
| flat | 自前で平らにできた | 471 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 463 | 471 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 463 | 463 |
| flat | 方程式の数が Rumoca の flat と一致 | 435 | 463 |
| dae | 自前で DAE にできた | 471 | 471 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 334 | 471 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 297 | 334 |
| dae | 方程式と未知数の数が一致 | 297 | 334 |
| dae | 状態の数が一致 | 294 | 334 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 296 | 334 |

未解決の参照の総数: 1330

resolve の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `Redeclare` | 99 | `Modelica.Blocks.Examples.Noise.ActuatorWithNoise` |

flat の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `unsized-array` | 28 | `Modelica.Blocks.Examples.BooleanNetwork1` |
| `composite-binding` | 25 | `Modelica.Electrical.QuasiStatic.Machines.Examples.TransformerTestbench` |
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
- `Modelica.ComplexBlocks.Examples.ShowTransferFunction`: flat: eq 14 vs 16 (rumoca); vars 45 vs 45; only mine []; only rumoca []
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DCBraking`: dae: eq 392 vs 392, unk 392 vs 392 (rumoca); states 18 vs 23, discrete 0 vs 0
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Initialize`: dae: eq 466 vs 466, unk 466 vs 466 (rumoca); states 18 vs 22, discrete 0 vs 0
- `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint`: flat: eq 1104 vs 1108 (rumoca); vars 1069 vs 1069; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint`: dae: eq 2403 vs 2429, unk 2429 vs 2429 (rumoca); states 63 vs 47, discrete 116 vs 116
- `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint`: flat: eq 1078 vs 1080 (rumoca); vars 1009 vs 1009; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint`: dae: eq 2255 vs 2312, unk 2312 vs 2312 (rumoca); states 65 vs 45, discrete 104 vs 104
- `Modelica.Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Constraints.SphericalConstraint`: flat: eq 1075 vs 1081 (rumoca); vars 1001 vs 1001; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Constraints.SphericalConstraint`: dae: eq 2262 vs 2323, unk 2323 vs 2323 (rumoca); states 66 vs 46, discrete 104 vs 104
- `Modelica.Mechanics.MultiBody.Examples.Constraints.SphericalConstraint`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Constraints.UniversalConstraint`: dae: eq 2521 vs 2598, unk 2598 vs 2598 (rumoca); states 72 vs 52, discrete 116 vs 116
- `Modelica.Mechanics.MultiBody.Examples.Constraints.UniversalConstraint`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum`: dae: eq 1258 vs 1302, unk 1302 vs 1302 (rumoca); states 57 vs 37, discrete 57 vs 57
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip`: dae: eq 1435 vs 1479, unk 1479 vs 1479 (rumoca); states 60 vs 40, discrete 69 vs 69
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ForceAndTorque`: flat: eq 506 vs 510 (rumoca); vars 644 vs 644; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ForceAndTorque`: dae: eq 1482 vs 1541, unk 1541 vs 1541 (rumoca); states 31 vs 21, discrete 73 vs 73
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ForceAndTorque`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.FreeBody`: flat: eq 422 vs 420 (rumoca); vars 584 vs 584; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.FreeBody`: dae: eq 1321 vs 1330, unk 1330 vs 1330 (rumoca); states 25 vs 19, discrete 69 vs 69
- `Modelica.Mechanics.MultiBody.Examples.Elementary.FreeBody`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.HeatLosses`: flat: eq 677 vs 671 (rumoca); vars 972 vs 972; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.HeatLosses`: dae: eq 2124 vs 2151, unk 2151 vs 2151 (rumoca); states 60 vs 42, discrete 102 vs 102
- `Modelica.Mechanics.MultiBody.Examples.Elementary.HeatLosses`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.InitSpringConstant`: dae: eq 1253 vs 1275, unk 1275 vs 1275 (rumoca); states 29 vs 19, discrete 60 vs 60
- `Modelica.Mechanics.MultiBody.Examples.Elementary.InitSpringConstant`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses`: dae: eq 2259 vs 2303, unk 2303 vs 2303 (rumoca); states 111 vs 71, discrete 99 vs 99
- `Modelica.Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`: dae: eq 912 vs 934, unk 934 vs 934 (rumoca); states 23 vs 13, discrete 36 vs 36
- `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper`: dae: eq 1327 vs 1349, unk 1349 vs 1349 (rumoca); states 25 vs 15, discrete 60 vs 60
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravity`: flat: eq 96 vs 92 (rumoca); vars 366 vs 366; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravity`: dae: eq 893 vs 911, unk 911 vs 911 (rumoca); states 38 vs 26, discrete 39 vs 39
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravity`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses`: flat: eq 210 vs 214 (rumoca); vars 422 vs 422; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses`: dae: eq 1087 vs 1113, unk 1113 vs 1113 (rumoca); states 24 vs 24, discrete 51 vs 51
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2`: flat: eq 911 vs 921 (rumoca); vars 1270 vs 1270; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2`: dae: eq 2817 vs 2860, unk 2860 vs 2860 (rumoca); states 160 vs 104, discrete 125 vs 125
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheel`: dae: eq 947 vs 947, unk 947 vs 947 (rumoca); states 25 vs 15, discrete 36 vs 36
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheel`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving`: dae: eq 1893 vs 1959, unk 1959 vs 1959 (rumoca); states 78 vs 48, discrete 81 vs 81
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling`: dae: eq 1981 vs 2047, unk 2047 vs 2047 (rumoca); states 78 vs 48, discrete 86 vs 86
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem`: flat: eq 526 vs 524 (rumoca); vars 740 vs 740; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem`: dae: eq 1677 vs 1686, unk 1686 vs 1686 (rumoca); states 41 vs 25, discrete 78 vs 78
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringMassSystem`: dae: eq 1469 vs 1469, unk 1469 vs 1469 (rumoca); states 42 vs 22, discrete 63 vs 63
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringWithMass`: flat: eq 181 vs 179 (rumoca); vars 388 vs 388; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringWithMass`: dae: eq 959 vs 968, unk 968 vs 968 (rumoca); states 25 vs 19, discrete 45 vs 45
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringWithMass`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ThreeSprings`: flat: eq 488 vs 486 (rumoca); vars 656 vs 656; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ThreeSprings`: dae: eq 1507 vs 1516, unk 1516 vs 1516 (rumoca); states 19 vs 13, discrete 75 vs 75
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ThreeSprings`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField`: dae: eq 929 vs 951, unk 951 vs 951 (rumoca); states 23 vs 13, discrete 39 vs 39
- `Modelica.Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1a`: dae: eq 2342 vs 2386, unk 2386 vs 2386 (rumoca); states 159 vs 99, discrete 117 vs 117
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1a`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b`: dae: eq 2444 vs 2488, unk 2488 vs 2488 (rumoca); states 160 vs 100, discrete 123 vs 123
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b_analytic`: dae: eq 2730 vs 2752, unk 2752 vs 2752 (rumoca); states 158 vs 98, discrete 147 vs 147
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b_analytic`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: flat: eq 7392 vs 7464 (rumoca); vars 6633 vs 6633; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: dae: eq 10683 vs 11107, unk 11107 vs 11107 (rumoca); states 943 vs 583, discrete 615 vs 615
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: flat: eq 6583 vs 6631 (rumoca); vars 4677 vs 4677; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: dae: eq 8799 vs 9001, unk 9001 vs 9001 (rumoca); states 356 vs 226, discrete 558 vs 558
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar1`: flat: eq 777 vs 780 (rumoca); vars 999 vs 999; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar1`: dae: eq 1923 vs 2058, unk 2058 vs 2058 (rumoca); states 95 vs 65, discrete 87 vs 87
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar1`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar2`: dae: eq 2220 vs 2242, unk 2242 vs 2242 (rumoca); states 58 vs 38, discrete 87 vs 87
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar2`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar_analytic`: dae: eq 1721 vs 1743, unk 1743 vs 1743 (rumoca); states 71 vs 51, discrete 84 vs 84
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar_analytic`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarFourbar`: dae: eq 2282 vs 2348, unk 2348 vs 2348 (rumoca); states 113 vs 73, discrete 90 vs 90
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarFourbar`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic`: dae: eq 3990 vs 4012, unk 4012 vs 4012 (rumoca); states 124 vs 94, discrete 168 vs 168
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive`: dae: eq 1391 vs 1413, unk 1413 vs 1413 (rumoca); states 35 vs 25, discrete 65 vs 65
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D`: dae: eq 1684 vs 1750, unk 1750 vs 1750 (rumoca); states 47 vs 37, discrete 63 vs 63
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint`: dae: eq 1687 vs 1731, unk 1731 vs 1731 (rumoca); states 69 vs 49, discrete 77 vs 77
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects`: flat: eq 881 vs 885 (rumoca); vars 1131 vs 1131; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects`: dae: eq 2206 vs 2276, unk 2276 vs 2276 (rumoca); states 110 vs 78, discrete 105 vs 105
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive`: dae: eq 1670 vs 1736, unk 1736 vs 1736 (rumoca); states 43 vs 33, discrete 71 vs 71
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.Rotational.Examples.CoupledClutches`: flat: eq 110 vs 113 (rumoca); vars 187 vs 187; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.Friction`: flat: eq 94 vs 96 (rumoca); vars 158 vs 158; only mine []; only rumoca []
- `Modelica.Mechanics.Rotational.Examples.HeatLosses`: flat: eq 239 vs 260 (rumoca); vars 364 vs 364; only mine []; only rumoca []

