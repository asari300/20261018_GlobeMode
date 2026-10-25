# MSL 4.1.0 の例題の実行と参照結果との比較（globe-mode-exec）

`cargo xtask msl-sim` が生成した。手で編集しない。`msl-funnel --json` で OK になった例題を、実行系 `globe-mode-exec`（Phase A は Rumoca の実行系）で別プロセスのワーカーから実行し、MAP-LIB の参照結果（MSL 4.1.0）と矩形の管（csv-compare の既定のアルゴリズムを書き直した `globe_mode_sim::compare`）で比べる。

| 項目 | 値 |
| --- | --- |
| 実行した日時 | 2026-10-24 23:01 UTC |
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
| ワーカー | 起動 15 回（外から停止 14 回・異常終了 0 回）、50 本ごとに `Checker::reset` |
| 結果 | **356 本: 実行できた 309（終了時刻まで 309・terminate 0）・コンパイル NG 0・S10 NG 0・実行 NG 41（うち時間切れ 4）・外から停止 6・ワーカー異常終了 0・panic 0** |
| 参照結果との比較 | 参照結果と比べた 308 本: 管に収まった 301（97.7%）・外れ 7・信号が足りない 0・参照結果なし 1・未取得 0・読めない 0 |
| 回し直し | 時間切れ（GM-E11005）と外から停止の回し直し（予算 180 s）: 13 本のうち実行できた 2・管に収まった 1 |
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
| 時間切れか外から止めたので回し直した（予算 180 s） | 13 |
| うち回し直して実行できた | 2 |
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
| 実行系: lowering | 6 |

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

## 時間とメモリ

| 項目 | 値 |
| --- | --- |
| 時間 | 全体 4132.2 s・ワーカーの中 2821.3 s（コンパイル 247.9 s・実行系の準備 279.81 s・初期化と積分 360.82 s・参照結果の読み込みと比較 19.60 s）・MSL の読み込み 15 回 64.3 s・1 本あたり（親から見た）平均 11.39 s・最大 196.02 s |
| ワーカーの最大の常駐メモリ（VmHWM。用意のとき / 全体） | 1.2 GiB / 2.1 GiB |

### lowering の段の長いもの

親が段の知らせの間隔で測った秒（回し直したものは回し直しの回。`--lowering-kill-seconds` を決める材料。S015）。

| # | モデル | lowering（秒） | 結果 |
| ---: | --- | ---: | --- |
| 1 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 150.0 | 外から停止 |
| 2 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 150.0 | 外から停止 |
| 3 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 150.0 | 外から停止 |
| 4 | `Fluid.Examples.NonCircularPipes` | 150.0 | 外から停止 |
| 5 | `Fluid.Examples.InverseParameterization` | 150.0 | 外から停止 |
| 6 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 150.0 | 外から停止 |
| 7 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 73.1 | 終了時刻まで |
| 8 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 46.0 | 実行 NG GM-E11001 |
| 9 | `Media.Examples.TwoPhaseWater.BaseProperties` | 20.8 | 実行 NG GM-U0004 |
| 10 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | 15.3 | 終了時刻まで |
| 11 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 10.5 | 実行 NG GM-U0004 |
| 12 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 9.0 | 終了時刻まで |
| 13 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 7.4 | 実行 NG GM-U0004 |
| 14 | `Fluid.Examples.HeatingSystem` | 3.3 | 実行 NG GM-U0004 |
| 15 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | 2.9 | 終了時刻まで |

## 実行できなかったモデル

「段階」は最初の Error の段階。外から止めたものと落ちたものは、ワーカーが最後に報告した段。「補足」は最初の Error の補足（Rumoca の文言など。全文は JSON の `notes`）。

| # | モデル | 結果 | 段階 | コード | 文言（先頭） | 補足（先頭） | 秒 |
| ---: | --- | --- | --- | --- | --- | --- | ---: |
| 1 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=8 original_row=8 owner=none(row reads a coordinate outside the planned init… | 0.26 |
| 2 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 14 matched out of 15 equations and 15 unknowns | 0.35 |
| 3 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（thd1.y の値が NaN になった） | rumoca: [Initialization] initial homotopy continuation failed at lambda=0.000000000 step=1.250e-1: non-finite (NaN) value computed for 'thd1.y' @ Span { source: SourceId(9025898607377294338), start: B… | 0.31 |
| 4 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid | 0.08 |
| 5 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=122 target=T2.C.i value=-6.000000e-5 ratio=6.000000e5 norm=6.000000e-5 row_scale=1.000000… | 0.49 |
| 6 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=120 target=T2.C.i value=6.000000e-5 ratio=6.000000e5 norm=6.000000e-5 row_scale=1.000000e… | 0.51 |
| 7 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | Run | GM-E11002 | t = 0.027 で積分に失敗した | rumoca: [ES016] the algebraic loop over 'opAmp.v_out', 'r2.v', 'r2.i', 'r1.i', 'r1.v', 'opAmp.in_p.v', and 1 more reached a fold of 'V0\*v_in&lt;vns': the relation switches its own loop under 'noEvent', s… | 0.14 |
| 8 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 積分の途中の方向微分（ヤコビアン）の計算で止まった | rumoca: [Integration] directional derivative is unavailable: algebraic projection produced a non-finite sensitivity for y[432] | 6.60 |
| 9 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 377 matched out of 392 equations and 392 unknowns | 1.05 |
| 10 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | Run | GM-E11002 | 積分に失敗した | rumoca: [Integration] algebraic projection did not establish coordinate convergence: worst scaled residual row=418 target=rectifier.diode_p.plug_p.pin[3].i value=-7.645419e0 ratio=7.645419e10 norm=7.6… | 3.54 |
| 11 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 899 matched out of 913 equations and 913 unknowns | 52.17 |
| 12 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 187.68 |
| 13 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 644 matched out of 659 equations and 659 unknowns | 2.35 |
| 14 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 469 matched out of 474 equations and 474 unknowns | 0.50 |
| 15 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 899 matched out of 913 equations and 913 unknowns | 2.64 |
| 16 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 1027 matched out of 1034 equations and 1034 unknowns | 1.07 |
| 17 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 569 matched out of 573 equations and 573 unknowns | 0.58 |
| 18 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 768 matched out of 772 equations and 772 unknowns | 0.76 |
| 19 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] DAE system is not computable: affine derivative coefficient is not compile-time numeric | 0.28 |
| 20 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | Run | GM-U0005 | 実行系（Rumoca）の内部エラー（実行時の契約違反） | rumoca: accepted step from t=5.0416408911668364 to t=5.041640891166937 made no progress beyond roundoff | 86.30 |
| 21 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 196.02 |
| 22 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 187.20 |
| 23 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | Run | GM-E11005 | 時間切れ（壁時計の予算 180 秒を超えた） | rumoca: timeout after 180.000s | 188.42 |
| 24 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 1760 matched out of 1767 equations and 1767 unknowns | 42.87 |
| 25 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 153.34 |
| 26 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 156.92 |
| 27 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 154.69 |
| 28 | `Fluid.Examples.HeatingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: invalid structural IR contract without source span: independent state selection: linear solve requires the… | 10.71 |
| 29 | `Fluid.Examples.InverseParameterization` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.2 s） | - | 156.15 |
| 30 | `Fluid.Examples.NonCircularPipes` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.2 s） | - | 155.51 |
| 31 | `Fluid.Examples.PumpingSystem` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 277 matched out of 278 equations and 278 unknowns | 5.79 |
| 32 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | Run | GM-E11003 | t = 1468.683452157 でイベントの反復が収束しない | rumoca: [EventIteration] event condition equations did not converge with fixed pre at t=1468.6834521574738 | 21.74 |
| 33 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 126 matched out of 128 equations and 128 unknowns | 0.18 |
| 34 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 233 matched out of 242 equations and 242 unknowns | 0.55 |
| 35 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 147 matched out of 151 equations and 151 unknowns | 0.31 |
| 36 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 287 matched out of 290 equations and 290 unknowns | 0.48 |
| 37 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | 実行系: lowering（150 s） | - | lowering が 150 s で終わらないので止めた（signal: 9 (SIGKILL)）。最後の段: 実行系: lowering（入ってから 150.3 s） | - | 161.92 |
| 38 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 3771 matched out of 3844 equations and 3844 unknowns | 8.85 |
| 39 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=9 original_row=9 owner=none(row reads a coordinate outside the planned init… | 52.58 |
| 40 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（初期化系が解けない） | rumoca: [Initialization] initial variable projection did not satisfy the complete residual system: max selected residual row=2 original_row=2 owner=none(row reads a coordinate outside the planned init… | 0.93 |
| 41 | `Media.Examples.SimpleLiquidWater` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 31 matched out of 34 equations and 34 unknowns | 0.35 |
| 42 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 実行系の組み立てで止まった | rumoca: [BackendBuild] solve-IR evaluation failed: [EX002] input 'p' has neither a checked default nor a runtime value | 21.64 |
| 43 | `Media.Examples.WaterIF97` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 構造解析（インデックスの低減・マッチング・tearing）で止まった | rumoca: [StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 28 matched out of 30 equations and 30 unknowns | 0.88 |
| 44 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（cylinder1.h の値が NaN になった） | rumoca: [Initialization] non-finite (NaN) value computed for 'cylinder1.h' / プリフライトの警告: start 値で評価すると t = 0 で式の値が有限にならない: der($state_coordinates[1]) = NaN, der($state_coordinates[2]) = NaN, der($state… | 0.19 |
| 45 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | Run | GM-E11001 | 初期化に失敗した（openTank.h の値が NaN になった） | rumoca: [Initialization] non-finite (NaN) value computed for 'openTank.h' / プリフライトの警告: start 値で評価すると t = 0 で式の値が有限にならない: der(openTank.m) = NaN, der(openTank.H) = NaN, openTank.h = NaN（GM-W10002） | 30.29 |
| 46 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: 積分の途中の方向微分（ヤコビアン）の計算で止まった | rumoca: [Integration] directional derivative is unavailable: algebraic projection sensitivity matrix is singular | 0.20 |
| 47 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | Run | GM-U0004 | 実行系（Rumoca）がこのモデルに未対応: Solve IR への変換で止まった | rumoca: [SolveLowering] solve-IR evaluation failed: [EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid | 0.06 |

## 回し直したモデル

| # | モデル | 1 回目（予算・秒） | 回し直し | 参照結果 | 秒 |
| ---: | --- | --- | --- | --- | ---: |
| 1 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 外から停止 （30 s・120.7 s） | 終了時刻まで | 管 OK 3/3 | 105.01 |
| 2 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG GM-E11005（30 s・32.9 s） | 実行 NG GM-E11005 | - | 187.68 |
| 3 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG GM-E11005（30 s・30.6 s） | 実行 NG GM-U0005 | - | 86.30 |
| 4 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG GM-E11005（30 s・45.1 s） | 実行 NG GM-E11005 | - | 196.02 |
| 5 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG GM-E11005（30 s・37.1 s） | 実行 NG GM-E11005 | - | 187.20 |
| 6 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG GM-E11005（30 s・31.0 s） | 実行 NG GM-E11005 | - | 188.42 |
| 7 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 153.34 |
| 8 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 156.92 |
| 9 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 154.69 |
| 10 | `Fluid.Examples.InverseParameterization` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 156.15 |
| 11 | `Fluid.Examples.NonCircularPipes` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 155.51 |
| 12 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 外から停止 （30 s・120.0 s） | 終了時刻まで | **管 NG** 2/4（外れ 2） | 190.66 |
| 13 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 （30 s・120.0 s） | 外から停止 | - | 161.92 |

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
| 1 | `Blocks.Examples.BusUsage` | 終了時刻まで | - | auto | 0 | 502 | 0.01 | 0.00 | 管 OK 5/5 | 2.03 |
| 2 | `Blocks.Examples.BusUsage_Utilities.Part` | 終了時刻まで | - | auto | 0 | 502 | 0.00 | 0.00 | 参照結果なし | 0.22 |
| 3 | `Blocks.Examples.CompareSincExpSine` | 終了時刻まで | - | auto | 0 | 10001 | 0.00 | 0.03 | 管 OK 3/3 | 0.26 |
| 4 | `Blocks.Examples.DemoSignalCharacteristic` | 終了時刻まで | - | auto | 5 | 1051 | 0.02 | 0.02 | **管 NG** 4/5（外れ 1） | 0.27 |
| 5 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 0.26 |
| 6 | `Blocks.Examples.DemonstrateSignalExtrema` | 終了時刻まで | - | auto | 1 | 165001 | 0.01 | 1.24 | 管 OK 8/8 | 2.01 |
| 7 | `Blocks.Examples.FilterWithDifferentiation` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.35 |
| 8 | `Blocks.Examples.FilterWithRiseTime` | 終了時刻まで | - | auto | 6 | 502 | 0.06 | 0.01 | 管 OK 6/6 | 0.39 |
| 9 | `Blocks.Examples.Interaction1` | 終了時刻まで | - | auto | 0 | 510 | 0.02 | 0.01 | 管 OK 6/6 | 0.50 |
| 10 | `Blocks.Examples.InverseModel` | 終了時刻まで | - | auto | 2 | 502 | 0.01 | 0.00 | 管 OK 2/2 | 0.31 |
| 11 | `Blocks.Examples.LogicalNetwork1` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 3/3 | 0.28 |
| 12 | `Blocks.Examples.Modulation` | 終了時刻まで | - | auto | 5 | 10001 | 0.02 | 0.01 | 管 OK 5/5 | 0.27 |
| 13 | `Blocks.Examples.Noise.Densities` | 終了時刻まで | - | auto | 0 | 1001 | 0.01 | 0.01 | 管 OK 3/3 | 0.26 |
| 14 | `Blocks.Examples.PID_Controller` | 終了時刻まで | - | auto | 6 | 511 | 0.06 | 0.01 | 管 OK 6/6 | 0.37 |
| 15 | `Blocks.Examples.ShowLogicalSources` | 終了時刻まで | - | auto | 0 | 536 | 0.00 | 0.00 | 管 OK 1/1 | 0.32 |
| 16 | `Blocks.Examples.SinCosEncoder` | 終了時刻まで | - | auto | 5 | 20002 | 0.03 | 0.07 | 管 OK 6/6 | 0.38 |
| 17 | `Blocks.Examples.SlewRateLimiter` | 終了時刻まで | - | auto | 3 | 1010 | 0.01 | 0.00 | 管 OK 3/3 | 0.22 |
| 18 | `Blocks.Examples.TotalHarmonicDistortion` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 0.31 |
| 19 | `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | 終了時刻まで | - | auto | 2 | 4009 | 0.08 | 0.26 | 管 OK 4/4 | 0.48 |
| 20 | `Clocked.Examples.CascadeControlledDrive.Continuous` | 終了時刻まで | - | auto | 3 | 509 | 0.08 | 0.00 | 管 OK 4/4 | 0.27 |
| 21 | `Clocked.Examples.CascadeControlledDrive.SubClocked` | 終了時刻まで | - | auto | 2 | 4009 | 0.08 | 0.28 | 管 OK 4/4 | 0.48 |
| 22 | `Clocked.Examples.CascadeControlledDrive.SuperSampled` | 終了時刻まで | - | auto | 2 | 4009 | 0.08 | 0.41 | 管 OK 4/4 | 0.64 |
| 23 | `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.12 |
| 24 | `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.10 |
| 25 | `Clocked.Examples.Elementary.BooleanSignals.BackSample` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 3/3 | 0.14 |
| 26 | `Clocked.Examples.Elementary.BooleanSignals.Hold` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 27 | `Clocked.Examples.Elementary.BooleanSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 28 | `Clocked.Examples.Elementary.BooleanSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.07 |
| 29 | `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 30 | `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.06 |
| 31 | `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.11 |
| 32 | `Clocked.Examples.Elementary.BooleanSignals.SubSample` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 2/2 | 0.15 |
| 33 | `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.13 |
| 34 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 35 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 36 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 37 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 38 | `Clocked.Examples.Elementary.BooleanSignals.UpSample` | 終了時刻まで | - | auto | 0 | 529 | 0.00 | 0.00 | 管 OK 3/3 | 0.12 |
| 39 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | 終了時刻まで | - | auto | 0 | 694 | 0.03 | 0.20 | **管 NG** 4/8（外れ 4） | 0.41 |
| 40 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | 終了時刻まで | - | auto | 0 | 594 | 0.01 | 0.01 | **管 NG** 1/4（外れ 3） | 0.11 |
| 41 | `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 1/1 | 0.09 |
| 42 | `Clocked.Examples.Elementary.ClockSignals.SubSample` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.09 |
| 43 | `Clocked.Examples.Elementary.ClockSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 521 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 44 | `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 45 | `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 46 | `Clocked.Examples.Elementary.IntegerSignals.BackSample` | 終了時刻まで | - | auto | 0 | 523 | 0.00 | 0.00 | 管 OK 3/3 | 0.12 |
| 47 | `Clocked.Examples.Elementary.IntegerSignals.Hold` | 終了時刻まで | - | auto | 0 | 513 | 0.00 | 0.00 | 管 OK 3/3 | 0.13 |
| 48 | `Clocked.Examples.Elementary.IntegerSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 49 | `Clocked.Examples.Elementary.IntegerSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.07 |
| 50 | `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 51 | `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 4.37 |
| 52 | `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 523 | 0.00 | 0.00 | 管 OK 2/2 | 0.14 |
| 53 | `Clocked.Examples.Elementary.IntegerSignals.SubSample` | 終了時刻まで | - | auto | 0 | 513 | 0.01 | 0.00 | 管 OK 2/2 | 0.15 |
| 54 | `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 553 | 0.00 | 0.00 | 管 OK 2/2 | 0.14 |
| 55 | `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 56 | `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 57 | `Clocked.Examples.Elementary.IntegerSignals.UpSample` | 終了時刻まで | - | auto | 0 | 533 | 0.00 | 0.00 | 管 OK 3/3 | 0.15 |
| 58 | `Clocked.Examples.Elementary.RealSignals.AssignClock` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 1/1 | 0.09 |
| 59 | `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 60 | `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 61 | `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | 終了時刻まで | - | auto | 0 | 509 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 62 | `Clocked.Examples.Elementary.RealSignals.BackSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 3/3 | 0.09 |
| 63 | `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | 終了時刻まで | - | auto | 0 | 516 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 64 | `Clocked.Examples.Elementary.RealSignals.Hold` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.09 |
| 65 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | 終了時刻まで | - | auto | 0 | 519 | 0.00 | 0.00 | 管 OK 2/2 | 0.10 |
| 66 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.12 |
| 67 | `Clocked.Examples.Elementary.RealSignals.Sample1` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 68 | `Clocked.Examples.Elementary.RealSignals.Sample2` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 69 | `Clocked.Examples.Elementary.RealSignals.Sample3` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 3/3 | 0.07 |
| 70 | `Clocked.Examples.Elementary.RealSignals.SampleClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.08 |
| 71 | `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.08 |
| 72 | `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.11 |
| 73 | `Clocked.Examples.Elementary.RealSignals.ShiftSample` | 終了時刻まで | - | auto | 0 | 517 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 74 | `Clocked.Examples.Elementary.RealSignals.SubSample` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 75 | `Clocked.Examples.Elementary.RealSignals.SuperSample` | 終了時刻まで | - | auto | 0 | 521 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 76 | `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | 終了時刻まで | - | auto | 0 | 518 | 0.00 | 0.00 | 管 OK 2/2 | 0.09 |
| 77 | `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 78 | `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.07 |
| 79 | `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 80 | `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 81 | `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 82 | `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | 終了時刻まで | - | auto | 0 | 511 | 0.00 | 0.00 | 管 OK 1/1 | 0.06 |
| 83 | `Clocked.Examples.Elementary.RealSignals.UniformNoise` | 終了時刻まで | - | auto | 0 | 606 | 0.00 | 0.00 | 管 OK 2/2 | 0.07 |
| 84 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.08 |
| 85 | `Clocked.Examples.Elementary.RealSignals.UpSample1` | 終了時刻まで | - | auto | 0 | 527 | 0.00 | 0.00 | 管 OK 3/3 | 0.10 |
| 86 | `Clocked.Examples.Elementary.RealSignals.UpSample2` | 終了時刻まで | - | auto | 0 | 518 | 0.00 | 0.00 | 管 OK 5/5 | 0.10 |
| 87 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.12 |
| 88 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 89 | `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | 終了時刻まで | - | auto | 2 | 601 | 0.03 | 0.01 | 管 OK 4/4 | 0.15 |
| 90 | `Clocked.Examples.SimpleControlledDrive.Continuous` | 終了時刻まで | - | auto | 3 | 502 | 0.01 | 0.00 | 管 OK 4/4 | 0.16 |
| 91 | `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | 終了時刻まで | - | auto | 2 | 551 | 0.01 | 0.00 | 管 OK 4/4 | 0.11 |
| 92 | `Clocked.Examples.Systems.EngineThrottleControl` | 終了時刻まで | - | auto | 5 | 645 | 0.05 | 0.02 | 管 OK 8/8 | 0.22 |
| 93 | `ComplexBlocks.Examples.TestConversionBlock` | 終了時刻まで | - | auto | 0 | 1012 | 0.01 | 0.00 | 管 OK 2/2 | 0.08 |
| 94 | `Electrical.Analog.Examples.AmplifierWithOpAmpDetailed` | 終了時刻まで | - | auto | 7 | 2501 | 0.05 | 0.07 | 管 OK 7/7 | 0.24 |
| 95 | `Electrical.Analog.Examples.CauerLowPassAnalog` | 終了時刻まで | - | auto | 5 | 503 | 0.03 | 0.01 | 管 OK 5/5 | 0.13 |
| 96 | `Electrical.Analog.Examples.CauerLowPassOPV` | 終了時刻まで | - | auto | 5 | 503 | 0.09 | 0.01 | 管 OK 5/5 | 0.24 |
| 97 | `Electrical.Analog.Examples.CauerLowPassSC` | 終了時刻まで | - | auto | 16 | 3601 | 0.24 | 18.29 | 管 OK 5/5 | 18.91 |
| 98 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | 終了時刻まで | - | auto | 0 | 510 | 0.03 | 0.02 | 管 OK 3/3 | 1.26 |
| 99 | `Electrical.Analog.Examples.CharacteristicThyristors` | 終了時刻まで | - | auto | 0 | 559 | 0.03 | 0.06 | 管 OK 3/3 | 0.30 |
| 100 | `Electrical.Analog.Examples.ChuaCircuit` | 終了時刻まで | - | auto | 3 | 50219 | 0.02 | 0.11 | 管 OK 3/3 | 0.23 |
| 101 | `Electrical.Analog.Examples.CompareTransformers` | 終了時刻まで | - | auto | 4 | 50001 | 0.08 | 0.66 | 管 OK 3/3 | 12.36 |
| 102 | `Electrical.Analog.Examples.ControlledSwitchWithArc` | 終了時刻まで | - | auto | 2 | 24061 | 0.03 | 0.24 | 管 OK 2/2 | 0.40 |
| 103 | `Electrical.Analog.Examples.DemoPowerSupply` | 終了時刻まで | - | auto | 0 | 10006 | 0.01 | 0.14 | 管 OK 3/3 | 0.24 |
| 104 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | 終了時刻まで | - | auto | 1 | 107683 | 0.03 | 2.25 | 管 OK 4/4 | 2.49 |
| 105 | `Electrical.Analog.Examples.DemonstrateLightning` | 終了時刻まで | - | auto | 2 | 10002 | 0.07 | 0.03 | 管 OK 6/6 | 0.26 |
| 106 | `Electrical.Analog.Examples.DifferenceAmplifier` | 終了時刻まで | - | auto | 9 | 502 | 0.16 | 0.02 | 管 OK 9/9 | 0.37 |
| 107 | `Electrical.Analog.Examples.GenerationOfFMUs` | 終了時刻まで | - | auto | 6 | 1001 | 0.18 | 0.02 | 管 OK 6/6 | 0.39 |
| 108 | `Electrical.Analog.Examples.HeatingMOSInverter` | 終了時刻まで | - | auto | 2 | 584 | 0.07 | 0.09 | 管 OK 2/2 | 0.30 |
| 109 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.49 |
| 110 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.51 |
| 111 | `Electrical.Analog.Examples.HeatingRectifier` | 終了時刻まで | - | auto | 2 | 501 | 0.03 | 0.01 | 管 OK 2/2 | 0.15 |
| 112 | `Electrical.Analog.Examples.IdealTriacCircuit` | 終了時刻まで | - | auto | 1 | 561 | 0.03 | 0.03 | 管 OK 3/3 | 0.17 |
| 113 | `Electrical.Analog.Examples.InvertingAmp` | 終了時刻まで | - | auto | 0 | 1046 | 0.03 | 0.02 | 管 OK 1/1 | 0.17 |
| 114 | `Electrical.Analog.Examples.Lines.CompareLineTrunks` | 終了時刻まで | - | auto | 57 | 1004 | 0.74 | 0.41 | 管 OK 9/9 | 1.90 |
| 115 | `Electrical.Analog.Examples.Lines.CompareLosslessLines` | 終了時刻まで | - | auto | 0 | 209 | 0.03 | 0.01 | 管 OK 12/12 | 0.17 |
| 116 | `Electrical.Analog.Examples.Lines.LightningLosslessTransmissionLine` | 終了時刻まで | - | auto | 0 | 25002 | 0.02 | 1.12 | 管 OK 3/3 | 1.30 |
| 117 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | 終了時刻まで | GM-I11002 | auto → dopri45 | 402 | 25002 | 36.88 | 61.97 | 管 OK 3/3 | 105.01 |
| 118 | `Electrical.Analog.Examples.Lines.SmoothStep` | 終了時刻まで | - | auto | 116 | 3002 | 1.81 | 0.70 | 管 OK 4/4 | 5.50 |
| 119 | `Electrical.Analog.Examples.NandGate` | 終了時刻まで | - | auto | 2 | 526 | 0.11 | 0.02 | 管 OK 2/2 | 0.39 |
| 120 | `Electrical.Analog.Examples.OpAmps.Adder` | 終了時刻まで | - | auto | 0 | 1001 | 0.02 | 0.05 | 管 OK 1/1 | 0.22 |
| 121 | `Electrical.Analog.Examples.OpAmps.Comparator` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 122 | `Electrical.Analog.Examples.OpAmps.ControlCircuit` | 終了時刻まで | - | auto | 6 | 1003 | 0.15 | 0.05 | 管 OK 8/8 | 0.49 |
| 123 | `Electrical.Analog.Examples.OpAmps.DifferentialAmplifier` | 終了時刻まで | - | auto | 1 | 2011 | 0.07 | 0.21 | 管 OK 2/2 | 0.44 |
| 124 | `Electrical.Analog.Examples.OpAmps.Differentiator` | 終了時刻まで | - | auto | 1 | 1046 | 0.02 | 0.02 | 管 OK 3/3 | 0.15 |
| 125 | `Electrical.Analog.Examples.OpAmps.HighPass` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.05 | 管 OK 3/3 | 0.20 |
| 126 | `Electrical.Analog.Examples.OpAmps.Integrator` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.02 | 管 OK 3/3 | 0.17 |
| 127 | `Electrical.Analog.Examples.OpAmps.InvertingAmplifier` | 終了時刻まで | - | auto | 0 | 1046 | 0.02 | 0.02 | 管 OK 2/2 | 0.15 |
| 128 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 0.14 |
| 129 | `Electrical.Analog.Examples.OpAmps.LCOscillator` | 終了時刻まで | - | auto | 2 | 1001 | 0.02 | 0.01 | 管 OK 4/4 | 0.13 |
| 130 | `Electrical.Analog.Examples.OpAmps.LowPass` | 終了時刻まで | - | auto | 1 | 1046 | 0.03 | 0.02 | 管 OK 3/3 | 0.16 |
| 131 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | 終了時刻まで | - | auto | 1 | 1001 | 0.02 | 0.02 | **管 NG** 0/3（外れ 3） | 0.13 |
| 132 | `Electrical.Analog.Examples.OpAmps.NonInvertingAmplifier` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.02 | 管 OK 2/2 | 0.14 |
| 133 | `Electrical.Analog.Examples.OpAmps.SchmittTrigger` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.01 | 管 OK 2/2 | 0.13 |
| 134 | `Electrical.Analog.Examples.OpAmps.SignalGenerator` | 終了時刻まで | - | auto | 1 | 1041 | 0.03 | 0.03 | 管 OK 5/5 | 0.17 |
| 135 | `Electrical.Analog.Examples.OpAmps.Subtracter` | 終了時刻まで | - | auto | 0 | 1001 | 0.02 | 0.01 | 管 OK 1/1 | 0.15 |
| 136 | `Electrical.Analog.Examples.OpAmps.VoltageFollower` | 終了時刻まで | - | auto | 0 | 1046 | 0.01 | 0.02 | 管 OK 2/2 | 0.16 |
| 137 | `Electrical.Analog.Examples.OvervoltageProtection` | 終了時刻まで | - | auto | 1 | 501 | 0.03 | 0.02 | 管 OK 1/1 | 0.16 |
| 138 | `Electrical.Analog.Examples.ParallelResonance` | 終了時刻まで | - | auto | 6 | 10002 | 0.03 | 0.03 | 管 OK 6/6 | 0.22 |
| 139 | `Electrical.Analog.Examples.Rectifier` | 終了時刻まで | - | auto | 5 | 14522 | 0.12 | 3.73 | 管 OK 4/4 | 4.01 |
| 140 | `Electrical.Analog.Examples.Resistor` | 終了時刻まで | - | auto | 0 | 501 | 0.01 | 0.01 | 管 OK 2/2 | 0.11 |
| 141 | `Electrical.Analog.Examples.ResonanceCircuits` | 終了時刻まで | - | auto | 4 | 10001 | 0.16 | 0.03 | 管 OK 3/3 | 0.38 |
| 142 | `Electrical.Analog.Examples.SeriesResonance` | 終了時刻まで | - | auto | 6 | 10002 | 0.02 | 0.03 | 管 OK 6/6 | 0.19 |
| 143 | `Electrical.Analog.Examples.ShowSaturatingInductor` | 終了時刻まで | - | auto | 2 | 630 | 0.01 | 0.01 | 管 OK 2/2 | 0.10 |
| 144 | `Electrical.Analog.Examples.ShowVariableResistor` | 終了時刻まで | - | auto | 0 | 501 | 0.02 | 0.01 | 管 OK 2/2 | 0.12 |
| 145 | `Electrical.Analog.Examples.SimpleTriacCircuit` | 終了時刻まで | GM-W10003 | auto | 3 | 2485 | 0.08 | 1.26 | 管 OK 3/3 | 1.47 |
| 146 | `Electrical.Analog.Examples.SwitchWithArc` | 終了時刻まで | - | auto | 2 | 8010 | 0.02 | 0.06 | 管 OK 2/2 | 0.19 |
| 147 | `Electrical.Analog.Examples.ThyristorBehaviourTest` | 終了時刻まで | - | auto | 2 | 517 | 0.03 | 0.03 | 管 OK 2/2 | 0.16 |
| 148 | `Electrical.Batteries.Examples.CCCV_Cell` | 終了時刻まで | - | auto | 3 | 12008 | 0.05 | 0.10 | 管 OK 3/3 | 0.37 |
| 149 | `Electrical.Batteries.Examples.SuperCapDischargeCharge` | 終了時刻まで | - | auto | 2 | 72021 | 0.03 | 0.11 | 管 OK 2/2 | 0.41 |
| 150 | `Electrical.Machines.Examples.DCMachines.DCEE_Start` | 終了時刻まで | - | auto | 4 | 20004 | 0.23 | 0.25 | 管 OK 4/4 | 1.00 |
| 151 | `Electrical.Machines.Examples.DCMachines.DCPM_Cooling` | 終了時刻まで | - | auto | 5 | 250051 | 0.19 | 6.48 | 管 OK 5/5 | 15.78 |
| 152 | `Electrical.Machines.Examples.DCMachines.DCPM_CurrentControlled` | 終了時刻まで | - | auto | 5 | 20002 | 0.21 | 0.16 | 管 OK 5/5 | 3.70 |
| 153 | `Electrical.Machines.Examples.DCMachines.DCPM_Drive` | 終了時刻まで | - | auto | 16 | 20006 | 0.78 | 0.39 | 管 OK 4/4 | 1.94 |
| 154 | `Electrical.Machines.Examples.DCMachines.DCPM_QuasiStatic` | 終了時刻まで | - | auto | 5 | 20005 | 0.42 | 0.28 | 管 OK 8/8 | 1.26 |
| 155 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | 終了時刻まで | - | auto | 3 | 20004 | 0.15 | 0.13 | 管 OK 3/3 | 0.60 |
| 156 | `Electrical.Machines.Examples.DCMachines.DCPM_Temperature` | 終了時刻まで | - | auto | 3 | 30002 | 0.17 | 0.22 | 管 OK 3/3 | 0.77 |
| 157 | `Electrical.Machines.Examples.DCMachines.DCPM_withLosses` | 終了時刻まで | - | auto | 6 | 20005 | 0.46 | 0.40 | 管 OK 6/6 | 1.40 |
| 158 | `Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase` | 終了時刻まで | - | auto | 3 | 20003 | 0.36 | 0.52 | 管 OK 3/3 | 1.36 |
| 159 | `Electrical.Machines.Examples.DCMachines.DCSE_Start` | 終了時刻まで | - | auto | 3 | 20003 | 0.33 | 0.28 | 管 OK 3/3 | 1.19 |
| 160 | `Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics` | 終了時刻まで | - | auto | 6 | 6003 | 0.82 | 0.18 | 管 OK 12/12 | 2.28 |
| 161 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 6.60 |
| 162 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 1.05 |
| 163 | `Electrical.Machines.Examples.InductionMachines.IMC_DOL` | 終了時刻まで | - | auto | 7 | 15002 | 0.58 | 0.51 | 管 OK 8/8 | 1.79 |
| 164 | `Electrical.Machines.Examples.InductionMachines.IMC_Initialize` | 終了時刻まで | - | auto | 7 | 15002 | 0.48 | 0.45 | 管 OK 8/8 | 1.61 |
| 165 | `Electrical.Machines.Examples.InductionMachines.IMC_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 1.13 | 0.68 | 管 OK 9/9 | 2.44 |
| 166 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | 実行 NG | GM-E11002・GM-I11002 | auto → dopri45 | - | - | - | - | - | 3.54 |
| 167 | `Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz` | 終了時刻まで | - | auto | 9 | 10004 | 0.51 | 0.81 | 管 OK 10/10 | 1.93 |
| 168 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 52.17 |
| 169 | `Electrical.Machines.Examples.InductionMachines.IMC_YD` | 終了時刻まで | - | auto | 7 | 25003 | 0.77 | 2.15 | 管 OK 8/8 | 4.51 |
| 170 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | 実行 NG | GM-E11005 | auto | - | - | - | - | - | 187.68 |
| 171 | `Electrical.Machines.Examples.InductionMachines.IMC_withLosses` | 終了時刻まで | - | auto | 8 | 50002 | 0.64 | 2.39 | 管 OK 9/9 | 4.09 |
| 172 | `Electrical.Machines.Examples.InductionMachines.IMS_Start` | 終了時刻まで | - | auto | 8 | 15003 | 0.79 | 1.18 | 管 OK 10/10 | 2.96 |
| 173 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL` | 終了時刻まで | - | auto | 7 | 30004 | 1.31 | 1.49 | 管 OK 9/9 | 3.81 |
| 174 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator` | 終了時刻まで | - | auto | 5 | 30002 | 1.71 | 1.32 | 管 OK 7/7 | 3.88 |
| 175 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier` | 終了時刻まで | - | auto | 12 | 12450 | 1.46 | 26.14 | 管 OK 14/14 | 28.70 |
| 176 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking` | 終了時刻まで | - | auto | 5 | 8289 | 0.78 | 1.22 | 管 OK 6/6 | 2.73 |
| 177 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 2.35 |
| 178 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 1.38 | 0.62 | 管 OK 9/9 | 2.67 |
| 179 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad` | 終了時刻まで | - | auto | 1 | 3221 | 0.75 | 0.04 | 管 OK 3/3 | 1.30 |
| 180 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking` | 終了時刻まで | - | auto | 4 | 1003 | 1.33 | 0.12 | 管 OK 9/9 | 2.17 |
| 181 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource` | 終了時刻まで | - | auto | 7 | 20001 | 2.21 | 0.85 | 管 OK 8/8 | 3.94 |
| 182 | `Electrical.Machines.Examples.SynchronousMachines.SMR_DOL` | 終了時刻まで | - | auto | 6 | 2503 | 1.18 | 0.22 | 管 OK 8/8 | 2.05 |
| 183 | `Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter` | 終了時刻まで | - | auto | 7 | 15003 | 1.07 | 0.66 | 管 OK 9/9 | 2.39 |
| 184 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.50 |
| 185 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 2.64 |
| 186 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 1.07 |
| 187 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.58 |
| 188 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.76 |
| 189 | `Electrical.Polyphase.Examples.Rectifier` | 終了時刻まで | - | auto | 4 | 10121 | 0.25 | 0.55 | 管 OK 4/4 | 1.24 |
| 190 | `Electrical.Polyphase.Examples.TestSensors` | 終了時刻まで | - | auto | 2 | 1001 | 0.23 | 0.02 | 管 OK 2/2 | 0.62 |
| 191 | `Electrical.Polyphase.Examples.TransformerYD` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.28 |
| 192 | `Electrical.Polyphase.Examples.TransformerYY` | 終了時刻まで | - | auto | 2 | 1001 | 0.10 | 0.01 | 管 OK 2/2 | 0.39 |
| 193 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | 終了時刻まで | - | auto | 10 | 83783 | 0.16 | 4.86 | **管 NG** 9/11（外れ 2） | 5.80 |
| 194 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | 実行 NG | GM-U0005 | auto | - | - | - | - | - | 86.30 |
| 195 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R` | 終了時刻まで | - | auto | 3 | 536 | 0.04 | 0.03 | 管 OK 9/9 | 0.24 |
| 196 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R_Characteristic` | 終了時刻まで | - | auto | 3 | 53224 | 0.04 | 2.02 | 管 OK 9/9 | 2.40 |
| 197 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.DiodeBridge2Pulse` | 終了時刻まで | - | auto | 3 | 519 | 0.05 | 0.04 | 管 OK 6/6 | 0.24 |
| 198 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.HalfControlledBridge2Pulse` | 終了時刻まで | - | auto | 3 | 539 | 0.07 | 0.06 | 管 OK 10/10 | 0.33 |
| 199 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | 実行 NG | GM-E11005・GM-I11002 | auto → dopri45 | - | - | - | - | - | 196.02 |
| 200 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_R` | 終了時刻まで | - | auto | 3 | 539 | 0.07 | 0.06 | 管 OK 12/12 | 0.33 |
| 201 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RL` | 終了時刻まで | - | auto | 4 | 559 | 0.08 | 0.08 | 管 OK 13/13 | 0.36 |
| 202 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV` | 終了時刻まで | - | auto | 4 | 559 | 0.07 | 0.07 | 管 OK 13/13 | 0.38 |
| 203 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 55456 | 0.07 | 4.57 | 管 OK 13/13 | 5.11 |
| 204 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse` | 終了時刻まで | - | auto | 3 | 671 | 0.10 | 0.21 | 管 OK 6/6 | 0.60 |
| 205 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse` | 終了時刻まで | - | auto | 3 | 722 | 0.15 | 0.43 | 管 OK 18/18 | 0.98 |
| 206 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | 実行 NG | GM-E11005・GM-I11002 | auto → dopri45 | - | - | - | - | - | 187.20 |
| 207 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R` | 終了時刻まで | - | auto | 3 | 690 | 0.15 | 0.41 | 管 OK 24/24 | 0.99 |
| 208 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL` | 終了時刻まで | - | auto | 4 | 694 | 0.15 | 0.33 | 管 OK 25/25 | 0.91 |
| 209 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV` | 終了時刻まで | - | auto | 4 | 697 | 0.15 | 0.33 | 管 OK 25/25 | 0.91 |
| 210 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 73993 | 0.18 | 48.33 | 管 OK 25/25 | 49.99 |
| 211 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.DiodeCenterTap2Pulse` | 終了時刻まで | - | auto | 3 | 519 | 0.05 | 0.03 | 管 OK 6/6 | 0.33 |
| 212 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_R` | 終了時刻まで | - | auto | 3 | 539 | 0.06 | 0.05 | 管 OK 10/10 | 0.34 |
| 213 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RL` | 終了時刻まで | - | auto | 4 | 559 | 0.07 | 0.04 | 管 OK 11/11 | 0.41 |
| 214 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV` | 終了時刻まで | - | auto | 4 | 559 | 0.07 | 0.05 | 管 OK 11/11 | 0.33 |
| 215 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 55456 | 0.07 | 2.62 | 管 OK 11/11 | 3.15 |
| 216 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.DiodeCenterTap2mPulse` | 終了時刻まで | - | auto | 3 | 671 | 0.11 | 0.14 | 管 OK 6/6 | 0.53 |
| 217 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R` | 終了時刻まで | - | auto | 3 | 736 | 0.15 | 0.27 | 管 OK 18/18 | 0.86 |
| 218 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL` | 終了時刻まで | - | auto | 4 | 736 | 0.23 | 0.26 | 管 OK 19/19 | 27.52 |
| 219 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV` | 終了時刻まで | - | auto | 4 | 740 | 0.20 | 0.26 | 管 OK 19/19 | 1.01 |
| 220 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 72509 | 0.22 | 16.53 | 管 OK 19/19 | 18.60 |
| 221 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.DiodeCenterTapmPulse` | 終了時刻まで | - | auto | 3 | 586 | 0.10 | 0.06 | 管 OK 6/6 | 0.40 |
| 222 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R` | 終了時刻まで | - | auto | 3 | 652 | 0.16 | 0.17 | 管 OK 15/15 | 0.71 |
| 223 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL` | 終了時刻まで | - | auto | 4 | 645 | 0.15 | 0.13 | 管 OK 16/16 | 0.65 |
| 224 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV` | 終了時刻まで | - | auto | 4 | 646 | 0.17 | 0.19 | 管 OK 16/16 | 0.82 |
| 225 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic` | 終了時刻まで | GM-I11002 | auto → dopri45 | 4 | 68902 | 0.17 | 9.90 | 管 OK 16/16 | 11.09 |
| 226 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | 終了時刻まで | - | auto | 12 | 5652 | 0.42 | 1.96 | 管 OK 24/24 | 3.25 |
| 227 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | 終了時刻まで | - | auto | 29 | 6318 | 1.24 | 7.56 | 管 OK 47/47 | 10.36 |
| 228 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM` | 終了時刻まで | - | auto | 7 | 4569 | 0.31 | 2.37 | 管 OK 14/14 | 3.39 |
| 229 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_R` | 終了時刻まで | - | auto | 4 | 781 | 0.09 | 0.36 | 管 OK 6/6 | 0.68 |
| 230 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_RL` | 終了時刻まで | - | auto | 5 | 801 | 0.08 | 0.33 | 管 OK 7/7 | 0.74 |
| 231 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | 終了時刻まで | GM-I11002 | auto → dopri45 | 7 | 108027 | 0.10 | 17.06 | 管 OK 4/4 | 17.73 |
| 232 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_R` | 終了時刻まで | - | auto | 4 | 10215 | 0.05 | 0.32 | 管 OK 8/8 | 0.63 |
| 233 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_RL` | 終了時刻まで | - | auto | 5 | 10245 | 0.06 | 0.24 | 管 OK 9/9 | 0.56 |
| 234 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepUp.ChopperStepUp_R` | 終了時刻まで | - | auto | 4 | 10226 | 0.05 | 0.21 | 管 OK 10/10 | 0.51 |
| 235 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | 実行 NG | GM-E11005 | auto | - | - | - | - | - | 188.42 |
| 236 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_R` | 終了時刻まで | - | auto | 2 | 701 | 0.11 | 0.29 | 管 OK 6/6 | 0.66 |
| 237 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_RL` | 終了時刻まで | - | auto | 3 | 701 | 0.10 | 0.37 | 管 OK 7/7 | 0.73 |
| 238 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_TrianglePWM_RL` | 終了時刻まで | - | auto | 3 | 1293 | 0.12 | 0.67 | 管 OK 7/7 | 1.03 |
| 239 | `Electrical.Spice3.Examples.CoupledInductors` | 終了時刻まで | - | auto | 5 | 501 | 0.05 | 0.05 | 管 OK 3/3 | 0.82 |
| 240 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 42.87 |
| 241 | `Fluid.Examples.AST_BatchPlant.Test.OneTank` | 終了時刻まで | - | auto | 2 | 1116 | 2.58 | 2.28 | 管 OK 8/8 | 9.46 |
| 242 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe1` | 終了時刻まで | - | auto | 2 | 505 | 2.29 | 0.04 | 管 OK 4/4 | 4.50 |
| 243 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe2` | 終了時刻まで | - | auto | 2 | 509 | 2.26 | 0.06 | 管 OK 4/4 | 4.58 |
| 244 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe1` | 終了時刻まで | - | auto | 4 | 523 | 2.57 | 0.23 | 管 OK 8/8 | 5.79 |
| 245 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe2` | 終了時刻まで | - | auto | 6 | 513 | 2.64 | 0.13 | 管 OK 12/12 | 8.19 |
| 246 | `Fluid.Examples.AST_BatchPlant.Test.TwoTanks` | 終了時刻まで | - | auto | 4 | 501 | 2.14 | 0.03 | 管 OK 6/6 | 3.82 |
| 247 | `Fluid.Examples.ControlledTankSystem.ControlledTanks` | 終了時刻まで | - | auto | 4 | 535 | 0.59 | 0.24 | 管 OK 23/23 | 3.97 |
| 248 | `Fluid.Examples.DrumBoiler.DrumBoiler` | 外から停止 | - | - | - | - | - | - | - | 153.34 |
| 249 | `Fluid.Examples.Explanatory.MeasuringTemperature` | 外から停止 | - | - | - | - | - | - | - | 156.92 |
| 250 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | 外から停止 | - | - | - | - | - | - | - | 154.69 |
| 251 | `Fluid.Examples.HeatingSystem` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 10.71 |
| 252 | `Fluid.Examples.InverseParameterization` | 外から停止 | - | - | - | - | - | - | - | 156.15 |
| 253 | `Fluid.Examples.NonCircularPipes` | 外から停止 | - | - | - | - | - | - | - | 155.51 |
| 254 | `Fluid.Examples.PumpingSystem` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 5.79 |
| 255 | `Fluid.Examples.Tanks.EmptyTanks` | 終了時刻まで | - | auto | 4 | 505 | 2.41 | 0.05 | 管 OK 3/3 | 3.99 |
| 256 | `Fluid.Examples.Tanks.TanksWithOverflow` | 実行 NG | GM-E11003・GM-I11002 | auto → dopri45 | - | - | - | - | - | 21.74 |
| 257 | `Fluid.Examples.Tanks.ThreeTanks` | 終了時刻まで | - | auto | 6 | 501 | 2.71 | 5.23 | 管 OK 6/6 | 11.50 |
| 258 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.18 |
| 259 | `Magnetic.FluxTubes.Examples.BasicExamples.SaturatedInductor` | 終了時刻まで | - | auto | 1 | 501 | 0.04 | 0.01 | 管 OK 6/6 | 0.18 |
| 260 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | 終了時刻まで | - | auto | 0 | 503 | 0.04 | 0.01 | 管 OK 2/2 | 0.17 |
| 261 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | 終了時刻まで | - | auto | 0 | 503 | 0.04 | 0.01 | 管 OK 2/2 | 0.17 |
| 262 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ArmatureStroke` | 終了時刻まで | - | auto | 6 | 5029 | 0.39 | 0.15 | 管 OK 9/9 | 0.84 |
| 263 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.55 |
| 264 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.31 |
| 265 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonPullInStroke` | 終了時刻まで | - | auto | 9 | 1045 | 0.84 | 2.63 | 管 OK 9/9 | 3.96 |
| 266 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonQuasiStatic` | 終了時刻まで | - | auto | 9 | 502 | 0.54 | 1.46 | 管 OK 28/28 | 2.42 |
| 267 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.48 |
| 268 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | 終了時刻まで | - | auto | 8 | 501 | 4.33 | 0.12 | 管 OK 10/10 | 5.68 |
| 269 | `Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint` | 終了時刻まで | - | auto | 4 | 501 | 2.89 | 0.47 | 管 OK 5/5 | 4.54 |
| 270 | `Mechanics.MultiBody.Examples.Constraints.SphericalConstraint` | 終了時刻まで | - | auto | 13 | 501 | 2.24 | 0.09 | 管 OK 3/3 | 3.58 |
| 271 | `Mechanics.MultiBody.Examples.Constraints.UniversalConstraint` | 終了時刻まで | - | auto | 8 | 509 | 3.67 | 0.63 | 管 OK 7/7 | 6.20 |
| 272 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulum` | 終了時刻まで | - | auto | 4 | 501 | 0.92 | 0.04 | 管 OK 4/4 | 1.70 |
| 273 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | 終了時刻まで | - | auto | 4 | 501 | 1.43 | 0.13 | **管 NG** 3/4（外れ 1） | 2.46 |
| 274 | `Mechanics.MultiBody.Examples.Elementary.ForceAndTorque` | 終了時刻まで | - | auto | 4 | 501 | 0.90 | 0.03 | 管 OK 4/4 | 1.75 |
| 275 | `Mechanics.MultiBody.Examples.Elementary.FreeBody` | 終了時刻まで | - | auto | 13 | 501 | 0.83 | 0.03 | 管 OK 9/9 | 1.53 |
| 276 | `Mechanics.MultiBody.Examples.Elementary.HeatLosses` | 終了時刻まで | - | auto | 40 | 501 | 1.51 | 0.06 | 管 OK 28/28 | 2.51 |
| 277 | `Mechanics.MultiBody.Examples.Elementary.InitSpringConstant` | 終了時刻まで | - | auto | 2 | 501 | 0.76 | 0.02 | 管 OK 2/2 | 1.41 |
| 278 | `Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses` | 終了時刻まで | - | auto | 4 | 501 | 2.25 | 0.07 | 管 OK 4/4 | 3.43 |
| 279 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | 終了時刻まで | - | auto | 2 | 501 | 0.55 | 0.01 | 管 OK 2/2 | 0.93 |
| 280 | `Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper` | 終了時刻まで | - | auto | 4 | 501 | 0.88 | 0.03 | 管 OK 4/4 | 1.52 |
| 281 | `Mechanics.MultiBody.Examples.Elementary.PointGravity` | 終了時刻まで | - | auto | 26 | 501 | 0.64 | 0.03 | 管 OK 18/18 | 1.04 |
| 282 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses` | 終了時刻まで | - | auto | 24 | 501 | 0.45 | 0.02 | 管 OK 24/24 | 0.91 |
| 283 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2` | 終了時刻まで | - | auto | 26 | 501 | 4.01 | 0.28 | 管 OK 15/15 | 5.29 |
| 284 | `Mechanics.MultiBody.Examples.Elementary.RollingWheel` | 終了時刻まで | - | auto | 8 | 501 | 0.77 | 0.08 | 管 OK 8/8 | 1.23 |
| 285 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving` | 終了時刻まで | - | auto | 10 | 501 | 2.05 | 0.32 | 管 OK 7/7 | 3.14 |
| 286 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling` | 終了時刻まで | - | auto | 10 | 506 | 2.08 | 0.12 | 管 OK 7/7 | 3.16 |
| 287 | `Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem` | 終了時刻まで | - | auto | 15 | 501 | 1.14 | 0.04 | 管 OK 11/11 | 2.00 |
| 288 | `Mechanics.MultiBody.Examples.Elementary.SpringMassSystem` | 終了時刻まで | - | auto | 4 | 501 | 0.90 | 0.02 | 管 OK 4/4 | 1.55 |
| 289 | `Mechanics.MultiBody.Examples.Elementary.SpringWithMass` | 終了時刻まで | - | auto | 13 | 501 | 0.61 | 0.02 | 管 OK 9/9 | 1.23 |
| 290 | `Mechanics.MultiBody.Examples.Elementary.Surfaces` | 終了時刻まで | - | auto | 2 | 501 | 0.54 | 0.02 | 管 OK 2/2 | 1.03 |
| 291 | `Mechanics.MultiBody.Examples.Elementary.ThreeSprings` | 終了時刻まで | - | auto | 13 | 501 | 0.82 | 0.09 | 管 OK 9/9 | 1.74 |
| 292 | `Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField` | 終了時刻まで | - | auto | 2 | 501 | 0.58 | 0.30 | 管 OK 2/2 | 1.28 |
| 293 | `Mechanics.MultiBody.Examples.Loops.Engine1a` | 終了時刻まで | - | auto | 2 | 501 | 3.06 | 0.19 | 管 OK 2/2 | 4.81 |
| 294 | `Mechanics.MultiBody.Examples.Loops.Engine1b` | 終了時刻まで | - | auto | 2 | 527 | 3.62 | 0.31 | 管 OK 2/2 | 5.60 |
| 295 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | 終了時刻まで | - | auto | 2 | 527 | 18.63 | 0.60 | 管 OK 2/2 | 21.07 |
| 296 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | 終了時刻まで | GM-W10004 | auto | 4 | 1162 | 101.18 | 68.41 | **管 NG** 2/4（外れ 2） | 190.66 |
| 297 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | 外から停止 | - | - | - | - | - | - | - | 161.92 |
| 298 | `Mechanics.MultiBody.Examples.Loops.Fourbar1` | 終了時刻まで | - | auto | 2 | 501 | 2.75 | 0.17 | 管 OK 2/2 | 5.28 |
| 299 | `Mechanics.MultiBody.Examples.Loops.Fourbar2` | 終了時刻まで | - | auto | 2 | 501 | 2.04 | 0.10 | 管 OK 2/2 | 3.08 |
| 300 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | 終了時刻まで | - | auto | 2 | 501 | 3.85 | 0.10 | 管 OK 2/2 | 5.94 |
| 301 | `Mechanics.MultiBody.Examples.Loops.PlanarFourbar` | 終了時刻まで | - | auto | 2 | 501 | 2.96 | 0.18 | 管 OK 2/2 | 4.54 |
| 302 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 8.85 |
| 303 | `Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive` | 終了時刻まで | - | auto | 4 | 501 | 0.87 | 0.02 | 管 OK 4/4 | 1.59 |
| 304 | `Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D` | 終了時刻まで | - | auto | 8 | 501 | 1.51 | 0.79 | 管 OK 8/8 | 3.28 |
| 305 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint` | 終了時刻まで | - | auto | 4 | 501 | 1.58 | 0.07 | 管 OK 4/4 | 2.62 |
| 306 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects` | 終了時刻まで | - | auto | 18 | 501 | 3.20 | 0.46 | 管 OK 10/10 | 5.09 |
| 307 | `Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive` | 終了時刻まで | - | auto | 8 | 501 | 1.11 | 0.03 | 管 OK 8/8 | 1.96 |
| 308 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 52.58 |
| 309 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | 実行 NG | GM-E11001 | auto | - | - | - | - | - | 0.93 |
| 310 | `Mechanics.Rotational.Examples.Backlash` | 終了時刻まで | - | auto | 6 | 1165 | 0.06 | 0.06 | 管 OK 4/4 | 0.23 |
| 311 | `Mechanics.Rotational.Examples.CompareBrakingTorque` | 終了時刻まで | - | auto | 8 | 50004 | 0.02 | 0.04 | 管 OK 4/4 | 0.22 |
| 312 | `Mechanics.Rotational.Examples.EddyCurrentBrake` | 終了時刻まで | - | auto | 3 | 1003 | 0.01 | 0.00 | 管 OK 3/3 | 0.12 |
| 313 | `Mechanics.Rotational.Examples.ElasticBearing` | 終了時刻まで | - | auto | 6 | 1002 | 0.03 | 0.02 | 管 OK 6/6 | 0.15 |
| 314 | `Mechanics.Rotational.Examples.First` | 終了時刻まで | - | auto | 4 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.14 |
| 315 | `Mechanics.Rotational.Examples.FirstGrounded` | 終了時刻まで | - | auto | 4 | 1001 | 0.03 | 0.01 | 管 OK 4/4 | 0.13 |
| 316 | `Mechanics.Rotational.Examples.RollingWheel` | 終了時刻まで | - | auto | 2 | 5002 | 0.02 | 0.02 | 管 OK 2/2 | 0.15 |
| 317 | `Mechanics.Translational.Examples.Accelerate` | 終了時刻まで | - | auto | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.08 |
| 318 | `Mechanics.Translational.Examples.CompareBrakingForce` | 終了時刻まで | - | auto | 8 | 50004 | 0.03 | 0.04 | 管 OK 4/4 | 0.20 |
| 319 | `Mechanics.Translational.Examples.Damper` | 終了時刻まで | - | auto | 6 | 1001 | 0.03 | 0.00 | 管 OK 6/6 | 0.12 |
| 320 | `Mechanics.Translational.Examples.EddyCurrentBrake` | 終了時刻まで | - | auto | 3 | 1003 | 0.02 | 0.00 | 管 OK 3/3 | 0.10 |
| 321 | `Mechanics.Translational.Examples.ElastoGap` | 終了時刻まで | - | auto | 4 | 513 | 0.06 | 0.01 | 管 OK 4/4 | 0.17 |
| 322 | `Mechanics.Translational.Examples.GenerationOfFMUs` | 終了時刻まで | - | auto | 10 | 1001 | 0.13 | 0.01 | 管 OK 10/10 | 0.31 |
| 323 | `Mechanics.Translational.Examples.InitialConditions` | 終了時刻まで | - | auto | 8 | 5001 | 0.04 | 0.01 | 管 OK 8/8 | 0.14 |
| 324 | `Mechanics.Translational.Examples.Oscillator` | 終了時刻まで | - | auto | 4 | 1001 | 0.03 | 0.00 | 管 OK 4/4 | 0.12 |
| 325 | `Mechanics.Translational.Examples.PreLoad` | 終了時刻まで | - | auto | 6 | 1022 | 0.11 | 0.02 | 管 OK 6/6 | 0.24 |
| 326 | `Mechanics.Translational.Examples.Sensors` | 終了時刻まで | - | auto | 2 | 1001 | 0.02 | 0.00 | 管 OK 2/2 | 0.13 |
| 327 | `Mechanics.Translational.Examples.SignConvention` | 終了時刻まで | - | auto | 6 | 1001 | 0.01 | 0.00 | 管 OK 6/6 | 0.09 |
| 328 | `Mechanics.Translational.Examples.Vehicle` | 終了時刻まで | - | auto | 10 | 6012 | 0.19 | 0.05 | 管 OK 6/6 | 0.53 |
| 329 | `Mechanics.Translational.Examples.WhyArrows` | 終了時刻まで | - | auto | 4 | 1001 | 0.02 | 0.00 | 管 OK 4/4 | 0.09 |
| 330 | `Media.Examples.SimpleLiquidWater` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.35 |
| 331 | `Media.Examples.TwoPhaseWater.BaseProperties` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 21.64 |
| 332 | `Media.Examples.WaterIF97` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.88 |
| 333 | `StateGraph.Examples.ControlledTanks` | 終了時刻まで | - | auto | 2 | 533 | 0.10 | 0.07 | 管 OK 21/21 | 0.65 |
| 334 | `StateGraph.Examples.ExecutionPaths` | 終了時刻まで | - | auto | 0 | 527 | 0.02 | 0.03 | 管 OK 9/9 | 0.28 |
| 335 | `StateGraph.Examples.FirstExample` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.17 |
| 336 | `StateGraph.Examples.FirstExample_Variant2` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.17 |
| 337 | `StateGraph.Examples.FirstExample_Variant3` | 終了時刻まで | - | auto | 0 | 511 | 0.01 | 0.00 | 管 OK 2/2 | 0.17 |
| 338 | `StateGraph.Examples.ShowCompositeStep` | 終了時刻まで | - | auto | 0 | 522 | 0.02 | 0.02 | 管 OK 9/9 | 0.27 |
| 339 | `StateGraph.Examples.ShowExceptions` | 終了時刻まで | - | auto | 0 | 525 | 0.02 | 0.04 | 管 OK 10/10 | 0.31 |
| 340 | `Thermal.FluidHeatFlow.Examples.IndirectCooling` | 終了時刻まで | - | auto | 4 | 1501 | 0.07 | 0.01 | 管 OK 4/4 | 0.32 |
| 341 | `Thermal.FluidHeatFlow.Examples.OneMass` | 終了時刻まで | - | auto | 2 | 1008 | 0.05 | 0.00 | 管 OK 2/2 | 0.18 |
| 342 | `Thermal.FluidHeatFlow.Examples.ParallelCooling` | 終了時刻まで | - | auto | 5 | 1001 | 0.07 | 0.01 | 管 OK 5/5 | 0.25 |
| 343 | `Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut` | 終了時刻まで | - | auto | 5 | 2009 | 0.08 | 0.03 | 管 OK 5/5 | 0.28 |
| 344 | `Thermal.FluidHeatFlow.Examples.PumpAndValve` | 終了時刻まで | - | auto | 3 | 2005 | 0.06 | 0.03 | 管 OK 3/3 | 0.26 |
| 345 | `Thermal.FluidHeatFlow.Examples.PumpDropOut` | 終了時刻まで | - | auto | 2 | 2007 | 0.04 | 0.01 | 管 OK 2/2 | 0.18 |
| 346 | `Thermal.FluidHeatFlow.Examples.SimpleCooling` | 終了時刻まで | - | auto | 2 | 1001 | 0.04 | 0.00 | 管 OK 2/2 | 0.17 |
| 347 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | 実行 NG | GM-W10002・GM-E11001 | auto | - | - | - | - | - | 0.19 |
| 348 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | 実行 NG | GM-W10002・GM-E11001 | auto | - | - | - | - | - | 30.29 |
| 349 | `Thermal.FluidHeatFlow.Examples.TwoMass` | 終了時刻まで | GM-I11002 | auto → dopri45 | 5 | 1011 | 0.09 | 0.09 | 管 OK 5/5 | 0.41 |
| 350 | `Thermal.FluidHeatFlow.Examples.TwoTanks` | 終了時刻まで | GM-W10002 | auto | 4 | 501 | 0.04 | 0.01 | 管 OK 4/4 | 0.16 |
| 351 | `Thermal.FluidHeatFlow.Examples.WaterPump` | 終了時刻まで | - | auto | 1 | 2015 | 0.07 | 0.04 | 管 OK 1/1 | 0.28 |
| 352 | `Thermal.HeatTransfer.Examples.ControlledTemperature` | 終了時刻まで | - | auto | 1 | 10062 | 0.03 | 0.07 | 管 OK 2/2 | 0.22 |
| 353 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.20 |
| 354 | `Thermal.HeatTransfer.Examples.Motor` | 終了時刻まで | - | auto | 2 | 720025 | 0.03 | 2.98 | 管 OK 2/2 | 4.27 |
| 355 | `Thermal.HeatTransfer.Examples.TwoMasses` | 終了時刻まで | - | auto | 2 | 1001 | 0.01 | 0.00 | 管 OK 2/2 | 0.11 |
| 356 | `Utilities.Examples.WriteRealMatrixToFile` | 実行 NG | GM-U0004 | auto | - | - | - | - | - | 0.06 |
