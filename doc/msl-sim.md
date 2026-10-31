# MSL 4.1.0 の例題の実行と参照結果との比較（globe-mode-exec）

`cargo xtask msl-sim` が生成した。手で編集しない。`msl-funnel --json` で OK になった例題を、実行系 `globe-mode-exec`（Phase A は Rumoca の実行系）で別プロセスのワーカーから実行し、MAP-LIB の参照結果（MSL 4.1.0）と矩形の管（csv-compare の既定のアルゴリズムを書き直した `globe_mode_sim::compare`）で比べる。

| 項目 | 値 |
| --- | --- |
| 実行した日時 | 2026-10-31 06:32 UTC |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| globe check の結果 | `target/msl-funnel.json`（OK 369 本） |
| 実行した本数 | 356 本（partial を除いて全部） |
| partial のため実行しない | 12 本（例題のテンプレートの partial なクラス。対象に数えない。名前は「partial のため実行しないモデル」の節） |
| 例題でないため実行しない | 1 本（experiment annotation が無く（GM-I10001）、値の無い入力かパラメータがある（GM-W8003）クラス。対象に数えない。名前は「例題でないため実行しないモデル」の節） |
| 参照結果 | [modelica/MAP-LIB_ReferenceResults](https://github.com/modelica/MAP-LIB_ReferenceResults) のブランチ `v4.1.0`（commit `dac67829004d`）を `data/msl-ref` に取得したもの。この実行での取得: 356 本: 取得済み 353・取得 0（0 ファイル、0.0 MB）・参照結果なし 3・失敗 0（0.0 s） |
| 実行の設定 | StartTime・StopTime・Interval・Tolerance は experiment annotation（無い項目は既定値: StartTime 0、StopTime は開始 + 1、Tolerance 1e-6、Interval は区間の 1/500）。注釈の Tolerance はその 0.1 倍（`--tolerance-scale`。注釈に無ければ 1e-6 のまま）。ソルバは `--solver auto`（実行できたもの: auto 301 本・auto → dopri45 8 本）。壁時計の予算 30 s（協調型。超えると GM-E11005）。応答が 120 s 無いか、lowering の段に入ってから 150 s で終わらなければ、親がワーカーを止める |
| 時間切れの回し直し | 時間切れ（GM-E11005）と外から止めたもの（lowering の段の上限で止めたものを除く。機械の負荷で応答の上限を超えたものを拾う）だけを、予算 180 s・応答が 240 s 無ければ外から止める設定でもう一度回す（1 回目は JSON の `first_attempt`） |
| 既知の外れの一覧 | `src/Modelica/msl-sim-known-deviations.txt`（3 本。管の外れのうち一覧にあるものを分けて数える。合否の数は変えない） |
| 参照結果の設定（参考） | Dymola の Dassl。Tolerance は experiment annotation の 1/10（無ければ 1e-6）、Interval は annotation より細かい。手法も許容誤差も違うので、点ごとには一致しない |
| 管 | 相対の幅 0.002（時間と値の両方向。csv-compare の既定）。比べる信号は `comparisonSignals.txt`。名前の中の空白は除いて比べる |
| ワーカー | 起動 14 回（外から停止 13 回・異常終了 0 回）、50 本ごとに `Checker::reset` |
| 結果 | **356 本: 実行できた 309（終了時刻まで 309・terminate 0）・コンパイル NG 0・S10 NG 0・実行 NG 41（うち時間切れ 4）・外から停止 6・ワーカー異常終了 0・panic 0** |
| 参照結果との比較 | 参照結果と比べた 308 本: 管に収まった 301（97.7%）・外れ 7・信号が足りない 0・参照結果なし 1・未取得 0・読めない 0 |
| 回し直し | 時間切れ（GM-E11005）と外から停止の回し直し（予算 180 s）: 12 本のうち実行できた 1・管に収まった 0 |
| 前回との差 | 前回との差（両方にある 356 本で）: 終了時刻まで 309 → 309（±0）・実行できた 309 → 309（±0）・管に収まった 301 → 301（±0）（比べた 308 → 308（±0）） |
| P3 の完了条件 | 実行できた 309 本（目標 153 本以上: 達成）・管に収まった 301/308（97.7%、目標 90% 以上: 達成。実行できたうちでは 97.4%）→ **満たす** |

## 集計

| 項目 | 本数 |
| --- | ---: |
| partial のため実行しない（対象に数えない） | 12 |
| 例題でないため実行しない（対象に数えない） | 1 |
| 対象 | 356 |
| 終了時刻まで実行できた | 309 |
| terminate() で終了時刻の前に終わった（GM-I11001） | 0 |
| **実行できた（計）** | 309 |
| DAE まで変換できなかった（compile_retaining の診断） | 0 |
| 設定・プリフライト（S10）で止まった | 0 |
| 実行で止まった | 41 |
| うち協調型の時間切れ（GM-E11005） | 4 |
| うち結果が有限でない（GM-E11006） | 0 |
| 時間切れか外から止めたので回し直した（予算 180 s） | 12 |
| うち回し直して実行できた | 1 |
| 外から止めた（120 s で応答が無いか、lowering が 150 s で終わらない） | 6 |
| ワーカーが応答の前に落ちた | 0 |
| panic（ワーカーの中で拾った） | 0 |
| t0 で有限にならない警告（GM-W10002）が出た | 3 |
| 初期化の解き直しで値が変わる警告（GM-W10004）が出た | 1 |
| ソルバの別名を置き換えた（GM-I10003） | 0 |
| auto で BDF が失敗し dopri45 で回し直した（GM-I11002） | 15 |

### 止まったモデルのコード

| コード | 本数 |
| --- | ---: |
| GM-U0004 | 25 |
| GM-E11001 | 6 |
| GM-E11002 | 4 |
| GM-E11005 | 4 |
| GM-E11003 | 1 |
| GM-U0005 | 1 |

### 外から止めたモデルの段

ワーカーが最後に報告した段（実行系の中の段は S014 から）。

| 段 | 本数 |
| --- | ---: |
| 実行系: lowering | 5 |
| 実行系: S10 のプリフライト | 1 |

### 実行できたモデルのソルバ

| ソルバ | 本数 |
| --- | ---: |
| auto | 301 |
| auto → dopri45 | 8 |

### 参照結果との比較

| 項目 | 本数 |
| --- | ---: |
| 実行できた | 309 |
| うち参照結果と比べた | 308 |
| **管に収まった（すべての信号）** | 301 |
| 管から外れた信号がある | 7 |
| うち既知の外れ（一覧にある） | 3 |
| うち既知の外れを除いたもの | 4 |
| 外れは無いが、どちらかに無い信号がある | 0 |
| 試験側に無い信号がある（外れの有無を問わず） | 0 |
| 参照側に無い信号がある（外れの有無を問わず） | 0 |
| MAP-LIB に参照結果が無い | 1 |
| 参照結果を取得していない | 0 |
| 参照結果を読めない | 0 |
| 管に収まった割合（比べたうち / 実行できたうち） | 97.7% / 97.4% |
| 信号の数で（合格 / 比べた信号） | 1757 / 1773 |

## P3 の完了条件と基準値

P3 の完了条件（[roadmap](roadmap.md)）は「MSL の例題 153 本以上が実行できる。そのうち 90% 以上が参照結果のファネルに収まる」。この実行では 実行できた 309 本（目標 153 本以上: 達成）・管に収まった 301/308（97.7%、目標 90% 以上: 達成。実行できたうちでは 97.4%）→ **満たす**。

基準値は物差しが違う。Rumoca の「一致」は OMC の結果との正規化した平均絶対誤差で、管ではない。OpenModelica の verified は MAP-LIB の参照結果との照合（細かい規則は【未確認】）。

| | 実行できた | 参照と比べた | 合う | 物差し | 出典（確認日） |
| --- | ---: | ---: | ---: | --- | --- |
| **GlobeMode（この実行）** | 309 | 308 | 301 | 矩形の管（幅 0.002、MAP-LIB の参照結果） | - |
| Rumoca の基準値（`msl_quality_baseline.json`） | 216 | - | - | `sim_ok`（試した 286 本、制限 12 s） | [rev 276e82bd](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json)（2026-10-19） |
| Rumoca の文書（2026-06-05 の全 MSL の実行） | 153 | 143（OMC と） | 89（high agreement） | 正規化した平均絶対誤差（管ではない） | [msl-baseline-promotion-analysis-2026-06-05.md](https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/docs/dev-guide/src/tooling/msl-baseline-promotion-analysis-2026-06-05.md)（2026-10-19） |
| OpenModelica master（2026-10-17 の実行、MSL 4.1.0） | 533 | - | 510（verified） | MAP-LIB の参照結果との照合 | [libraries.openmodelica.org](https://libraries.openmodelica.org/branches/master/Modelica_4.1.0/Modelica_4.1.0.html)（2026-10-19） |

## 前回の結果との差

前回の結果 `target/msl-sim.json`（2026-10-31 05:13 UTC、356 本）と、両方にある 356 本で比べる（この実行にあって前回に無いもの 0 本）。

| 項目 | 前回 | この実行 | 差 |
| --- | ---: | ---: | ---: |
| 終了時刻まで進んだ | 309 | 309 | ±0 |
| 実行できた（terminate を含む） | 309 | 309 | ±0 |
| 参照結果と比べた | 308 | 308 | ±0 |
| **管に収まった** | 301 | 301 | ±0 |

- 新しく実行できた: 無い
- 実行できなくなった: 無い
- 新しく管に収まった: 無い
- 管に収まらなくなった: 無い

## 時間とメモリ

| 項目 | 値 |
| --- | --- |
| 時間 | 全体 3890.0 s・ワーカーの中 2829.0 s（コンパイル 209.7 s・実行系の準備 274.12 s・初期化と積分 348.79 s・参照結果の読み込みと比較 6.31 s）・MSL の読み込み 14 回 33.4 s・1 本あたり（親から見た）平均 10.82 s・最大 240.00 s |
| ワーカーの最大の常駐メモリ（VmHWM。用意のとき / 全体） | 1.2 GiB / 3.9 GiB |

### lowering の段の長いもの

親が段の知らせの間隔で測った秒（回し直したものは回し直しの回。`--lowering-kill-seconds` を決める材料。S015）。

| # | モデル | lowering（秒） | 結果 |
| ---: | --- | ---: | --- |
| 1 | `Fluid.Examples.NonCircularPipes` | 150.0 | 外から停止 |
| 2 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 150.0 | 外から停止 |
| 3 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 150.0 | 外から停止 |
| 4 | `Fluid.Examples.InverseParameterization` | 150.0 | 外から停止 |
| 5 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 150.0 | 外から停止 |
| 6 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 115.0 | 外から停止 |
| 7 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 64.1 | 終了時刻まで |
| 8 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 46.7 | 実行 NG GM-E11001 |
| 9 | `Media.Examples.TwoPhaseWater.BaseProperties` | 22.9 | 実行 NG GM-U0004 |
| 10 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | 14.9 | 終了時刻まで |
| 11 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 9.6 | 終了時刻まで |
| 12 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 7.6 | 実行 NG GM-U0004 |
| 13 | `Fluid.Examples.HeatingSystem` | 3.5 | 実行 NG GM-U0004 |
| 14 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 3.4 | 実行 NG GM-U0004 |
| 15 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | 3.0 | 終了時刻まで |

## 実行できなかったモデル

「段階」は最初の Error の段階。外から止めたものと落ちたものは、ワーカーが最後に報告した段。「補足」は最初の Error の補足（Rumoca の文言など。全文は JSON の `notes`）。

| # | モデル | 結果 | 段階 | コード | 文言（先頭） | 補足（先頭） | 秒 |
| ---: | --- | --- | --- | --- | --- | --- | ---: |
| 1 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=8 original_row=8 owner=none(row reads a coordinate outside the planned init… | 0.25 |
| 2 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 14 matched out of 15 equations and 15 unknowns | 0.31 |
| 3 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（thd1.y の値が NaN になった） | rumoca: [Initialization] initial homotopy continuation failed at lambda=0.000000000 step=1.250e-1: non-finite (NaN) value computed for 'thd1.y' @ Span { source: SourceId(9025898607377294338), start: B… | 0.35 |
| 4 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid | 0.08 |
| 5 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=122 target=T2.C.i value=-6.000000e-5 ratio=6.000000e5 norm=6.000000e-5 row_scale=1.000000… | 0.50 |
| 6 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=120 target=T2.C.i value=6.000000e-5 ratio=6.000000e5 norm=6.000000e-5 row_scale=1.000000e… | 0.49 |
| 7 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | Run | GM-E11002 | t = 0.027 で積分に失敗した | rumoca: [ES016] the algebraic loop over 'opAmp.v_out', 'r2.v', 'r2.i', 'r1.i', 'r1.v', 'opAmp.in_p.v', and 1 more reached a fold of 'V0\*v_in&lt;vns': the relation switches its own loop under 'noEvent', s… | 0.13 |
| 8 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 積分の途中の方向微分（ヤコビアン）の計算で止まった | rumoca: [Integration] directional derivative is unavailable: algebraic projection produced a non-finite sensitivity for y[432] | 5.49 |
| 9 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 377 matched out of 392 equations and 392 unknowns | 0.98 |
| 10 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=418 target=rectifier.diode_p.plug_p.pin[3].i value=-7.645419e0 ratio=7.645419e10 norm=7.6… | 3.60 |
| 11 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 899 matched out of 913 equations and 913 unknowns | 2.52 |
| 12 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 183.35 |
| 13 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 644 matched out of 659 equations and 659 unknowns | 2.05 |
| 14 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 469 matched out of 474 equations and 474 unknowns | 0.50 |
| 15 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 899 matched out of 913 equations and 913 unknowns | 2.36 |
| 16 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 1027 matched out of 1034 equations and 1034 unknowns | 1.07 |
| 17 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 569 matched out of 573 equations and 573 unknowns | 0.60 |
| 18 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 768 matched out of 772 equations and 772 unknowns | 0.77 |
| 19 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] DAE system is not computable: affine derivative coefficient is not compile-time numeric | 0.28 |
| 20 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | rumoca: accepted step from t=5.0416408911668364 to t=5.041640891166937 made no progress beyond roundoff | 98.88 |
| 21 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 197.07 |
| 22 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 187.59 |
| 23 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 180.85 |
| 24 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 1760 matched out of 1767 equations and 1767 unknowns | 32.77 |
| 25 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | 実行系: S10 のプリフライト（123 s） | - | 240 s で応答が無いので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: S10 のプリフライト（入ってから 122.9 s） | - | 240.00 |
| 26 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.2 s） | - | 156.06 |
| 27 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.2 s） | - | 154.34 |
| 28 | `Fluid.Examples.HeatingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: invalid structural IR contract without source span: independent state selection: linear solve requires the… | 10.71 |
| 29 | `Fluid.Examples.InverseParameterization` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 155.53 |
| 30 | `Fluid.Examples.NonCircularPipes` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.2 s） | - | 155.09 |
| 31 | `Fluid.Examples.PumpingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 277 matched out of 278 equations and 278 unknowns | 6.15 |
| 32 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | Run | GM-E11003 | t = 1468.683452157 でイベントの反復が収束しない | rumoca: [EventIteration] event condition equations did not converge with fixed pre at t=1468.6834521574738 | 23.46 |
| 33 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 126 matched out of 128 equations and 128 unknowns | 0.17 |
| 34 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 233 matched out of 242 equations and 242 unknowns | 0.44 |
| 35 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 147 matched out of 151 equations and 151 unknowns | 0.32 |
| 36 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 287 matched out of 290 equations and 290 unknowns | 0.54 |
| 37 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 156.40 |
| 38 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 3771 matched out of 3844 equations and 3844 unknowns | 9.11 |
| 39 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=9 original_row=9 owner=none(row reads a coordinate outside the planned init… | 53.92 |
| 40 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=2 original_row=2 owner=none(row reads a coordinate outside the planned init… | 1.01 |
| 41 | `Media.Examples.SimpleLiquidWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 31 matched out of 34 equations and 34 unknowns | 0.31 |
| 42 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 実行系の組み立てで止まった | rumoca: [BackendBuild] solve-IR evaluation failed: [EX002] input 'p' has neither a checked default nor a runtime value | 23.62 |
| 43 | `Media.Examples.WaterIF97` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 28 matched out of 30 equations and 30 unknowns | 0.95 |
| 44 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（cylinder1.h の値が NaN になった） | rumoca: [Initialization] non-finite (NaN) value computed for 'cylinder1.h' / プリフライトの警告: start 値で評価すると t = 0 で式の値が有限にならない: der($state_coordinates[1]) = NaN, der($state_coordinates[2]) = NaN, der($state… | 0.19 |
| 45 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（openTank.h の値が NaN になった） | rumoca: [Initialization] non-finite (NaN) value computed for 'openTank.h' / プリフライトの警告: start 値で評価すると t = 0 で式の値が有限にならない: der(openTank.m) = NaN, der(openTank.H) = NaN, openTank.h = NaN（GM-W10002） | 1.87 |
| 46 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 積分の途中の方向微分（ヤコビアン）の計算で止まった | rumoca: [Integration] directional derivative is unavailable: algebraic projection sensitivity matrix is singular | 0.19 |
| 47 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid | 0.05 |

## 回し直したモデル

| # | モデル | 1 回目（予算・秒） | 回し直し | 参照結果 | 秒 |
| ---: | --- | --- | --- | --- | ---: |
| 1 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG GM-E11005（30 s・32.4 s） | 実行 NG GM-E11005 | - | 183.35 |
| 2 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG GM-E11005（30 s・30.6 s） | 実行 NG GM-U0005 | - | 98.88 |
| 3 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG GM-E11005（30 s・47.6 s） | 実行 NG GM-E11005 | - | 197.07 |
| 4 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG GM-E11005（30 s・38.2 s） | 実行 NG GM-E11005 | - | 187.59 |
| 5 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG GM-E11005（30 s・30.9 s） | 実行 NG GM-E11005 | - | 180.85 |
| 6 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 240.00 |
| 7 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 156.06 |
| 8 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 154.34 |
| 9 | `Fluid.Examples.InverseParameterization` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 155.53 |
| 10 | `Fluid.Examples.NonCircularPipes` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 155.09 |
| 11 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 外から停止 （30 s・120.0 s） | 終了時刻まで | **管 NG** 2/4（外れ 2） | 152.36 |
| 12 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 156.40 |

## 管から外れたモデル

「t」は最初に管の外に出た時刻、「はみ出し」は管からはみ出した量の最大（値の単位）。

| # | モデル | 合格 / 信号 | 外れた信号 | 既知の外れ |
| ---: | --- | --- | --- | --- |
| 1 | `Blocks.Examples.DemoSignalCharacteristic` | 4/5 | `harmonic.mean1.y`（t = 0.0215・はみ出し 1.410e-4・外れ 1005/1050 点） | **既知**: 解析解が 0 の信号（harmonic.mean1.y、第 1 高調波の cos の係数）で、参照も GlobeMode も積分の残差（-1.5e-4 と -9.6e-5）。δy は床 0.001 で決まり 2e-6 しか無い。Rumoca の除外の一覧も comparator_limitation とする（https… |
| 2 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 4/8 | `rotational_clock_1.direction`（t = 1.3976・はみ出し 1.996e0・外れ 5/674 点）、`rotational_clock_1.direction_changed`（t = 1.3976・はみ出し 9.980e-1・外れ 9/674 点）、`sample_conjunctive.y`（t = 0.0560・はみ出し 3.502e0・外れ 48/674 点）、`sample_disjunctive.y`（t = 1.0000・はみ出し 1.960e0・外れ 62/674 点） | - |
| 3 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 1/4 | `sample_angle.y`（t = 1.0000・はみ出し 1.964e0・外れ 126/592 点）、`rotationalClock.direction`（t = 1.3976・はみ出し 1.996e0・外れ 5/592 点）、`rotationalClock.direction_changed`（t = 1.3976・はみ出し 9.980e-1・外れ 9/592 点） | - |
| 4 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 0/3 | `c.v`（t = 0.0340・はみ出し 1.374e1・外れ 934/1001 点）、`opAmp.out.v`（t = 0.0810・はみ出し 2.994e1・外れ 518/1001 点）、`opAmp.out.i`（t = 0.0370・はみ出し 5.711e-2・外れ 874/1001 点） | **既知**: noEvent で包んだ飽和の弛張発振で、周期がソルバに依存し位相がずれる（Rumoca の半周期 0.0472 s、OMC 0.0495 s）。Rumoca の除外の一覧も model_issue とする（https://github.com/CogniPilot/rumoca/blob/276e82bd0fd489… |
| 5 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | 9/11 | `vH01.y_arg`（t = 0.2800・はみ出し 1.313e-2・外れ 201/83395 点）、`iH01.y_arg`（t = 0.2800・はみ出し 1.313e-2・外れ 201/83395 点） | - |
| 6 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | 3/4 | `revolute1.phi`（t = 4.8300・はみ出し 4.857e-3・外れ 3/501 点） | **既知**: 二重振り子（カオス的）で、ずれが積み重なり終わり際（t = 4.83）に revolute1.phi が 0.0049 rad はみ出す。許容誤差を 1/10 にすると 3/4 → 2/4 に悪くなる（解が許容誤差に敏感で、参照の側も同じ）。doc/research/msl-sim-failures.md の節「管から… |
| 7 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 2/4 | `filter.x[1]`（t = 0.0000・はみ出し 5.620e2・外れ 47/1162 点）、`filter.x[2]`（t = 0.0000・はみ出し 5.620e2・外れ 104/1162 点） | - |

## 既知の外れの一覧

| モデル | この実行 | 理由と出典 |
| --- | --- | --- |
| `Blocks.Examples.DemoSignalCharacteristic` | **管 NG** 4/5（外れ 1） | 解析解が 0 の信号（harmonic.mean1.y、第 1 高調波の cos の係数）で、参照も GlobeMode も積分の残差（-1.5e-4 と -9.6e-5）。δy は床 0.001 で決まり 2e-6 しか無い。Rumoca の除外の一覧も comparator_limitation とする（https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_trace_compare_exclu… |
| `Electrical.Analog.Examples.OpAmps.Multivibrator` | **管 NG** 0/3（外れ 3） | noEvent で包んだ飽和の弛張発振で、周期がソルバに依存し位相がずれる（Rumoca の半周期 0.0472 s、OMC 0.0495 s）。Rumoca の除外の一覧も model_issue とする（https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/crates/rumoca-test-msl/tests/msl_tests/msl_trace_compare_exclusions.json の 136 行目、2026-10-19 確認） |
| `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | **管 NG** 3/4（外れ 1） | 二重振り子（カオス的）で、ずれが積み重なり終わり際（t = 4.83）に revolute1.phi が 0.0049 rad はみ出す。許容誤差を 1/10 にすると 3/4 → 2/4 に悪くなる（解が許容誤差に敏感で、参照の側も同じ）。doc/research/msl-sim-failures.md の節「管から外れた 10 本」の 5（2026-10-19、S013 で一覧に載せた） |

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

## 例題でないため実行しないモデル

`msl-funnel` では OK だが、experiment annotation が無く、値の無い入力かパラメータがあるので、値を与えないと実行できない部品とみなして実行しない。

- `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter`

## モデルごとの結果

- 「準備」は実行系の準備（lowering と組み立て）、「実行」は初期化と積分の時間（どちらも実行系が測った秒）。「計」は親が測った 1 本の時間（コンパイルと参照結果の比較を含む）。
- 参照結果の欄の「管 OK a/b」は b 個の信号のうち a 個が管に収まったこと。

| # | モデル | 結果 | コード | ソルバ | 状態 | 点 | 準備 s | 実行 s | 参照結果 | 計 s |
| ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: |
| 1 | `Blocks.Examples.BusUsage` | 終了時刻まで | - | auto | 0 | 502 | 0.00 | 0.00 | 管 OK 5/5 | 1.29 |
| 2 | `Blocks.Examples.BusUsage_Utilities.Part` | 終了時刻まで | - | auto | 0 | 502 | 0.00 | 0.00 | 参照結果なし | 0.21 |
| 3 | `Blocks.Examples.CompareSincExpSine` | 終了時刻まで | - | auto | 0 | 10001 | 0.01 | 0.03 | 管 OK 3/3 | 0.26 |
| 4 | `Blocks.Examples.DemoSignalCharacteristic` | 終了時刻まで | - | auto | 5 | 1051 | 0.03 | 0.02 | **管 NG** 4/5（外れ 1） | 0.29 |
| 5 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 0.25 |
| 6 | `Blocks.Examples.DemonstrateSignalExtrema` | 終了時刻まで | - | auto | 1 | 165001 | 0.01 | 0.96 | 管 OK 8/8 | 1.50 |
| 7 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.31 |
| 8 | `Blocks.Examples.FilterWithRiseTime` | 終了時刻まで | - | auto | 6 | 502 | 0.05 | 0.01 | 管 OK 6/6 | 0.34 |
| 9 | `Blocks.Examples.Interaction1` | 終了時刻まで | - | auto | 0 | 510 | 0.01 | 0.01 | 管 OK 6/6 | 0.36 |
| 10 | `Blocks.Examples.InverseModel` | 終了時刻まで | - | auto | 2 | 502 | 0.01 | 0.00 | 管 OK 2/2 | 0.23 |
| 11 | `Blocks.Examples.LogicalNetwork1` | 終了時刻まで | - | auto | 0 | 509 | 0.01 | 0.00 | 管 OK 3/3 | 0.28 |
| 12 | `Blocks.Examples.Modulation` | 終了時刻まで | - | auto | 5 | 10001 | 0.02 | 0.01 | 管 OK 5/5 | 0.25 |
| 13 | `Blocks.Examples.Noise.Densities` | 終了時刻まで | - | auto | 0 | 1001 | 0.01 | 0.01 | 管 OK 3/3 | 0.22 |
| 14 | `Blocks.Examples.PID_Controller` | 終了時刻まで | - | auto | 6 | 511 | 0.07 | 0.01 | 管 OK 6/6 | 0.42 |
| 15 | `Blocks.Examples.ShowLogicalSources` | 終了時刻まで | - | auto | 0 | 536 | 0.00 | 0.00 | 管 OK 1/1 | 0.27 |
| 16 | `Blocks.Examples.SinCosEncoder` | 終了時刻まで | - | auto | 5 | 20002 | 0.03 | 0.03 | 管 OK 6/6 | 0.34 |
| 17 | `Blocks.Examples.SlewRateLimiter` | 終了時刻まで | - | auto | 3 | 1010 | 0.01 | 0.00 | 管 OK 3/3 | 0.23 |
| 18 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 0.35 |
| 19 | `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | 終了時刻まで | - | auto | 2 | 4009 | 0.09 | 0.30 | 管 OK 4/4 | 0.53 |
| 20 | `Clocked.Examples.CascadeControlledDrive.Continuous` | 終了時刻まで | - | auto | 3 | 509 | 0.09 | 0.01 | 管 OK 4/4 | 0.28 |
| 21 | `Clocked.Examples.CascadeControlledDrive.SubClocked` | 終了時刻まで | - | auto | 2 | 4009 | 0.09 | 0.30 | 管 OK 4/4 | 0.51 |
| 22 | `Clocked.Examples.CascadeControlledDrive.SuperSampled` | 終了時刻まで | - | auto | 2 | 4009 | 0.09 | 0.29 | 管 OK 4/4 | 0.51 |
| 23 | `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 24 | `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 25 | `Clocked.Examples.Elementary.BooleanSignals.BackSample` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 26 | `Clocked.Examples.Elementary.BooleanSignals.Hold` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 27 | `Clocked.Examples.Elementary.BooleanSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 28 | `Clocked.Examples.Elementary.BooleanSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.07 |
| 29 | `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 30 | `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 31 | `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.13 |
| 32 | `Clocked.Examples.Elementary.BooleanSignals.SubSample` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 2/2 | 0.13 |
| 33 | `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.13 |
| 34 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 35 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 36 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 37 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 38 | `Clocked.Examples.Elementary.BooleanSignals.UpSample` | 終了時刻まで | - | auto | 0 | 529 | 0.00 | 0.00 | 管 OK 3/3 | 0.14 |
| 39 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 終了時刻まで | - | auto | 0 | 694 | 0.03 | 0.25 | **管 NG** 4/8（外れ 4） | 0.45 |
| 40 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 終了時刻まで | - | auto | 0 | 594 | 0.01 | 0.01 | **管 NG** 1/4（外れ 3） | 0.11 |
| 41 | `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 1/1 | 0.09 |
| 42 | `Clocked.Examples.Elementary.ClockSignals.SubSample` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 43 | `Clocked.Examples.Elementary.ClockSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 521 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 44 | `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 45 | `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 46 | `Clocked.Examples.Elementary.IntegerSignals.BackSample` | 終了時刻まで | - | auto | 0 | 523 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 47 | `Clocked.Examples.Elementary.IntegerSignals.Hold` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 48 | `Clocked.Examples.Elementary.IntegerSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 49 | `Clocked.Examples.Elementary.IntegerSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.08 |
| 50 | `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 51 | `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 1.86 |
| 52 | `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 523 | 0.00 | 0.00 | 管 OK 2/2 | 0.15 |
| 53 | `Clocked.Examples.Elementary.IntegerSignals.SubSample` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 2/2 | 0.14 |
| 54 | `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.15 |
| 55 | `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 56 | `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 57 | `Clocked.Examples.Elementary.IntegerSignals.UpSample` | 終了時刻まで | - | auto | 0 | 533 | 0.00 | 0.00 | 管 OK 3/3 | 0.15 |
| 58 | `Clocked.Examples.Elementary.RealSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 59 | `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 60 | `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 61 | `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 62 | `Clocked.Examples.Elementary.RealSignals.BackSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 3/3 | 0.09 |
| 63 | `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | 終了時刻まで | - | auto | 0 | 516 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 64 | `Clocked.Examples.Elementary.RealSignals.Hold` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.09 |
| 65 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.10 |
| 66 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.11 |
| 67 | `Clocked.Examples.Elementary.RealSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 68 | `Clocked.Examples.Elementary.RealSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 69 | `Clocked.Examples.Elementary.RealSignals.Sample3` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.08 |
| 70 | `Clocked.Examples.Elementary.RealSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 71 | `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 72 | `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.11 |
| 73 | `Clocked.Examples.Elementary.RealSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 74 | `Clocked.Examples.Elementary.RealSignals.SubSample` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 75 | `Clocked.Examples.Elementary.RealSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 521 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 76 | `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | 終了時刻まで | - | auto | 0 | 518 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 77 | `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 78 | `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 79 | `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 80 | `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 81 | `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 82 | `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 83 | `Clocked.Examples.Elementary.RealSignals.UniformNoise` | 終了時刻まで | - | auto | 0 | 606 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 84 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.08 |
| 85 | `Clocked.Examples.Elementary.RealSignals.UpSample1` | 終了時刻まで | - | auto | 0 | 527 | 0.00 | 0.00 | 管 OK 3/3 | 0.08 |
| 86 | `Clocked.Examples.Elementary.RealSignals.UpSample2` | 終了時刻まで | - | auto | 0 | 518 | 0.00 | 0.00 | 管 OK 5/5 | 0.10 |
| 87 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 88 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.01 | 管 OK 4/4 | 0.11 |
| 89 | `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | 終了時刻まで | - | auto | 2 | 601 | 0.03 | 0.01 | 管 OK 4/4 | 0.17 |
| 90 | `Clocked.Examples.SimpleControlledDrive.Continuous` | 終了時刻まで | - | auto | 3 | 502 | 0.01 | 0.00 | 管 OK 4/4 | 0.18 |
| 91 | `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.01 | 管 OK 4/4 | 0.13 |
| 92 | `Clocked.Examples.Systems.EngineThrottleControl` | 終了時刻まで | - | auto | 5 | 645 | 0.05 | 0.02 | 管 OK 8/8 | 0.23 |
| 93 | `ComplexBlocks.Examples.TestConversionBlock` | 終了時刻まで | - | auto | 0 | 1012 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 94 | `Electrical.Analog.Examples.AmplifierWithOpAmpDetailed` | 終了時刻まで | - | auto | 7 | 2501 | 0.05 | 0.06 | 管 OK 7/7 | 0.23 |
| 95 | `Electrical.Analog.Examples.CauerLowPassAnalog` | 終了時刻まで | - | auto | 5 | 503 | 0.03 | 0.01 | 管 OK 5/5 | 0.15 |
| 96 | `Electrical.Analog.Examples.CauerLowPassOPV` | 終了時刻まで | - | auto | 5 | 503 | 0.10 | 0.01 | 管 OK 5/5 | 0.29 |
| 97 | `Electrical.Analog.Examples.CauerLowPassSC` | 終了時刻まで | - | auto | 16 | 3601 | 0.28 | 16.86 | 管 OK 5/5 | 17.49 |
| 98 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | 終了時刻まで | - | auto | 0 | 510 | 0.03 | 0.02 | 管 OK 3/3 | 0.15 |
| 99 | `Electrical.Analog.Examples.CharacteristicThyristors` | 終了時刻まで | - | auto | 0 | 559 | 0.03 | 0.06 | 管 OK 3/3 | 0.27 |
| 100 | `Electrical.Analog.Examples.ChuaCircuit` | 終了時刻まで | - | auto | 3 | 50219 | 0.02 | 0.11 | 管 OK 3/3 | 0.23 |
| 101 | `Electrical.Analog.Examples.CompareTransformers` | 終了時刻まで | - | auto | 4 | 50001 | 0.07 | 0.59 | 管 OK 3/3 | 2.39 |
| 102 | `Electrical.Analog.Examples.ControlledSwitchWithArc` | 終了時刻まで | - | auto | 2 | 24061 | 0.03 | 0.21 | 管 OK 2/2 | 0.38 |
| 103 | `Electrical.Analog.Examples.DemoPowerSupply` | 終了時刻まで | - | auto | 0 | 10006 | 0.01 | 0.15 | 管 OK 3/3 | 0.26 |
| 104 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | 終了時刻まで | - | auto | 1 | 107683 | 0.03 | 2.33 | 管 OK 4/4 | 2.56 |
| 105 | `Electrical.Analog.Examples.DemonstrateLightning` | 終了時刻まで | - | auto | 2 | 10002 | 0.07 | 0.02 | 管 OK 6/6 | 0.24 |
| 106 | `Electrical.Analog.Examples.DifferenceAmplifier` | 終了時刻まで | - | auto | 9 | 502 | 0.17 | 0.02 | 管 OK 9/9 | 0.38 |
| 107 | `Electrical.Analog.Examples.GenerationOfFMUs` | 終了時刻まで | - | auto | 6 | 1001 | 0.19 | 0.02 | 管 OK 6/6 | 0.40 |
| 108 | `Electrical.Analog.Examples.HeatingMOSInverter` | 終了時刻まで | - | auto | 2 | 584 | 0.07 | 0.11 | 管 OK 2/2 | 0.31 |
| 109 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.50 |
| 110 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.49 |
| 111 | `Electrical.Analog.Examples.HeatingRectifier` | 終了時刻まで | - | auto | 2 | 501 | 0.04 | 0.02 | 管 OK 2/2 | 0.16 |
| 112 | `Electrical.Analog.Examples.IdealTriacCircuit` | 終了時刻まで | - | auto | 1 | 561 | 0.03 | 0.03 | 管 OK 3/3 | 0.17 |
| 113 | `Electrical.Analog.Examples.InvertingAmp` | 終了時刻まで | - | auto | 0 | 1046 | 0.02 | 0.02 | 管 OK 1/1 | 0.14 |
| 114 | `Electrical.Analog.Examples.Lines.CompareLineTrunks` | 終了時刻まで | - | auto | 57 | 1004 | 0.68 | 0.39 | 管 OK 9/9 | 1.74 |
| 115 | `Electrical.Analog.Examples.Lines.CompareLosslessLines` | 終了時刻まで | - | auto | 0 | 209 | 0.03 | 0.01 | 管 OK 12/12 | 0.16 |
| 116 | `Electrical.Analog.Examples.Lines.LightningLosslessTransmissionLine` | 終了時刻まで | - | auto | 0 | 25002 | 0.02 | 0.68 | 管 OK 3/3 | 0.84 |
| 117 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 終了時刻まで | GM-I11002 | auto → dopri45 | 402 | 25002 | 41.19 | 63.79 | 管 OK 3/3 | 110.26 |
| 118 | `Electrical.Analog.Examples.Lines.SmoothStep` | 終了時刻まで | - | auto | 116 | 3002 | 1.81 | 0.65 | 管 OK 4/4 | 3.74 |
| 119 | `Electrical.Analog.Examples.NandGate` | 終了時刻まで | - | auto | 2 | 526 | 0.08 | 0.01 | 管 OK 2/2 | 0.25 |
| 120 | `Electrical.Analog.Examples.OpAmps.Adder` | 終了時刻まで | - | auto | 0 | 1001 | 0.02 | 0.06 | 管 OK 1/1 | 0.19 |
| 121 | `Electrical.Analog.Examples.OpAmps.Comparator` | 終了時刻まで | - | auto | 0 | 1046 | 0.02 | 0.01 | 管 OK 2/2 | 0.14 |
| 122 | `Electrical.Analog.Examples.OpAmps.ControlCircuit` | 終了時刻まで | - | auto | 6 | 1003 | 0.13 | 0.06 | 管 OK 8/8 | 0.42 |
| 123 | `Electrical.Analog.Examples.OpAmps.DifferentialAmplifier` | 終了時刻まで | - | auto | 1 | 2011 | 0.06 | 0.22 | 管 OK 2/2 | 0.43 |
| 124 | `Electrical.Analog.Examples.OpAmps.Differentiator` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.02 | 管 OK 3/3 | 0.16 |
| 125 | `Electrical.Analog.Examples.OpAmps.HighPass` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.05 | 管 OK 3/3 | 0.20 |
| 126 | `Electrical.Analog.Examples.OpAmps.Integrator` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.02 | 管 OK 3/3 | 0.16 |
| 127 | `Electrical.Analog.Examples.OpAmps.InvertingAmplifier` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.03 | 管 OK 2/2 | 0.15 |
| 128 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.13 |
| 129 | `Electrical.Analog.Examples.OpAmps.LCOscillator` | 終了時刻まで | - | auto | 2 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.13 |
| 130 | `Electrical.Analog.Examples.OpAmps.LowPass` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.02 | 管 OK 3/3 | 0.16 |
| 131 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 終了時刻まで | - | auto | 1 | 1001 | 0.02 | 0.02 | **管 NG** 0/3（外れ 3） | 0.14 |
| 132 | `Electrical.Analog.Examples.OpAmps.NonInvertingAmplifier` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.02 | 管 OK 2/2 | 0.15 |
| 133 | `Electrical.Analog.Examples.OpAmps.SchmittTrigger` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 134 | `Electrical.Analog.Examples.OpAmps.SignalGenerator` | 終了時刻まで | - | auto | 1 | 1041 | 0.03 | 0.04 | 管 OK 5/5 | 0.18 |
| 135 | `Electrical.Analog.Examples.OpAmps.Subtracter` | 終了時刻まで | - | auto | 0 | 1001 | 0.02 | 0.01 | 管 OK 1/1 | 0.14 |
| 136 | `Electrical.Analog.Examples.OpAmps.VoltageFollower` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 137 | `Electrical.Analog.Examples.OvervoltageProtection` | 終了時刻まで | - | auto | 1 | 501 | 0.03 | 0.02 | 管 OK 1/1 | 0.15 |
| 138 | `Electrical.Analog.Examples.ParallelResonance` | 終了時刻まで | - | auto | 6 | 10002 | 0.03 | 0.04 | 管 OK 6/6 | 0.19 |
| 139 | `Electrical.Analog.Examples.Rectifier` | 終了時刻まで | - | auto | 5 | 14522 | 0.08 | 4.07 | 管 OK 4/4 | 4.30 |
| 140 | `Electrical.Analog.Examples.Resistor` | 終了時刻まで | - | auto | 0 | 501 | 0.01 | 0.01 | 管 OK 2/2 | 0.11 |
| 141 | `Electrical.Analog.Examples.ResonanceCircuits` | 終了時刻まで | - | auto | 4 | 10001 | 0.16 | 0.04 | 管 OK 3/3 | 0.39 |
| 142 | `Electrical.Analog.Examples.SeriesResonance` | 終了時刻まで | - | auto | 6 | 10002 | 0.03 | 0.03 | 管 OK 6/6 | 0.18 |
| 143 | `Electrical.Analog.Examples.ShowSaturatingInductor` | 終了時刻まで | - | auto | 2 | 630 | 0.01 | 0.01 | 管 OK 2/2 | 0.10 |
| 144 | `Electrical.Analog.Examples.ShowVariableResistor` | 終了時刻まで | - | auto | 0 | 501 | 0.02 | 0.01 | 管 OK 2/2 | 0.12 |
| 145 | `Electrical.Analog.Examples.SimpleTriacCircuit` | 終了時刻まで | GM-W10003 | auto | 3 | 2485 | 0.08 | 1.43 | 管 OK 3/3 | 1.64 |
| 146 | `Electrical.Analog.Examples.SwitchWithArc` | 終了時刻まで | - | auto | 2 | 8010 | 0.03 | 0.06 | 管 OK 2/2 | 0.20 |
| 147 | `Electrical.Analog.Examples.ThyristorBehaviourTest` | 終了時刻まで | - | auto | 2 | 517 | 0.04 | 0.04 | 管 OK 2/2 | 0.18 |
| 148 | `Electrical.Batteries.Examples.CCCV_Cell` | 終了時刻まで | - | auto | 3 | 12008 | 0.05 | 0.10 | 管 OK 3/3 | 0.38 |
| 149 | `Electrical.Batteries.Examples.SuperCapDischargeCharge` | 終了時刻まで | - | auto | 2 | 72021 | 0.03 | 0.09 | 管 OK 2/2 | 0.31 |
| 150 | `Electrical.Machines.Examples.DCMachines.DCEE_Start` | 終了時刻まで | - | auto | 4 | 20004 | 0.25 | 0.20 | 管 OK 4/4 | 0.77 |
| 151 | `Electrical.Machines.Examples.DCMachines.DCPM_Cooling` | 終了時刻まで | - | auto | 5 | 250051 | 0.21 | 4.20 | 管 OK 5/5 | 7.19 |
| 152 | `Electrical.Machines.Examples.DCMachines.DCPM_CurrentControlled` | 終了時刻まで | - | auto | 5 | 20002 | 0.22 | 0.15 | 管 OK 5/5 | 0.89 |
| 153 | `Electrical.Machines.Examples.DCMachines.DCPM_Drive` | 終了時刻まで | - | auto | 16 | 20006 | 0.81 | 0.38 | 管 OK 4/4 | 1.85 |
| 154 | `Electrical.Machines.Examples.DCMachines.DCPM_QuasiStatic` | 終了時刻まで | - | auto | 5 | 20005 | 0.49 | 0.24 | 管 OK 8/8 | 1.23 |
| 155 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | 終了時刻まで | - | auto | 3 | 20004 | 0.16 | 0.12 | 管 OK 3/3 | 0.61 |
| 156 | `Electrical.Machines.Examples.DCMachines.DCPM_Temperature` | 終了時刻まで | - | auto | 3 | 30002 | 0.19 | 0.18 | 管 OK 3/3 | 0.73 |
| 157 | `Electrical.Machines.Examples.DCMachines.DCPM_withLosses` | 終了時刻まで | - | auto | 6 | 20005 | 0.48 | 0.36 | 管 OK 6/6 | 1.34 |
| 158 | `Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase` | 終了時刻まで | - | auto | 3 | 20003 | 0.37 | 0.44 | 管 OK 3/3 | 1.19 |
| 159 | `Electrical.Machines.Examples.DCMachines.DCSE_Start` | 終了時刻まで | - | auto | 3 | 20003 | 0.32 | 0.28 | 管 OK 3/3 | 0.93 |
| 160 | `Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics` | 終了時刻まで | - | auto | 6 | 6003 | 0.72 | 0.18 | 管 OK 12/12 | 1.57 |
| 161 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 5.49 |
| 162 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.98 |
| 163 | `Electrical.Machines.Examples.InductionMachines.IMC_DOL` | 終了時刻まで | - | auto | 7 | 15002 | 0.51 | 0.49 | 管 OK 8/8 | 1.65 |
| 164 | `Electrical.Machines.Examples.InductionMachines.IMC_Initialize` | 終了時刻まで | - | auto | 7 | 15002 | 0.50 | 0.42 | 管 OK 8/8 | 1.52 |
| 165 | `Electrical.Machines.Examples.InductionMachines.IMC_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 0.90 | 0.60 | 管 OK 9/9 | 2.05 |
| 166 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 3.60 |
| 167 | `Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz` | 終了時刻まで | - | auto | 9 | 10004 | 0.51 | 0.89 | 管 OK 10/10 | 1.98 |
| 168 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 2.52 |
| 169 | `Electrical.Machines.Examples.InductionMachines.IMC_YD` | 終了時刻まで | - | auto | 7 | 25003 | 0.63 | 1.63 | 管 OK 8/8 | 3.06 |
| 170 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | GM-E11005 | auto | - | - | - | - | - | 183.35 |
| 171 | `Electrical.Machines.Examples.InductionMachines.IMC_withLosses` | 終了時刻まで | - | auto | 8 | 50002 | 0.65 | 2.09 | 管 OK 9/9 | 3.64 |
| 172 | `Electrical.Machines.Examples.InductionMachines.IMS_Start` | 終了時刻まで | - | auto | 8 | 15003 | 0.76 | 1.05 | 管 OK 10/10 | 2.74 |
| 173 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL` | 終了時刻まで | - | auto | 7 | 30004 | 1.31 | 1.37 | 管 OK 9/9 | 3.56 |
| 174 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator` | 終了時刻まで | - | auto | 5 | 30002 | 1.64 | 1.19 | 管 OK 7/7 | 3.65 |
| 175 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier` | 終了時刻まで | - | auto | 12 | 12450 | 1.43 | 29.22 | 管 OK 14/14 | 31.75 |
| 176 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking` | 終了時刻まで | - | auto | 5 | 8289 | 0.74 | 1.26 | 管 OK 6/6 | 2.76 |
| 177 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 2.05 |
| 178 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 1.40 | 0.63 | 管 OK 9/9 | 2.71 |
| 179 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad` | 終了時刻まで | - | auto | 1 | 3221 | 0.78 | 0.05 | 管 OK 3/3 | 1.35 |
| 180 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking` | 終了時刻まで | - | auto | 4 | 1003 | 1.32 | 0.11 | 管 OK 9/9 | 2.19 |
| 181 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource` | 終了時刻まで | - | auto | 7 | 20001 | 2.04 | 0.77 | 管 OK 8/8 | 3.66 |
| 182 | `Electrical.Machines.Examples.SynchronousMachines.SMR_DOL` | 終了時刻まで | - | auto | 6 | 2503 | 1.16 | 0.21 | 管 OK 8/8 | 2.04 |
| 183 | `Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 1.07 | 0.59 | 管 OK 9/9 | 2.29 |
| 184 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.50 |
| 185 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 2.36 |
| 186 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 1.07 |
| 187 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.60 |
| 188 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.77 |
| 189 | `Electrical.Polyphase.Examples.Rectifier` | 終了時刻まで | - | auto | 4 | 10121 | 0.27 | 0.61 | 管 OK 4/4 | 1.29 |
| 190 | `Electrical.Polyphase.Examples.TestSensors` | 終了時刻まで | - | auto | 2 | 1001 | 0.25 | 0.02 | 管 OK 2/2 | 0.68 |
| 191 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.28 |
| 192 | `Electrical.Polyphase.Examples.TransformerYY` | 終了時刻まで | - | auto | 2 | 1001 | 0.11 | 0.01 | 管 OK 2/2 | 0.37 |
| 193 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | 終了時刻まで | - | auto | 10 | 83783 | 0.17 | 5.05 | **管 NG** 9/11（外れ 2） | 5.88 |
| 194 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | GM-U0005 | auto | - | - | - | - | - | 98.88 |
| 195 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R` | 終了時刻まで | - | auto | 3 | 536 | 0.05 | 0.03 | 管 OK 9/9 | 0.25 |
| 196 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R_Characteristic` | 終了時刻まで | - | auto | 3 | 53224 | 0.05 | 2.38 | 管 OK 9/9 | 2.77 |
| 197 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.DiodeBridge2Pulse` | 終了時刻まで | - | auto | 3 | 519 | 0.05 | 0.05 | 管 OK 6/6 | 0.25 |
| 198 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.HalfControlledBridge2Pulse` | 終了時刻まで | - | auto | 3 | 539 | 0.07 | 0.07 | 管 OK 10/10 | 0.37 |
| 199 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | GM-E11005・GM-I11002 | auto → dopri45 | - | - | - | - | - | 197.07 |
| 200 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_R` | 終了時刻まで | - | auto | 3 | 539 | 0.07 | 0.07 | 管 OK 12/12 | 0.35 |
| 201 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RL` | 終了時刻まで | - | auto | 4 | 559 | 0.09 | 0.08 | 管 OK 13/13 | 2.38 |
| 202 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV` | 終了時刻まで | - | auto | 4 | 559 | 0.09 | 0.08 | 管 OK 13/13 | 0.42 |
| 203 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 55456 | 0.09 | 5.18 | 管 OK 13/13 | 5.72 |
| 204 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse` | 終了時刻まで | - | auto | 3 | 671 | 0.13 | 0.21 | 管 OK 6/6 | 0.65 |
| 205 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse` | 終了時刻まで | - | auto | 3 | 722 | 0.19 | 0.52 | 管 OK 18/18 | 1.17 |
| 206 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | GM-E11005・GM-I11002 | auto → dopri45 | - | - | - | - | - | 187.59 |
| 207 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R` | 終了時刻まで | - | auto | 3 | 690 | 0.18 | 0.48 | 管 OK 24/24 | 1.09 |
| 208 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL` | 終了時刻まで | - | auto | 4 | 694 | 0.18 | 0.37 | 管 OK 25/25 | 0.99 |
| 209 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV` | 終了時刻まで | - | auto | 4 | 697 | 0.18 | 0.37 | 管 OK 25/25 | 1.02 |
| 210 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 73993 | 0.19 | 52.09 | 管 OK 25/25 | 53.26 |
| 211 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.DiodeCenterTap2Pulse` | 終了時刻まで | - | auto | 3 | 519 | 0.05 | 0.03 | 管 OK 6/6 | 0.25 |
| 212 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_R` | 終了時刻まで | - | auto | 3 | 539 | 0.07 | 0.05 | 管 OK 10/10 | 0.34 |
| 213 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RL` | 終了時刻まで | - | auto | 4 | 559 | 0.07 | 0.05 | 管 OK 11/11 | 0.34 |
| 214 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV` | 終了時刻まで | - | auto | 4 | 559 | 0.07 | 0.05 | 管 OK 11/11 | 0.34 |
| 215 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 55456 | 0.08 | 2.91 | 管 OK 11/11 | 3.39 |
| 216 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.DiodeCenterTap2mPulse` | 終了時刻まで | - | auto | 3 | 671 | 0.13 | 0.16 | 管 OK 6/6 | 0.59 |
| 217 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R` | 終了時刻まで | - | auto | 3 | 736 | 0.18 | 0.31 | 管 OK 18/18 | 0.95 |
| 218 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL` | 終了時刻まで | - | auto | 4 | 736 | 0.18 | 0.27 | 管 OK 19/19 | 0.92 |
| 219 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV` | 終了時刻まで | - | auto | 4 | 740 | 0.18 | 0.28 | 管 OK 19/19 | 0.93 |
| 220 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 72509 | 0.20 | 15.78 | 管 OK 19/19 | 16.85 |
| 221 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.DiodeCenterTapmPulse` | 終了時刻まで | - | auto | 3 | 586 | 0.08 | 0.07 | 管 OK 6/6 | 0.39 |
| 222 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R` | 終了時刻まで | - | auto | 3 | 652 | 0.13 | 0.19 | 管 OK 15/15 | 0.67 |
| 223 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL` | 終了時刻まで | - | auto | 4 | 645 | 0.14 | 0.15 | 管 OK 16/16 | 0.64 |
| 224 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV` | 終了時刻まで | - | auto | 4 | 646 | 0.13 | 0.15 | 管 OK 16/16 | 0.64 |
| 225 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 68902 | 0.14 | 10.96 | 管 OK 16/16 | 11.80 |
| 226 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | 終了時刻まで | - | auto | 12 | 5652 | 0.28 | 1.53 | 管 OK 24/24 | 2.40 |
| 227 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | 終了時刻まで | - | auto | 29 | 6318 | 1.10 | 6.29 | 管 OK 47/47 | 8.45 |
| 228 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM` | 終了時刻まで | - | auto | 7 | 4569 | 0.24 | 2.37 | 管 OK 14/14 | 3.19 |
| 229 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_R` | 終了時刻まで | - | auto | 4 | 781 | 0.09 | 0.38 | 管 OK 6/6 | 0.69 |
| 230 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_RL` | 終了時刻まで | - | auto | 5 | 801 | 0.09 | 0.36 | 管 OK 7/7 | 0.68 |
| 231 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | 終了時刻まで | GM-I11002 | auto → dopri45 | 7 | 108027 | 0.08 | 18.72 | 管 OK 4/4 | 19.15 |
| 232 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_R` | 終了時刻まで | - | auto | 4 | 10215 | 0.06 | 0.26 | 管 OK 8/8 | 0.52 |
| 233 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_RL` | 終了時刻まで | - | auto | 5 | 10245 | 0.05 | 0.27 | 管 OK 9/9 | 0.54 |
| 234 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepUp.ChopperStepUp_R` | 終了時刻まで | - | auto | 4 | 10226 | 0.05 | 0.24 | 管 OK 10/10 | 0.50 |
| 235 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | GM-E11005 | auto | - | - | - | - | - | 180.85 |
| 236 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_R` | 終了時刻まで | - | auto | 2 | 701 | 0.09 | 0.29 | 管 OK 6/6 | 0.61 |
| 237 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_RL` | 終了時刻まで | - | auto | 3 | 701 | 0.09 | 0.40 | 管 OK 7/7 | 0.73 |
| 238 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_TrianglePWM_RL` | 終了時刻まで | - | auto | 3 | 1293 | 0.10 | 0.72 | 管 OK 7/7 | 1.07 |
| 239 | `Electrical.Spice3.Examples.CoupledInductors` | 終了時刻まで | - | auto | 5 | 501 | 0.05 | 0.06 | 管 OK 3/3 | 0.61 |
| 240 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 32.77 |
| 241 | `Fluid.Examples.AST_BatchPlant.Test.OneTank` | 終了時刻まで | - | auto | 2 | 1116 | 2.01 | 2.40 | 管 OK 8/8 | 6.68 |
| 242 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe1` | 終了時刻まで | - | auto | 2 | 505 | 2.31 | 0.04 | 管 OK 4/4 | 4.53 |
| 243 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe2` | 終了時刻まで | - | auto | 2 | 509 | 2.25 | 0.08 | 管 OK 4/4 | 4.71 |
| 244 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe1` | 終了時刻まで | - | auto | 4 | 523 | 2.49 | 0.26 | 管 OK 8/8 | 5.87 |
| 245 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe2` | 終了時刻まで | - | auto | 6 | 513 | 2.66 | 0.15 | 管 OK 12/12 | 7.68 |
| 246 | `Fluid.Examples.AST_BatchPlant.Test.TwoTanks` | 終了時刻まで | - | auto | 4 | 501 | 2.06 | 0.04 | 管 OK 6/6 | 3.83 |
| 247 | `Fluid.Examples.ControlledTankSystem.ControlledTanks` | 終了時刻まで | - | auto | 4 | 535 | 0.63 | 0.25 | 管 OK 23/23 | 3.95 |
| 248 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | - | - | - | - | - | - | - | 240.00 |
| 249 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | - | - | - | - | - | - | - | 156.06 |
| 250 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | - | - | - | - | - | - | - | 154.34 |
| 251 | `Fluid.Examples.HeatingSystem` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 10.71 |
| 252 | `Fluid.Examples.InverseParameterization` | 外から停止 | - | - | - | - | - | - | - | 155.53 |
| 253 | `Fluid.Examples.NonCircularPipes` | 外から停止 | - | - | - | - | - | - | - | 155.09 |
| 254 | `Fluid.Examples.PumpingSystem` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 6.15 |
| 255 | `Fluid.Examples.Tanks.EmptyTanks` | 終了時刻まで | - | auto | 4 | 505 | 2.39 | 0.05 | 管 OK 3/3 | 4.10 |
| 256 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | GM-E11003・GM-I11002 | auto → dopri45 | - | - | - | - | - | 23.46 |
| 257 | `Fluid.Examples.Tanks.ThreeTanks` | 終了時刻まで | - | auto | 6 | 501 | 2.83 | 5.76 | 管 OK 6/6 | 12.32 |
| 258 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.17 |
| 259 | `Magnetic.FluxTubes.Examples.BasicExamples.SaturatedInductor` | 終了時刻まで | - | auto | 1 | 501 | 0.04 | 0.01 | 管 OK 6/6 | 0.18 |
| 260 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | 終了時刻まで | - | auto | 0 | 503 | 0.04 | 0.01 | 管 OK 2/2 | 0.17 |
| 261 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | 終了時刻まで | - | auto | 0 | 503 | 0.04 | 0.01 | 管 OK 2/2 | 0.17 |
| 262 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ArmatureStroke` | 終了時刻まで | - | auto | 6 | 5029 | 0.31 | 0.13 | 管 OK 9/9 | 0.70 |
| 263 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.44 |
| 264 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.32 |
| 265 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonPullInStroke` | 終了時刻まで | - | auto | 9 | 1045 | 0.89 | 2.84 | 管 OK 9/9 | 4.26 |
| 266 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonQuasiStatic` | 終了時刻まで | - | auto | 9 | 502 | 0.57 | 1.68 | 管 OK 28/28 | 2.70 |
| 267 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.54 |
| 268 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | 終了時刻まで | - | auto | 8 | 501 | 4.65 | 0.13 | 管 OK 10/10 | 6.24 |
| 269 | `Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint` | 終了時刻まで | - | auto | 4 | 501 | 3.19 | 0.48 | 管 OK 5/5 | 4.99 |
| 270 | `Mechanics.MultiBody.Examples.Constraints.SphericalConstraint` | 終了時刻まで | - | auto | 13 | 501 | 2.47 | 0.10 | 管 OK 3/3 | 3.85 |
| 271 | `Mechanics.MultiBody.Examples.Constraints.UniversalConstraint` | 終了時刻まで | - | auto | 8 | 509 | 4.06 | 0.69 | 管 OK 7/7 | 6.87 |
| 272 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulum` | 終了時刻まで | - | auto | 4 | 501 | 1.02 | 0.04 | 管 OK 4/4 | 1.85 |
| 273 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | 終了時刻まで | - | auto | 4 | 501 | 1.64 | 0.14 | **管 NG** 3/4（外れ 1） | 2.76 |
| 274 | `Mechanics.MultiBody.Examples.Elementary.ForceAndTorque` | 終了時刻まで | - | auto | 4 | 501 | 1.04 | 0.04 | 管 OK 4/4 | 2.01 |
| 275 | `Mechanics.MultiBody.Examples.Elementary.FreeBody` | 終了時刻まで | - | auto | 13 | 501 | 0.95 | 0.03 | 管 OK 9/9 | 1.73 |
| 276 | `Mechanics.MultiBody.Examples.Elementary.HeatLosses` | 終了時刻まで | - | auto | 40 | 501 | 1.72 | 0.06 | 管 OK 28/28 | 2.79 |
| 277 | `Mechanics.MultiBody.Examples.Elementary.InitSpringConstant` | 終了時刻まで | - | auto | 2 | 501 | 0.87 | 0.02 | 管 OK 2/2 | 1.55 |
| 278 | `Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses` | 終了時刻まで | - | auto | 4 | 501 | 2.49 | 0.07 | 管 OK 4/4 | 3.75 |
| 279 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | 終了時刻まで | - | auto | 2 | 501 | 0.61 | 0.01 | 管 OK 2/2 | 1.01 |
| 280 | `Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper` | 終了時刻まで | - | auto | 4 | 501 | 0.88 | 0.03 | 管 OK 4/4 | 1.56 |
| 281 | `Mechanics.MultiBody.Examples.Elementary.PointGravity` | 終了時刻まで | - | auto | 26 | 501 | 0.70 | 0.04 | 管 OK 18/18 | 1.11 |
| 282 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses` | 終了時刻まで | - | auto | 24 | 501 | 0.51 | 0.02 | 管 OK 24/24 | 0.98 |
| 283 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2` | 終了時刻まで | - | auto | 26 | 501 | 4.57 | 0.30 | 管 OK 15/15 | 6.05 |
| 284 | `Mechanics.MultiBody.Examples.Elementary.RollingWheel` | 終了時刻まで | - | auto | 8 | 501 | 0.92 | 0.08 | 管 OK 8/8 | 1.41 |
| 285 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving` | 終了時刻まで | - | auto | 10 | 501 | 2.37 | 0.33 | 管 OK 7/7 | 3.57 |
| 286 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling` | 終了時刻まで | - | auto | 10 | 506 | 2.31 | 0.13 | 管 OK 7/7 | 3.50 |
| 287 | `Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem` | 終了時刻まで | - | auto | 15 | 501 | 1.30 | 0.04 | 管 OK 11/11 | 2.18 |
| 288 | `Mechanics.MultiBody.Examples.Elementary.SpringMassSystem` | 終了時刻まで | - | auto | 4 | 501 | 1.01 | 0.02 | 管 OK 4/4 | 1.71 |
| 289 | `Mechanics.MultiBody.Examples.Elementary.SpringWithMass` | 終了時刻まで | - | auto | 13 | 501 | 0.69 | 0.02 | 管 OK 9/9 | 1.31 |
| 290 | `Mechanics.MultiBody.Examples.Elementary.Surfaces` | 終了時刻まで | - | auto | 2 | 501 | 0.59 | 0.02 | 管 OK 2/2 | 1.08 |
| 291 | `Mechanics.MultiBody.Examples.Elementary.ThreeSprings` | 終了時刻まで | - | auto | 13 | 501 | 0.95 | 0.10 | 管 OK 9/9 | 1.90 |
| 292 | `Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField` | 終了時刻まで | - | auto | 2 | 501 | 0.68 | 0.32 | 管 OK 2/2 | 1.46 |
| 293 | `Mechanics.MultiBody.Examples.Loops.Engine1a` | 終了時刻まで | - | auto | 2 | 501 | 3.26 | 0.21 | 管 OK 2/2 | 5.15 |
| 294 | `Mechanics.MultiBody.Examples.Loops.Engine1b` | 終了時刻まで | - | auto | 2 | 527 | 3.57 | 0.35 | 管 OK 2/2 | 5.66 |
| 295 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | 終了時刻まで | - | auto | 2 | 527 | 18.69 | 0.62 | 管 OK 2/2 | 21.33 |
| 296 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 終了時刻まで | GM-W10004 | auto | 4 | 1162 | 86.40 | 51.54 | **管 NG** 2/4（外れ 2） | 152.36 |
| 297 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | - | - | - | - | - | - | - | 156.40 |
| 298 | `Mechanics.MultiBody.Examples.Loops.Fourbar1` | 終了時刻まで | - | auto | 2 | 501 | 2.87 | 0.19 | 管 OK 2/2 | 5.34 |
| 299 | `Mechanics.MultiBody.Examples.Loops.Fourbar2` | 終了時刻まで | - | auto | 2 | 501 | 2.29 | 0.10 | 管 OK 2/2 | 3.45 |
| 300 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | 終了時刻まで | - | auto | 2 | 501 | 4.18 | 0.11 | 管 OK 2/2 | 6.55 |
| 301 | `Mechanics.MultiBody.Examples.Loops.PlanarFourbar` | 終了時刻まで | - | auto | 2 | 501 | 3.10 | 0.20 | 管 OK 2/2 | 4.91 |
| 302 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 9.11 |
| 303 | `Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive` | 終了時刻まで | - | auto | 4 | 501 | 0.92 | 0.02 | 管 OK 4/4 | 1.70 |
| 304 | `Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D` | 終了時刻まで | - | auto | 8 | 501 | 1.65 | 0.90 | 管 OK 8/8 | 3.63 |
| 305 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint` | 終了時刻まで | - | auto | 4 | 501 | 1.73 | 0.08 | 管 OK 4/4 | 2.85 |
| 306 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects` | 終了時刻まで | - | auto | 18 | 501 | 3.44 | 0.52 | 管 OK 10/10 | 5.54 |
| 307 | `Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive` | 終了時刻まで | - | auto | 8 | 501 | 1.21 | 0.03 | 管 OK 8/8 | 2.15 |
| 308 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 53.92 |
| 309 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 1.01 |
| 310 | `Mechanics.Rotational.Examples.Backlash` | 終了時刻まで | - | auto | 6 | 1165 | 0.06 | 0.06 | 管 OK 4/4 | 0.23 |
| 311 | `Mechanics.Rotational.Examples.CompareBrakingTorque` | 終了時刻まで | - | auto | 8 | 50004 | 0.02 | 0.05 | 管 OK 4/4 | 0.21 |
| 312 | `Mechanics.Rotational.Examples.EddyCurrentBrake` | 終了時刻まで | - | auto | 3 | 1003 | 0.02 | 0.00 | 管 OK 3/3 | 0.11 |
| 313 | `Mechanics.Rotational.Examples.ElasticBearing` | 終了時刻まで | - | auto | 6 | 1002 | 0.03 | 0.02 | 管 OK 6/6 | 0.15 |
| 314 | `Mechanics.Rotational.Examples.First` | 終了時刻まで | - | auto | 4 | 1001 | 0.04 | 0.01 | 管 OK 4/4 | 0.15 |
| 315 | `Mechanics.Rotational.Examples.FirstGrounded` | 終了時刻まで | - | auto | 4 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.14 |
| 316 | `Mechanics.Rotational.Examples.RollingWheel` | 終了時刻まで | - | auto | 2 | 5002 | 0.03 | 0.02 | 管 OK 2/2 | 0.15 |
| 317 | `Mechanics.Translational.Examples.Accelerate` | 終了時刻まで | - | auto | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.08 |
| 318 | `Mechanics.Translational.Examples.CompareBrakingForce` | 終了時刻まで | - | auto | 8 | 50004 | 0.03 | 0.05 | 管 OK 4/4 | 0.21 |
| 319 | `Mechanics.Translational.Examples.Damper` | 終了時刻まで | - | auto | 6 | 1001 | 0.04 | 0.00 | 管 OK 6/6 | 0.12 |
| 320 | `Mechanics.Translational.Examples.EddyCurrentBrake` | 終了時刻まで | - | auto | 3 | 1003 | 0.02 | 0.00 | 管 OK 3/3 | 0.11 |
| 321 | `Mechanics.Translational.Examples.ElastoGap` | 終了時刻まで | - | auto | 4 | 513 | 0.06 | 0.01 | 管 OK 4/4 | 0.16 |
| 322 | `Mechanics.Translational.Examples.GenerationOfFMUs` | 終了時刻まで | - | auto | 10 | 1001 | 0.15 | 0.01 | 管 OK 10/10 | 0.33 |
| 323 | `Mechanics.Translational.Examples.InitialConditions` | 終了時刻まで | - | auto | 8 | 5001 | 0.04 | 0.01 | 管 OK 8/8 | 0.15 |
| 324 | `Mechanics.Translational.Examples.Oscillator` | 終了時刻まで | - | auto | 4 | 1001 | 0.03 | 0.00 | 管 OK 4/4 | 0.13 |
| 325 | `Mechanics.Translational.Examples.PreLoad` | 終了時刻まで | - | auto | 6 | 1022 | 0.11 | 0.02 | 管 OK 6/6 | 0.26 |
| 326 | `Mechanics.Translational.Examples.Sensors` | 終了時刻まで | - | auto | 2 | 1001 | 0.02 | 0.00 | 管 OK 2/2 | 0.12 |
| 327 | `Mechanics.Translational.Examples.SignConvention` | 終了時刻まで | - | auto | 6 | 1001 | 0.01 | 0.00 | 管 OK 6/6 | 0.09 |
| 328 | `Mechanics.Translational.Examples.Vehicle` | 終了時刻まで | - | auto | 10 | 6012 | 0.22 | 0.06 | 管 OK 6/6 | 0.57 |
| 329 | `Mechanics.Translational.Examples.WhyArrows` | 終了時刻まで | - | auto | 4 | 1001 | 0.02 | 0.00 | 管 OK 4/4 | 0.10 |
| 330 | `Media.Examples.SimpleLiquidWater` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.31 |
| 331 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 23.62 |
| 332 | `Media.Examples.WaterIF97` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.95 |
| 333 | `StateGraph.Examples.ControlledTanks` | 終了時刻まで | - | auto | 2 | 533 | 0.10 | 0.07 | 管 OK 21/21 | 0.60 |
| 334 | `StateGraph.Examples.ExecutionPaths` | 終了時刻まで | - | auto | 0 | 527 | 0.02 | 0.03 | 管 OK 9/9 | 0.29 |
| 335 | `StateGraph.Examples.FirstExample` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.18 |
| 336 | `StateGraph.Examples.FirstExample_Variant2` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.18 |
| 337 | `StateGraph.Examples.FirstExample_Variant3` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.19 |
| 338 | `StateGraph.Examples.ShowCompositeStep` | 終了時刻まで | - | auto | 0 | 522 | 0.02 | 0.03 | 管 OK 9/9 | 0.28 |
| 339 | `StateGraph.Examples.ShowExceptions` | 終了時刻まで | - | auto | 0 | 525 | 0.02 | 0.05 | 管 OK 10/10 | 0.32 |
| 340 | `Thermal.FluidHeatFlow.Examples.IndirectCooling` | 終了時刻まで | - | auto | 4 | 1501 | 0.08 | 0.01 | 管 OK 4/4 | 0.27 |
| 341 | `Thermal.FluidHeatFlow.Examples.OneMass` | 終了時刻まで | - | auto | 2 | 1008 | 0.05 | 0.00 | 管 OK 2/2 | 0.18 |
| 342 | `Thermal.FluidHeatFlow.Examples.ParallelCooling` | 終了時刻まで | - | auto | 5 | 1001 | 0.08 | 0.01 | 管 OK 5/5 | 0.26 |
| 343 | `Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut` | 終了時刻まで | - | auto | 5 | 2009 | 0.09 | 0.04 | 管 OK 5/5 | 0.31 |
| 344 | `Thermal.FluidHeatFlow.Examples.PumpAndValve` | 終了時刻まで | - | auto | 3 | 2005 | 0.07 | 0.03 | 管 OK 3/3 | 0.27 |
| 345 | `Thermal.FluidHeatFlow.Examples.PumpDropOut` | 終了時刻まで | - | auto | 2 | 2007 | 0.05 | 0.01 | 管 OK 2/2 | 0.19 |
| 346 | `Thermal.FluidHeatFlow.Examples.SimpleCooling` | 終了時刻まで | - | auto | 2 | 1001 | 0.04 | 0.00 | 管 OK 2/2 | 0.17 |
| 347 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | GM-W10002・GM-E11001 | auto | - | - | - | - | - | 0.19 |
| 348 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | GM-W10002・GM-E11001 | auto | - | - | - | - | - | 1.87 |
| 349 | `Thermal.FluidHeatFlow.Examples.TwoMass` | 終了時刻まで | GM-I11002 | auto → dopri45 | 5 | 1011 | 0.08 | 0.10 | 管 OK 5/5 | 0.36 |
| 350 | `Thermal.FluidHeatFlow.Examples.TwoTanks` | 終了時刻まで | GM-W10002 | auto | 4 | 501 | 0.04 | 0.01 | 管 OK 4/4 | 0.15 |
| 351 | `Thermal.FluidHeatFlow.Examples.WaterPump` | 終了時刻まで | - | auto | 1 | 2015 | 0.07 | 0.04 | 管 OK 1/1 | 0.28 |
| 352 | `Thermal.HeatTransfer.Examples.ControlledTemperature` | 終了時刻まで | - | auto | 1 | 10062 | 0.03 | 0.06 | 管 OK 2/2 | 0.20 |
| 353 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.19 |
| 354 | `Thermal.HeatTransfer.Examples.Motor` | 終了時刻まで | - | auto | 2 | 720025 | 0.02 | 0.74 | 管 OK 2/2 | 1.71 |
| 355 | `Thermal.HeatTransfer.Examples.TwoMasses` | 終了時刻まで | - | auto | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.08 |
| 356 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.05 |
