# MSL 4.1.0 の例題の実行と参照結果との比較（globe-mode-exec）

`cargo xtask msl-sim` が生成した。手で編集しない。`msl-funnel --json` で OK になった例題を、実行系 `globe-mode-exec`（Phase A は Rumoca の実行系）で別プロセスのワーカーから実行し、MAP-LIB の参照結果（MSL 4.1.0）と矩形の管（csv-compare の既定のアルゴリズムを書き直した `globe_mode_sim::compare`）で比べる。

| 項目 | 値 |
| --- | --- |
| 実行した日時 | 2026-10-19 05:23 UTC |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| globe check の結果 | `target/msl-funnel.json`（OK 369 本） |
| 実行した本数 | 357 本（partial を除いて全部） |
| partial のため実行しない | 12 本（例題のテンプレートの partial なクラス。対象に数えない。名前は「partial のため実行しないモデル」の節） |
| 参照結果 | [modelica/MAP-LIB_ReferenceResults](https://github.com/modelica/MAP-LIB_ReferenceResults) のブランチ `v4.1.0`（commit `dac67829004d`）を `data/msl-ref` に取得したもの（この実行では取得していない） |
| 実行の設定 | StartTime・StopTime・Interval・Tolerance は experiment annotation（無い項目は既定値: StartTime 0、StopTime は開始 + 1、Tolerance 1e-6、Interval は区間の 1/500）。ソルバは既定（bdf）（実行できたもの: bdf 303 本）。壁時計の予算 30 s（協調型。超えると GM-E11005）。応答が 120 s 無ければ親がワーカーを止める |
| 参照結果の設定（参考） | Dymola の Dassl。Tolerance は experiment annotation の 1/10（無ければ 1e-6）、Interval は annotation より細かい。手法も許容誤差も違うので、点ごとには一致しない |
| 管 | 相対の幅 0.002（時間と値の両方向。csv-compare の既定）。比べる信号は `comparisonSignals.txt`。名前の中の空白は除いて比べる |
| ワーカー | 起動 8 回（外から停止 7 回・異常終了 0 回）、50 本ごとに `Checker::reset` |
| 結果 | **357 本: 実行できた 303（終了時刻まで 303・terminate 0）・コンパイル NG 0・S10 NG 0・実行 NG 47（うち時間切れ 5）・外から停止 7・ワーカー異常終了 0・panic 0** |
| 参照結果との比較 | 参照結果と比べた 302 本: 管に収まった 292（96.7%）・外れ 10・信号が足りない 0・参照結果なし 1・未取得 0・読めない 0 |
| P3 の完了条件 | 実行できた 303 本（目標 153 本以上: 達成）・管に収まった 292/302（96.7%、目標 90% 以上: 達成。実行できたうちでは 96.4%）→ **満たす** |

## 集計

| 項目 | 本数 |
| --- | ---: |
| partial のため実行しない（対象に数えない） | 12 |
| 対象 | 357 |
| 終了時刻まで実行できた | 303 |
| terminate() で終了時刻の前に終わった（GM-I11001） | 0 |
| **実行できた（計）** | 303 |
| DAE まで変換できなかった（compile_retaining の診断） | 0 |
| 設定・プリフライト（S10）で止まった | 0 |
| 実行で止まった | 47 |
| うち協調型の時間切れ（GM-E11005） | 5 |
| うち結果が有限でない（GM-E11006） | 0 |
| 外から止めた（120 s で応答が無い） | 7 |
| ワーカーが応答の前に落ちた | 0 |
| panic（ワーカーの中で拾った） | 0 |
| t0 で有限にならない警告（GM-W10002）が出た | 3 |
| ソルバの別名を置き換えた（GM-I10003） | 0 |

### 止まったモデルのコード

| コード | 本数 |
| --- | ---: |
| GM-U0004 | 26 |
| GM-E11001 | 6 |
| GM-E11002 | 6 |
| GM-E11005 | 5 |
| GM-U0005 | 4 |

### 参照結果との比較

| 項目 | 本数 |
| --- | ---: |
| 実行できた | 303 |
| うち参照結果と比べた | 302 |
| **管に収まった（すべての信号）** | 292 |
| 管から外れた信号がある | 10 |
| 外れは無いが、どちらかに無い信号がある | 0 |
| 試験側に無い信号がある（外れの有無を問わず） | 0 |
| 参照側に無い信号がある（外れの有無を問わず） | 0 |
| MAP-LIB に参照結果が無い | 1 |
| 参照結果を取得していない | 0 |
| 参照結果を読めない | 0 |
| 管に収まった割合（比べたうち / 実行できたうち） | 96.7% / 96.4% |
| 信号の数で（合格 / 比べた信号） | 1687 / 1713 |

## P3 の完了条件と基準値

P3 の完了条件（[roadmap](roadmap.md)）は「MSL の例題 153 本以上が実行できる。そのうち 90% 以上が参照結果のファネルに収まる」。この実行では 実行できた 303 本（目標 153 本以上: 達成）・管に収まった 292/302（96.7%、目標 90% 以上: 達成。実行できたうちでは 96.4%）→ **満たす**。

基準値は物差しが違う。Rumoca の「一致」は OMC の結果との正規化した平均絶対誤差で、管ではない。OpenModelica の verified は MAP-LIB の参照結果との照合（細かい規則は【未確認】）。

| | 実行できた | 参照と比べた | 合う | 物差し | 出典（確認日） |
| --- | ---: | ---: | ---: | --- | --- |
| **GlobeMode（この実行）** | 303 | 302 | 292 | 矩形の管（幅 0.002、MAP-LIB の参照結果） | - |
| Rumoca の基準値（`msl_quality_baseline.json`） | 216 | - | - | `sim_ok`（試した 286 本、制限 12 s） | [rev 276e82bd](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json)（2026-10-19） |
| Rumoca の文書（2026-06-05 の全 MSL の実行） | 153 | 143（OMC と） | 89（high agreement） | 正規化した平均絶対誤差（管ではない） | [msl-baseline-promotion-analysis-2026-06-05.md](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/docs/dev-guide/src/tooling/msl-baseline-promotion-analysis-2026-06-05.md)（2026-10-19） |
| OpenModelica master（2026-10-17 の実行、MSL 4.1.0） | 533 | - | 510（verified） | MAP-LIB の参照結果との照合 | [libraries.openmodelica.org](https://libraries.openmodelica.org/branches/master/Modelica_4.1.0/Modelica_4.1.0.html)（2026-10-19） |

## 時間とメモリ

| 項目 | 値 |
| --- | --- |
| 時間 | 全体 1911.7 s・ワーカーの中 1887.5 s（コンパイル 183.7 s・実行系の準備 139.75 s・初期化と積分 159.50 s・参照結果の読み込みと比較 5.59 s）・MSL の読み込み 8 回 18.9 s・1 本あたり（親から見た）平均 5.30 s・最大 120.00 s |
| ワーカーの最大の常駐メモリ（VmHWM。用意のとき / 全体） | 1.2 GiB / 3.5 GiB |

## 実行できなかったモデル

| # | モデル | 結果 | 段階 | コード | 文言（先頭） | 秒 |
| ---: | --- | --- | --- | --- | --- | ---: |
| 1 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | 0.26 |
| 2 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.29 |
| 3 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（thd1.y の値が NaN になった） | 0.32 |
| 4 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | 0.08 |
| 5 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | 0.40 |
| 6 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | 0.40 |
| 7 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | Run | GM-E11002 | t = 0.027 で積分に失敗した | 0.11 |
| 8 | `Electrical.Analog.Examples.SimpleTriacCircuit` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | 1.11 |
| 9 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 方向微分（ヤコビアン）の準備で止まった | 5.76 |
| 10 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 1.44 |
| 11 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | Run | GM-E11002 | 積分に失敗した | 2.81 |
| 12 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 3.82 |
| 13 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 30 秒を超えた） | 33.24 |
| 14 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 3.52 |
| 15 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.57 |
| 16 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 3.93 |
| 17 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 1.23 |
| 18 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.65 |
| 19 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.88 |
| 20 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | 0.29 |
| 21 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 30 秒を超えた） | 30.53 |
| 22 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | 6.90 |
| 23 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 30 秒を超えた） | 32.53 |
| 24 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 30 秒を超えた） | 30.68 |
| 25 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | 19.66 |
| 26 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | 6.20 |
| 27 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 30 秒を超えた） | 30.96 |
| 28 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 35.55 |
| 29 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 30 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 31 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 32 | `Fluid.Examples.HeatingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 12.73 |
| 33 | `Fluid.Examples.InverseParameterization` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 34 | `Fluid.Examples.NonCircularPipes` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 35 | `Fluid.Examples.PumpingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 6.39 |
| 36 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | Run | GM-E11002 | 積分に失敗した | 4.78 |
| 37 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.18 |
| 38 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.55 |
| 39 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.36 |
| 40 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.81 |
| 41 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 実行系の組み立てで止まった | 0.05 |
| 42 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 43 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | - | - | 120 s で応答が無いので止めた（signal: 9 (SIGKILL)） | 120.00 |
| 44 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 15.15 |
| 45 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | 93.64 |
| 46 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | 1.42 |
| 47 | `Media.Examples.SimpleLiquidWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.29 |
| 48 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 実行系の組み立てで止まった | 40.09 |
| 49 | `Media.Examples.WaterIF97` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | 0.94 |
| 50 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（cylinder1.h の値が NaN になった） | 0.19 |
| 51 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（openTank.h の値が NaN になった） | 1.74 |
| 52 | `Thermal.FluidHeatFlow.Examples.TwoMass` | 実行 NG | Run | GM-E11002 | 積分に失敗した | 0.26 |
| 53 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 方向微分（ヤコビアン）の準備で止まった | 0.21 |
| 54 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | 0.05 |

## 管から外れたモデル

「t」は最初に管の外に出た時刻、「はみ出し」は管からはみ出した量の最大（値の単位）。

| # | モデル | 合格 / 信号 | 外れた信号 |
| ---: | --- | --- | --- |
| 1 | `Blocks.Examples.DemoSignalCharacteristic` | 4/5 | `harmonic.mean1.y`（t = 0.0215・はみ出し 5.636e-5・外れ 1006/1051 点） |
| 2 | `Blocks.Examples.DemonstrateSignalExtrema` | 6/8 | `signalExtrema1.y_min`（t = 0.6730・はみ出し 1.471e-2・外れ 29369/165001 点）、`signalExtrema1.y_max`（t = 0.6430・はみ出し 6.801e-3・外れ 94272/165001 点） |
| 3 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 4/8 | `rotational_clock_1.direction`（t = 1.3976・はみ出し 1.996e0・外れ 5/694 点）、`rotational_clock_1.direction_changed`（t = 1.3976・はみ出し 9.980e-1・外れ 9/694 点）、`sample_conjunctive.y`（t = 0.0560・はみ出し 3.502e0・外れ 49/694 点）、`sample_disjunctive.y`（t = 1.0000・はみ出し 1.960e0・外れ 63/694 点） |
| 4 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 1/4 | `sample_angle.y`（t = 1.0000・はみ出し 1.964e0・外れ 127/594 点）、`rotationalClock.direction`（t = 1.3976・はみ出し 1.996e0・外れ 5/594 点）、`rotationalClock.direction_changed`（t = 1.3976・はみ出し 9.980e-1・外れ 9/594 点） |
| 5 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | 2/4 | `dcPowerSupply.i`（t = 0.5215・はみ出し 1.340e-1・外れ 208/107690 点）、`bufferCapacitor.i`（t = 0.5255・はみ出し 8.405e-2・外れ 102/107690 点） |
| 6 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 0/3 | `c.v`（t = 0.0330・はみ出し 1.376e1・外れ 930/1001 点）、`opAmp.out.v`（t = 0.0320・はみ出し 2.994e1・外れ 588/1001 点）、`opAmp.out.i`（t = 0.0320・はみ出し 5.658e-2・外れ 918/1001 点） |
| 7 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | 9/11 | `vH01.y_arg`（t = 0.2600・はみ出し 1.065e-1・外れ 1370/83780 点）、`iH01.y_arg`（t = 0.2600・はみ出し 1.065e-1・外れ 1370/83780 点） |
| 8 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | 18/24 | `inverter.ac.pin[2].i`（t = 0.0452・はみ出し 3.307e-1・外れ 1/5652 点）、`inverter.ac.pin[3].i`（t = 0.0452・はみ出し 3.307e-1・外れ 1/5652 点）、`currentSensor.i[2]`（t = 0.0452・はみ出し 3.307e-1・外れ 1/5652 点）、`currentSensor.i[3]`（t = 0.0452・はみ出し 3.307e-1・外れ 1/5652 点）、`voltageSensor.v[2]`（t = 0.0452・はみ出し 3.307e1・外れ 1/5652 点）、ほか 1 個 |
| 9 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | 45/47 | `voltageSensor.v[1]`（t = 0.1000・はみ出し 1.633e1・外れ 1/6317 点）、`voltageSensor.v[2]`（t = 0.1000・はみ出し 1.633e1・外れ 1/6317 点） |
| 10 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | 3/4 | `revolute1.phi`（t = 4.8300・はみ出し 4.857e-3・外れ 3/501 点） |

## partial のため実行しないモデル

`msl-funnel` では OK だが、partial なクラス（例題のテンプレート）なので実行しない。

- `Electrical.PowerConverters.Examples.ACAC.ExampleTemplates.Dimmer`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.Thyristor1Pulse`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2Pulse`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2mPulse`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2Pulse`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2mPulse`
- `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTapmPulse`
- `Electrical.PowerConverters.Examples.DCAC.ExampleTemplates.SinglePhaseTwoLevel`
- `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperBuckBoost`
- `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepDown`
- `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepUp`
- `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.HBridge`

## モデルごとの結果

- 「準備」は実行系の準備（lowering と組み立て）、「実行」は初期化と積分の時間（どちらも実行系が測った秒）。「計」は親が測った 1 本の時間（コンパイルと参照結果の比較を含む）。
- 参照結果の欄の「管 OK a/b」は b 個の信号のうち a 個が管に収まったこと。

| # | モデル | 結果 | コード | ソルバ | 状態 | 点 | 準備 s | 実行 s | 参照結果 | 計 s |
| ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: |
| 1 | `Blocks.Examples.BusUsage` | 終了時刻まで | - | bdf | 0 | 502 | 0.00 | 0.00 | 管 OK 5/5 | 1.44 |
| 2 | `Blocks.Examples.BusUsage_Utilities.Part` | 終了時刻まで | - | bdf | 0 | 502 | 0.00 | 0.00 | 参照結果なし | 0.19 |
| 3 | `Blocks.Examples.CompareSincExpSine` | 終了時刻まで | - | bdf | 0 | 10001 | 0.00 | 0.02 | 管 OK 3/3 | 0.25 |
| 4 | `Blocks.Examples.DemoSignalCharacteristic` | 終了時刻まで | - | bdf | 5 | 1051 | 0.02 | 0.01 | **管 NG** 4/5（外れ 1） | 0.28 |
| 5 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | GM-E11001 | bdf | - | - | - | - | - | 0.26 |
| 6 | `Blocks.Examples.DemonstrateSignalExtrema` | 終了時刻まで | - | bdf | 1 | 165001 | 0.01 | 0.74 | **管 NG** 6/8（外れ 2） | 1.78 |
| 7 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.29 |
| 8 | `Blocks.Examples.FilterWithRiseTime` | 終了時刻まで | - | bdf | 6 | 502 | 0.03 | 0.01 | 管 OK 6/6 | 0.32 |
| 9 | `Blocks.Examples.Interaction1` | 終了時刻まで | - | bdf | 0 | 510 | 0.01 | 0.01 | 管 OK 6/6 | 0.39 |
| 10 | `Blocks.Examples.InverseModel` | 終了時刻まで | - | bdf | 2 | 502 | 0.01 | 0.00 | 管 OK 2/2 | 0.22 |
| 11 | `Blocks.Examples.LogicalNetwork1` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 3/3 | 0.26 |
| 12 | `Blocks.Examples.Modulation` | 終了時刻まで | - | bdf | 5 | 10001 | 0.01 | 0.01 | 管 OK 5/5 | 0.28 |
| 13 | `Blocks.Examples.Noise.Densities` | 終了時刻まで | - | bdf | 0 | 1001 | 0.00 | 0.01 | 管 OK 3/3 | 0.20 |
| 14 | `Blocks.Examples.PID_Controller` | 終了時刻まで | - | bdf | 6 | 511 | 0.04 | 0.01 | 管 OK 6/6 | 0.41 |
| 15 | `Blocks.Examples.ShowLogicalSources` | 終了時刻まで | - | bdf | 0 | 536 | 0.00 | 0.00 | 管 OK 1/1 | 0.25 |
| 16 | `Blocks.Examples.SinCosEncoder` | 終了時刻まで | - | bdf | 5 | 20002 | 0.02 | 0.03 | 管 OK 6/6 | 0.52 |
| 17 | `Blocks.Examples.SlewRateLimiter` | 終了時刻まで | - | bdf | 3 | 1010 | 0.01 | 0.00 | 管 OK 3/3 | 0.23 |
| 18 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | GM-E11001 | bdf | - | - | - | - | - | 0.32 |
| 19 | `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | 終了時刻まで | - | bdf | 2 | 4009 | 0.06 | 0.25 | 管 OK 4/4 | 0.53 |
| 20 | `Clocked.Examples.CascadeControlledDrive.Continuous` | 終了時刻まで | - | bdf | 3 | 509 | 0.07 | 0.01 | 管 OK 4/4 | 0.28 |
| 21 | `Clocked.Examples.CascadeControlledDrive.SubClocked` | 終了時刻まで | - | bdf | 2 | 4009 | 0.06 | 0.26 | 管 OK 4/4 | 0.54 |
| 22 | `Clocked.Examples.CascadeControlledDrive.SuperSampled` | 終了時刻まで | - | bdf | 2 | 4009 | 0.07 | 0.25 | 管 OK 4/4 | 0.54 |
| 23 | `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 24 | `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 25 | `Clocked.Examples.Elementary.BooleanSignals.BackSample` | 終了時刻まで | - | bdf | 0 | 519 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 26 | `Clocked.Examples.Elementary.BooleanSignals.Hold` | 終了時刻まで | - | bdf | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 27 | `Clocked.Examples.Elementary.BooleanSignals.Sample1` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 28 | `Clocked.Examples.Elementary.BooleanSignals.Sample2` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.09 |
| 29 | `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 30 | `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 31 | `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | 終了時刻まで | - | bdf | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.12 |
| 32 | `Clocked.Examples.Elementary.BooleanSignals.SubSample` | 終了時刻まで | - | bdf | 0 | 513 | 0.00 | 0.00 | 管 OK 2/2 | 0.12 |
| 33 | `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | 終了時刻まで | - | bdf | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.12 |
| 34 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 35 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 36 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 37 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.05 |
| 38 | `Clocked.Examples.Elementary.BooleanSignals.UpSample` | 終了時刻まで | - | bdf | 0 | 529 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 39 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 終了時刻まで | - | bdf | 0 | 694 | 0.02 | 0.20 | **管 NG** 4/8（外れ 4） | 0.41 |
| 40 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 終了時刻まで | - | bdf | 0 | 594 | 0.01 | 0.01 | **管 NG** 1/4（外れ 3） | 0.11 |
| 41 | `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | 終了時刻まで | - | bdf | 0 | 517 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 42 | `Clocked.Examples.Elementary.ClockSignals.SubSample` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 43 | `Clocked.Examples.Elementary.ClockSignals.SuperSample` | 終了時刻まで | - | bdf | 0 | 521 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 44 | `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 45 | `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 46 | `Clocked.Examples.Elementary.IntegerSignals.BackSample` | 終了時刻まで | - | bdf | 0 | 523 | 0.00 | 0.00 | 管 OK 3/3 | 0.12 |
| 47 | `Clocked.Examples.Elementary.IntegerSignals.Hold` | 終了時刻まで | - | bdf | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.14 |
| 48 | `Clocked.Examples.Elementary.IntegerSignals.Sample1` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 49 | `Clocked.Examples.Elementary.IntegerSignals.Sample2` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.07 |
| 50 | `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 51 | `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 1.66 |
| 52 | `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | 終了時刻まで | - | bdf | 0 | 523 | 0.00 | 0.00 | 管 OK 2/2 | 0.16 |
| 53 | `Clocked.Examples.Elementary.IntegerSignals.SubSample` | 終了時刻まで | - | bdf | 0 | 513 | 0.00 | 0.00 | 管 OK 2/2 | 0.14 |
| 54 | `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | 終了時刻まで | - | bdf | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.14 |
| 55 | `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 56 | `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 57 | `Clocked.Examples.Elementary.IntegerSignals.UpSample` | 終了時刻まで | - | bdf | 0 | 533 | 0.00 | 0.00 | 管 OK 3/3 | 0.14 |
| 58 | `Clocked.Examples.Elementary.RealSignals.AssignClock` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 59 | `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 60 | `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 61 | `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | 終了時刻まで | - | bdf | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 62 | `Clocked.Examples.Elementary.RealSignals.BackSample` | 終了時刻まで | - | bdf | 0 | 517 | 0.00 | 0.00 | 管 OK 3/3 | 0.11 |
| 63 | `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | 終了時刻まで | - | bdf | 0 | 516 | 0.00 | 0.00 | 管 OK 2/2 | 0.11 |
| 64 | `Clocked.Examples.Elementary.RealSignals.Hold` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.11 |
| 65 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | 終了時刻まで | - | bdf | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.10 |
| 66 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 67 | `Clocked.Examples.Elementary.RealSignals.Sample1` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 68 | `Clocked.Examples.Elementary.RealSignals.Sample2` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 69 | `Clocked.Examples.Elementary.RealSignals.Sample3` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.08 |
| 70 | `Clocked.Examples.Elementary.RealSignals.SampleClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 71 | `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 72 | `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.10 |
| 73 | `Clocked.Examples.Elementary.RealSignals.ShiftSample` | 終了時刻まで | - | bdf | 0 | 517 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 74 | `Clocked.Examples.Elementary.RealSignals.SubSample` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 75 | `Clocked.Examples.Elementary.RealSignals.SuperSample` | 終了時刻まで | - | bdf | 0 | 521 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 76 | `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | 終了時刻まで | - | bdf | 0 | 518 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 77 | `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 78 | `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 79 | `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 80 | `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 81 | `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 82 | `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 83 | `Clocked.Examples.Elementary.RealSignals.UniformNoise` | 終了時刻まで | - | bdf | 0 | 606 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 84 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.08 |
| 85 | `Clocked.Examples.Elementary.RealSignals.UpSample1` | 終了時刻まで | - | bdf | 0 | 527 | 0.00 | 0.00 | 管 OK 3/3 | 0.08 |
| 86 | `Clocked.Examples.Elementary.RealSignals.UpSample2` | 終了時刻まで | - | bdf | 0 | 518 | 0.00 | 0.00 | 管 OK 5/5 | 0.11 |
| 87 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | 終了時刻まで | - | bdf | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 88 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | 終了時刻まで | - | bdf | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 89 | `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | 終了時刻まで | - | bdf | 2 | 601 | 0.02 | 0.01 | 管 OK 4/4 | 0.17 |
| 90 | `Clocked.Examples.SimpleControlledDrive.Continuous` | 終了時刻まで | - | bdf | 3 | 502 | 0.01 | 0.00 | 管 OK 4/4 | 0.17 |
| 91 | `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | 終了時刻まで | - | bdf | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 92 | `Clocked.Examples.Systems.EngineThrottleControl` | 終了時刻まで | - | bdf | 5 | 645 | 0.04 | 0.02 | 管 OK 8/8 | 0.23 |
| 93 | `ComplexBlocks.Examples.TestConversionBlock` | 終了時刻まで | - | bdf | 0 | 1012 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 94 | `Electrical.Analog.Examples.AmplifierWithOpAmpDetailed` | 終了時刻まで | - | bdf | 7 | 2501 | 0.03 | 0.04 | 管 OK 7/7 | 0.24 |
| 95 | `Electrical.Analog.Examples.CauerLowPassAnalog` | 終了時刻まで | - | bdf | 5 | 503 | 0.02 | 0.01 | 管 OK 5/5 | 0.15 |
| 96 | `Electrical.Analog.Examples.CauerLowPassOPV` | 終了時刻まで | - | bdf | 5 | 503 | 0.07 | 0.01 | 管 OK 5/5 | 0.30 |
| 97 | `Electrical.Analog.Examples.CauerLowPassSC` | 終了時刻まで | - | bdf | 16 | 3601 | 0.20 | 15.33 | 管 OK 5/5 | 16.30 |
| 98 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | 終了時刻まで | - | bdf | 0 | 510 | 0.02 | 0.02 | 管 OK 3/3 | 0.19 |
| 99 | `Electrical.Analog.Examples.CharacteristicThyristors` | 終了時刻まで | - | bdf | 0 | 559 | 0.02 | 0.05 | 管 OK 3/3 | 0.27 |
| 100 | `Electrical.Analog.Examples.ChuaCircuit` | 終了時刻まで | - | bdf | 3 | 50219 | 0.01 | 0.12 | 管 OK 3/3 | 0.70 |
| 101 | `Electrical.Analog.Examples.CompareTransformers` | 終了時刻まで | - | bdf | 4 | 50001 | 0.05 | 0.53 | 管 OK 3/3 | 3.29 |
| 102 | `Electrical.Analog.Examples.ControlledSwitchWithArc` | 終了時刻まで | - | bdf | 2 | 24041 | 0.03 | 0.17 | 管 OK 2/2 | 0.65 |
| 103 | `Electrical.Analog.Examples.DemoPowerSupply` | 終了時刻まで | - | bdf | 0 | 10006 | 0.01 | 0.13 | 管 OK 3/3 | 0.30 |
| 104 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | 終了時刻まで | - | bdf | 1 | 107690 | 0.03 | 1.94 | **管 NG** 2/4（外れ 2） | 3.02 |
| 105 | `Electrical.Analog.Examples.DemonstrateLightning` | 終了時刻まで | - | bdf | 2 | 10002 | 0.05 | 0.02 | 管 OK 6/6 | 0.35 |
| 106 | `Electrical.Analog.Examples.DifferenceAmplifier` | 終了時刻まで | - | bdf | 9 | 502 | 0.18 | 0.02 | 管 OK 9/9 | 0.49 |
| 107 | `Electrical.Analog.Examples.GenerationOfFMUs` | 終了時刻まで | - | bdf | 6 | 1001 | 0.17 | 0.02 | 管 OK 6/6 | 0.58 |
| 108 | `Electrical.Analog.Examples.HeatingMOSInverter` | 終了時刻まで | - | bdf | 2 | 584 | 0.06 | 0.09 | 管 OK 2/2 | 0.33 |
| 109 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 0.40 |
| 110 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 0.40 |
| 111 | `Electrical.Analog.Examples.HeatingRectifier` | 終了時刻まで | - | bdf | 2 | 501 | 0.03 | 0.01 | 管 OK 2/2 | 0.18 |
| 112 | `Electrical.Analog.Examples.IdealTriacCircuit` | 終了時刻まで | - | bdf | 1 | 561 | 0.03 | 0.03 | 管 OK 3/3 | 0.20 |
| 113 | `Electrical.Analog.Examples.InvertingAmp` | 終了時刻まで | - | bdf | 0 | 1046 | 0.02 | 0.02 | 管 OK 1/1 | 0.21 |
| 114 | `Electrical.Analog.Examples.Lines.CompareLineTrunks` | 終了時刻まで | - | bdf | 57 | 1004 | 0.44 | 0.28 | 管 OK 9/9 | 1.84 |
| 115 | `Electrical.Analog.Examples.Lines.CompareLosslessLines` | 終了時刻まで | - | bdf | 0 | 209 | 0.02 | 0.01 | 管 OK 12/12 | 0.16 |
| 116 | `Electrical.Analog.Examples.Lines.LightningLosslessTransmissionLine` | 終了時刻まで | - | bdf | 0 | 25002 | 0.01 | 0.59 | 管 OK 3/3 | 0.97 |
| 117 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 終了時刻まで | - | bdf | 402 | 25002 | 21.37 | 31.63 | 管 OK 3/3 | 85.64 |
| 118 | `Electrical.Analog.Examples.Lines.SmoothStep` | 終了時刻まで | - | bdf | 116 | 3002 | 1.08 | 0.65 | 管 OK 4/4 | 4.83 |
| 119 | `Electrical.Analog.Examples.NandGate` | 終了時刻まで | - | bdf | 2 | 526 | 0.06 | 0.01 | 管 OK 2/2 | 0.25 |
| 120 | `Electrical.Analog.Examples.OpAmps.Adder` | 終了時刻まで | - | bdf | 0 | 1001 | 0.01 | 0.06 | 管 OK 1/1 | 0.23 |
| 121 | `Electrical.Analog.Examples.OpAmps.Comparator` | 終了時刻まで | - | bdf | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.15 |
| 122 | `Electrical.Analog.Examples.OpAmps.ControlCircuit` | 終了時刻まで | - | bdf | 6 | 1003 | 0.09 | 0.04 | 管 OK 8/8 | 0.43 |
| 123 | `Electrical.Analog.Examples.OpAmps.DifferentialAmplifier` | 終了時刻まで | - | bdf | 1 | 2011 | 0.04 | 0.16 | 管 OK 2/2 | 0.43 |
| 124 | `Electrical.Analog.Examples.OpAmps.Differentiator` | 終了時刻まで | - | bdf | 1 | 1046 | 0.02 | 0.01 | 管 OK 3/3 | 0.17 |
| 125 | `Electrical.Analog.Examples.OpAmps.HighPass` | 終了時刻まで | - | bdf | 1 | 1046 | 0.02 | 0.04 | 管 OK 3/3 | 0.20 |
| 126 | `Electrical.Analog.Examples.OpAmps.Integrator` | 終了時刻まで | - | bdf | 1 | 1046 | 0.02 | 0.01 | 管 OK 3/3 | 0.16 |
| 127 | `Electrical.Analog.Examples.OpAmps.InvertingAmplifier` | 終了時刻まで | - | bdf | 0 | 1046 | 0.01 | 0.02 | 管 OK 2/2 | 0.15 |
| 128 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 0.11 |
| 129 | `Electrical.Analog.Examples.OpAmps.LCOscillator` | 終了時刻まで | - | bdf | 2 | 1001 | 0.02 | 0.01 | 管 OK 4/4 | 0.15 |
| 130 | `Electrical.Analog.Examples.OpAmps.LowPass` | 終了時刻まで | - | bdf | 1 | 1046 | 0.02 | 0.02 | 管 OK 3/3 | 0.18 |
| 131 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 終了時刻まで | - | bdf | 1 | 1001 | 0.02 | 0.01 | **管 NG** 0/3（外れ 3） | 0.14 |
| 132 | `Electrical.Analog.Examples.OpAmps.NonInvertingAmplifier` | 終了時刻まで | - | bdf | 0 | 1046 | 0.01 | 0.02 | 管 OK 2/2 | 0.15 |
| 133 | `Electrical.Analog.Examples.OpAmps.SchmittTrigger` | 終了時刻まで | - | bdf | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 134 | `Electrical.Analog.Examples.OpAmps.SignalGenerator` | 終了時刻まで | - | bdf | 1 | 1041 | 0.02 | 0.03 | 管 OK 5/5 | 0.18 |
| 135 | `Electrical.Analog.Examples.OpAmps.Subtracter` | 終了時刻まで | - | bdf | 0 | 1001 | 0.01 | 0.01 | 管 OK 1/1 | 0.15 |
| 136 | `Electrical.Analog.Examples.OpAmps.VoltageFollower` | 終了時刻まで | - | bdf | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 137 | `Electrical.Analog.Examples.OvervoltageProtection` | 終了時刻まで | - | bdf | 1 | 501 | 0.03 | 0.02 | 管 OK 1/1 | 0.16 |
| 138 | `Electrical.Analog.Examples.ParallelResonance` | 終了時刻まで | - | bdf | 6 | 10002 | 0.02 | 0.03 | 管 OK 6/6 | 0.31 |
| 139 | `Electrical.Analog.Examples.Rectifier` | 終了時刻まで | - | bdf | 5 | 14522 | 0.05 | 3.49 | 管 OK 4/4 | 4.07 |
| 140 | `Electrical.Analog.Examples.Resistor` | 終了時刻まで | - | bdf | 0 | 501 | 0.01 | 0.01 | 管 OK 2/2 | 0.11 |
| 141 | `Electrical.Analog.Examples.ResonanceCircuits` | 終了時刻まで | - | bdf | 4 | 10001 | 0.12 | 0.04 | 管 OK 3/3 | 0.85 |
| 142 | `Electrical.Analog.Examples.SeriesResonance` | 終了時刻まで | - | bdf | 6 | 10002 | 0.02 | 0.05 | 管 OK 6/6 | 0.37 |
| 143 | `Electrical.Analog.Examples.ShowSaturatingInductor` | 終了時刻まで | - | bdf | 2 | 630 | 0.01 | 0.00 | 管 OK 2/2 | 0.11 |
| 144 | `Electrical.Analog.Examples.ShowVariableResistor` | 終了時刻まで | - | bdf | 0 | 501 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 145 | `Electrical.Analog.Examples.SimpleTriacCircuit` | 実行 NG | GM-U0005 | bdf | - | - | - | - | - | 1.11 |
| 146 | `Electrical.Analog.Examples.SwitchWithArc` | 終了時刻まで | - | bdf | 2 | 8010 | 0.02 | 0.06 | 管 OK 2/2 | 0.32 |
| 147 | `Electrical.Analog.Examples.ThyristorBehaviourTest` | 終了時刻まで | - | bdf | 2 | 517 | 0.03 | 0.03 | 管 OK 2/2 | 0.18 |
| 148 | `Electrical.Batteries.Examples.CCCV_Cell` | 終了時刻まで | - | bdf | 3 | 12010 | 0.04 | 0.09 | 管 OK 3/3 | 0.58 |
| 149 | `Electrical.Batteries.Examples.SuperCapDischargeCharge` | 終了時刻まで | - | bdf | 2 | 72021 | 0.02 | 0.13 | 管 OK 2/2 | 1.00 |
| 150 | `Electrical.Machines.Examples.DCMachines.DCEE_Start` | 終了時刻まで | - | bdf | 4 | 20004 | 0.21 | 0.19 | 管 OK 4/4 | 1.54 |
| 151 | `Electrical.Machines.Examples.DCMachines.DCPM_Cooling` | 終了時刻まで | - | bdf | 5 | 250051 | 0.18 | 3.62 | 管 OK 5/5 | 16.04 |
| 152 | `Electrical.Machines.Examples.DCMachines.DCPM_CurrentControlled` | 終了時刻まで | - | bdf | 5 | 20002 | 0.17 | 0.13 | 管 OK 5/5 | 1.58 |
| 153 | `Electrical.Machines.Examples.DCMachines.DCPM_Drive` | 終了時刻まで | - | bdf | 16 | 20006 | 0.60 | 0.32 | 管 OK 4/4 | 4.30 |
| 154 | `Electrical.Machines.Examples.DCMachines.DCPM_QuasiStatic` | 終了時刻まで | - | bdf | 5 | 20005 | 0.40 | 0.22 | 管 OK 8/8 | 2.41 |
| 155 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | 終了時刻まで | - | bdf | 3 | 20004 | 0.13 | 0.11 | 管 OK 3/3 | 1.21 |
| 156 | `Electrical.Machines.Examples.DCMachines.DCPM_Temperature` | 終了時刻まで | - | bdf | 3 | 30002 | 0.14 | 0.16 | 管 OK 3/3 | 1.65 |
| 157 | `Electrical.Machines.Examples.DCMachines.DCPM_withLosses` | 終了時刻まで | - | bdf | 6 | 20005 | 0.37 | 0.29 | 管 OK 6/6 | 2.53 |
| 158 | `Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase` | 終了時刻まで | - | bdf | 3 | 20003 | 0.32 | 0.36 | 管 OK 3/3 | 2.11 |
| 159 | `Electrical.Machines.Examples.DCMachines.DCSE_Start` | 終了時刻まで | - | bdf | 3 | 20003 | 0.29 | 0.23 | 管 OK 3/3 | 2.00 |
| 160 | `Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics` | 終了時刻まで | - | bdf | 6 | 6003 | 0.70 | 0.23 | 管 OK 12/12 | 2.79 |
| 161 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 5.76 |
| 162 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 1.44 |
| 163 | `Electrical.Machines.Examples.InductionMachines.IMC_DOL` | 終了時刻まで | - | bdf | 7 | 15002 | 0.42 | 0.45 | 管 OK 8/8 | 3.35 |
| 164 | `Electrical.Machines.Examples.InductionMachines.IMC_Initialize` | 終了時刻まで | - | bdf | 7 | 15002 | 0.39 | 0.37 | 管 OK 8/8 | 3.05 |
| 165 | `Electrical.Machines.Examples.InductionMachines.IMC_Inverter` | 終了時刻まで | - | bdf | 7 | 15003 | 0.81 | 0.54 | 管 OK 9/9 | 4.02 |
| 166 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 2.81 |
| 167 | `Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz` | 終了時刻まで | - | bdf | 9 | 10004 | 0.40 | 0.73 | 管 OK 10/10 | 2.97 |
| 168 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 3.82 |
| 169 | `Electrical.Machines.Examples.InductionMachines.IMC_YD` | 終了時刻まで | - | bdf | 7 | 25003 | 0.53 | 1.47 | 管 OK 8/8 | 6.28 |
| 170 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | GM-E11005 | bdf | - | - | - | - | - | 33.24 |
| 171 | `Electrical.Machines.Examples.InductionMachines.IMC_withLosses` | 終了時刻まで | - | bdf | 8 | 50002 | 0.55 | 1.85 | 管 OK 9/9 | 9.04 |
| 172 | `Electrical.Machines.Examples.InductionMachines.IMS_Start` | 終了時刻まで | - | bdf | 8 | 15003 | 0.68 | 1.04 | 管 OK 10/10 | 5.84 |
| 173 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL` | 終了時刻まで | - | bdf | 7 | 30004 | 1.26 | 1.33 | 管 OK 9/9 | 8.41 |
| 174 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator` | 終了時刻まで | - | bdf | 5 | 30002 | 1.58 | 1.11 | 管 OK 7/7 | 8.51 |
| 175 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier` | 終了時刻まで | - | bdf | 12 | 12806 | 1.29 | 23.62 | 管 OK 14/14 | 28.46 |
| 176 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking` | 終了時刻まで | - | bdf | 5 | 8294 | 0.61 | 1.06 | 管 OK 6/6 | 3.87 |
| 177 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 3.52 |
| 178 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter` | 終了時刻まで | - | bdf | 7 | 15003 | 1.37 | 0.57 | 管 OK 9/9 | 5.39 |
| 179 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad` | 終了時刻まで | - | bdf | 1 | 3221 | 0.73 | 0.04 | 管 OK 3/3 | 2.08 |
| 180 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking` | 終了時刻まで | - | bdf | 4 | 1003 | 1.23 | 0.10 | 管 OK 9/9 | 3.23 |
| 181 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource` | 終了時刻まで | - | bdf | 7 | 20001 | 2.06 | 0.71 | 管 OK 8/8 | 7.94 |
| 182 | `Electrical.Machines.Examples.SynchronousMachines.SMR_DOL` | 終了時刻まで | - | bdf | 6 | 2503 | 1.12 | 0.19 | 管 OK 8/8 | 3.26 |
| 183 | `Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter` | 終了時刻まで | - | bdf | 7 | 15003 | 1.08 | 0.54 | 管 OK 9/9 | 4.88 |
| 184 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.57 |
| 185 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 3.93 |
| 186 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 1.23 |
| 187 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.65 |
| 188 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.88 |
| 189 | `Electrical.Polyphase.Examples.Rectifier` | 終了時刻まで | - | bdf | 4 | 10121 | 0.20 | 0.56 | 管 OK 4/4 | 2.42 |
| 190 | `Electrical.Polyphase.Examples.TestSensors` | 終了時刻まで | - | bdf | 2 | 1001 | 0.20 | 0.02 | 管 OK 2/2 | 0.80 |
| 191 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.29 |
| 192 | `Electrical.Polyphase.Examples.TransformerYY` | 終了時刻まで | - | bdf | 2 | 1001 | 0.10 | 0.01 | 管 OK 2/2 | 0.42 |
| 193 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | 終了時刻まで | - | bdf | 10 | 83780 | 0.10 | 4.05 | **管 NG** 9/11（外れ 2） | 7.89 |
| 194 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | GM-E11005 | bdf | - | - | - | - | - | 30.53 |
| 195 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R` | 終了時刻まで | - | bdf | 3 | 536 | 0.04 | 0.02 | 管 OK 9/9 | 0.26 |
| 196 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R_Characteristic` | 終了時刻まで | - | bdf | 3 | 53224 | 0.04 | 1.79 | 管 OK 9/9 | 3.09 |
| 197 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.DiodeBridge2Pulse` | 終了時刻まで | - | bdf | 3 | 519 | 0.04 | 0.03 | 管 OK 6/6 | 0.25 |
| 198 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.HalfControlledBridge2Pulse` | 終了時刻まで | - | bdf | 3 | 539 | 0.05 | 0.06 | 管 OK 10/10 | 0.35 |
| 199 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | GM-U0005 | bdf | - | - | - | - | - | 6.90 |
| 200 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_R` | 終了時刻まで | - | bdf | 3 | 539 | 0.06 | 0.05 | 管 OK 12/12 | 0.35 |
| 201 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RL` | 終了時刻まで | - | bdf | 4 | 559 | 0.06 | 0.06 | 管 OK 13/13 | 2.19 |
| 202 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV` | 終了時刻まで | - | bdf | 4 | 559 | 0.06 | 0.06 | 管 OK 13/13 | 0.42 |
| 203 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic` | 終了時刻まで | - | bdf | 4 | 55456 | 0.06 | 5.25 | 管 OK 13/13 | 7.49 |
| 204 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse` | 終了時刻まで | - | bdf | 3 | 671 | 0.10 | 0.17 | 管 OK 6/6 | 0.66 |
| 205 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse` | 終了時刻まで | - | bdf | 3 | 720 | 0.14 | 0.36 | 管 OK 18/18 | 1.05 |
| 206 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | GM-E11005 | bdf | - | - | - | - | - | 32.53 |
| 207 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R` | 終了時刻まで | - | bdf | 3 | 688 | 0.14 | 0.35 | 管 OK 24/24 | 1.07 |
| 208 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL` | 終了時刻まで | - | bdf | 4 | 688 | 0.15 | 0.32 | 管 OK 25/25 | 1.13 |
| 209 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV` | 終了時刻まで | - | bdf | 4 | 691 | 0.14 | 0.35 | 管 OK 25/25 | 1.08 |
| 210 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | 実行 NG | GM-E11005 | bdf | - | - | - | - | - | 30.68 |
| 211 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.DiodeCenterTap2Pulse` | 終了時刻まで | - | bdf | 3 | 519 | 0.04 | 0.02 | 管 OK 6/6 | 0.24 |
| 212 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_R` | 終了時刻まで | - | bdf | 3 | 539 | 0.06 | 0.03 | 管 OK 10/10 | 0.35 |
| 213 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RL` | 終了時刻まで | - | bdf | 4 | 559 | 0.05 | 0.04 | 管 OK 11/11 | 0.35 |
| 214 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV` | 終了時刻まで | - | bdf | 4 | 559 | 0.05 | 0.04 | 管 OK 11/11 | 0.37 |
| 215 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV_Characteristic` | 終了時刻まで | - | bdf | 4 | 55456 | 0.05 | 3.29 | 管 OK 11/11 | 5.26 |
| 216 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.DiodeCenterTap2mPulse` | 終了時刻まで | - | bdf | 3 | 671 | 0.09 | 0.13 | 管 OK 6/6 | 0.59 |
| 217 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R` | 終了時刻まで | - | bdf | 3 | 736 | 0.13 | 0.23 | 管 OK 18/18 | 0.94 |
| 218 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL` | 終了時刻まで | - | bdf | 4 | 736 | 0.15 | 0.27 | 管 OK 19/19 | 1.01 |
| 219 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV` | 終了時刻まで | - | bdf | 4 | 739 | 0.14 | 0.23 | 管 OK 19/19 | 0.95 |
| 220 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | 実行 NG | GM-U0005 | bdf | - | - | - | - | - | 19.66 |
| 221 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.DiodeCenterTapmPulse` | 終了時刻まで | - | bdf | 3 | 586 | 0.06 | 0.05 | 管 OK 6/6 | 0.40 |
| 222 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R` | 終了時刻まで | - | bdf | 3 | 656 | 0.12 | 0.15 | 管 OK 15/15 | 0.68 |
| 223 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL` | 終了時刻まで | - | bdf | 4 | 650 | 0.10 | 0.11 | 管 OK 16/16 | 0.63 |
| 224 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV` | 終了時刻まで | - | bdf | 4 | 650 | 0.10 | 0.12 | 管 OK 16/16 | 0.64 |
| 225 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic` | 終了時刻まで | - | bdf | 4 | 68902 | 0.13 | 13.17 | 管 OK 16/16 | 17.63 |
| 226 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | 終了時刻まで | - | bdf | 12 | 5652 | 0.28 | 1.33 | **管 NG** 18/24（外れ 6） | 2.88 |
| 227 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | 終了時刻まで | - | bdf | 29 | 6317 | 0.65 | 5.07 | **管 NG** 45/47（外れ 2） | 8.47 |
| 228 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM` | 終了時刻まで | - | bdf | 7 | 4569 | 0.18 | 1.90 | 管 OK 14/14 | 3.06 |
| 229 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_R` | 終了時刻まで | - | bdf | 4 | 781 | 0.06 | 0.28 | 管 OK 6/6 | 0.60 |
| 230 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_RL` | 終了時刻まで | - | bdf | 5 | 801 | 0.06 | 0.25 | 管 OK 7/7 | 0.57 |
| 231 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | 実行 NG | GM-U0005 | bdf | - | - | - | - | - | 6.20 |
| 232 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_R` | 終了時刻まで | - | bdf | 4 | 10215 | 0.04 | 0.19 | 管 OK 8/8 | 0.64 |
| 233 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_RL` | 終了時刻まで | - | bdf | 5 | 10245 | 0.04 | 0.22 | 管 OK 9/9 | 0.68 |
| 234 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepUp.ChopperStepUp_R` | 終了時刻まで | - | bdf | 4 | 10226 | 0.04 | 0.19 | 管 OK 10/10 | 0.64 |
| 235 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | GM-E11005 | bdf | - | - | - | - | - | 30.96 |
| 236 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_R` | 終了時刻まで | - | bdf | 2 | 701 | 0.07 | 0.22 | 管 OK 6/6 | 0.59 |
| 237 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_RL` | 終了時刻まで | - | bdf | 3 | 701 | 0.07 | 0.32 | 管 OK 7/7 | 0.70 |
| 238 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_TrianglePWM_RL` | 終了時刻まで | - | bdf | 3 | 1293 | 0.08 | 0.55 | 管 OK 7/7 | 0.99 |
| 239 | `Electrical.Spice3.Examples.CoupledInductors` | 終了時刻まで | - | bdf | 5 | 501 | 0.04 | 0.06 | 管 OK 3/3 | 0.60 |
| 240 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 35.55 |
| 241 | `Fluid.Examples.AST_BatchPlant.Test.OneTank` | 終了時刻まで | - | bdf | 2 | 1116 | 1.83 | 2.25 | 管 OK 8/8 | 6.37 |
| 242 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe1` | 終了時刻まで | - | bdf | 2 | 505 | 1.91 | 0.03 | 管 OK 4/4 | 4.19 |
| 243 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe2` | 終了時刻まで | - | bdf | 2 | 509 | 1.94 | 0.06 | 管 OK 4/4 | 4.51 |
| 244 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe1` | 終了時刻まで | - | bdf | 4 | 523 | 2.04 | 0.21 | 管 OK 8/8 | 5.50 |
| 245 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe2` | 終了時刻まで | - | bdf | 6 | 513 | 2.17 | 0.12 | 管 OK 12/12 | 7.66 |
| 246 | `Fluid.Examples.AST_BatchPlant.Test.TwoTanks` | 終了時刻まで | - | bdf | 4 | 501 | 1.90 | 0.03 | 管 OK 6/6 | 3.83 |
| 247 | `Fluid.Examples.ControlledTankSystem.ControlledTanks` | 終了時刻まで | - | bdf | 4 | 535 | 0.48 | 0.21 | 管 OK 23/23 | 3.71 |
| 248 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 249 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 250 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 251 | `Fluid.Examples.HeatingSystem` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 12.73 |
| 252 | `Fluid.Examples.InverseParameterization` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 253 | `Fluid.Examples.NonCircularPipes` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 254 | `Fluid.Examples.PumpingSystem` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 6.39 |
| 255 | `Fluid.Examples.Tanks.EmptyTanks` | 終了時刻まで | - | bdf | 4 | 505 | 2.42 | 0.05 | 管 OK 3/3 | 4.17 |
| 256 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 4.78 |
| 257 | `Fluid.Examples.Tanks.ThreeTanks` | 終了時刻まで | - | bdf | 6 | 501 | 2.41 | 5.15 | 管 OK 6/6 | 11.30 |
| 258 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.18 |
| 259 | `Magnetic.FluxTubes.Examples.BasicExamples.SaturatedInductor` | 終了時刻まで | - | bdf | 1 | 501 | 0.03 | 0.01 | 管 OK 6/6 | 0.18 |
| 260 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | 終了時刻まで | - | bdf | 0 | 503 | 0.03 | 0.00 | 管 OK 2/2 | 0.18 |
| 261 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | 終了時刻まで | - | bdf | 0 | 503 | 0.03 | 0.00 | 管 OK 2/2 | 0.18 |
| 262 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ArmatureStroke` | 終了時刻まで | - | bdf | 6 | 5029 | 0.25 | 0.10 | 管 OK 9/9 | 1.04 |
| 263 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.55 |
| 264 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.36 |
| 265 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonPullInStroke` | 終了時刻まで | - | bdf | 9 | 1045 | 0.80 | 2.04 | 管 OK 9/9 | 4.06 |
| 266 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonQuasiStatic` | 終了時刻まで | - | bdf | 9 | 502 | 0.46 | 0.38 | 管 OK 28/28 | 1.67 |
| 267 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.81 |
| 268 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.05 |
| 269 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | 終了時刻まで | - | bdf | 8 | 501 | 3.76 | 0.11 | 管 OK 10/10 | 8.18 |
| 270 | `Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint` | 終了時刻まで | - | bdf | 4 | 501 | 2.49 | 0.44 | 管 OK 5/5 | 5.86 |
| 271 | `Mechanics.MultiBody.Examples.Constraints.SphericalConstraint` | 終了時刻まで | - | bdf | 13 | 501 | 1.79 | 0.10 | 管 OK 3/3 | 4.13 |
| 272 | `Mechanics.MultiBody.Examples.Constraints.UniversalConstraint` | 終了時刻まで | - | bdf | 8 | 509 | 3.14 | 0.64 | 管 OK 7/7 | 7.85 |
| 273 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulum` | 終了時刻まで | - | bdf | 4 | 501 | 0.79 | 0.04 | 管 OK 4/4 | 2.03 |
| 274 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | 終了時刻まで | - | bdf | 4 | 501 | 0.96 | 0.13 | **管 NG** 3/4（外れ 1） | 2.79 |
| 275 | `Mechanics.MultiBody.Examples.Elementary.ForceAndTorque` | 終了時刻まで | - | bdf | 4 | 501 | 0.76 | 0.03 | 管 OK 4/4 | 2.06 |
| 276 | `Mechanics.MultiBody.Examples.Elementary.FreeBody` | 終了時刻まで | - | bdf | 13 | 501 | 0.70 | 0.03 | 管 OK 9/9 | 1.81 |
| 277 | `Mechanics.MultiBody.Examples.Elementary.HeatLosses` | 終了時刻まで | - | bdf | 40 | 501 | 1.37 | 0.05 | 管 OK 28/28 | 3.12 |
| 278 | `Mechanics.MultiBody.Examples.Elementary.InitSpringConstant` | 終了時刻まで | - | bdf | 2 | 501 | 0.63 | 0.02 | 管 OK 2/2 | 1.55 |
| 279 | `Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses` | 終了時刻まで | - | bdf | 4 | 501 | 2.03 | 0.07 | 管 OK 4/4 | 4.32 |
| 280 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | 終了時刻まで | - | bdf | 2 | 501 | 0.47 | 0.02 | 管 OK 2/2 | 1.10 |
| 281 | `Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper` | 終了時刻まで | - | bdf | 4 | 501 | 0.69 | 0.03 | 管 OK 4/4 | 1.65 |
| 282 | `Mechanics.MultiBody.Examples.Elementary.PointGravity` | 終了時刻まで | - | bdf | 26 | 501 | 0.58 | 0.03 | 管 OK 18/18 | 1.24 |
| 283 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses` | 終了時刻まで | - | bdf | 24 | 501 | 0.40 | 0.02 | 管 OK 24/24 | 1.09 |
| 284 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2` | 終了時刻まで | - | bdf | 26 | 501 | 2.91 | 0.25 | 管 OK 15/15 | 5.90 |
| 285 | `Mechanics.MultiBody.Examples.Elementary.RollingWheel` | 終了時刻まで | - | bdf | 8 | 501 | 0.68 | 0.07 | 管 OK 8/8 | 1.45 |
| 286 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving` | 終了時刻まで | - | bdf | 10 | 501 | 1.82 | 0.31 | 管 OK 7/7 | 3.90 |
| 287 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling` | 終了時刻まで | - | bdf | 10 | 506 | 2.04 | 0.14 | 管 OK 7/7 | 4.15 |
| 288 | `Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem` | 終了時刻まで | - | bdf | 15 | 501 | 1.05 | 0.04 | 管 OK 11/11 | 2.43 |
| 289 | `Mechanics.MultiBody.Examples.Elementary.SpringMassSystem` | 終了時刻まで | - | bdf | 4 | 501 | 0.82 | 0.03 | 管 OK 4/4 | 1.89 |
| 290 | `Mechanics.MultiBody.Examples.Elementary.SpringWithMass` | 終了時刻まで | - | bdf | 13 | 501 | 0.54 | 0.02 | 管 OK 9/9 | 1.42 |
| 291 | `Mechanics.MultiBody.Examples.Elementary.Surfaces` | 終了時刻まで | - | bdf | 2 | 501 | 0.42 | 0.01 | 管 OK 2/2 | 1.10 |
| 292 | `Mechanics.MultiBody.Examples.Elementary.ThreeSprings` | 終了時刻まで | - | bdf | 13 | 501 | 0.78 | 0.10 | 管 OK 9/9 | 2.06 |
| 293 | `Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField` | 終了時刻まで | - | bdf | 2 | 501 | 0.51 | 0.28 | 管 OK 2/2 | 1.52 |
| 294 | `Mechanics.MultiBody.Examples.Loops.Engine1a` | 終了時刻まで | - | bdf | 2 | 501 | 2.57 | 0.19 | 管 OK 2/2 | 5.99 |
| 295 | `Mechanics.MultiBody.Examples.Loops.Engine1b` | 終了時刻まで | - | bdf | 2 | 527 | 2.89 | 0.33 | 管 OK 2/2 | 6.56 |
| 296 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | 終了時刻まで | - | bdf | 2 | 527 | 16.34 | 0.63 | 管 OK 2/2 | 33.73 |
| 297 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 298 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | - | - | - | - | - | - | - | 120.00 |
| 299 | `Mechanics.MultiBody.Examples.Loops.Fourbar1` | 終了時刻まで | - | bdf | 2 | 501 | 2.34 | 0.16 | 管 OK 2/2 | 6.58 |
| 300 | `Mechanics.MultiBody.Examples.Loops.Fourbar2` | 終了時刻まで | - | bdf | 2 | 501 | 1.72 | 0.08 | 管 OK 2/2 | 3.69 |
| 301 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | 終了時刻まで | - | bdf | 2 | 501 | 3.33 | 0.09 | 管 OK 2/2 | 8.31 |
| 302 | `Mechanics.MultiBody.Examples.Loops.PlanarFourbar` | 終了時刻まで | - | bdf | 2 | 501 | 2.49 | 0.17 | 管 OK 2/2 | 5.73 |
| 303 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 15.15 |
| 304 | `Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive` | 終了時刻まで | - | bdf | 4 | 501 | 0.76 | 0.02 | 管 OK 4/4 | 1.84 |
| 305 | `Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D` | 終了時刻まで | - | bdf | 8 | 501 | 1.10 | 0.83 | 管 OK 8/8 | 3.55 |
| 306 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint` | 終了時刻まで | - | bdf | 4 | 501 | 1.30 | 0.07 | 管 OK 4/4 | 3.18 |
| 307 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects` | 終了時刻まで | - | bdf | 18 | 501 | 2.18 | 0.35 | 管 OK 10/10 | 5.09 |
| 308 | `Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive` | 終了時刻まで | - | bdf | 8 | 501 | 0.96 | 0.02 | 管 OK 8/8 | 2.29 |
| 309 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | GM-E11001 | bdf | - | - | - | - | - | 93.64 |
| 310 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | GM-E11001 | bdf | - | - | - | - | - | 1.42 |
| 311 | `Mechanics.Rotational.Examples.Backlash` | 終了時刻まで | - | bdf | 6 | 1165 | 0.05 | 0.05 | 管 OK 4/4 | 0.26 |
| 312 | `Mechanics.Rotational.Examples.CompareBrakingTorque` | 終了時刻まで | - | bdf | 8 | 50004 | 0.02 | 0.04 | 管 OK 4/4 | 0.67 |
| 313 | `Mechanics.Rotational.Examples.EddyCurrentBrake` | 終了時刻まで | - | bdf | 3 | 1003 | 0.01 | 0.00 | 管 OK 3/3 | 0.11 |
| 314 | `Mechanics.Rotational.Examples.ElasticBearing` | 終了時刻まで | - | bdf | 6 | 1002 | 0.02 | 0.02 | 管 OK 6/6 | 0.15 |
| 315 | `Mechanics.Rotational.Examples.First` | 終了時刻まで | - | bdf | 4 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.16 |
| 316 | `Mechanics.Rotational.Examples.FirstGrounded` | 終了時刻まで | - | bdf | 4 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.15 |
| 317 | `Mechanics.Rotational.Examples.RollingWheel` | 終了時刻まで | - | bdf | 2 | 5002 | 0.02 | 0.02 | 管 OK 2/2 | 0.19 |
| 318 | `Mechanics.Translational.Examples.Accelerate` | 終了時刻まで | - | bdf | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.07 |
| 319 | `Mechanics.Translational.Examples.CompareBrakingForce` | 終了時刻まで | - | bdf | 8 | 50004 | 0.02 | 0.04 | 管 OK 4/4 | 0.65 |
| 320 | `Mechanics.Translational.Examples.Damper` | 終了時刻まで | - | bdf | 6 | 1001 | 0.03 | 0.00 | 管 OK 6/6 | 0.14 |
| 321 | `Mechanics.Translational.Examples.EddyCurrentBrake` | 終了時刻まで | - | bdf | 3 | 1003 | 0.01 | 0.00 | 管 OK 3/3 | 0.11 |
| 322 | `Mechanics.Translational.Examples.ElastoGap` | 終了時刻まで | - | bdf | 4 | 513 | 0.04 | 0.00 | 管 OK 4/4 | 0.16 |
| 323 | `Mechanics.Translational.Examples.GenerationOfFMUs` | 終了時刻まで | - | bdf | 10 | 1001 | 0.12 | 0.01 | 管 OK 10/10 | 0.39 |
| 324 | `Mechanics.Translational.Examples.InitialConditions` | 終了時刻まで | - | bdf | 8 | 5001 | 0.03 | 0.01 | 管 OK 8/8 | 0.21 |
| 325 | `Mechanics.Translational.Examples.Oscillator` | 終了時刻まで | - | bdf | 4 | 1001 | 0.03 | 0.00 | 管 OK 4/4 | 0.15 |
| 326 | `Mechanics.Translational.Examples.PreLoad` | 終了時刻まで | - | bdf | 6 | 1022 | 0.08 | 0.01 | 管 OK 6/6 | 0.31 |
| 327 | `Mechanics.Translational.Examples.Sensors` | 終了時刻まで | - | bdf | 2 | 1001 | 0.02 | 0.00 | 管 OK 2/2 | 0.13 |
| 328 | `Mechanics.Translational.Examples.SignConvention` | 終了時刻まで | - | bdf | 6 | 1001 | 0.01 | 0.00 | 管 OK 6/6 | 0.09 |
| 329 | `Mechanics.Translational.Examples.Vehicle` | 終了時刻まで | - | bdf | 10 | 6012 | 0.19 | 0.06 | 管 OK 6/6 | 0.89 |
| 330 | `Mechanics.Translational.Examples.WhyArrows` | 終了時刻まで | - | bdf | 4 | 1001 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 331 | `Media.Examples.SimpleLiquidWater` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.29 |
| 332 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 40.09 |
| 333 | `Media.Examples.WaterIF97` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.94 |
| 334 | `StateGraph.Examples.ControlledTanks` | 終了時刻まで | - | bdf | 2 | 533 | 0.08 | 0.06 | 管 OK 21/21 | 0.61 |
| 335 | `StateGraph.Examples.ExecutionPaths` | 終了時刻まで | - | bdf | 0 | 527 | 0.01 | 0.02 | 管 OK 9/9 | 0.28 |
| 336 | `StateGraph.Examples.FirstExample` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.18 |
| 337 | `StateGraph.Examples.FirstExample_Variant2` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.18 |
| 338 | `StateGraph.Examples.FirstExample_Variant3` | 終了時刻まで | - | bdf | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.18 |
| 339 | `StateGraph.Examples.ShowCompositeStep` | 終了時刻まで | - | bdf | 0 | 522 | 0.01 | 0.02 | 管 OK 9/9 | 0.27 |
| 340 | `StateGraph.Examples.ShowExceptions` | 終了時刻まで | - | bdf | 0 | 525 | 0.01 | 0.04 | 管 OK 10/10 | 0.29 |
| 341 | `Thermal.FluidHeatFlow.Examples.IndirectCooling` | 終了時刻まで | - | bdf | 4 | 1501 | 0.06 | 0.01 | 管 OK 4/4 | 0.29 |
| 342 | `Thermal.FluidHeatFlow.Examples.OneMass` | 終了時刻まで | - | bdf | 2 | 1008 | 0.03 | 0.00 | 管 OK 2/2 | 0.18 |
| 343 | `Thermal.FluidHeatFlow.Examples.ParallelCooling` | 終了時刻まで | - | bdf | 5 | 1001 | 0.06 | 0.01 | 管 OK 5/5 | 0.26 |
| 344 | `Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut` | 終了時刻まで | - | bdf | 5 | 2009 | 0.06 | 0.03 | 管 OK 5/5 | 0.34 |
| 345 | `Thermal.FluidHeatFlow.Examples.PumpAndValve` | 終了時刻まで | - | bdf | 3 | 2005 | 0.06 | 0.03 | 管 OK 3/3 | 0.31 |
| 346 | `Thermal.FluidHeatFlow.Examples.PumpDropOut` | 終了時刻まで | - | bdf | 2 | 2007 | 0.03 | 0.00 | 管 OK 2/2 | 0.20 |
| 347 | `Thermal.FluidHeatFlow.Examples.SimpleCooling` | 終了時刻まで | - | bdf | 2 | 1001 | 0.03 | 0.00 | 管 OK 2/2 | 0.18 |
| 348 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | GM-W10002・GM-E11001 | bdf | - | - | - | - | - | 0.19 |
| 349 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | GM-W10002・GM-E11001 | bdf | - | - | - | - | - | 1.74 |
| 350 | `Thermal.FluidHeatFlow.Examples.TwoMass` | 実行 NG | GM-E11002 | bdf | - | - | - | - | - | 0.26 |
| 351 | `Thermal.FluidHeatFlow.Examples.TwoTanks` | 終了時刻まで | GM-W10002 | bdf | 4 | 501 | 0.03 | 0.01 | 管 OK 4/4 | 0.15 |
| 352 | `Thermal.FluidHeatFlow.Examples.WaterPump` | 終了時刻まで | - | bdf | 1 | 2015 | 0.06 | 0.04 | 管 OK 1/1 | 0.36 |
| 353 | `Thermal.HeatTransfer.Examples.ControlledTemperature` | 終了時刻まで | - | bdf | 1 | 10062 | 0.02 | 0.05 | 管 OK 2/2 | 0.29 |
| 354 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.21 |
| 355 | `Thermal.HeatTransfer.Examples.Motor` | 終了時刻まで | - | bdf | 2 | 720025 | 0.02 | 0.61 | 管 OK 2/2 | 6.80 |
| 356 | `Thermal.HeatTransfer.Examples.TwoMasses` | 終了時刻まで | - | bdf | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.09 |
| 357 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | GM-U0004 | bdf | - | - | - | - | - | 0.05 |
