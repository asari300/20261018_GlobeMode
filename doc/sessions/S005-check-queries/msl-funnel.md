# MSL 4.1.0 の全例題のファネル（globe check）

`cargo xtask msl-funnel` が生成した。手で編集しない。

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | v0.10.2（rev `276e82bd`、ライブラリとして呼ぶ） |
| 例題 | 566 本（Rumoca の基準値 566 本と一致。うち partial 13 本（Rumoca の基準値は 13 本）） |
| 検査した本数 | 566 本（全部） |
| 検査器 | `globe_mode_frontend::Checker`（MSL を一度だけ読み、Rumoca の Session を使い回す） |
| 結果 | **566 本: OK 369（うち S6 判定不能 0・S7 の情報 154）・NG 13・GM-U 184・panic 0・S6 の Rumoca との照合 381/381 一致** |

## 集計

| 項目 | 本数 |
| --- | ---: |
| 対象 | 566 |
| OK（S0〜S6 で Error 無し） | 369 |
| うち S6 判定不能（構造が無い・正方でない） | 0 |
| うち S7 の情報 GM-I7001（インデックス > 1 の見込み） | 154 |
| NG S0（構文） | 0 |
| NG S1（名前解決） | 0 |
| NG S2（型と次元） | 0 |
| NG S3（instantiate） | 0 |
| NG S4（接続） | 0 |
| NG S5（方程式数と変数数） | 1 |
| NG S6（構造特異） | 12 |
| GM-U0001（コンパイラが未対応） | 82 |
| GM-U0002（コンパイラの内部エラー） | 52 |
| GM-U0003（既知の制約の一覧） | 50 |
| panic | 0 |
| S6 の der の形のマッチングを Rumoca の構造解析（`phase_structural::analyze`）と照合（一致 / 照合した本数） | 381 / 381 |
| S6 を見たモデルのうち、der の形で未知数を含まない方程式（状態・パラメータ・入力だけの拘束）があるもの | 22 |
| partial（参考。うち OK） | 13（12） |
| 時間 | 検査 1068.3 s（1 本あたり 1.89 s）・MSL の読み込み 3.5 s・Session を戻す 11 回 16.2 s（50 本ごと）・例題の索引 7.3 s |
| 最大の常駐メモリ（VmHWM） | 7.2 GiB |

## Rumoca の基準値との比較

Rumoca の基準値は `crates/rumoca-test-msl/tests/msl_tests/msl_quality_baseline.json`（rev `276e82bd` に同梱）。
GlobeMode の列は、構文 = S0 を越えた、flatten = S4 を越えた（S5 以降で止まったか OK）、DAE = DAE まで変換できた、釣り合い = S5 を越えた、の本数。panic は数えない。

| | 例題 | 構文 | flatten | DAE | 釣り合い |
| --- | ---: | ---: | ---: | ---: | ---: |
| Rumoca の基準値 | 566 | 566 | 494 | 305 | 293 |
| GlobeMode（この実行） | 566 | 566 | 532 | 382 | 381 |

## Rumoca のエラーコードの度数

| コード | 本数 | 振り分け |
| --- | ---: | --- |
| ED019 | 52 | GM-U0001（コードで決まる） |
| ED001 | 41 | コードだけでは決めない（一覧で決める） |
| ED008 | 28 | GM-U0002（コードで決まる） |
| ED013 | 21 | GM-U0001（コードで決まる） |
| EF023 | 17 | GM-U0002（コードで決まる） |
| EF025 | 4 | GM-U0002（コードで決まる） |
| ET004 | 4 | コードだけでは決めない（一覧で決める） |
| ED010 | 3 | コードだけでは決めない（一覧で決める） |
| ED020 | 3 | GM-U0001（コードで決まる） |
| EF024 | 3 | GM-U0002（コードで決まる） |
| ED018 | 2 | GM-U0001（コードで決まる） |
| EF020 | 2 | GM-U0001（コードで決まる） |
| ER130 | 2 | コードだけでは決めない（一覧で決める） |
| EF004 | 1 | GM-U0001（コードで決まる） |
| EI035 | 1 | GM-U0001（コードで決まる） |

## 既知の制約の一覧の原稿

コードだけでは決まらない Rumoca のコード（ED001 など）で止まった（モデル, コード）の組。理由を確かめて書き直してから `src/Modelica/msl-known-unsupported.txt` に移す（ADR-0007）。

（候補は無い）

## モデルごとの結果

- 数は、状態 / 代数 / 出力（配列は 1 個）と、方程式（スカラー。S5 の釣り合いで数える本数で、離散の方程式を含む）。
- S6 は、マッチングの大きさ / 方程式の数（連続の部分だけ。未知数は `der(x)` と代数・出力のスカラー）。構造特異の判定は `x` と `der(x)` を 1 つの列にまとめた形で行う。der の形（`der(x)` だけを未知数に数える形）のマッチングが完全でないときは括弧に書き、S7 の情報 `GM-I7001`（インデックス > 1 の見込み）を付ける。der の形のマッチングが Rumoca の構造解析と食い違うときは、その数も括弧に書く。
- 秒は、検査（と S6 の照合）にかかった時間。MSL の読み込みは含まない。

| # | モデル | 判定 | 段階 | GM コード | Rumoca | 状態 / 代数 / 出力・方程式 | S6 | 秒 |
| ---: | --- | --- | --- | --- | --- | --- | --- | ---: |
| 1 | `Blocks.Examples.BooleanNetwork1` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.69 |
| 2 | `Blocks.Examples.BusUsage` | OK | - | - | - | 0 / 4 / 3・14 | 正則 7/7 | 0.11 |
| 3 | `Blocks.Examples.BusUsage_Utilities.Part` | OK | - | - | - | 0 / 1 / 1・4 | 正則 2/2 | 0.12 |
| 4 | `Blocks.Examples.CompareSincExpSine` | OK | - | - | - | 0 / 1 / 3・4 | 正則 4/4 | 0.12 |
| 5 | `Blocks.Examples.DemoSignalCharacteristic` | OK | - | - | - | 5 / 18 / 20・53 | 正則 46/46 | 0.31 |
| 6 | `Blocks.Examples.DemonstrateContinuousSignalExtrema` | OK | - | - | - | 3 / 12 / 10・43 | 正則 25/25 | 0.16 |
| 7 | `Blocks.Examples.DemonstrateSignalExtrema` | OK | - | - | - | 1 / 6 / 3・18 | 正則 10/10 | 0.12 |
| 8 | `Blocks.Examples.Filter` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.37 |
| 9 | `Blocks.Examples.FilterWithDifferentiation` | OK | - | GM-I7001（情報） | - | 4 / 2 / 5・15 | 正則 15/15（der 14/15） | 0.20 |
| 10 | `Blocks.Examples.FilterWithRiseTime` | OK | - | - | - | 3 / 6 / 4・22 | 正則 22/22 | 0.19 |
| 11 | `Blocks.Examples.IntegerNetwork1` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.13 |
| 12 | `Blocks.Examples.Interaction1` | OK | - | - | - | 0 / 11 / 6・50 | 正則 17/17 | 0.27 |
| 13 | `Blocks.Examples.InverseModel` | OK | - | GM-I7001（情報） | - | 3 / 7 / 5・15 | 正則 15/15（der 14/15） | 0.12 |
| 14 | `Blocks.Examples.LogicalNetwork1` | OK | - | - | - | 0 / 4 / 2・24 | 正則 6/6 | 0.17 |
| 15 | `Blocks.Examples.Modulation` | OK | - | - | - | 5 / 14 / 13・32 | 正則 32/32 | 0.10 |
| 16 | `Blocks.Examples.Noise.ActuatorWithNoise` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.16 |
| 17 | `Blocks.Examples.Noise.AutomaticSeed` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.16 |
| 18 | `Blocks.Examples.Noise.Densities` | OK | - | - | - | 0 / 5 / 9・14 | 正則 14/14 | 0.13 |
| 19 | `Blocks.Examples.Noise.Distributions` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.16 |
| 20 | `Blocks.Examples.Noise.DrydenContinuousTurbulence` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.15 |
| 21 | `Blocks.Examples.Noise.ImpureGenerator` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.11 |
| 22 | `Blocks.Examples.Noise.NormalNoiseProperties` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.15 |
| 23 | `Blocks.Examples.Noise.UniformNoise` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.13 |
| 24 | `Blocks.Examples.Noise.UniformNoiseProperties` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.17 |
| 25 | `Blocks.Examples.PID_Controller` | OK | - | GM-I7001（情報） | - | 10 / 59 / 17・89 | 正則 86/86（der 83/86） | 0.28 |
| 26 | `Blocks.Examples.RealNetwork1` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.12 |
| 27 | `Blocks.Examples.Rectifier12pulseFFT` | GM-U | S2 | GM-U0003 | ET004 | - | - | 0.38 |
| 28 | `Blocks.Examples.Rectifier6pulseFFT` | GM-U | S2 | GM-U0003 | ET004 | - | - | 0.26 |
| 29 | `Blocks.Examples.ShowLogicalSources` | OK | - | - | - | 0 / 2 / 1・13 | 正則 3/3 | 0.19 |
| 30 | `Blocks.Examples.SinCosEncoder` | OK | - | GM-I7001（情報） | - | 6 / 23 / 16・53 | 正則 52/52（der 51/52） | 0.13 |
| 31 | `Blocks.Examples.SlewRateLimiter` | OK | - | GM-I7001（情報） | - | 5 / 9 / 4・19 | 正則 18/18（der 16/18・der の形で未知数を含まない方程式 1） | 0.14 |
| 32 | `Blocks.Examples.TotalHarmonicDistortion` | OK | - | - | - | 6 / 64 / 43・141 | 正則 119/119 | 0.23 |
| 33 | `Clocked.Examples.CascadeControlledDrive.AbsoluteClocks` | OK | - | GM-I7001（情報） | - | 5 / 34 / 7・67 | 正則 46/46（der 44/46） | 0.19 |
| 34 | `Clocked.Examples.CascadeControlledDrive.Continuous` | OK | - | GM-I7001（情報） | - | 6 / 37 / 10・56 | 正則 53/53（der 51/53） | 0.18 |
| 35 | `Clocked.Examples.CascadeControlledDrive.SubClocked` | OK | - | GM-I7001（情報） | - | 5 / 34 / 7・67 | 正則 46/46（der 44/46） | 0.21 |
| 36 | `Clocked.Examples.CascadeControlledDrive.SuperSampled` | OK | - | GM-I7001（情報） | - | 5 / 34 / 7・67 | 正則 46/46（der 44/46） | 0.18 |
| 37 | `Clocked.Examples.Elementary.BooleanSignals.AssignClock` | OK | - | - | - | 0 / 0 / 0・8 | 正則 0/0 | 0.11 |
| 38 | `Clocked.Examples.Elementary.BooleanSignals.AssignClockVectorized` | OK | - | - | - | 0 / 0 / 0・16 | 正則 0/0 | 0.11 |
| 39 | `Clocked.Examples.Elementary.BooleanSignals.BackSample` | OK | - | - | - | 0 / 2 / 1・13 | 正則 3/3 | 0.19 |
| 40 | `Clocked.Examples.Elementary.BooleanSignals.Hold` | OK | - | - | - | 0 / 2 / 1・13 | 正則 3/3 | 0.20 |
| 41 | `Clocked.Examples.Elementary.BooleanSignals.Sample1` | OK | - | - | - | 0 / 0 / 0・5 | 正則 0/0 | 0.15 |
| 42 | `Clocked.Examples.Elementary.BooleanSignals.Sample2` | OK | - | - | - | 0 / 0 / 0・14 | 正則 0/0 | 0.12 |
| 43 | `Clocked.Examples.Elementary.BooleanSignals.SampleClocked` | OK | - | - | - | 0 / 0 / 0・3 | 正則 0/0 | 0.11 |
| 44 | `Clocked.Examples.Elementary.BooleanSignals.SampleVectorizedAndClocked` | OK | - | - | - | 0 / 0 / 0・6 | 正則 0/0 | 0.10 |
| 45 | `Clocked.Examples.Elementary.BooleanSignals.ShiftSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.19 |
| 46 | `Clocked.Examples.Elementary.BooleanSignals.SubSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.18 |
| 47 | `Clocked.Examples.Elementary.BooleanSignals.SuperSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.19 |
| 48 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedPulse` | OK | - | - | - | 0 / 0 / 0・5 | 正則 0/0 | 0.13 |
| 49 | `Clocked.Examples.Elementary.BooleanSignals.TickBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.10 |
| 50 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedPulse` | OK | - | - | - | 0 / 0 / 0・6 | 正則 0/0 | 0.11 |
| 51 | `Clocked.Examples.Elementary.BooleanSignals.TimeBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 1.27 |
| 52 | `Clocked.Examples.Elementary.BooleanSignals.UpSample` | OK | - | - | - | 0 / 2 / 1・24 | 正則 3/3 | 0.25 |
| 53 | `Clocked.Examples.Elementary.ClockSignals.LogicalSample` | OK | - | - | - | 0 / 23 / 11・212 | 正則 34/34 | 0.29 |
| 54 | `Clocked.Examples.Elementary.ClockSignals.RotationalSample` | OK | - | - | - | 0 / 12 / 6・34 | 正則 18/18 | 0.18 |
| 55 | `Clocked.Examples.Elementary.ClockSignals.ShiftSample` | OK | - | - | - | 0 / 1 / 1・3 | 正則 2/2 | 0.15 |
| 56 | `Clocked.Examples.Elementary.ClockSignals.SubSample` | OK | - | - | - | 0 / 1 / 1・3 | 正則 2/2 | 0.14 |
| 57 | `Clocked.Examples.Elementary.ClockSignals.SuperSample` | OK | - | - | - | 0 / 1 / 1・3 | 正則 2/2 | 0.14 |
| 58 | `Clocked.Examples.Elementary.IntegerSignals.AssignClock` | OK | - | - | - | 0 / 0 / 0・8 | 正則 0/0 | 0.13 |
| 59 | `Clocked.Examples.Elementary.IntegerSignals.AssignClockVectorized` | OK | - | - | - | 0 / 0 / 0・16 | 正則 0/0 | 0.13 |
| 60 | `Clocked.Examples.Elementary.IntegerSignals.BackSample` | OK | - | - | - | 0 / 2 / 1・13 | 正則 3/3 | 0.22 |
| 61 | `Clocked.Examples.Elementary.IntegerSignals.Hold` | OK | - | - | - | 0 / 2 / 1・13 | 正則 3/3 | 0.20 |
| 62 | `Clocked.Examples.Elementary.IntegerSignals.Sample1` | OK | - | - | - | 0 / 0 / 0・5 | 正則 0/0 | 0.13 |
| 63 | `Clocked.Examples.Elementary.IntegerSignals.Sample2` | OK | - | - | - | 0 / 0 / 0・14 | 正則 0/0 | 0.14 |
| 64 | `Clocked.Examples.Elementary.IntegerSignals.SampleClocked` | OK | - | - | - | 0 / 0 / 0・3 | 正則 0/0 | 0.15 |
| 65 | `Clocked.Examples.Elementary.IntegerSignals.SampleVectorizedAndClocked` | OK | - | - | - | 0 / 0 / 0・6 | 正則 0/0 | 0.15 |
| 66 | `Clocked.Examples.Elementary.IntegerSignals.ShiftSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.20 |
| 67 | `Clocked.Examples.Elementary.IntegerSignals.SubSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.20 |
| 68 | `Clocked.Examples.Elementary.IntegerSignals.SuperSample` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.20 |
| 69 | `Clocked.Examples.Elementary.IntegerSignals.TickBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.11 |
| 70 | `Clocked.Examples.Elementary.IntegerSignals.TimeBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.11 |
| 71 | `Clocked.Examples.Elementary.IntegerSignals.UpSample` | OK | - | - | - | 0 / 2 / 1・24 | 正則 3/3 | 0.18 |
| 72 | `Clocked.Examples.Elementary.RealSignals.AssignClock` | OK | - | - | - | 0 / 0 / 0・8 | 正則 0/0 | 0.12 |
| 73 | `Clocked.Examples.Elementary.RealSignals.AssignClockToSquareWaveHold` | OK | - | - | - | 0 / 1 / 1・7 | 正則 2/2 | 0.14 |
| 74 | `Clocked.Examples.Elementary.RealSignals.AssignClockToTriggerHold` | OK | - | - | - | 0 / 2 / 1・11 | 正則 3/3 | 0.13 |
| 75 | `Clocked.Examples.Elementary.RealSignals.AssignClockVectorized` | OK | - | - | - | 0 / 0 / 0・16 | 正則 0/0 | 0.12 |
| 76 | `Clocked.Examples.Elementary.RealSignals.BackSample` | OK | - | - | - | 0 / 1 / 1・7 | 正則 2/2 | 0.12 |
| 77 | `Clocked.Examples.Elementary.RealSignals.FractionalDelay` | OK | - | - | - | 0 / 1 / 1・8 | 正則 2/2 | 0.14 |
| 78 | `Clocked.Examples.Elementary.RealSignals.Hold` | OK | - | - | - | 0 / 1 / 2・7 | 正則 3/3 | 0.13 |
| 79 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects1` | OK | - | - | - | 0 / 1 / 3・19 | 正則 4/4 | 0.15 |
| 80 | `Clocked.Examples.Elementary.RealSignals.HoldWithDAeffects2` | OK | - | - | - | 0 / 1 / 3・19 | 正則 4/4 | 0.16 |
| 81 | `Clocked.Examples.Elementary.RealSignals.Sample1` | OK | - | - | - | 0 / 1 / 1・5 | 正則 2/2 | 0.13 |
| 82 | `Clocked.Examples.Elementary.RealSignals.Sample2` | OK | - | - | - | 0 / 1 / 1・5 | 正則 2/2 | 0.11 |
| 83 | `Clocked.Examples.Elementary.RealSignals.Sample3` | OK | - | - | - | 0 / 2 / 2・12 | 正則 4/4 | 0.12 |
| 84 | `Clocked.Examples.Elementary.RealSignals.SampleClocked` | OK | - | - | - | 0 / 1 / 1・3 | 正則 2/2 | 0.14 |
| 85 | `Clocked.Examples.Elementary.RealSignals.SampleVectorizedAndClocked` | OK | - | - | - | 0 / 1 / 2・6 | 正則 4/4 | 0.14 |
| 86 | `Clocked.Examples.Elementary.RealSignals.SampleWithADeffects` | OK | - | - | - | 0 / 2 / 1・21 | 正則 3/3 | 0.18 |
| 87 | `Clocked.Examples.Elementary.RealSignals.ShiftSample` | OK | - | - | - | 0 / 1 / 1・5 | 正則 2/2 | 0.14 |
| 88 | `Clocked.Examples.Elementary.RealSignals.SubSample` | OK | - | - | - | 0 / 1 / 1・5 | 正則 2/2 | 0.13 |
| 89 | `Clocked.Examples.Elementary.RealSignals.SuperSample` | OK | - | - | - | 0 / 1 / 1・5 | 正則 2/2 | 0.12 |
| 90 | `Clocked.Examples.Elementary.RealSignals.SuperSampleInterpolated` | OK | - | - | - | 0 / 1 / 1・12 | 正則 2/2 | 0.12 |
| 91 | `Clocked.Examples.Elementary.RealSignals.TickBasedRamp` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.12 |
| 92 | `Clocked.Examples.Elementary.RealSignals.TickBasedSine` | OK | - | - | - | 0 / 0 / 1・7 | 正則 1/1 | 0.12 |
| 93 | `Clocked.Examples.Elementary.RealSignals.TickBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.10 |
| 94 | `Clocked.Examples.Elementary.RealSignals.TimeBasedRamp` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.13 |
| 95 | `Clocked.Examples.Elementary.RealSignals.TimeBasedSine` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.13 |
| 96 | `Clocked.Examples.Elementary.RealSignals.TimeBasedStep` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.10 |
| 97 | `Clocked.Examples.Elementary.RealSignals.UniformNoise` | OK | - | - | - | 0 / 1 / 1・9 | 正則 2/2 | 0.12 |
| 98 | `Clocked.Examples.Elementary.RealSignals.UniformNoiseXorshift64star` | OK | - | - | - | 0 / 1 / 1・8 | 正則 2/2 | 0.12 |
| 99 | `Clocked.Examples.Elementary.RealSignals.UpSample1` | OK | - | - | - | 0 / 1 / 1・18 | 正則 2/2 | 0.12 |
| 100 | `Clocked.Examples.Elementary.RealSignals.UpSample2` | OK | - | - | - | 0 / 1 / 1・40 | 正則 2/2 | 0.14 |
| 101 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteController` | OK | - | GM-I7001（情報） | - | 3 / 12 / 3・28 | 正則 18/18（der 17/18） | 1.37 |
| 102 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscreteTextbookController` | OK | - | GM-I7001（情報） | - | 3 / 12 / 3・27 | 正則 18/18（der 17/18） | 0.21 |
| 103 | `Clocked.Examples.SimpleControlledDrive.ClockedWithDiscretizedContinuousController` | GM-U | S5 | GM-U0001 | ED018 | - | - | 0.19 |
| 104 | `Clocked.Examples.SimpleControlledDrive.ClockedWith_AD_DA_Effects` | OK | - | GM-I7001（情報） | - | 3 / 14 / 4・68 | 正則 21/21（der 20/21） | 0.25 |
| 105 | `Clocked.Examples.SimpleControlledDrive.Continuous` | OK | - | GM-I7001（情報） | - | 4 / 13 / 4・21 | 正則 21/21（der 20/21） | 0.15 |
| 106 | `Clocked.Examples.SimpleControlledDrive.ExactlyClockedWithDiscreteController` | OK | - | GM-I7001（情報） | - | 3 / 12 / 3・28 | 正則 18/18（der 17/18） | 0.18 |
| 107 | `Clocked.Examples.Systems.ControlledMixingUnit` | GM-U | S5 | GM-U0001 | ED018 | - | - | 0.17 |
| 108 | `Clocked.Examples.Systems.EngineThrottleControl` | OK | - | GM-I7001（情報） | - | 7 / 55 / 23・117 | 正則 85/85（der 83/85） | 0.25 |
| 109 | `ComplexBlocks.Examples.ShowTransferFunction` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.21 |
| 110 | `ComplexBlocks.Examples.TestConversionBlock` | OK | - | - | - | 0 / 4 / 6・10 | 正則 10/10 | 0.13 |
| 111 | `Electrical.Analog.Examples.AD_DA_conversion` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.17 |
| 112 | `Electrical.Analog.Examples.AmplifierWithOpAmpDetailed` | OK | - | - | - | 7 / 71 / 1・79 | 正則 79/79 | 0.16 |
| 113 | `Electrical.Analog.Examples.CauerLowPassAnalog` | OK | - | GM-I7001（情報） | - | 7 / 61 / 1・69 | 正則 69/69（der 67/69） | 0.16 |
| 114 | `Electrical.Analog.Examples.CauerLowPassOPV` | OK | - | GM-I7001（情報） | - | 9 / 230 / 1・240 | 正則 240/240（der 236/240） | 0.21 |
| 115 | `Electrical.Analog.Examples.CauerLowPassSC` | OK | - | GM-I7001（情報） | - | 20 / 505 / 12・658 | 正則 537/537（der 533/537） | 0.39 |
| 116 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | OK | - | - | - | 0 / 74 / 3・80 | 正則 77/77 | 0.18 |
| 117 | `Electrical.Analog.Examples.CharacteristicThyristors` | OK | - | - | - | 0 / 82 / 2・98 | 正則 84/84 | 0.23 |
| 118 | `Electrical.Analog.Examples.ChuaCircuit` | OK | - | - | - | 3 / 41 / 0・44 | 正則 44/44 | 0.17 |
| 119 | `Electrical.Analog.Examples.CompareTransformers` | OK | - | GM-I7001（情報） | - | 4 / 109 / 10・123 | 正則 123/123（der 122/123） | 0.21 |
| 120 | `Electrical.Analog.Examples.ControlledSwitchWithArc` | OK | - | - | - | 2 / 73 / 1・80 | 正則 76/76 | 0.16 |
| 121 | `Electrical.Analog.Examples.DemoPowerSupply` | OK | - | - | - | 0 / 20 / 2・24 | 正則 22/22 | 0.14 |
| 122 | `Electrical.Analog.Examples.DemoPowerSupplyWithBuffer` | OK | - | - | - | 1 / 41 / 3・50 | 正則 45/45 | 0.15 |
| 123 | `Electrical.Analog.Examples.DemonstrateLightning` | OK | - | GM-I7001（情報） | - | 4 / 44 / 8・58 | 正則 56/56（der 54/56） | 0.18 |
| 124 | `Electrical.Analog.Examples.DifferenceAmplifier` | OK | - | GM-I7001（情報） | - | 13 / 205 / 3・221 | 正則 221/221（der 217/221） | 0.23 |
| 125 | `Electrical.Analog.Examples.GenerationOfFMUs` | OK | - | GM-I7001（情報） | - | 10 / 246 / 50・306 | 正則 306/306（der 302/306） | 0.26 |
| 126 | `Electrical.Analog.Examples.HeatingMOSInverter` | OK | - | - | - | 2 / 88 / 2・92 | 正則 92/92 | 0.19 |
| 127 | `Electrical.Analog.Examples.HeatingNPN_NORGate` | OK | - | GM-I7001（情報） | - | 10 / 141 / 3・158 | 正則 154/154（der 148/154） | 0.22 |
| 128 | `Electrical.Analog.Examples.HeatingPNP_NORGate` | OK | - | GM-I7001（情報） | - | 10 / 141 / 3・158 | 正則 154/154（der 148/154） | 0.22 |
| 129 | `Electrical.Analog.Examples.HeatingRectifier` | OK | - | - | - | 2 / 46 / 1・49 | 正則 49/49 | 0.20 |
| 130 | `Electrical.Analog.Examples.IdealTriacCircuit` | OK | - | - | - | 1 / 53 / 1・62 | 正則 55/55 | 0.20 |
| 131 | `Electrical.Analog.Examples.InvertingAmp` | OK | - | - | - | 0 / 56 / 2・60 | 正則 58/58 | 0.20 |
| 132 | `Electrical.Analog.Examples.Lines.CompareLineTrunks` | OK | - | GM-I7001（情報） | - | 60 / 970 / 9・1091 | 正則 1091/1091（der 1088/1091） | 0.76 |
| 133 | `Electrical.Analog.Examples.Lines.CompareLosslessLines` | OK | - | - | - | 0 / 115 / 1・118 | 正則 116/116 | 0.20 |
| 134 | `Electrical.Analog.Examples.Lines.LightningLosslessTransmissionLine` | OK | - | - | - | 0 / 51 / 2・53 | 正則 53/53 | 0.18 |
| 135 | `Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` | OK | - | GM-I7001（情報） | - | 402 / 5671 / 6・6477 | 正則 6477/6477（der 6476/6477） | 6.19 |
| 136 | `Electrical.Analog.Examples.Lines.SmoothStep` | OK | - | - | - | 116 / 1703 / 7・1935 | 正則 1935/1935 | 1.45 |
| 137 | `Electrical.Analog.Examples.NandGate` | OK | - | - | - | 2 / 138 / 3・147 | 正則 143/143 | 0.20 |
| 138 | `Electrical.Analog.Examples.OpAmps.Adder` | OK | - | - | - | 0 / 76 / 2・78 | 正則 78/78 | 0.20 |
| 139 | `Electrical.Analog.Examples.OpAmps.Comparator` | OK | - | - | - | 0 / 56 / 3・61 | 正則 59/59 | 0.19 |
| 140 | `Electrical.Analog.Examples.OpAmps.ControlCircuit` | OK | - | - | - | 6 / 298 / 7・311 | 正則 311/311 | 0.32 |
| 141 | `Electrical.Analog.Examples.OpAmps.DifferentialAmplifier` | OK | - | - | - | 1 / 125 / 9・137 | 正則 136/136 | 0.23 |
| 142 | `Electrical.Analog.Examples.OpAmps.Differentiator` | OK | - | - | - | 1 / 54 / 2・59 | 正則 57/57 | 0.19 |
| 143 | `Electrical.Analog.Examples.OpAmps.HighPass` | OK | - | - | - | 1 / 63 / 2・68 | 正則 66/66 | 0.18 |
| 144 | `Electrical.Analog.Examples.OpAmps.Integrator` | OK | - | - | - | 1 / 54 / 2・59 | 正則 57/57 | 0.17 |
| 145 | `Electrical.Analog.Examples.OpAmps.InvertingAmplifier` | OK | - | - | - | 0 / 57 / 2・61 | 正則 59/59 | 0.21 |
| 146 | `Electrical.Analog.Examples.OpAmps.InvertingSchmittTrigger` | OK | - | - | - | 0 / 45 / 2・49 | 正則 47/47 | 0.18 |
| 147 | `Electrical.Analog.Examples.OpAmps.LCOscillator` | OK | - | - | - | 2 / 58 / 1・61 | 正則 61/61 | 0.17 |
| 148 | `Electrical.Analog.Examples.OpAmps.LowPass` | OK | - | - | - | 1 / 63 / 2・68 | 正則 66/66 | 0.21 |
| 149 | `Electrical.Analog.Examples.OpAmps.Multivibrator` | OK | - | - | - | 1 / 53 / 1・55 | 正則 55/55 | 0.18 |
| 150 | `Electrical.Analog.Examples.OpAmps.NonInvertingAmplifier` | OK | - | - | - | 0 / 57 / 2・61 | 正則 59/59 | 0.17 |
| 151 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Add` | NG | S6 | GM-E6001 | - | 0 / 58 / 0・58 | **特異** 57/58（der 57/58） | 1.44 |
| 152 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Buffer` | NG | S6 | GM-E6001 | - | 0 / 45 / 0・45 | **特異** 44/45（der 44/45） | 0.17 |
| 153 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Der` | NG | S6 | GM-E6001 | - | 1 / 42 / 0・43 | **特異** 42/43（der 42/43） | 0.15 |
| 154 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Derivative` | NG | S6 | GM-E6001 | - | 1 / 51 / 0・52 | **特異** 51/52（der 51/52） | 0.18 |
| 155 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Feedback` | NG | S6 | GM-E6001 | - | 0 / 67 / 0・67 | **特異** 66/67（der 66/67） | 0.22 |
| 156 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.FirstOrder` | NG | S6 | GM-E6001 | - | 1 / 51 / 0・52 | **特異** 51/52（der 51/52） | 0.17 |
| 157 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Gain` | NG | S6 | GM-E6001 | - | 0 / 45 / 0・45 | **特異** 44/45（der 44/45） | 0.15 |
| 158 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.Integrator` | NG | S6 | GM-E6001 | - | 1 / 42 / 0・43 | **特異** 42/43（der 42/43） | 0.16 |
| 159 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.PI` | NG | S6 | GM-E6001 | - | 1 / 50 / 0・51 | **特異** 50/51（der 50/51） | 0.15 |
| 160 | `Electrical.Analog.Examples.OpAmps.OpAmpCircuits.PartialOpAmp`（partial） | NG | S5 | GM-E5002 | - | 0 / 27 / 0・23 | - | 0.06 |
| 161 | `Electrical.Analog.Examples.OpAmps.SchmittTrigger` | OK | - | - | - | 0 / 45 / 2・49 | 正則 47/47 | 0.19 |
| 162 | `Electrical.Analog.Examples.OpAmps.SignalGenerator` | OK | - | - | - | 1 / 72 / 2・75 | 正則 75/75 | 0.18 |
| 163 | `Electrical.Analog.Examples.OpAmps.Subtracter` | OK | - | - | - | 0 / 85 / 2・87 | 正則 87/87 | 0.19 |
| 164 | `Electrical.Analog.Examples.OpAmps.VoltageFollower` | OK | - | - | - | 0 / 45 / 2・49 | 正則 47/47 | 0.17 |
| 165 | `Electrical.Analog.Examples.OvervoltageProtection` | OK | - | - | - | 1 / 47 / 1・49 | 正則 49/49 | 0.20 |
| 166 | `Electrical.Analog.Examples.ParallelResonance` | OK | - | - | - | 6 / 68 / 5・79 | 正則 79/79 | 0.17 |
| 167 | `Electrical.Analog.Examples.Rectifier` | OK | - | GM-I7001（情報） | - | 5 / 105 / 8・128 | 正則 122/122（der 121/122） | 0.19 |
| 168 | `Electrical.Analog.Examples.Resistor` | OK | - | - | - | 0 / 27 / 1・28 | 正則 28/28 | 0.18 |
| 169 | `Electrical.Analog.Examples.ResonanceCircuits` | OK | - | GM-I7001（情報） | - | 10 / 198 / 44・252 | 正則 252/252（der 246/252） | 0.23 |
| 170 | `Electrical.Analog.Examples.SeriesResonance` | OK | - | - | - | 6 / 68 / 5・79 | 正則 79/79 | 0.17 |
| 171 | `Electrical.Analog.Examples.ShowSaturatingInductor` | OK | - | - | - | 2 / 20 / 1・23 | 正則 23/23 | 0.14 |
| 172 | `Electrical.Analog.Examples.ShowVariableResistor` | OK | - | - | - | 0 / 65 / 2・67 | 正則 67/67 | 0.16 |
| 173 | `Electrical.Analog.Examples.SimpleTriacCircuit` | OK | - | - | - | 3 / 80 / 2・89 | 正則 85/85 | 0.21 |
| 174 | `Electrical.Analog.Examples.SwitchWithArc` | OK | - | - | - | 2 / 61 / 0・71 | 正則 63/63 | 0.16 |
| 175 | `Electrical.Analog.Examples.ThyristorBehaviourTest` | OK | - | - | - | 2 / 42 / 2・48 | 正則 46/46 | 0.16 |
| 176 | `Electrical.Batteries.Examples.BatteryDischargeCharge` | GM-U | S5 | GM-U0001 | ED020 | - | - | 0.41 |
| 177 | `Electrical.Batteries.Examples.CCCV_Cell` | OK | - | - | - | 3 / 95 / 18・120 | 正則 116/116 | 0.32 |
| 178 | `Electrical.Batteries.Examples.CCCV_CellRC` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.37 |
| 179 | `Electrical.Batteries.Examples.CCCV_Stack` | GM-U | S4 | GM-U0001 | EF020 | - | - | 1.09 |
| 180 | `Electrical.Batteries.Examples.CCCV_StackRC` | GM-U | S4 | GM-U0001 | EF020 | - | - | 1.78 |
| 181 | `Electrical.Batteries.Examples.CCCVcharging` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.35 |
| 182 | `Electrical.Batteries.Examples.ShowImpedance` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.18 |
| 183 | `Electrical.Batteries.Examples.SuperCapDischargeCharge` | OK | - | - | - | 2 / 65 / 7・77 | 正則 74/74 | 0.24 |
| 184 | `Electrical.Digital.Examples.Adder4` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.75 |
| 185 | `Electrical.Digital.Examples.BUF3S` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.51 |
| 186 | `Electrical.Digital.Examples.Counter` | GM-U | S5 | GM-U0003 | ED010 | - | - | 0.42 |
| 187 | `Electrical.Digital.Examples.Counter3` | GM-U | S5 | GM-U0003 | ED010 | - | - | 0.35 |
| 188 | `Electrical.Digital.Examples.DFFREG` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.48 |
| 189 | `Electrical.Digital.Examples.DFFREGL` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.47 |
| 190 | `Electrical.Digital.Examples.DFFREGSRH` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.51 |
| 191 | `Electrical.Digital.Examples.DFFREGSRL` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.47 |
| 192 | `Electrical.Digital.Examples.DLATREG` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.42 |
| 193 | `Electrical.Digital.Examples.DLATREGL` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.45 |
| 194 | `Electrical.Digital.Examples.DLATREGSRH` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.48 |
| 195 | `Electrical.Digital.Examples.DLATREGSRL` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.44 |
| 196 | `Electrical.Digital.Examples.FlipFlop` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.29 |
| 197 | `Electrical.Digital.Examples.FullAdder` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.73 |
| 198 | `Electrical.Digital.Examples.HalfAdder` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.55 |
| 199 | `Electrical.Digital.Examples.INV3S` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.44 |
| 200 | `Electrical.Digital.Examples.MUX2x1` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.48 |
| 201 | `Electrical.Digital.Examples.Multiplexer` | GM-U | S5 | GM-U0001 | ED013 | - | - | 1.64 |
| 202 | `Electrical.Digital.Examples.NRXFER` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.56 |
| 203 | `Electrical.Digital.Examples.NXFER` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.56 |
| 204 | `Electrical.Digital.Examples.RAM` | GM-U | S4 | GM-U0002 | EF025 | - | - | 0.10 |
| 205 | `Electrical.Digital.Examples.VectorDelay` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.48 |
| 206 | `Electrical.Digital.Examples.WiredX` | GM-U | S5 | GM-U0001 | ED013 | - | - | 0.58 |
| 207 | `Electrical.Machines.Examples.ControlledDCDrives.CurrentControlledDCPM` | GM-U | S4 | GM-U0002 | EF024 | - | - | 0.43 |
| 208 | `Electrical.Machines.Examples.ControlledDCDrives.PositionControlledDCPM` | GM-U | S4 | GM-U0002 | EF024 | - | - | 0.47 |
| 209 | `Electrical.Machines.Examples.ControlledDCDrives.SpeedControlledDCPM` | GM-U | S4 | GM-U0002 | EF024 | - | - | 0.45 |
| 210 | `Electrical.Machines.Examples.DCMachines.DCEE_Start` | OK | - | GM-I7001（情報） | - | 14 / 201 / 31・246 | 正則 246/246（der 238/246） | 0.47 |
| 211 | `Electrical.Machines.Examples.DCMachines.DCPM_Cooling` | OK | - | GM-I7001（情報） | - | 15 / 224 / 31・272 | 正則 270/270（der 262/270） | 0.57 |
| 212 | `Electrical.Machines.Examples.DCMachines.DCPM_CurrentControlled` | OK | - | GM-I7001（情報） | - | 17 / 186 / 43・247 | 正則 246/246（der 236/246） | 0.61 |
| 213 | `Electrical.Machines.Examples.DCMachines.DCPM_Drive` | OK | - | GM-I7001（情報） | - | 37 / 524 / 105・671 | 正則 666/666（der 648/666・der の形で未知数を含まない方程式 1） | 0.84 |
| 214 | `Electrical.Machines.Examples.DCMachines.DCPM_QuasiStatic` | OK | - | GM-I7001（情報） | - | 24 / 311 / 55・392 | 正則 390/390（der 375/390） | 0.51 |
| 215 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | OK | - | GM-I7001（情報） | - | 13 / 159 / 28・200 | 正則 200/200（der 192/200） | 0.39 |
| 216 | `Electrical.Machines.Examples.DCMachines.DCPM_Temperature` | OK | - | GM-I7001（情報） | - | 13 / 174 / 27・214 | 正則 214/214（der 206/214） | 0.45 |
| 217 | `Electrical.Machines.Examples.DCMachines.DCPM_withLosses` | OK | - | GM-I7001（情報） | - | 28 / 310 / 55・393 | 正則 393/393（der 375/393） | 0.59 |
| 218 | `Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase` | OK | - | GM-I7001（情報） | - | 15 / 203 / 32・250 | 正則 250/250（der 240/250） | 0.46 |
| 219 | `Electrical.Machines.Examples.DCMachines.DCSE_Start` | OK | - | GM-I7001（情報） | - | 15 / 203 / 31・249 | 正則 249/249（der 239/249） | 0.41 |
| 220 | `Electrical.Machines.Examples.DCMachines.DC_CompareCharacteristics` | OK | - | GM-I7001（情報） | - | 33 / 544 / 89・666 | 正則 666/666（der 645/666） | 0.79 |
| 221 | `Electrical.Machines.Examples.InductionMachines.IMC_Conveyor` | OK | - | GM-I7001（情報） | - | 16 / 369 / 42・490 | 正則 488/488（der 477/488） | 0.86 |
| 222 | `Electrical.Machines.Examples.InductionMachines.IMC_DCBraking` | OK | - | GM-I7001（情報） | - | 16 / 300 / 28・392 | 正則 392/392（der 377/392） | 0.62 |
| 223 | `Electrical.Machines.Examples.InductionMachines.IMC_DOL` | OK | - | GM-I7001（情報） | - | 15 / 393 / 41・524 | 正則 512/512（der 502/512） | 0.80 |
| 224 | `Electrical.Machines.Examples.InductionMachines.IMC_Initialize` | OK | - | GM-I7001（情報） | - | 16 / 350 / 41・466 | 正則 466/466（der 453/466） | 0.83 |
| 225 | `Electrical.Machines.Examples.InductionMachines.IMC_Inverter` | OK | - | GM-I7001（情報） | - | 15 / 351 / 41・468 | 正則 468/468（der 458/468） | 1.12 |
| 226 | `Electrical.Machines.Examples.InductionMachines.IMC_InverterDrive` | OK | - | GM-I7001（情報） | - | 20 / 786 / 55・1064 | 正則 978/978（der 968/978） | 1.60 |
| 227 | `Electrical.Machines.Examples.InductionMachines.IMC_Steinmetz` | OK | - | GM-I7001（情報） | - | 18 / 372 / 40・491 | 正則 485/485（der 474/485） | 0.71 |
| 228 | `Electrical.Machines.Examples.InductionMachines.IMC_Transformer` | OK | - | GM-I7001（情報） | - | 21 / 734 / 57・934 | 正則 913/913（der 899/913） | 1.13 |
| 229 | `Electrical.Machines.Examples.InductionMachines.IMC_YD` | OK | - | GM-I7001（情報） | - | 15 / 483 / 41・658 | 正則 610/610（der 600/610） | 0.95 |
| 230 | `Electrical.Machines.Examples.InductionMachines.IMC_YDarc` | OK | - | GM-I7001（情報） | - | 18 / 547 / 41・745 | 正則 685/685（der 674/685） | 1.05 |
| 231 | `Electrical.Machines.Examples.InductionMachines.IMC_withLosses` | OK | - | GM-I7001（情報） | - | 18 / 380 / 62・524 | 正則 524/524（der 510/524） | 0.93 |
| 232 | `Electrical.Machines.Examples.InductionMachines.IMS_Start` | OK | - | GM-I7001（情報） | - | 16 / 631 / 45・798 | 正則 777/777（der 767/777） | 1.08 |
| 233 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_DOL` | OK | - | GM-I7001（情報） | - | 14 / 521 / 71・701 | 正則 687/687（der 677/687） | 1.04 |
| 234 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Generator` | OK | - | GM-I7001（情報） | - | 15 / 492 / 72・656 | 正則 656/656（der 644/656） | 1.12 |
| 235 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_LoadDump` | GM-U | S5 | GM-U0003 | ED010 | - | - | 1.13 |
| 236 | `Electrical.Machines.Examples.SynchronousMachines.SMEE_Rectifier` | OK | - | GM-I7001（情報） | - | 20 / 470 / 58・617 | 正則 610/610（der 599/610） | 1.26 |
| 237 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Braking` | OK | - | GM-I7001（情報） | - | 15 / 503 / 53・653 | 正則 647/647（der 636/647） | 0.92 |
| 238 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_CurrentSource` | OK | - | GM-I7001（情報） | - | 16 / 478 / 76・659 | 正則 659/659（der 644/659） | 0.89 |
| 239 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_Inverter` | OK | - | GM-I7001（情報） | - | 16 / 425 / 61・581 | 正則 581/581（der 570/581） | 0.85 |
| 240 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_NoLoad` | OK | - | GM-I7001（情報） | - | 13 / 295 / 41・404 | 正則 404/404（der 391/404） | 0.68 |
| 241 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_ResistiveBraking` | OK | - | GM-I7001（情報） | - | 14 / 570 / 43・739 | 正則 711/711（der 700/711） | 0.96 |
| 242 | `Electrical.Machines.Examples.SynchronousMachines.SMPM_VoltageSource` | OK | - | GM-I7001（情報） | - | 21 / 534 / 101・775 | 正則 774/774（der 759/774・der の形で未知数を含まない方程式 1） | 1.00 |
| 243 | `Electrical.Machines.Examples.SynchronousMachines.SMR_DOL` | OK | - | GM-I7001（情報） | - | 14 / 445 / 58・606 | 正則 594/594（der 584/594） | 0.89 |
| 244 | `Electrical.Machines.Examples.SynchronousMachines.SMR_Inverter` | OK | - | GM-I7001（情報） | - | 15 / 405 / 58・553 | 正則 553/553（der 543/553） | 0.78 |
| 245 | `Electrical.Machines.Examples.Transformers.AsymmetricalLoad` | OK | - | GM-I7001（情報） | - | 6 / 401 / 23・474 | 正則 474/474（der 469/474） | 0.55 |
| 246 | `Electrical.Machines.Examples.Transformers.IMC_Transformer` | OK | - | GM-I7001（情報） | - | 21 / 734 / 57・934 | 正則 913/913（der 899/913） | 1.30 |
| 247 | `Electrical.Machines.Examples.Transformers.Rectifier12pulse` | OK | - | GM-I7001（情報） | - | 14 / 883 / 39・1046 | 正則 1034/1034（der 1027/1034） | 1.01 |
| 248 | `Electrical.Machines.Examples.Transformers.Rectifier6pulse` | OK | - | GM-I7001（情報） | - | 8 / 490 / 23・579 | 正則 573/573（der 569/573） | 0.70 |
| 249 | `Electrical.Machines.Examples.Transformers.TransformerTestbench` | OK | - | GM-I7001（情報） | - | 6 / 629 / 59・772 | 正則 772/772（der 768/772） | 1.10 |
| 250 | `Electrical.Polyphase.Examples.PolyphaseRectifier` | GM-U | S5 | GM-U0001 | ED019 | - | - | 2.02 |
| 251 | `Electrical.Polyphase.Examples.Rectifier` | OK | - | GM-I7001（情報） | - | 5 / 454 / 42・531 | 正則 525/525（der 524/525） | 1.96 |
| 252 | `Electrical.Polyphase.Examples.TestSensors` | OK | - | GM-I7001（情報） | - | 3 / 476 / 63・578 | 正則 578/578（der 577/578） | 0.55 |
| 253 | `Electrical.Polyphase.Examples.TransformerYD` | OK | - | - | - | 3 / 255 / 3・285 | 正則 285/285 | 0.44 |
| 254 | `Electrical.Polyphase.Examples.TransformerYY` | OK | - | GM-I7001（情報） | - | 3 / 251 / 3・281 | 正則 281/281（der 280/281） | 0.39 |
| 255 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_R` | OK | - | - | - | 10 / 102 / 54・197 | 正則 172/172 | 0.67 |
| 256 | `Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` | OK | - | - | - | 11 / 107 / 54・203 | 正則 178/178 | 0.50 |
| 257 | `Electrical.PowerConverters.Examples.ACAC.ExampleTemplates.Dimmer`（partial） | OK | - | - | - | 10 / 93 / 54・188 | 正則 163/163 | 0.47 |
| 258 | `Electrical.PowerConverters.Examples.ACAC.SoftStarter` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.11 |
| 259 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.Thyristor1Pulse`（partial） | OK | - | - | - | 3 / 53 / 19・93 | 正則 76/76 | 0.28 |
| 260 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2Pulse`（partial） | OK | - | - | - | 3 / 96 / 19・154 | 正則 119/119 | 0.33 |
| 261 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorBridge2mPulse`（partial） | OK | - | - | - | 3 / 274 / 34・443 | 正則 336/336 | 0.62 |
| 262 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2Pulse`（partial） | OK | - | - | - | 3 / 82 / 20・137 | 正則 106/106 | 0.33 |
| 263 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTap2mPulse`（partial） | OK | - | - | - | 3 / 274 / 40・441 | 正則 346/346 | 0.57 |
| 264 | `Electrical.PowerConverters.Examples.ACDC.ExampleTemplates.ThyristorCenterTapmPulse`（partial） | OK | - | - | - | 3 / 196 / 34・325 | 正則 254/254 | 0.50 |
| 265 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R` | OK | - | - | - | 3 / 62 / 19・102 | 正則 85/85 | 0.28 |
| 266 | `Electrical.PowerConverters.Examples.ACDC.Rectifier1Pulse.Thyristor1Pulse_R_Characteristic` | OK | - | - | - | 3 / 64 / 19・104 | 正則 87/87 | 0.27 |
| 267 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.DiodeBridge2Pulse` | OK | - | - | - | 3 / 83 / 9・103 | 正則 96/96 | 0.27 |
| 268 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.HalfControlledBridge2Pulse` | OK | - | - | - | 3 / 105 / 19・161 | 正則 128/128 | 0.36 |
| 269 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive` | OK | - | GM-I7001（情報） | - | 19 / 265 / 53・373 | 正則 338/338（der 330/338） | 0.77 |
| 270 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_R` | OK | - | - | - | 3 / 105 / 19・163 | 正則 128/128 | 0.34 |
| 271 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RL` | OK | - | - | - | 4 / 110 / 19・169 | 正則 134/134 | 0.34 |
| 272 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV` | OK | - | - | - | 4 / 116 / 19・175 | 正則 140/140 | 0.39 |
| 273 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic` | OK | - | - | - | 4 / 118 / 19・177 | 正則 142/142 | 0.35 |
| 274 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.DiodeBridge2mPulse` | OK | - | - | - | 3 / 209 / 11・251 | 正則 242/242 | 0.50 |
| 275 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.HalfControlledBridge2mPulse` | OK | - | - | - | 3 / 283 / 34・425 | 正則 345/345 | 0.56 |
| 276 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_DC_Drive` | OK | - | GM-I7001（情報） | - | 27 / 505 / 76・748 | 正則 641/641（der 632/641） | 1.36 |
| 277 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_R` | OK | - | - | - | 3 / 283 / 34・452 | 正則 345/345 | 0.60 |
| 278 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RL` | OK | - | - | - | 4 / 288 / 34・458 | 正則 351/351 | 0.65 |
| 279 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV` | OK | - | - | - | 4 / 294 / 34・464 | 正則 357/357 | 0.63 |
| 280 | `Electrical.PowerConverters.Examples.ACDC.RectifierBridge2mPulse.ThyristorBridge2mPulse_RLV_Characteristic` | OK | - | - | - | 4 / 296 / 34・466 | 正則 359/359 | 0.59 |
| 281 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.DiodeCenterTap2Pulse` | OK | - | - | - | 3 / 69 / 10・88 | 正則 83/83 | 0.23 |
| 282 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_R` | OK | - | - | - | 3 / 91 / 20・146 | 正則 115/115 | 0.37 |
| 283 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RL` | OK | - | - | - | 4 / 96 / 20・152 | 正則 121/121 | 0.33 |
| 284 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV` | OK | - | - | - | 4 / 102 / 20・158 | 正則 127/127 | 0.32 |
| 285 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2Pulse.ThyristorCenterTap2Pulse_RLV_Characteristic` | OK | - | - | - | 4 / 104 / 20・160 | 正則 129/129 | 0.37 |
| 286 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.DiodeCenterTap2mPulse` | OK | - | - | - | 3 / 218 / 14・267 | 正則 258/258 | 0.42 |
| 287 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_R` | OK | - | - | - | 3 / 283 / 40・450 | 正則 355/355 | 0.56 |
| 288 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RL` | OK | - | - | - | 4 / 288 / 40・456 | 正則 361/361 | 0.58 |
| 289 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV` | OK | - | - | - | 4 / 294 / 40・462 | 正則 367/367 | 0.61 |
| 290 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTap2mPulse.ThyristorCenterTap2mPulse_RLV_Characteristic` | OK | - | - | - | 4 / 296 / 40・464 | 正則 369/369 | 0.57 |
| 291 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.DiodeCenterTapmPulse` | OK | - | - | - | 3 / 131 / 11・166 | 正則 160/160 | 0.34 |
| 292 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_R` | OK | - | - | - | 3 / 205 / 34・334 | 正則 263/263 | 0.48 |
| 293 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RL` | OK | - | - | - | 4 / 210 / 34・340 | 正則 269/269 | 0.49 |
| 294 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV` | OK | - | - | - | 4 / 216 / 34・346 | 正則 275/275 | 0.54 |
| 295 | `Electrical.PowerConverters.Examples.ACDC.RectifierCenterTapmPulse.ThyristorCenterTapmPulse_RLV_Characteristic` | OK | - | - | - | 4 / 218 / 34・348 | 正則 277/277 | 0.56 |
| 296 | `Electrical.PowerConverters.Examples.DCAC.ExampleTemplates.SinglePhaseTwoLevel`（partial） | OK | - | - | - | 4 / 94 / 28・162 | 正則 130/130 | 0.32 |
| 297 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_R` | OK | - | - | - | 12 / 376 / 86・614 | 正則 516/516 | 0.68 |
| 298 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.PolyphaseTwoLevel_RL` | OK | - | GM-I7001（情報） | - | 30 / 765 / 170・1268 | 正則 1074/1074（der 1073/1074） | 1.12 |
| 299 | `Electrical.PowerConverters.Examples.DCAC.PolyphaseTwoLevel.ThreePhaseTwoLevel_PWM` | OK | - | - | - | 5 / 245 / 25・402 | 正則 319/319 | 0.66 |
| 300 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_R` | OK | - | - | - | 4 / 103 / 28・171 | 正則 139/139 | 0.31 |
| 301 | `Electrical.PowerConverters.Examples.DCAC.SinglePhaseTwoLevel.SinglePhaseTwoLevel_RL` | OK | - | - | - | 5 / 108 / 28・177 | 正則 145/145 | 1.76 |
| 302 | `Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle` | OK | - | - | - | 7 / 122 / 13・174 | 正則 142/142 | 0.36 |
| 303 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_R` | OK | - | - | - | 4 / 79 / 10・116 | 正則 93/93 | 0.31 |
| 304 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepDown.ChopperStepDown_RL` | OK | - | - | - | 5 / 84 / 10・122 | 正則 99/99 | 0.29 |
| 305 | `Electrical.PowerConverters.Examples.DCDC.ChopperStepUp.ChopperStepUp_R` | OK | - | - | - | 4 / 79 / 10・116 | 正則 93/93 | 0.30 |
| 306 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperBuckBoost`（partial） | OK | - | - | - | 7 / 121 / 13・173 | 正則 141/141 | 0.36 |
| 307 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepDown`（partial） | OK | - | - | - | 4 / 67 / 8・102 | 正則 79/79 | 0.30 |
| 308 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.ChopperStepUp`（partial） | OK | - | - | - | 4 / 67 / 8・102 | 正則 79/79 | 0.31 |
| 309 | `Electrical.PowerConverters.Examples.DCDC.ExampleTemplates.HBridge`（partial） | OK | - | - | - | 2 / 139 / 8・213 | 正則 149/149 | 0.36 |
| 310 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_DC_Drive` | OK | - | GM-I7001（情報） | - | 14 / 296 / 38・421 | 正則 348/348（der 340/348） | 0.71 |
| 311 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_R` | OK | - | - | - | 2 / 148 / 8・222 | 正則 158/158 | 0.38 |
| 312 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_RL` | OK | - | - | - | 3 / 153 / 8・228 | 正則 164/164 | 0.51 |
| 313 | `Electrical.PowerConverters.Examples.DCDC.HBridge.HBridge_TrianglePWM_RL` | OK | - | - | - | 3 / 154 / 9・233 | 正則 166/166 | 0.43 |
| 314 | `Electrical.QuasiStatic.Machines.Examples.TransformerTestbench` | GM-U | S5 | GM-U0002 | ED008 | - | - | 5.75 |
| 315 | `Electrical.QuasiStatic.Polyphase.Examples.BalancingDelta` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.93 |
| 316 | `Electrical.QuasiStatic.Polyphase.Examples.BalancingStar` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.94 |
| 317 | `Electrical.QuasiStatic.Polyphase.Examples.TestSensors` | GM-U | S4 | GM-U0002 | EF023 | - | - | 1.78 |
| 318 | `Electrical.QuasiStatic.Polyphase.Examples.UnsymmetricalLoad` | GM-U | S5 | GM-U0002 | ED008 | - | - | 1.31 |
| 319 | `Electrical.QuasiStatic.SinglePhase.Examples.MultipleResonance` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.39 |
| 320 | `Electrical.QuasiStatic.SinglePhase.Examples.ParallelResonance` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.34 |
| 321 | `Electrical.QuasiStatic.SinglePhase.Examples.Rectifier` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.34 |
| 322 | `Electrical.QuasiStatic.SinglePhase.Examples.SeriesBode` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.28 |
| 323 | `Electrical.QuasiStatic.SinglePhase.Examples.SeriesResonance` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.22 |
| 324 | `Electrical.QuasiStatic.SinglePhase.Examples.Transformer` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.29 |
| 325 | `Electrical.Spice3.Examples.CascodeCircuit` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.30 |
| 326 | `Electrical.Spice3.Examples.CoupledInductors` | OK | - | - | - | 5 / 95 / 3・103 | 正則 103/103 | 0.17 |
| 327 | `Electrical.Spice3.Examples.FourInverters` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.70 |
| 328 | `Electrical.Spice3.Examples.Graetz` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.44 |
| 329 | `Electrical.Spice3.Examples.Inverter` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.71 |
| 330 | `Electrical.Spice3.Examples.InvertersApartRecord` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.06 |
| 331 | `Electrical.Spice3.Examples.InvertersExtendedModel` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.19 |
| 332 | `Electrical.Spice3.Examples.Nand` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.07 |
| 333 | `Electrical.Spice3.Examples.Nor` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.18 |
| 334 | `Electrical.Spice3.Examples.Oscillator` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.53 |
| 335 | `Electrical.Spice3.Examples.Spice3BenchmarkDifferentialPair` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.51 |
| 336 | `Electrical.Spice3.Examples.Spice3BenchmarkFourBitBinaryAdder` | GM-U | S5 | GM-U0001 | ED019 | - | - | 230.15 |
| 337 | `Electrical.Spice3.Examples.Spice3BenchmarkMosfetCharacterization` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.98 |
| 338 | `Electrical.Spice3.Examples.Spice3BenchmarkRtlInverter` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.64 |
| 339 | `Fluid.Examples.AST_BatchPlant.BatchPlant_StandardWater` | OK | - | GM-I7001（情報） | - | 35 / 1877 / 59・2389 | 正則 1767/1767（der 1760/1767） | 57.77 |
| 340 | `Fluid.Examples.AST_BatchPlant.Test.OneTank` | OK | - | - | - | 2 / 136 / 2・126 | 正則 119/119 | 3.55 |
| 341 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe1` | OK | - | - | - | 2 / 153 / 1・145 | 正則 139/139 | 3.78 |
| 342 | `Fluid.Examples.AST_BatchPlant.Test.TankWithEmptyingPipe2` | OK | - | - | - | 2 / 168 / 1・164 | 正則 160/160 | 3.92 |
| 343 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe1` | OK | - | - | - | 4 / 258 / 2・256 | 正則 248/248 | 5.15 |
| 344 | `Fluid.Examples.AST_BatchPlant.Test.TanksWithEmptyingPipe2` | OK | - | - | - | 6 / 316 / 3・315 | 正則 301/301 | 10.73 |
| 345 | `Fluid.Examples.AST_BatchPlant.Test.TwoTanks` | OK | - | - | - | 4 / 136 / 2・120 | 正則 116/116 | 3.39 |
| 346 | `Fluid.Examples.BranchingDynamicPipes` | GM-U | S5 | GM-U0002 | ED008 | - | - | 14.75 |
| 347 | `Fluid.Examples.ControlledTankSystem.ControlledTanks` | OK | - | - | - | 4 / 270 / 17・527 | 正則 275/275 | 6.08 |
| 348 | `Fluid.Examples.DrumBoiler.DrumBoiler` | OK | - | - | - | 3 / 138 / 17・152 | 正則 135/135 | 4.76 |
| 349 | `Fluid.Examples.Explanatory.MeasuringTemperature` | OK | - | GM-I7001（情報） | - | 18 / 502 / 10・462 | 正則 426/426（der 420/426） | 9.63 |
| 350 | `Fluid.Examples.Explanatory.MomentumBalanceFittings` | OK | - | - | - | 0 / 264 / 0・256 | 正則 232/232 | 7.43 |
| 351 | `Fluid.Examples.HeatExchanger.HeatExchangerSimulation` | GM-U | S5 | GM-U0001 | ED019 | - | - | 30.09 |
| 352 | `Fluid.Examples.HeatingSystem` | OK | - | GM-I7001（情報） | - | 9 / 468 / 15・538 | 正則 534/534（der 533/534） | 13.09 |
| 353 | `Fluid.Examples.IncompressibleFluidNetwork` | GM-U | S5 | GM-U0001 | ED019 | - | - | 28.48 |
| 354 | `Fluid.Examples.InverseParameterization` | OK | - | - | - | 0 / 285 / 2・279 | 正則 260/260 | 11.12 |
| 355 | `Fluid.Examples.NonCircularPipes` | OK | - | GM-I7001（情報） | - | 4 / 339 / 2・391 | 正則 361/361（der 359/361） | 9.70 |
| 356 | `Fluid.Examples.PumpingSystem` | OK | - | GM-I7001（情報） | - | 6 / 274 / 6・307 | 正則 278/278（der 277/278） | 10.07 |
| 357 | `Fluid.Examples.Tanks.EmptyTanks` | OK | - | - | - | 4 / 152 / 2・138 | 正則 134/134 | 3.88 |
| 358 | `Fluid.Examples.Tanks.TanksWithOverflow` | OK | - | GM-I7001（情報） | - | 6 / 230 / 6・271 | 正則 259/259（der 257/259） | 4.60 |
| 359 | `Fluid.Examples.Tanks.ThreeTanks` | OK | - | - | - | 6 / 279 / 3・261 | 正則 255/255 | 7.19 |
| 360 | `Fluid.Examples.TraceSubstances.RoomCO2` | GM-U | S5 | GM-U0002 | ED008 | - | - | 6.66 |
| 361 | `Fluid.Examples.TraceSubstances.RoomCO2WithControls` | GM-U | S5 | GM-U0001 | ED019 | - | - | 12.04 |
| 362 | `Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | OK | - | GM-I7001（情報） | - | 2 / 122 / 4・128 | 正則 128/128（der 126/128） | 0.51 |
| 363 | `Magnetic.FluxTubes.Examples.BasicExamples.SaturatedInductor` | OK | - | - | - | 1 / 65 / 1・67 | 正則 67/67 | 0.44 |
| 364 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | OK | - | GM-I7001（情報） | - | 2 / 74 / 3・79 | 正則 79/79（der 77/79） | 0.42 |
| 365 | `Magnetic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | OK | - | GM-I7001（情報） | - | 2 / 74 / 3・79 | 正則 79/79（der 77/79） | 0.39 |
| 366 | `Magnetic.FluxTubes.Examples.Hysteresis.Components.Transformer1PhaseWithHysteresis` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.49 |
| 367 | `Magnetic.FluxTubes.Examples.Hysteresis.Components.Transformer3PhaseYyWithHysteresis` | GM-U | S3 | GM-U0001 | EI035 | - | - | 0.22 |
| 368 | `Magnetic.FluxTubes.Examples.Hysteresis.HysteresisModelComparison` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.69 |
| 369 | `Magnetic.FluxTubes.Examples.Hysteresis.InductorWithHysteresis` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.45 |
| 370 | `Magnetic.FluxTubes.Examples.Hysteresis.SinglePhaseTransformerWithHysteresis1` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.47 |
| 371 | `Magnetic.FluxTubes.Examples.Hysteresis.SinglePhaseTransformerWithHysteresis2` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.46 |
| 372 | `Magnetic.FluxTubes.Examples.Hysteresis.ThreePhaseTransformerWithRectifier` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.96 |
| 373 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ArmatureStroke` | OK | - | GM-I7001（情報） | - | 15 / 228 / 2・249 | 正則 245/245（der 238/245） | 0.51 |
| 374 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.ConstantActuator` | NG | S6 | GM-E6001 | - | 6 / 76 / 0・84 | **特異** 81/82（der 77/82） | 0.31 |
| 375 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.Components.PermeanceActuator` | NG | S6 | GM-E6001 | - | 5 / 126 / 0・133 | **特異** 130/131（der 127/131） | 0.39 |
| 376 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceCurrentBehaviour` | OK | - | GM-I7001（情報） | - | 11 / 227 / 4・246 | 正則 242/242（der 233/242） | 0.57 |
| 377 | `Magnetic.FluxTubes.Examples.MovingCoilActuator.ForceStrokeBehaviour` | OK | - | GM-I7001（情報） | - | 5 / 142 / 2・153 | 正則 151/151（der 147/151） | 0.50 |
| 378 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonPullInStroke` | OK | - | GM-I7001（情報） | - | 17 / 504 / 3・532 | 正則 526/526（der 520/526） | 1.00 |
| 379 | `Magnetic.FluxTubes.Examples.SolenoidActuator.ComparisonQuasiStatic` | OK | - | GM-I7001（情報） | - | 17 / 504 / 4・531 | 正則 527/527（der 521/527） | 1.16 |
| 380 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid` | OK | - | GM-I7001（情報） | - | 8 / 282 / 0・292 | 正則 290/290（der 287/290） | 0.54 |
| 381 | `Magnetic.FluxTubes.Examples.SolenoidActuator.Components.SimpleSolenoid` | NG | S6 | GM-E6001 | - | 5 / 195 / 0・202 | **特異** 199/200（der 196/200） | 0.45 |
| 382 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_CommonLeakage` | GM-U | S5 | GM-U0003 | ED001 | - | - | 5.81 |
| 383 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMC_DOL_Polyphase` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.38 |
| 384 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.ComparisonPolyphase.IMS_Start_Polyphase` | GM-U | S5 | GM-U0003 | ED001 | - | - | 5.15 |
| 385 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Conveyor` | GM-U | S5 | GM-U0003 | ED001 | - | - | 1.86 |
| 386 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_DOL` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.60 |
| 387 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Initialize` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.82 |
| 388 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Inverter` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.34 |
| 389 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Steinmetz` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.16 |
| 390 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Transformer` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.66 |
| 391 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_YD` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.16 |
| 392 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_withLosses` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.17 |
| 393 | `Magnetic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Start` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.04 |
| 394 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMEE_Generator_Polyphase` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.82 |
| 395 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMPM_Inverter_Polyphase` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.31 |
| 396 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.ComparisonPolyphase.SMR_Inverter_Polyphase` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.16 |
| 397 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_DOL` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.00 |
| 398 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Generator` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.62 |
| 399 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_LoadDump` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.41 |
| 400 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Rectifier` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.21 |
| 401 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Braking` | GM-U | S5 | GM-U0003 | ED001 | - | - | 4.60 |
| 402 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_CurrentSource` | GM-U | S5 | GM-U0003 | ED001 | - | - | 1.86 |
| 403 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Inverter` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.66 |
| 404 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_VoltageSource` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.26 |
| 405 | `Magnetic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMR_Inverter` | GM-U | S5 | GM-U0003 | ED001 | - | - | 2.48 |
| 406 | `Magnetic.FundamentalWave.Examples.Components.EddyCurrentLosses` | GM-U | S5 | GM-U0003 | ED001 | - | - | 1.06 |
| 407 | `Magnetic.FundamentalWave.Examples.Components.PolyphaseInductance` | GM-U | S5 | GM-U0003 | ED001 | - | - | 1.01 |
| 408 | `Magnetic.FundamentalWave.Examples.Components.SinglePhaseInductance` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.56 |
| 409 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.54 |
| 410 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreAirgap` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.45 |
| 411 | `Magnetic.QuasiStatic.FluxTubes.Examples.BasicExamples.ToroidalCoreQuadraticCrossSection` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.45 |
| 412 | `Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CuboidSections` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.38 |
| 413 | `Magnetic.QuasiStatic.FluxTubes.Examples.FixedShapes.CylinderSections` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.37 |
| 414 | `Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.CylinderLeakage` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.38 |
| 415 | `Magnetic.QuasiStatic.FluxTubes.Examples.Leakage.GeneralLeakage` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.30 |
| 416 | `Magnetic.QuasiStatic.FluxTubes.Examples.LinearInductor` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.53 |
| 417 | `Magnetic.QuasiStatic.FluxTubes.Examples.NonLinearInductor` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.69 |
| 418 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Characteristics` | GM-U | S4 | GM-U0002 | EF023 | - | - | 3.66 |
| 419 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Conveyor` | GM-U | S4 | GM-U0002 | EF023 | - | - | 6.26 |
| 420 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_DOL` | GM-U | S4 | GM-U0002 | EF023 | - | - | 7.36 |
| 421 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Initialize` | GM-U | S4 | GM-U0002 | EF023 | - | - | 4.71 |
| 422 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Inverter` | GM-U | S4 | GM-U0002 | EF023 | - | - | 2.97 |
| 423 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_Transformer` | GM-U | S4 | GM-U0002 | EF023 | - | - | 12.31 |
| 424 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_YD` | GM-U | S4 | GM-U0002 | EF023 | - | - | 8.13 |
| 425 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMC_withLosses` | GM-U | S4 | GM-U0001 | EF004 | - | - | 4.15 |
| 426 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Characteristics` | GM-U | S4 | GM-U0002 | EF023 | - | - | 4.65 |
| 427 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.InductionMachines.IMS_Start` | GM-U | S4 | GM-U0002 | EF023 | - | - | 9.24 |
| 428 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMEE_Generator` | GM-U | S4 | GM-U0002 | EF023 | - | - | 5.80 |
| 429 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_CurrentSource` | GM-U | S4 | GM-U0002 | EF023 | - | - | 5.25 |
| 430 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_FieldWeakening` | GM-U | S4 | GM-U0002 | EF023 | - | - | 2.82 |
| 431 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_MTPA` | GM-U | S4 | GM-U0002 | EF023 | - | - | 3.19 |
| 432 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_Mains` | GM-U | S4 | GM-U0002 | EF023 | - | - | 6.06 |
| 433 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMPM_OpenCircuit` | GM-U | S4 | GM-U0002 | EF023 | - | - | 3.38 |
| 434 | `Magnetic.QuasiStatic.FundamentalWave.Examples.BasicMachines.SynchronousMachines.SMR_CurrentSource` | GM-U | S4 | GM-U0002 | EF023 | - | - | 4.95 |
| 435 | `Magnetic.QuasiStatic.FundamentalWave.Examples.Components.EddyCurrentLosses` | GM-U | S5 | GM-U0002 | ED008 | - | - | 2.93 |
| 436 | `Magnetic.QuasiStatic.FundamentalWave.Examples.Components.PolyphaseInductance` | GM-U | S5 | GM-U0002 | ED008 | - | - | 3.12 |
| 437 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.FieldWeakeningController` | GM-U | S5 | GM-U0001 | ED020 | - | - | 0.40 |
| 438 | `Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter` | OK | - | - | - | 0 / 0 / 1・1 | 正則 1/1 | 0.20 |
| 439 | `Math.FastFourierTransform.Examples.RealFFT1` | GM-U | S2 | GM-U0003 | ET004 | - | - | 0.13 |
| 440 | `Math.FastFourierTransform.Examples.RealFFT2` | GM-U | S2 | GM-U0003 | ET004 | - | - | 0.11 |
| 441 | `Math.Nonlinear.Examples.QuadratureLobatto3` | GM-U | S5 | GM-U0001 | ED020 | - | - | 0.24 |
| 442 | `Math.Random.Examples.GenerateRandomNumbers` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.21 |
| 443 | `Mechanics.MultiBody.Examples.Constraints.PrismaticConstraint` | OK | - | GM-I7001（情報） | - | 20 / 665 / 19・2429 | 正則 2313/2313（der 2287/2313・der の形で未知数を含まない方程式 1） | 2.76 |
| 444 | `Mechanics.MultiBody.Examples.Constraints.RevoluteConstraint` | OK | - | GM-I7001（情報） | - | 19 / 626 / 14・2312 | 正則 2208/2208（der 2181/2208） | 2.52 |
| 445 | `Mechanics.MultiBody.Examples.Constraints.SphericalConstraint` | OK | - | GM-I7001（情報） | - | 17 / 618 / 14・2323 | 正則 2219/2219（der 2197/2219・der の形で未知数を含まない方程式 1） | 2.87 |
| 446 | `Mechanics.MultiBody.Examples.Constraints.UniversalConstraint` | OK | - | GM-I7001（情報） | - | 26 / 713 / 14・2598 | 正則 2482/2482（der 2453/2482・der の形で未知数を含まない方程式 2） | 4.82 |
| 447 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulum` | OK | - | GM-I7001（情報） | - | 17 / 353 / 0・1302 | 正則 1245/1245（der 1225/1245） | 1.70 |
| 448 | `Mechanics.MultiBody.Examples.Elementary.DoublePendulumInitTip` | OK | - | GM-I7001（情報） | - | 20 / 391 / 10・1479 | 正則 1410/1410（der 1387/1410） | 2.08 |
| 449 | `Mechanics.MultiBody.Examples.Elementary.ForceAndTorque` | OK | - | GM-I7001（情報） | - | 11 / 411 / 6・1541 | 正則 1468/1468（der 1457/1468） | 1.61 |
| 450 | `Mechanics.MultiBody.Examples.Elementary.FreeBody` | OK | - | GM-I7001（情報） | - | 6 / 375 / 0・1330 | 正則 1261/1261（der 1257/1261・der の形で未知数を含まない方程式 1） | 1.62 |
| 451 | `Mechanics.MultiBody.Examples.Elementary.HeatLosses` | OK | - | GM-I7001（情報） | - | 15 / 663 / 1・2151 | 正則 2049/2049（der 2044/2049・der の形で未知数を含まない方程式 3） | 6.17 |
| 452 | `Mechanics.MultiBody.Examples.Elementary.InitSpringConstant` | OK | - | GM-I7001（情報） | - | 9 / 362 / 0・1275 | 正則 1215/1215（der 1205/1215） | 2.19 |
| 453 | `Mechanics.MultiBody.Examples.Elementary.LineForceWithTwoMasses` | OK | - | GM-I7001（情報） | - | 29 / 666 / 0・2303 | 正則 2204/2204（der 2164/2204） | 3.25 |
| 454 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | OK | - | GM-I7001（情報） | - | 7 / 255 / 0・934 | 正則 898/898（der 891/898） | 1.02 |
| 455 | `Mechanics.MultiBody.Examples.Elementary.PendulumWithSpringDamper` | OK | - | GM-I7001（情報） | - | 9 / 396 / 0・1349 | 正則 1289/1289（der 1281/1289） | 1.59 |
| 456 | `Mechanics.MultiBody.Examples.Elementary.PointGravity` | OK | - | GM-I7001（情報） | - | 8 / 233 / 0・911 | 正則 872/872（der 870/872・der の形で未知数を含まない方程式 2） | 1.11 |
| 457 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses` | OK | - | - | - | 8 / 307 / 0・1113 | 正則 1062/1062 | 1.13 |
| 458 | `Mechanics.MultiBody.Examples.Elementary.PointGravityWithPointMasses2` | OK | - | GM-I7001（情報） | - | 34 / 803 / 0・2860 | 正則 2735/2735（der 2685/2735・der の形で未知数を含まない方程式 2） | 2.39 |
| 459 | `Mechanics.MultiBody.Examples.Elementary.RollingWheel` | OK | - | GM-I7001（情報） | - | 5 / 252 / 0・947 | 正則 911/911（der 907/911） | 0.97 |
| 460 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetDriving` | OK | - | GM-I7001（情報） | - | 26 / 578 / 2・1959 | 正則 1878/1878（der 1849/1878・der の形で未知数を含まない方程式 10） | 1.49 |
| 461 | `Mechanics.MultiBody.Examples.Elementary.RollingWheelSetPulling` | OK | - | GM-I7001（情報） | - | 26 / 592 / 1・2047 | 正則 1961/1961（der 1932/1961・der の形で未知数を含まない方程式 10） | 2.41 |
| 462 | `Mechanics.MultiBody.Examples.Elementary.SpringDamperSystem` | OK | - | GM-I7001（情報） | - | 10 / 505 / 0・1686 | 正則 1608/1608（der 1600/1608・der の形で未知数を含まない方程式 1） | 2.36 |
| 463 | `Mechanics.MultiBody.Examples.Elementary.SpringMassSystem` | OK | - | GM-I7001（情報） | - | 10 / 435 / 0・1469 | 正則 1406/1406（der 1394/1406） | 1.55 |
| 464 | `Mechanics.MultiBody.Examples.Elementary.SpringWithMass` | OK | - | GM-I7001（情報） | - | 6 / 261 / 0・968 | 正則 923/923（der 919/923・der の形で未知数を含まない方程式 1） | 1.66 |
| 465 | `Mechanics.MultiBody.Examples.Elementary.Surfaces` | OK | - | GM-I7001（情報） | - | 4 / 300 / 1・1143 | 正則 1104/1104（der 1103/1104） | 1.00 |
| 466 | `Mechanics.MultiBody.Examples.Elementary.ThreeSprings` | OK | - | GM-I7001（情報） | - | 4 / 453 / 0・1516 | 正則 1441/1441（der 1440/1441・der の形で未知数を含まない方程式 1） | 2.12 |
| 467 | `Mechanics.MultiBody.Examples.Elementary.UserDefinedGravityField` | OK | - | GM-I7001（情報） | - | 7 / 255 / 0・951 | 正則 912/912（der 905/912） | 1.20 |
| 468 | `Mechanics.MultiBody.Examples.Loops.Engine1a` | OK | - | GM-I7001（情報） | - | 39 / 642 / 0・2386 | 正則 2269/2269（der 2211/2269） | 3.89 |
| 469 | `Mechanics.MultiBody.Examples.Loops.Engine1b` | OK | - | GM-I7001（情報） | - | 40 / 673 / 0・2488 | 正則 2365/2365（der 2306/2365） | 4.40 |
| 470 | `Mechanics.MultiBody.Examples.Loops.Engine1b_analytic` | OK | - | GM-I7001（情報） | - | 36 / 721 / 5・2752 | 正則 2605/2605（der 2549/2605） | 4.50 |
| 471 | `Mechanics.MultiBody.Examples.Loops.EngineV6` | OK | - | GM-I7001（情報） | - | 222 / 2980 / 5・11107 | 正則 10492/10492（der 10142/10492） | 30.70 |
| 472 | `Mechanics.MultiBody.Examples.Loops.EngineV6_analytic` | OK | - | GM-I7001（情報） | - | 83 / 2375 / 35・9001 | 正則 8443/8443（der 8318/8443） | 11.20 |
| 473 | `Mechanics.MultiBody.Examples.Loops.Fourbar1` | OK | - | GM-I7001（情報） | - | 35 / 598 / 4・2058 | 正則 1971/1971（der 1932/1971） | 2.39 |
| 474 | `Mechanics.MultiBody.Examples.Loops.Fourbar2` | OK | - | GM-I7001（情報） | - | 16 / 599 / 4・2242 | 正則 2155/2155（der 2132/2155） | 2.31 |
| 475 | `Mechanics.MultiBody.Examples.Loops.Fourbar_analytic` | OK | - | GM-I7001（情報） | - | 21 / 456 / 9・1743 | 正則 1659/1659（der 1622/1659・der の形で未知数を含まない方程式 3） | 6.39 |
| 476 | `Mechanics.MultiBody.Examples.Loops.PlanarFourbar` | OK | - | GM-I7001（情報） | - | 33 / 616 / 11・2348 | 正則 2258/2258（der 2214/2258） | 4.27 |
| 477 | `Mechanics.MultiBody.Examples.Loops.PlanarLoops_analytic` | OK | - | GM-I7001（情報） | - | 40 / 1124 / 16・4012 | 正則 3844/3844（der 3771/3844） | 4.24 |
| 478 | `Mechanics.MultiBody.Examples.Rotational3DEffects.ActuatedDrive` | OK | - | GM-I7001（情報） | - | 13 / 384 / 3・1413 | 正則 1348/1348（der 1334/1348） | 1.81 |
| 479 | `Mechanics.MultiBody.Examples.Rotational3DEffects.BevelGear1D` | OK | - | GM-I7001（情報） | - | 23 / 500 / 8・1750 | 正則 1687/1687（der 1667/1687・der の形で未知数を含まない方程式 1） | 2.33 |
| 480 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GearConstraint` | OK | - | GM-I7001（情報） | - | 23 / 473 / 3・1731 | 正則 1654/1654（der 1624/1654・der の形で未知数を含まない方程式 1） | 2.25 |
| 481 | `Mechanics.MultiBody.Examples.Rotational3DEffects.GyroscopicEffects` | OK | - | GM-I7001（情報） | - | 30 / 610 / 0・2276 | 正則 2171/2171（der 2134/2171・der の形で未知数を含まない方程式 2） | 2.96 |
| 482 | `Mechanics.MultiBody.Examples.Rotational3DEffects.MovingActuatedDrive` | OK | - | GM-I7001（情報） | - | 21 / 500 / 4・1736 | 正則 1665/1665（der 1649/1665） | 1.64 |
| 483 | `Mechanics.MultiBody.Examples.Systems.RobotR3.FullRobot` | OK | - | GM-I7001（情報） | - | 158 / 2456 / 112・4570 | 正則 4291/4291（der 4158/4291・der の形で未知数を含まない方程式 18） | 4.95 |
| 484 | `Mechanics.MultiBody.Examples.Systems.RobotR3.OneAxis` | OK | - | GM-I7001（情報） | - | 22 / 328 / 22・391 | 正則 373/373（der 363/373・der の形で未知数を含まない方程式 2） | 0.73 |
| 485 | `Mechanics.Rotational.Examples.Backlash` | OK | - | GM-I7001（情報） | - | 12 / 50 / 0・62 | 正則 62/62（der 59/62） | 0.28 |
| 486 | `Mechanics.Rotational.Examples.CompareBrakingTorque` | OK | - | GM-I7001（情報） | - | 12 / 40 / 0・52 | 正則 52/52（der 48/52） | 0.27 |
| 487 | `Mechanics.Rotational.Examples.CoupledClutches` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.33 |
| 488 | `Mechanics.Rotational.Examples.EddyCurrentBrake` | OK | - | GM-I7001（情報） | - | 4 / 20 / 1・25 | 正則 25/25（der 24/25） | 0.27 |
| 489 | `Mechanics.Rotational.Examples.ElasticBearing` | OK | - | GM-I7001（情報） | - | 9 / 50 / 4・63 | 正則 63/63（der 61/63） | 0.33 |
| 490 | `Mechanics.Rotational.Examples.First` | OK | - | GM-I7001（情報） | - | 8 / 45 / 1・54 | 正則 54/54（der 52/54） | 0.31 |
| 491 | `Mechanics.Rotational.Examples.FirstGrounded` | OK | - | GM-I7001（情報） | - | 8 / 41 / 1・50 | 正則 50/50（der 48/50） | 0.26 |
| 492 | `Mechanics.Rotational.Examples.Friction` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.36 |
| 493 | `Mechanics.Rotational.Examples.GenerationOfFMUs` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.42 |
| 494 | `Mechanics.Rotational.Examples.HeatLosses` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.49 |
| 495 | `Mechanics.Rotational.Examples.LossyGearDemo1` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.34 |
| 496 | `Mechanics.Rotational.Examples.LossyGearDemo2` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.30 |
| 497 | `Mechanics.Rotational.Examples.LossyGearDemo3` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.25 |
| 498 | `Mechanics.Rotational.Examples.OneWayClutch` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.29 |
| 499 | `Mechanics.Rotational.Examples.OneWayClutchDisengaged` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.26 |
| 500 | `Mechanics.Rotational.Examples.RollingWheel` | OK | - | GM-I7001（情報） | - | 5 / 36 / 0・41 | 正則 41/41（der 39/41） | 0.24 |
| 501 | `Mechanics.Rotational.Examples.SimpleGearShift` | GM-U | S5 | GM-U0003 | ED001 | - | - | 3.14 |
| 502 | `Mechanics.Translational.Examples.Accelerate` | OK | - | GM-I7001（情報） | - | 4 / 10 / 1・15 | 正則 15/15（der 14/15） | 0.26 |
| 503 | `Mechanics.Translational.Examples.Brake` | GM-U | S5 | GM-U0003 | ED001 | - | - | 0.37 |
| 504 | `Mechanics.Translational.Examples.CompareBrakingForce` | OK | - | GM-I7001（情報） | - | 12 / 40 / 0・52 | 正則 52/52（der 48/52） | 0.37 |
| 505 | `Mechanics.Translational.Examples.Damper` | OK | - | GM-I7001（情報） | - | 9 / 50 / 0・59 | 正則 59/59（der 56/59） | 0.45 |
| 506 | `Mechanics.Translational.Examples.EddyCurrentBrake` | OK | - | GM-I7001（情報） | - | 4 / 20 / 1・25 | 正則 25/25（der 24/25） | 0.56 |
| 507 | `Mechanics.Translational.Examples.ElastoGap` | OK | - | GM-I7001（情報） | - | 8 / 62 / 0・72 | 正則 70/70（der 66/70） | 0.40 |
| 508 | `Mechanics.Translational.Examples.Friction` | GM-U | S1 | GM-U0003 | ER130 | - | - | 0.14 |
| 509 | `Mechanics.Translational.Examples.GenerationOfFMUs` | OK | - | GM-I7001（情報） | - | 17 / 142 / 42・201 | 正則 201/201（der 196/201） | 0.53 |
| 510 | `Mechanics.Translational.Examples.HeatLosses` | GM-U | S1 | GM-U0003 | ER130 | - | - | 0.14 |
| 511 | `Mechanics.Translational.Examples.InitialConditions` | OK | - | GM-I7001（情報） | - | 10 / 54 / 0・64 | 正則 64/64（der 62/64） | 0.27 |
| 512 | `Mechanics.Translational.Examples.Oscillator` | OK | - | GM-I7001（情報） | - | 5 / 43 / 2・50 | 正則 50/50（der 49/50） | 0.56 |
| 513 | `Mechanics.Translational.Examples.PreLoad` | OK | - | GM-I7001（情報） | - | 11 / 104 / 1・120 | 正則 116/116（der 111/116） | 0.41 |
| 514 | `Mechanics.Translational.Examples.Sensors` | OK | - | GM-I7001（情報） | - | 6 / 23 / 9・38 | 正則 38/38（der 35/38・der の形で未知数を含まない方程式 1） | 0.38 |
| 515 | `Mechanics.Translational.Examples.SignConvention` | OK | - | - | - | 6 / 32 / 3・41 | 正則 41/41 | 0.27 |
| 516 | `Mechanics.Translational.Examples.Vehicle` | OK | - | GM-I7001（情報） | - | 25 / 203 / 18・249 | 正則 247/247（der 236/247） | 0.62 |
| 517 | `Mechanics.Translational.Examples.WhyArrows` | OK | - | - | - | 4 / 49 / 3・56 | 正則 56/56 | 0.25 |
| 518 | `Media.Examples.IdealGasH2O` | GM-U | S5 | GM-U0002 | ED008 | - | - | 2.42 |
| 519 | `Media.Examples.MixtureGases` | GM-U | S4 | GM-U0002 | EF025 | - | - | 1.47 |
| 520 | `Media.Examples.MoistAir` | GM-U | S5 | GM-U0001 | ED019 | - | - | 6.00 |
| 521 | `Media.Examples.PsychrometricData` | GM-U | S5 | GM-U0001 | ED019 | - | - | 6.24 |
| 522 | `Media.Examples.R134a.R134a1` | GM-U | S5 | GM-U0001 | ED019 | - | - | 3.34 |
| 523 | `Media.Examples.R134a.R134a2` | GM-U | S5 | GM-U0001 | ED019 | - | - | 4.43 |
| 524 | `Media.Examples.ReferenceAir.DryAir1` | GM-U | S5 | GM-U0002 | ED008 | - | - | 1.48 |
| 525 | `Media.Examples.ReferenceAir.DryAir2` | GM-U | S5 | GM-U0002 | ED008 | - | - | 1.98 |
| 526 | `Media.Examples.ReferenceAir.Inverse_sh_T` | GM-U | S5 | GM-U0002 | ED008 | - | - | 0.56 |
| 527 | `Media.Examples.ReferenceAir.Inverse_sh_TX` | GM-U | S5 | GM-U0002 | ED008 | - | - | 1.63 |
| 528 | `Media.Examples.ReferenceAir.MoistAir` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.86 |
| 529 | `Media.Examples.ReferenceAir.MoistAir1` | GM-U | S5 | GM-U0002 | ED008 | - | - | 2.75 |
| 530 | `Media.Examples.ReferenceAir.MoistAir2` | GM-U | S5 | GM-U0001 | ED019 | - | - | 3.40 |
| 531 | `Media.Examples.SimpleLiquidWater` | OK | - | GM-I7001（情報） | - | 4 / 32 / 0・34 | 正則 34/34（der 31/34） | 0.49 |
| 532 | `Media.Examples.SolveOneNonlinearEquation.InverseIncompressible_sh_T` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.82 |
| 533 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sh_T` | GM-U | S5 | GM-U0002 | ED008 | - | - | 4.70 |
| 534 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sh_TX` | GM-U | S5 | GM-U0002 | ED008 | - | - | 2.93 |
| 535 | `Media.Examples.SolveOneNonlinearEquation.Inverse_sine` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.31 |
| 536 | `Media.Examples.TwoPhaseWater.BaseProperties` | OK | - | - | - | 0 / 14 / 0・16 | 正則 14/14 | 1.75 |
| 537 | `Media.Examples.TwoPhaseWater.ExtendedProperties` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.30 |
| 538 | `Media.Examples.TwoPhaseWater.TestTwoPhaseStates` | GM-U | S5 | GM-U0001 | ED019 | - | - | 1.35 |
| 539 | `Media.Examples.WaterIF97` | OK | - | GM-I7001（情報） | - | 5 / 26 / 0・34 | 正則 30/30（der 28/30） | 1.98 |
| 540 | `Media.Incompressible.Examples.TestGlycol` | GM-U | S5 | GM-U0001 | ED019 | - | - | 0.58 |
| 541 | `StateGraph.Examples.ControlledTanks` | OK | - | - | - | 2 / 46 / 21・319 | 正則 69/69 | 0.82 |
| 542 | `StateGraph.Examples.ExecutionPaths` | OK | - | - | - | 0 / 29 / 11・243 | 正則 40/40 | 0.37 |
| 543 | `StateGraph.Examples.FirstExample` | OK | - | - | - | 0 / 8 / 2・55 | 正則 10/10 | 0.20 |
| 544 | `StateGraph.Examples.FirstExample_Variant2` | OK | - | - | - | 0 / 9 / 3・60 | 正則 12/12 | 0.21 |
| 545 | `StateGraph.Examples.FirstExample_Variant3` | OK | - | - | - | 0 / 8 / 3・60 | 正則 11/11 | 0.21 |
| 546 | `StateGraph.Examples.ShowCompositeStep` | OK | - | - | - | 0 / 31 / 9・261 | 正則 40/40 | 0.34 |
| 547 | `StateGraph.Examples.ShowExceptions` | OK | - | - | - | 0 / 40 / 10・299 | 正則 50/50 | 0.34 |
| 548 | `Thermal.FluidHeatFlow.Examples.IndirectCooling` | OK | - | - | - | 4 / 132 / 33・169 | 正則 169/169 | 0.41 |
| 549 | `Thermal.FluidHeatFlow.Examples.OneMass` | OK | - | - | - | 2 / 59 / 18・79 | 正則 79/79 | 0.38 |
| 550 | `Thermal.FluidHeatFlow.Examples.ParallelCooling` | OK | - | - | - | 5 / 110 / 29・144 | 正則 144/144 | 0.48 |
| 551 | `Thermal.FluidHeatFlow.Examples.ParallelPumpDropOut` | OK | - | - | - | 5 / 112 / 32・149 | 正則 149/149 | 3.11 |
| 552 | `Thermal.FluidHeatFlow.Examples.PumpAndValve` | OK | - | GM-I7001（情報） | - | 4 / 85 / 22・111 | 正則 111/111（der 110/111） | 0.78 |
| 553 | `Thermal.FluidHeatFlow.Examples.PumpDropOut` | OK | - | - | - | 2 / 63 / 20・85 | 正則 85/85 | 0.52 |
| 554 | `Thermal.FluidHeatFlow.Examples.SimpleCooling` | OK | - | - | - | 2 / 61 / 17・80 | 正則 80/80 | 0.48 |
| 555 | `Thermal.FluidHeatFlow.Examples.TestCylinder` | OK | - | GM-I7001（情報） | - | 7 / 42 / 10・61 | 正則 59/59（der 57/59） | 0.52 |
| 556 | `Thermal.FluidHeatFlow.Examples.TestOpenTank` | OK | - | - | - | 2 / 29 / 14・47 | 正則 45/45 | 0.51 |
| 557 | `Thermal.FluidHeatFlow.Examples.TwoMass` | OK | - | - | - | 5 / 104 / 28・137 | 正則 137/137 | 0.61 |
| 558 | `Thermal.FluidHeatFlow.Examples.TwoTanks` | OK | - | - | - | 4 / 29 / 17・50 | 正則 50/50 | 0.46 |
| 559 | `Thermal.FluidHeatFlow.Examples.WaterPump` | OK | - | GM-I7001（情報） | - | 3 / 89 / 32・127 | 正則 124/124（der 122/124） | 0.51 |
| 560 | `Thermal.HeatTransfer.Examples.ControlledTemperature` | OK | - | - | - | 1 / 43 / 3・52 | 正則 47/47 | 0.37 |
| 561 | `Thermal.HeatTransfer.Examples.GenerationOfFMUs` | OK | - | GM-I7001（情報） | - | 5 / 72 / 25・102 | 正則 102/102（der 100/102） | 0.36 |
| 562 | `Thermal.HeatTransfer.Examples.Motor` | OK | - | - | - | 2 / 32 / 4・41 | 正則 39/39 | 0.44 |
| 563 | `Thermal.HeatTransfer.Examples.TwoMasses` | OK | - | - | - | 2 / 16 / 2・20 | 正則 20/20 | 0.30 |
| 564 | `Utilities.Examples.ReadRealMatrixFromFile` | GM-U | S4 | GM-U0002 | EF025 | - | - | 0.11 |
| 565 | `Utilities.Examples.WriteRealMatrixToFile` | OK | - | - | - | 0 / 0 / 0・4 | 正則 0/0 | 0.18 |
| 566 | `Utilities.Examples.readRealParameterModel` | GM-U | S4 | GM-U0002 | EF025 | - | - | 0.11 |
