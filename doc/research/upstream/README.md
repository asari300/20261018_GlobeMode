# 上流（Rumoca・diffsol）への報告の下書き

S045（計画書の WP-H）で書いた（15 は S055、16 は S068、17〜21 は S070、24〜26 は S072。20・25 の追記と `20b`・`25b` の再現は S080）、[Rumoca](https://github.com/CogniPilot/rumoca) と [diffsol](https://github.com/martinjrobins/diffsol) に出す issue の下書き。元の記録は [../msl-sim-failures.md](../msl-sim-failures.md) の「上流に報告する候補」と、[../../roadmap.md](../../roadmap.md) の候補 8。

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
| 20 | [extends が 2 つの関数（MSL `Files.loadResource`）の呼び出しと、2 つの Medium の別名の関数を EF025 で断る](20-ef025-function-selection-identity.md) — どちらも Rumoca の穴。138 本（`loadResource` 136・Medium 2） | Rumoca | [20](20-ef025-function-selection-identity.mo)（MSL だけ、自己完結の版も）、[20b](20b-ef025-two-medium-aliases-modified-extends.mo)（自己完結。Medium が 1 つ・修正の無い extends・同じ package なら通る） | なし | 下書き |
| 21 | [`getInstanceName()` を parameter の束縛に書くと、成分配列を含む式の if 方程式で EF004](21-ef004-getinstancename-and-if-equation-branches.md) — どちらも Rumoca の穴（原因は 2 つ）。217 本 | Rumoca | [21](21-ef004-getinstancename-and-if-equation-branches.mo)（自己完結） | なし | 下書き |
| 24 | [条件付き成分の条件が別の package の定数を読むと EI006 で断る](24-ei006-condition-on-package-constant.md) — Rumoca の穴（MLS §4.4.5・§3.8）。117 本 | Rumoca | [24](24-ei006-condition-on-package-constant.mo)（自己完結。モデルの定数・parameter なら通る） | なし | 下書き |
| 25 | [Boolean の出力の関数で、if の枝が代入のあと for を回すと ED019 `function conditional`](25-ed019-function-conditional-boolean-loop.md) — Rumoca の未対応（MLS §11.2・§12.2 は制限しない）。ED019 125 本のうち同じメッセージ 44 本。追記: 入力の次元が package の定数で、その定数を extends の修正で変えると ED019 `function shape proof`（MSL 4.0.0 の `PartialSimpleMedium`、`SensorVerification`） | Rumoca | [25](25-ed019-function-conditional-boolean-loop.mo)（自己完結。Real の出力なら通る）、[25b](25b-ed019-function-shape-proof-modified-package-constant.mo)（自己完結。修正の無い extends・`x[:]` なら通る） | なし | 下書き（`splineDerivatives` は要再現） |
| 26 | [package の record 定数の成分を読むと ED008 unresolved Flat reference](26-ed008-field-of-package-record-constant.md) — Rumoca の内部エラー。ED008 108 本のうち `Media.Air.dryair.cp` 47 本 | Rumoca | [26](26-ed008-field-of-package-record-constant.mo)（自己完結。スカラの定数・モデルの record 定数なら通る） | なし | 下書き（`i` の 18 本は要再現） |

## 公開ライブラリの失敗から（17〜21、S070）

S063 の失敗の分類（[../libraries-2026-11-01.md](../libraries-2026-11-01.md)）の上位 5 つの Rumoca のコードを、ライブラリを使わない最小の再現にし、MLS 3.6 の該当節と omc 1.27.2（`checkModel`）で「ライブラリの誤りか Rumoca の穴か」を判定した（確認日 2026-11-01）。**5 件とも Rumoca の穴で、ライブラリ側の誤りは無かった**（omc は全部通す）。再現は Rumoca v0.10.2 の CLI（`cargo xtask fetch-rumoca`）と `globe check` の両方で同じコードが出る。20 の Medium の関数の分は S080 で最小の再現 [20b](20b-ef025-two-medium-aliases-modified-extends.mo) を作った。これらの再現ファイルは同じフォルダの `.mo` を互いに読むので、モデル名は `Rep17`〜`Rep21` で始めて重ならないようにしてある。

## 公開ライブラリの失敗から（22〜26、S072）

S070 と同じ手順で、S070 の「次回」に挙げた 5 つのコード（ER023・ER003・EI006・ED019・ED008）を判定した（確認日 2026-11-02）。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出る（対照のモデルは両方で通る）。omc 1.27.2 の `checkModel` は ER003 の再現だけを断り、ほかは通す。モデル名は `Rep22`〜`Rep26` で始める。

| # | コード | 判定 | 根拠 |
| ---: | --- | --- | --- |
| 22 | ER023（ClaRa 3 本） | **正当**（MLS の規定上の拒否）。下書きは書かない | MLS 3.6 §4.6「record に直接宣言する成分は record か type に限る」。`class … extends ExternalObject`（TILMedia）は class。omc は緩く通す。Rumoca のメッセージの節番号 §4.7 は 3.6 では §4.6（Balanced Models が §4.7）。再現は [22](22-er023-record-component-externalobject.mo) |
| 23 | ER003（ClaRa 2・AixLib 1） | **正当**（無い基底クラス）。ClaRa の 2 本は依存の取得で避けられる（GlobeMode 側の回避。SMArtInt は取得していない）、AixLib の 1 本はライブラリの誤り。下書きは書かない | ClaRa は取得しない依存 `SMArtInt.Internal.ClaRaDelay`。AixLib は `Modelica.Media.Examples.Tests.Components.PartialTestModel2` を引くが、MSL 4.0.0 にも 4.1.0 にも無い（`Modelica.Media.Examples.Utilities.PartialTestModel2` にある）。omc も同じ再現を断る。再現は [23](23-er003-base-class-not-found.mo) |
| 24 | EI006 | **Rumoca の穴** | 別の package の定数を読む条件は parameter 式（MLS §4.4.5・§3.8.2・§3.8.3）。omc は通す |
| 25 | ED019 | **Rumoca の未対応**（`function conditional`。代表の形だけ） | 文の組み合わせに MLS の制限は無い。omc は通す。`function shape proof` は S080 で最小の再現 [25b](25b-ed019-function-shape-proof-modified-package-constant.mo) を作った（MSL 4.0.0 の `SensorVerification`。MSL 4.1.0 で出るほかの 12 本が同じ形かは【未確認】） |
| 26 | ED008 | **Rumoca の穴**（内部エラー） | package の定数は名前で引ける（MLS §5.3.2）。omc は通す |

## 残りの 2 つの形（20b・25b、S080）

S070・S072 で【未確認】だった 2 つを、ライブラリの例題から MSL の該当の関数を写して削り、自己完結の再現にした（確認日 2026-11-02）。新しい番号の下書きは作らず、20 と 25 に「Minimal reproduction」の節を足した。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出て、対照のモデルは両方で通る。omc 1.27.2 の `checkModel` は全部通す。モデル名は `Rep27`（25b）と `Rep28`（20b）で始める。

| 再現 | コード | 判定 | 消える条件 |
| --- | --- | --- | --- |
| [25b](25b-ed019-function-shape-proof-modified-package-constant.mo) | ED019 `function shape proof` | **Rumoca の穴**（MLS §10.1・§7.2） | 関数の入力 `x[n]` を `x[:]` にする、または package の `extends P(n = 2)` の修正を外す。MSL 4.0.0 の `PartialSimpleMedium.specificEnthalpy_pTX`（`X[nX]`）が原因で、4.1.0 は `X[:]` |
| [20b](20b-ef025-two-medium-aliases-modified-extends.mo) | EF025 `multiple exact override package selections` | **Rumoca の穴**（MLS §5.3） | 同じモデルの Medium の別名を 1 つにする、extends の修正を外す、または同じ package を 2 回指す |

## 下書きにしなかったもの

[msl-sim-failures.md](../msl-sim-failures.md) の候補のうち、次は再現の最小化か相手の特定がまだなので書いていない。

- 4（理想スイッチの射影の判定）、5（fixed = true の代数変数から状態を解かない）、7（RK の道で壁時計の予算が守られない）、8（関数呼び出しの Solve IR の契約）、9（同時イベントと clock。Rumoca の基準値にも載っている）、20（diffsol の Hermite の補間が硬い準静的な変数で外れる）。
- 公開の範囲の依頼（`admitted_native_execution_backend` を公開に、diffsol の `ClosureNoJac` を公開に）は 14 の Notes に 1 行だけ書いた。
