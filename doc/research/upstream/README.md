# 上流（Rumoca・diffsol）への報告の下書き

S045（計画書の WP-H）で書いた（15 は S055、16 は S068、17〜21 は S070）、[Rumoca](https://github.com/CogniPilot/rumoca) と [diffsol](https://github.com/martinjrobins/diffsol) に出す issue の下書き。元の記録は [../msl-sim-failures.md](../msl-sim-failures.md) の「上流に報告する候補」と、[../../roadmap.md](../../roadmap.md) の候補 8。

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
| 15 | [BDF の補間が host の左端の検査に通らない（rtol 1e-7、1e-6 でも）](15-bdf-sampler-left-endpoint-tight-tolerance.md) | Rumoca | MSL `ThyristorBridge2Pulse_RLV_Characteristic` ほか 9 本（`--solver bdf --tolerance 1e-7`） | 一部（auto が 7/10 を dopri45 で救う） | 下書き |
| 16 | [`external "builtin"` が組み込み関数を呼ぶだけの関数を実行できない（MSL 4.0.0 の `Modelica.Math`）](16-external-builtin-math-functions.md) | Rumoca | [16](16-external-builtin-math-functions.mo) | あり（Dae を直列化の形で書き換える、ADR-0029） | 下書き |
| 17 | [`redeclare function extends product`（組み込みと同じ名前）を ER132 で断る](17-er132-redeclare-function-extends-product.md) — Rumoca の穴。267 本 | Rumoca | [17](17-er132-redeclare-function-extends-product.mo)（自己完結。`product` を `prod` に変えると通る） | なし | 下書き |
| 18 | [非 parameter の成分への `annotation(Evaluate = true)` を ER070 で断る](18-er070-evaluate-on-non-parameter-component.md) — Rumoca の穴（MLS §18.3 は誤りと言わない）。258 本 | Rumoca | [18](18-er070-evaluate-on-non-parameter-component.mo)（MSL の `Constant` だけ） | なし | 下書き |
| 19 | [`stateSelect` の parameter 式を EI032 で断る](19-ei032-statselect-parameter-expression.md) — Rumoca の穴（MLS §4.8.1 は parameter 属性）。235 本 | Rumoca | [19](19-ei032-statselect-parameter-expression.mo)（自己完結） | なし | 下書き |
| 20 | [extends が 2 つの関数（MSL `Files.loadResource`）の呼び出しを EF025 で断る](20-ef025-function-selection-identity.md) — Rumoca の穴（`loadResource` 分だけ。replaceable な Medium の関数は不明）。138 本 | Rumoca | [20](20-ef025-function-selection-identity.mo)（MSL だけ、自己完結の版も） | なし | 下書き（Medium の関数は要再現） |
| 21 | [`getInstanceName()` を parameter の束縛に書くと、成分配列を含む式の if 方程式で EF004](21-ef004-getinstancename-and-if-equation-branches.md) — どちらも Rumoca の穴（原因は 2 つ）。217 本 | Rumoca | [21](21-ef004-getinstancename-and-if-equation-branches.mo)（自己完結） | なし | 下書き |

## 公開ライブラリの失敗から（17〜21、S070）

S063 の失敗の分類（[../libraries-2026-11-01.md](../libraries-2026-11-01.md)）の上位 5 つの Rumoca のコードを、ライブラリを使わない最小の再現にし、MLS 3.6 の該当節と omc 1.27.2（`checkModel`）で「ライブラリの誤りか Rumoca の穴か」を判定した（確認日 2026-11-01）。**5 件とも Rumoca の穴で、ライブラリ側の誤りは無かった**（omc は全部通す）。再現は Rumoca v0.10.2 の CLI（`cargo xtask fetch-rumoca`）と `globe check` の両方で同じコードが出る。20 の Medium の関数の分は最小の再現を作れていない（本文の Notes）。これらの再現ファイルは同じフォルダの `.mo` を互いに読むので、モデル名は `Rep17`〜`Rep21` で始めて重ならないようにしてある。

## 下書きにしなかったもの

[msl-sim-failures.md](../msl-sim-failures.md) の候補のうち、次は再現の最小化か相手の特定がまだなので書いていない。

- 4（理想スイッチの射影の判定）、5（fixed = true の代数変数から状態を解かない）、7（RK の道で壁時計の予算が守られない）、8（関数呼び出しの Solve IR の契約）、9（同時イベントと clock。Rumoca の基準値にも載っている）、20（diffsol の Hermite の補間が硬い準静的な変数で外れる）。
- 公開の範囲の依頼（`admitted_native_execution_backend` を公開に、diffsol の `ClosureNoJac` を公開に）は 14 の Notes に 1 行だけ書いた。
