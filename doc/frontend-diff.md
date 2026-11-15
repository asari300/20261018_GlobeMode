# 自前の段と Rumoca の突き合わせ（Track B）

`cargo xtask diff-frontend` が生成した。手で編集しない。

## 前回の結果との差

前回の結果 `target/frontend-diff.json` と、両方にある 641 本で比べる（この実行にだけある 0 本、前回にだけある 0 本）。
**回帰 0 本**（前回は通った項目が通らなくなった）、改善 113 本。

- 改善 `Modelica.Blocks.Examples.Noise.ActuatorWithNoise`: flat・rumoca_flat・flat_names・dae
- 改善 `Modelica.ComplexBlocks.Examples.ShowTransferFunction`: flat・rumoca_flat
- 改善 `Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Analog.Examples.Lines.SmoothStep`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Conveyor`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DCBraking`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DOL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Initialize`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Inverter`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Transformer`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states
- 改善 `Modelica.Electrical.Machines.Examples.InductionMachines.IMS_Start`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMEE_LoadDump`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMR_DOL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.Machines.Examples.Transformers.IMC_Transformer`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2mPulse`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2mPulse`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTapmPulse`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM`: flat・rumoca_flat・flat_names・flat_eqs・dae・rumoca_balance・balance・counts・states・regular
- 改善 `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.MultipleResonance`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.Rectifier`: flat・rumoca_flat・flat_names・flat_eqs
- 改善 `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.SeriesBode`: flat・rumoca_flat・flat_names・flat_eqs
- 改善 `Modelica.Electrical.QuasiStatic.SinglePhase.Examples.Transformer`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善 `Modelica.Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_Polyphase`: flat・rumoca_flat・flat_names・flat_eqs・dae
- 改善: ほか 63 本

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 検査した本数 | 641 本 |
| 時間 | 全体 646 s（1 本ずつの検査 537 s） |
| 最大の常駐メモリ（VmHWM） | 4.8 GiB |

自前の parse・resolve・flat・dae を、Rumoca は自前で平らにできたものにだけ呼んで比べる（母数はその本数）。
resolve は、例題のクラスから、中のクラス・`extends`・部品の型（解決できたもの）をたどって、届くクラスすべての参照を数える。

## MSL の例題（566 本）

| 段 | 項目 | 本数 | 母数 |
| --- | --- | ---: | ---: |
| parse | 構文の誤りが 0 | 566 | 566 |
| resolve | 未解決の参照が 0（`extends`・部品の型をたどって） | 467 | 566 |
| flat | 自前で平らにできた | 471 | 566 |
| flat | Rumoca も flat まで通った（比べられた本数） | 463 | 471 |
| flat | 変数の名前の集合が Rumoca の flat と一致 | 425 | 463 |
| flat | 方程式の数が Rumoca の flat と一致 | 435 | 463 |
| dae | 自前で DAE にできた | 430 | 471 |
| dae | Rumoca も釣り合いまで通った（比べられた本数） | 297 | 430 |
| dae | 釣り合いの判定（釣り合う / 釣り合わない）が一致 | 295 | 297 |
| dae | 方程式と未知数の数が一致 | 295 | 297 |
| dae | 状態の数が一致 | 293 | 297 |
| dae | 構造が正則かの判定が一致（Rumoca の構造が取れたもの） | 289 | 297 |

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

dae の理由（本数の多い順）:

| 理由 | 本数 | 例 |
| --- | ---: | --- |
| `dae-parse` | 41 | `Modelica.ComplexBlocks.Examples.ShowTransferFunction` |

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

- `Modelica.Blocks.Examples.Noise.ActuatorWithNoise`: flat: eq 728 vs 736 (rumoca); vars 1115 vs 1115; only mine []; only rumoca []
- `Modelica.ComplexBlocks.Examples.ShowTransferFunction`: flat: eq 14 vs 16 (rumoca); vars 49 vs 45; only mine ["transferFunction.aw[1].im", "transferFunction.aw[1].re", "transferFunction.aw[2].im", "transferFunction.aw[2].re"]; only rumoca ["transferFunction.aw.im", "transferFunction.aw.re", "transferFunction.bw.im", "transferFunction.bw.re"]
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DCBraking`: dae: eq 392 vs 392, unk 392 vs 392 (rumoca); states 18 vs 23, discrete 0 vs 0
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Initialize`: dae: eq 466 vs 466, unk 466 vs 466 (rumoca); states 18 vs 22, discrete 0 vs 0
- `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_Transformer`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.Transformers.AsymmetricalLoad`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.Transformers.IMC_Transformer`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.Transformers.Rectifier12pulse`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.Transformers.Rectifier6pulse`: dae: 正則 自前 false / rumoca true
- `Modelica.Electrical.Machines.Examples.Transformers.TransformerTestbench`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint`: flat: eq 1104 vs 1108 (rumoca); vars 1125 vs 1073; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint`: flat: eq 1078 vs 1080 (rumoca); vars 1065 vs 1013; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Constraints.SphericalConstraint`: flat: eq 1075 vs 1081 (rumoca); vars 1057 vs 1005; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Constraints.UniversalConstraint`: flat: eq 1241 vs 1241 (rumoca); vars 1196 vs 1144; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulum`: flat: eq 381 vs 381 (rumoca); vars 649 vs 597; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip`: flat: eq 504 vs 504 (rumoca); vars 719 vs 667; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ForceAndTorque`: flat: eq 506 vs 510 (rumoca); vars 704 vs 652; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.FreeBody`: flat: eq 422 vs 420 (rumoca); vars 636 vs 584; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.HeatLosses`: flat: eq 677 vs 671 (rumoca); vars 1024 vs 972; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.InitSpringConstant`: flat: eq 337 vs 337 (rumoca); vars 612 vs 560; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses`: flat: eq 806 vs 806 (rumoca); vars 1124 vs 1072; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`: flat: eq 117 vs 117 (rumoca); vars 422 vs 370; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper`: flat: eq 373 vs 373 (rumoca); vars 625 vs 573; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravity`: flat: eq 96 vs 92 (rumoca); vars 418 vs 366; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses`: flat: eq 210 vs 214 (rumoca); vars 474 vs 422; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2`: flat: eq 911 vs 921 (rumoca); vars 1322 vs 1270; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheel`: flat: eq 173 vs 173 (rumoca); vars 420 vs 368; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving`: flat: eq 876 vs 876 (rumoca); vars 947 vs 895; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling`: flat: eq 947 vs 947 (rumoca); vars 978 vs 926; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem`: flat: eq 526 vs 524 (rumoca); vars 792 vs 740; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringMassSystem`: flat: eq 398 vs 398 (rumoca); vars 696 vs 644; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.SpringWithMass`: flat: eq 181 vs 179 (rumoca); vars 440 vs 388; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.ThreeSprings`: flat: eq 488 vs 486 (rumoca); vars 708 vs 656; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField`: flat: eq 148 vs 148 (rumoca); vars 434 vs 382; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1a`: flat: eq 1076 vs 1076 (rumoca); vars 1279 vs 1227; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b`: flat: eq 1148 vs 1148 (rumoca); vars 1336 vs 1284; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.Engine1b_analytic`: flat: eq 1467 vs 1467 (rumoca); vars 1426 vs 1374; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: flat: eq 7392 vs 7464 (rumoca); vars 6633 vs 6633; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: dae: eq 10683 vs 11107, unk 11107 vs 11107 (rumoca); states 943 vs 583, discrete 615 vs 615
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: flat: eq 6583 vs 6631 (rumoca); vars 4701 vs 4701; only mine []; only rumoca []
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: dae: eq 8799 vs 9001, unk 9001 vs 9001 (rumoca); states 356 vs 226, discrete 558 vs 558
- `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6_analytic`: dae: 正則 自前 false / rumoca true
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar1`: flat: eq 777 vs 780 (rumoca); vars 1051 vs 999; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar2`: flat: eq 522 vs 522 (rumoca); vars 1008 vs 904; only mine ["fixedFrame.x_label.cylinders[1].R.T", "fixedFrame.x_label.cylinders[1].R.w", "fixedFrame.x_label.cylinders[1].color", "fixedFrame.x_label.cylinders[1].extra"]; only rumoca ["fixedFrame.x_label.cylinders.R.T", "fixedFrame.x_label.cylinders.R.w", "fixedFrame.x_label.cylinders.color", "fixedFrame.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.Fourbar_analytic`: flat: eq 731 vs 731 (rumoca); vars 825 vs 773; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarFourbar`: flat: eq 1146 vs 1146 (rumoca); vars 1118 vs 1066; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic`: flat: eq 1969 vs 1969 (rumoca); vars 1743 vs 1691; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive`: flat: eq 439 vs 439 (rumoca); vars 664 vs 612; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D`: flat: eq 700 vs 700 (rumoca); vars 831 vs 779; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint`: flat: eq 715 vs 715 (rumoca); vars 848 vs 796; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects`: flat: eq 881 vs 885 (rumoca); vars 1183 vs 1131; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
- `Modelica.Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive`: flat: eq 634 vs 634 (rumoca); vars 829 vs 777; only mine ["world.x_label.cylinders[1].R.T", "world.x_label.cylinders[1].R.w", "world.x_label.cylinders[1].color", "world.x_label.cylinders[1].extra"]; only rumoca ["world.x_label.cylinders.R.T", "world.x_label.cylinders.R.w", "world.x_label.cylinders.color", "world.x_label.cylinders.extra"]
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

