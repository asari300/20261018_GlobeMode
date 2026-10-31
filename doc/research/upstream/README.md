# 上流（Rumoca・diffsol）への報告の下書き

S045（計画書の WP-H）で書いた、[Rumoca](https://github.com/CogniPilot/rumoca) と [diffsol](https://github.com/martinjrobins/diffsol) に出す issue の下書き。元の記録は [../msl-sim-failures.md](../msl-sim-failures.md) の「上流に報告する候補」と、[../../roadmap.md](../../roadmap.md) の候補 8。

- **出すのはユーザー。エージェントは出さない**（`gh issue create` を使わない）。
- **出すときは、再現を Rumoca の最新（main）で確かめてから出す。** 下書きの環境は Rumoca v0.10.2（rev `276e82bd`）、MSL 4.1.0。Rumoca の CLI がこの環境に無いまま書いた件は、本文に「[not verified]」と書いてある。
- 本文は英語で、issue にそのまま貼れる形（Summary / Steps to reproduce / Expected / Actual / Environment / Workaround in GlobeMode / Notes）。
- 再現ファイル（`.mo`）は、MSL の例題で再現する件では置かない（モデル名とコマンドを本文に書く）。`src/Modelica/` には置かない（MODELICAPATH として開けるパッケージだけを置く規則）。
- diffsol の件（10〜12）は Rumoca が同梱する 0.13.2 での観測。diffsol 本家で同じかは確かめていない（本文に "not verified against upstream diffsol main"）。単体の diffsol のプログラムに縮めてから出す。
- 状態は「下書き」のまま。出したら issue の URL と日付をこの表に足す。

| # | 件名 | 対象 | 再現 | GlobeMode 側の回避 | 状態 |
| ---: | --- | --- | --- | --- | --- |
| 01 | [インデックス 2 以上の DAE で EL005](01-el005-index2-structurally-singular.md) | Rumoca | MSL `FilterWithDifferentiation`（CLI） | なし | 下書き |
| 02 | [サンプラの端点の判定に丸めの床が無い](02-sampler-endpoint-roundoff.md) | Rumoca | MSL `SimpleTriacCircuit` ほか | あり（許容誤差の下限、auto の切り替え） | 下書き |
| 03 | [Solve IR の変換と実行時オブジェクトの組み立てが終わらない](03-solve-runtime-construction-does-not-finish.md) | Rumoca | MSL `DrumBoiler`（CLI） | なし | 下書き |
| 04 | [初期化方程式を関係式の既定値で解き、解き直さない](04-init-equations-solved-with-relation-default.md) | Rumoca | [04](04-init-equations-solved-with-relation-default.mo) | 検出だけ（GM-W10004） | 下書き |
| 05 | [実行時のパラメータ上書きが同名の修飾の束縛に届かない](05-parameter-override-not-reaching-inlined-bindings.md) | Rumoca | [05](05-parameter-override-not-reaching-inlined-bindings.mo) | あり（構造的な上書き、ADR-0022） | 下書き |
| 06 | [構造的な上書き: コンパイル後の ED020、名前の誤りの黙殺、スカラだけ](06-structural-overrides-ed020-and-scalar-only.md) | Rumoca | [06](06-structural-overrides-ed020-and-scalar-only.mo) | あり（複製した session、名前の形で断る） | 下書き |
| 07 | [outer の参照が inner の修正でなく型の既定を読む](07-outer-reference-ignores-inner-modification.md) | Rumoca | [07](07-outer-reference-ignores-inner-modification.mo) | なし（既知の限界として記録） | 下書き |
| 08 | [部品への修正が中の同名の変数に漏れる](08-modifier-leaks-into-subcomponent.md) | Rumoca | [08](08-modifier-leaks-into-subcomponent.mo) | なし（既知の限界として記録） | 下書き |
| 09 | [試しの点の射影の失敗を復帰不能として記憶する](09-host-projection-failure-is-unrecoverable.md) | Rumoca | MSL `SimpleTriacCircuit`（dopri45） | 一部（bdf・esdirk34 を勧める） | 下書き |
| 10 | [SDIRK が Newton 法の失敗を積分全体で累積して数える](10-diffsol-sdirk-newton-failures-accumulate.md) | diffsol | [10](10-diffsol-sdirk-newton-failures-accumulate.mo) | あり（上限を外す） | 下書き |
| 11 | [SDIRK がヤコビ行列を状態の点で取る](11-diffsol-sdirk-jacobian-at-wrong-point.md) | diffsol | MSL `OvervoltageProtection`（esdirk34） | あり（受理した状態で取る） | 下書き |
| 12 | [最初の刻みの見積もりが min_timestep を下回る](12-diffsol-initial-step-below-min-timestep.md) | diffsol | [12](12-diffsol-initial-step-below-min-timestep.mo) | あり（下限の 10 倍に持ち上げる） | 下書き |
| 13 | [RK で黙って誤った結果になる](13-rk-silently-wrong-result-generationoffmus.md) | Rumoca | MSL `GenerationOfFMUs` | あり（auto は RK に切り替えない） | 下書き |
| 14 | [wasm32 で型付きの道が `Instant::now()` で panic](14-wasm32-instant-now-in-typed-route.md) | Rumoca | 本文の手順 | あり（host の道） | 下書き |

## 下書きにしなかったもの

[msl-sim-failures.md](../msl-sim-failures.md) の候補のうち、次は再現の最小化か相手の特定がまだなので書いていない。

- 4（理想スイッチの射影の判定）、5（fixed = true の代数変数から状態を解かない）、7（RK の道で壁時計の予算が守られない）、8（関数呼び出しの Solve IR の契約）、9（同時イベントと clock。Rumoca の基準値にも載っている）、20（diffsol の Hermite の補間が硬い準静的な変数で外れる）。
- 公開の範囲の依頼（`admitted_native_execution_backend` を公開に、diffsol の `ClosureNoJac` を公開に）は 14 の Notes に 1 行だけ書いた。
