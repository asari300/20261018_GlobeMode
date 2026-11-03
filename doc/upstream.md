# 上流（Rumoca・diffsol）への報告のまとめ

> **English summary.** GlobeMode keeps 38 draft issues for upstream projects (Rumoca, diffsol as vendored by Rumoca, and one for AixLib; 5 of them are fixed in Rumoca v0.10.4 and kept only as records) in [research/upstream/](research/upstream/README.md). None has been filed yet. This page lists them in one table (target, diagnostic code, symptom, minimal `.mo`, impact counts, GlobeMode's workaround, state), gives the filing order by impact (the top five are 17 ER132, 18 ER070, 19 EI032, 21 EF004, 24 EI006, each over 110 library examples), Japanese versions with a one-slide explainer for every draft that still reproduces on Rumoca v0.10.4 (17 to 33; `*_jp.md`, `*_jp.html`, S113 and S115), the re-check of 17 to 32 on v0.10.4 (25, 25b, 25c, 25g and 26 are fixed and dropped from the order; 20, 25e and 28 are partly fixed; the rest still reproduce), the steps to file (re-check on Rumoca main, file, write the issue number here), and when GlobeMode may drop each workaround. The user files the issues; agents do not.

この文書は上流への報告の正典。下書きの本文（英語、issue にそのまま貼れる形）と、作った経緯は [research/upstream/README.md](research/upstream/README.md) にある。表の数字は下書きから写した（確認日 2026-11-02。31・32 と 28 の追記は 2026-11-03）。28〜32 は 2026-11-03（S113）に、17〜27 は同じ日（S115）に Rumoca の最新のリリース v0.10.4 で確かめ直し、再現する件に日本語版（`_jp.md`）と 1 枚の解説（`_jp.html`）を足した。33 は AixLib 向け（S113）。

- **出すのはユーザー。エージェントは出さない**（`gh issue create` を使わない）。
- 下書きの環境は Rumoca v0.10.2（rev `276e82bd`）、MSL 4.1.0。17〜33 は v0.10.4（タグのコミット `9c8c56fc`、2026-11-02 公開、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）でも確かめた。Rumoca の main（`91d6e936`、v0.10.4 の 10 コミット後、確認日 2026-11-03）はビルドしていない【未確認】。Rumoca: <https://github.com/CogniPilot/rumoca>、diffsol: <https://github.com/martinjrobins/diffsol>（確認日 2026-11-02）。
- 影響の本数の出典:
  - 「MSL」は GlobeMode の MSL 4.1.0 の検査（S011 以降、[msl-sim.md](msl-sim.md)・[research/msl-sim-failures.md](research/msl-sim-failures.md)）。
  - 「ライブラリ」は公開ライブラリ 7 つの検査。17〜26 は [run 41250636800](https://github.com/asari300/20261018_GlobeMode/actions/runs/41250636800)（S063、失敗 1,781 本、[research/libraries-2026-11-01.md](research/libraries-2026-11-01.md)）、25b〜25h・27〜32 は [run 41365960542](https://github.com/asari300/20261018_GlobeMode/actions/runs/41365960542)（S083・S091・S108）、16 は S067 の MSL 4.0.0 の回し直し。
  - 1 つの例題が 2 つの下書きに数えられることがある（25 の 44 本は 25g の `splineDerivatives` を含む）。本数の和は失敗の本数にならない。
- 22（ER023）と 23（ER003）は正当な拒否と判定したので下書きは無い（[research/upstream/README.md](research/upstream/README.md) の S072 の節）。

## 一覧

状態:

- **未提出**: Rumoca v0.10.2 の CLI（または自己完結の `.mo`）で再現を確かめてある。出す前に Rumoca main で確かめる（下の手順の 1）のは同じ。
- **未提出（v0.10.4 で再現）**: 最新のリリース v0.10.4 でも再現する（S113・S115）。main は【未確認】。
- **一部は v0.10.4 で直った**: 再現のモデルの一部が v0.10.4 で通る。残りの形だけを出す。
- **v0.10.4 で修正**: 再現のモデルがすべて v0.10.4 で通る（S115）。出さない。下書きは記録として残す。GlobeMode の Rumoca の rev を上げたら、ライブラリの例題で本当に消えるかを確かめる。
- **Rumoca main で再確認が要る**: 再現を Rumoca のライブラリの API だけで見た、CLI での形が【未確認】（本文に `[not verified]`）、または diffsol 本家で確かめていない（単体の diffsol のプログラムに縮めてから出す）。
- **提出済み**: issue の番号と日付を書く（今は 0 件）。

| # | 対象 | 診断コード | 症状 | 最小の再現 | 影響の本数 | GlobeMode 側の回避 | 状態 | 下書き |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 01 | Rumoca | EL005（GM-U0004） | インデックス 2 以上の DAE を構造的に特異として断る | MSL `FilterWithDifferentiation`（CLI） | MSL 18（S011） | なし | 未提出 | [01](research/upstream/01-el005-index2-structurally-singular.md) |
| 02 | Rumoca | （契約の誤り） | サンプラの端点の判定に丸めの床が無い | MSL `SimpleTriacCircuit` ほか（API） | MSL 2 | あり（S012。GM-W10003、auto の切り替え GM-I11002） | Rumoca main で再確認が要る | [02](research/upstream/02-sampler-endpoint-roundoff.md) |
| 03 | Rumoca | （終わらない） | Solve IR の変換と実行時オブジェクトの組み立てが終わらない | MSL `DrumBoiler`（CLI） | MSL 2 | なし（ADR-0018 で前もって止めない） | 未提出 | [03](research/upstream/03-solve-runtime-construction-does-not-finish.md) |
| 04 | Rumoca | （GM-W10004） | 初期化方程式を関係式の既定値で解き、解き直さない | [04](research/upstream/04-init-equations-solved-with-relation-default.mo) | MSL 1 以上（`EngineV6`）【本数未確認】 | 検出だけ（ADR-0020、S018） | Rumoca main で再確認が要る | [04](research/upstream/04-init-equations-solved-with-relation-default.md) |
| 05 | Rumoca | （API） | 実行時のパラメータ上書きが同名の修飾の束縛に届かない | [05](research/upstream/05-parameter-override-not-reaching-inlined-bindings.mo) | MSL の `SineVoltage`・`StepVoltage` を使う例題【本数未確認】 | あり（ADR-0022、S024） | Rumoca main で再確認が要る | [05](research/upstream/05-parameter-override-not-reaching-inlined-bindings.md) |
| 06 | Rumoca | ED020（API） | 構造的な上書き: コンパイル後の ED020、名前の誤りの黙殺、スカラだけ | [06](research/upstream/06-structural-overrides-ed020-and-scalar-only.mo) | API の制限（例題の本数でない） | あり（ADR-0022、S025。GM-E10006） | Rumoca main で再確認が要る | [06](research/upstream/06-structural-overrides-ed020-and-scalar-only.md) |
| 07 | Rumoca | （GM-W8001） | outer の参照が inner の修正でなく型の既定を読む | [07](research/upstream/07-outer-reference-ignores-inner-modification.mo) | MSL Fluid の `FixedInitial` の例題【本数未確認】 | なし（既知の限界。S009） | Rumoca main で再確認が要る | [07](research/upstream/07-outer-reference-ignores-inner-modification.md) |
| 08 | Rumoca | — | 部品への修正が中の同名の変数に漏れる | [08](research/upstream/08-modifier-leaks-into-subcomponent.mo) | MSL `RollingWheelSet` の形【本数未確認】 | なし（既知の限界。S009） | Rumoca main で再確認が要る | [08](research/upstream/08-modifier-leaks-into-subcomponent.md) |
| 09 | Rumoca | （射影の失敗） | 試しの点の射影の失敗を復帰不能として記憶する | MSL `SimpleTriacCircuit`（dopri45） | MSL 2 以上 | 一部（bdf・esdirk34 を勧める。S021〜S024） | Rumoca main で再確認が要る | [09](research/upstream/09-host-projection-failure-is-unrecoverable.md) |
| 10 | diffsol | — | SDIRK が Newton 法の失敗を積分全体で累積して数える | [10](research/upstream/10-diffsol-sdirk-newton-failures-accumulate.mo) | MSL 7（esdirk34） | あり（S021） | Rumoca main で再確認が要る | [10](research/upstream/10-diffsol-sdirk-newton-failures-accumulate.md) |
| 11 | diffsol | — | SDIRK がヤコビ行列を状態の点で取る | MSL `OvervoltageProtection`（esdirk34） | MSL 1 以上 | あり（S022） | Rumoca main で再確認が要る | [11](research/upstream/11-diffsol-sdirk-jacobian-at-wrong-point.md) |
| 12 | diffsol | — | 最初の刻みの見積もりが min_timestep を下回る | [12](research/upstream/12-diffsol-initial-step-below-min-timestep.mo) | MSL 1 以上（`CauerLowPassSC`） | あり（S023） | Rumoca main で再確認が要る | [12](research/upstream/12-diffsol-initial-step-below-min-timestep.md) |
| 13 | Rumoca | （GM-U0004） | RK で黙って誤った結果になる | MSL `GenerationOfFMUs` | MSL 1 | あり（auto は RK に切り替えない。S013） | Rumoca main で再確認が要る | [13](research/upstream/13-rk-silently-wrong-result-generationoffmus.md) |
| 14 | Rumoca | （panic） | wasm32 で型付きの道が `Instant::now()` で panic | 本文の手順 | wasm の全体（例題の本数でない） | あり（host の道。ADR-0014、S012） | Rumoca main で再確認が要る | [14](research/upstream/14-wasm32-instant-now-in-typed-route.md) |
| 15 | Rumoca | （契約の誤り） | BDF の補間が host の左端の検査に通らない（rtol 1e-7） | MSL `ThyristorBridge2Pulse_RLV_Characteristic` ほか（API） | MSL 9 | 一部（auto が 7/10 を救う。ADR-0015、S050） | Rumoca main で再確認が要る | [15](research/upstream/15-bdf-sampler-left-endpoint-tight-tolerance.md) |
| 16 | Rumoca | EL005・EX002 | `external "builtin"` の組み込み関数を実行できない | [16](research/upstream/16-external-builtin-math-functions.mo) | ライブラリ 70（MSL 4.0.0。S067） | あり（ADR-0029、S068） | Rumoca main で再確認が要る | [16](research/upstream/16-external-builtin-math-functions.md) |
| 17 | Rumoca | ER132 | `redeclare function extends product` を断る | [17](research/upstream/17-er132-redeclare-function-extends-product.mo) | ライブラリ 267 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [17](research/upstream/17-er132-redeclare-function-extends-product.md)（[日本語](research/upstream/17-er132-redeclare-function-extends-product_jp.md)・[1 枚](research/upstream/17-er132-redeclare-function-extends-product_jp.html)） |
| 18 | Rumoca | ER070 | 非 parameter の成分への `Evaluate = true` を断る | [18](research/upstream/18-er070-evaluate-on-non-parameter-component.mo) | ライブラリ 258 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [18](research/upstream/18-er070-evaluate-on-non-parameter-component.md)（[日本語](research/upstream/18-er070-evaluate-on-non-parameter-component_jp.md)・[1 枚](research/upstream/18-er070-evaluate-on-non-parameter-component_jp.html)） |
| 19 | Rumoca | EI032 | `stateSelect` の parameter 式を断る | [19](research/upstream/19-ei032-statselect-parameter-expression.mo) | ライブラリ 235 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [19](research/upstream/19-ei032-statselect-parameter-expression.md)（[日本語](research/upstream/19-ei032-statselect-parameter-expression_jp.md)・[1 枚](research/upstream/19-ei032-statselect-parameter-expression_jp.html)） |
| 20 | Rumoca | EF025 | extends が 2 つの関数（`loadResource`）・2 つの Medium の別名 | [20](research/upstream/20-ef025-function-selection-identity.mo)、[20b](research/upstream/20b-ef025-two-medium-aliases-modified-extends.mo) | ライブラリ 138（`loadResource` 136・Medium 2） | なし | 一部は v0.10.4 で直った（`loadResource` の形。20b の 2 つの Medium の形は再現。2026-11-03） | [20](research/upstream/20-ef025-function-selection-identity.md)（[日本語](research/upstream/20-ef025-function-selection-identity_jp.md)・[1 枚](research/upstream/20-ef025-function-selection-identity_jp.html)） |
| 21 | Rumoca | EF004 | `getInstanceName()` の束縛、成分配列の if 方程式 | [21](research/upstream/21-ef004-getinstancename-and-if-equation-branches.mo) | ライブラリ 217（A 119・B 41 ほか） | なし | 未提出（v0.10.4 で A・B とも再現、2026-11-03） | [21](research/upstream/21-ef004-getinstancename-and-if-equation-branches.md)（[日本語](research/upstream/21-ef004-getinstancename-and-if-equation-branches_jp.md)・[1 枚](research/upstream/21-ef004-getinstancename-and-if-equation-branches_jp.html)） |
| 24 | Rumoca | EI006 | 条件付き成分の条件が別の package の定数を読む | [24](research/upstream/24-ei006-condition-on-package-constant.mo) | ライブラリ 117 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [24](research/upstream/24-ei006-condition-on-package-constant.md)（[日本語](research/upstream/24-ei006-condition-on-package-constant_jp.md)・[1 枚](research/upstream/24-ei006-condition-on-package-constant_jp.html)） |
| 25 | Rumoca | ED019 | Boolean の出力の関数で、if の枝が代入のあと for（`function conditional`） | [25](research/upstream/25-ed019-function-conditional-boolean-loop.mo) | ライブラリ 44（S063。25g と重なる） | なし | v0.10.4 で修正（2026-11-03） | [25](research/upstream/25-ed019-function-conditional-boolean-loop.md) |
| 25b | Rumoca | ED019 | 入力の次元の package 定数を extends の修正で変える | [25b](research/upstream/25b-ed019-function-shape-proof-modified-package-constant.mo) | ライブラリ 1（4.1.0）、MSL 4.0.0 では `ConstantPropertyLiquidWater` を使う例題すべて | なし | v0.10.4 で修正（2026-11-03） | [25b](research/upstream/25b-ed019-function-shape-proof-modified-package-constant.md) |
| 25c | Rumoca | ED019 | 呼び出しで大きさの決まる定数の `size` を `Integer` の局所変数に | [25c](research/upstream/25c-ed019-function-shape-proof-size-of-call-sized-constant.mo) | ライブラリ 2・MSL 2 | なし | v0.10.4 で修正（2026-11-03） | [25c](research/upstream/25c-ed019-function-shape-proof-size-of-call-sized-constant.md) |
| 25d | Rumoca | ED019 | 配列の入力が後で宣言する `Integer` の入力で次元を取る | [25d](research/upstream/25d-ed019-function-shape-proof-dimension-before-input.mo) | ライブラリ 3 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [25d](research/upstream/25d-ed019-function-shape-proof-dimension-before-input.md)（[日本語](research/upstream/25d-ed019-function-shape-proof-dimension-before-input_jp.md)・[1 枚](research/upstream/25d-ed019-function-shape-proof-dimension-before-input_jp.html)） |
| 25e | Rumoca | ED019 | record の成分を次元にした record を関数の局所変数に | [25e](research/upstream/25e-ed019-function-shape-proof-record-local-dimension.mo) | ライブラリ 6・MSL 2 | なし | 一部は v0.10.4 で直った（`MC` の constant の形。`M` の parameter の形は再現。2026-11-03） | [25e](research/upstream/25e-ed019-function-shape-proof-record-local-dimension.md)（[日本語](research/upstream/25e-ed019-function-shape-proof-record-local-dimension_jp.md)・[1 枚](research/upstream/25e-ed019-function-shape-proof-record-local-dimension_jp.html)） |
| 25f | Rumoca | ED019 | `b[:]` の束縛が後で宣言する定数を読む | [25f](research/upstream/25f-ed019-function-shape-proof-binding-reads-later-constant.mo) | ライブラリ 3 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [25f](research/upstream/25f-ed019-function-shape-proof-binding-reads-later-constant.md)（[日本語](research/upstream/25f-ed019-function-shape-proof-binding-reads-later-constant_jp.md)・[1 枚](research/upstream/25f-ed019-function-shape-proof-binding-reads-later-constant_jp.html)） |
| 25g | Rumoca | ED019 | if の枝が for だけ（`splineDerivatives`） | [25g](research/upstream/25g-ed019-function-conditional-loop-only-branch.mo) | ライブラリ 25 | なし | v0.10.4 で修正（2026-11-03） | [25g](research/upstream/25g-ed019-function-conditional-loop-only-branch.md) |
| 25h | Rumoca | ED019 | `Real[:]` の局所変数を呼び出しの代入だけで大きさを決める | [25h](research/upstream/25h-ed019-function-shape-proof-flexible-array-from-call.mo) | ライブラリ 2・MSL 2 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [25h](research/upstream/25h-ed019-function-shape-proof-flexible-array-from-call.md)（[日本語](research/upstream/25h-ed019-function-shape-proof-flexible-array-from-call_jp.md)・[1 枚](research/upstream/25h-ed019-function-shape-proof-flexible-array-from-call_jp.html)） |
| 26 | Rumoca | ED008 | package の record 定数の成分を読む | [26](research/upstream/26-ed008-field-of-package-record-constant.mo) | ライブラリ 47（ED008 108 のうち） | なし | v0.10.4 で修正（2026-11-03） | [26](research/upstream/26-ed008-field-of-package-record-constant.md) |
| 27 | Rumoca | ER002 | extends で継いだ入れ子のクラスを継いだ package の名前で引く | [27](research/upstream/27-er002-inherited-class-through-extending-package.mo) | ライブラリ 0（S091） | なし | 未提出（v0.10.4 で再現、2026-11-03） | [27](research/upstream/27-er002-inherited-class-through-extending-package.md)（[日本語](research/upstream/27-er002-inherited-class-through-extending-package_jp.md)・[1 枚](research/upstream/27-er002-inherited-class-through-extending-package_jp.html)） |
| 28 | Rumoca | ET004・EF024 | 配列の次元が replaceable な package を通して定数を読む（既定のまま・モデルの中の record）。別の package の定数を直接読む形は v0.10.4 で直った | [28](research/upstream/28-et004-ef024-dimension-from-package-constant.mo) | ライブラリ 4（ET004 3・EF024 1）。v0.10.4 で残る形は 3【未確認】 | なし | 一部は v0.10.4 で直った（`M`・`M2`。`M1`・`M3`・`M5` は再現。2026-11-03） | [28](research/upstream/28-et004-ef024-dimension-from-package-constant.md)（[日本語](research/upstream/28-et004-ef024-dimension-from-package-constant_jp.md)・[1 枚](research/upstream/28-et004-ef024-dimension-from-package-constant_jp.html)） |
| 29 | Rumoca | EF024 | 関数の protected の束縛が replaceable な package の関数を呼ぶ | [29](research/upstream/29-ef024-local-function-binding-calls-replaceable-package.mo) | ライブラリ 18 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [29](research/upstream/29-ef024-local-function-binding-calls-replaceable-package.md)（[日本語](research/upstream/29-ef024-local-function-binding-calls-replaceable-package_jp.md)・[1 枚](research/upstream/29-ef024-local-function-binding-calls-replaceable-package_jp.html)） |
| 30 | Rumoca | ET004 | record の `eta[size(y, 1)]` を関数の呼び出しで束縛 | [30](research/upstream/30-et004-record-size-dimension-bound-to-call.mo) | ライブラリ 15 | なし | 未提出（v0.10.4 で再現、2026-11-03） | [30](research/upstream/30-et004-record-size-dimension-bound-to-call.md)（[日本語](research/upstream/30-et004-record-size-dimension-bound-to-call_jp.md)・[1 枚](research/upstream/30-et004-record-size-dimension-bound-to-call_jp.html)） |
| 31 | Rumoca | EF024 | 束縛が replaceable な record（成分・クラス）を通して成分を読む | [31](research/upstream/31-ef024-binding-reads-field-of-replaceable-record.mo) | ライブラリ 8（`nomVal` 7・`T` 1） | なし | 未提出（v0.10.4 で再現、2026-11-03） | [31](research/upstream/31-ef024-binding-reads-field-of-replaceable-record.md)（[日本語](research/upstream/31-ef024-binding-reads-field-of-replaceable-record_jp.md)・[1 枚](research/upstream/31-ef024-binding-reads-field-of-replaceable-record_jp.html)） |
| 32 | Rumoca | ER002 | モデルが使わない別の派生クラスの Medium の、壊れた関数まで名前を解決する | [32](research/upstream/32-er002-unused-function-of-other-subclass-medium.mo) | ライブラリ 39（AixLib。全部読みのときだけ） | 一部（`--library` は閉包だけ読む。ADR-0025） | 未提出（v0.10.4 で再現、2026-11-03） | [32](research/upstream/32-er002-unused-function-of-other-subclass-medium.md)（[日本語](research/upstream/32-er002-unused-function-of-other-subclass-medium_jp.md)・[1 枚](research/upstream/32-er002-unused-function-of-other-subclass-medium_jp.html)） |
| 33 | **AixLib** | （ER002） | `CHPCombustionMixtureGasNasa.T_psX` が宣言の無い `specificEntropyOfpTX` を呼ぶ（MSL から protected の補助関数を写していない）。AixLib main（`e5df9a8d` = v3.0.1）でも同じ | [33](research/upstream/33-aixlib-specific-entropy-of-ptx-missing.mo) | omc・Dymola では例題 0（呼ぶモデルが無い）。Rumoca では 32 を通して 39 | 対象外（ライブラリの誤り） | 未提出（AixLib main で再現、2026-11-03） | [33](research/upstream/33-aixlib-specific-entropy-of-ptx-missing.md)（[日本語](research/upstream/33-aixlib-specific-entropy-of-ptx-missing_jp.md)・[1 枚](research/upstream/33-aixlib-specific-entropy-of-ptx-missing_jp.html)） |

## 提出の順（影響の本数順）

本数の多い順に出す。v0.10.4 で直った件（S115）は出さない。GlobeMode 側に回避のある件（02・05・06・10〜16）は、本数が多くても急がない。

| 順 | # | 本数 | 理由 |
| ---: | --- | ---: | --- |
| 1 | 17 ER132 | 267 | IBPSA の `OnePhase` を使う IBPSA・Buildings・AixLib の例題すべて |
| 2 | 18 ER070 | 258 | `LimPID` を使う例題すべて |
| 3 | 19 EI032 | 235 | Buildings・IBPSA・AixLib・DeltaRobot |
| 4 | 21 EF004 | 217 | 原因が 2 つ（`getInstanceName()` 119、Chemical の if 方程式 41）。分けて出してもよい |
| 5 | 24 EI006 | 117 | S115 で 20 が下がったので 6 位から上がった |
| 6 | 16 EL005・EX002 | 70 | MSL 4.0.0 だけ。GlobeMode に回避あり |
| 7 | 32 ER002 | 39 | AixLib の全部読みのときだけ（`globe check` は閉包だけ読むので出ない）。呼ばれる関数そのものは AixLib の誤り（S108。AixLib 向けは 33） |
| 8 | 01 EL005 | MSL 18 | 指数の低減の予定を尋ねる |
| 9 | 29 EF024 | 18 | |
| 10 | 30 ET004 | 15 | |
| 11 | 31 EF024 | 8 | 29 と同じく関数・record の束縛の形。続けて出してもよい（S108） |
| 12 | 25e・28・25d・25f・25h | 6・4（v0.10.4 では 3【未確認】）・3・3・2 | ED019 の `function shape proof` は 1 つの issue にまとめてもよい。28 は ET004・EF024 で、v0.10.4 で残る `M1`・`M3`・`M5` に絞る（S113）。25e は v0.10.4 で残る `parameter` の形に絞る（S115） |
| 13 | 20 EF025（20b） | 2 | v0.10.4 で `loadResource` の 136 本の形は直り、2 つの Medium の形だけが残る（S115。5 位から下げた） |
| 14 | 残り（02〜15） | MSL 1〜9 | 実行系。回避のあるものは後に。10〜12 は diffsol に出す |
| — | 27 ER002 | 0 | 例題では出ない。出すかはユーザーが決める |
| — | 33 AixLib | omc では 0 | AixLib に出す（Rumoca ではない）。AixLib の CI のホワイトリストに既に載る。出すかはユーザーが決める（S113） |
| — | 25・25g・25b・25c・26 | 44・25・1・2・47 | v0.10.4 で修正（S115）。出さない。8 位（26）・9 位（25 → 25g）と 15 位の一部から外した |

## 提出のときの手順

1. **Rumoca main で確かめる。** Rumoca の main を取得して CLI を作り（`cargo xtask fetch-rumoca` の rev を変えるか、Rumoca を手元で `cargo build --release`）、下書きの「Steps to reproduce」をそのまま回す。失敗の形（`M`）が同じコードで落ち、対照（`M0`・`M1`）が通るかを見る。
   - 直っていたら出さず、表の状態を「main で直っている（確認日）」に書き換える。
   - メッセージが変わっていたら、下書きの「Actual」と「Environment」（rev）を書き換える。
   - diffsol の件（10〜12）は、単体の diffsol のプログラムに縮め、diffsol の main で確かめる。
2. **issue を出す。** 下書きの本文の見出しの「DRAFT (not filed) - 」を外し、`.mo` を添えて出す（出すのはユーザー）。
3. **番号を表に書く。** この表の「状態」を「提出済み（issue の URL、日付）」に、[research/upstream/README.md](research/upstream/README.md) の表の状態も同じに書き換え、同じコミットで直す。

## GlobeMode 側の回避を外す条件

回避は Rumoca の rev を上げたときに外す。外すかは、上げた rev で次を確かめてから決める（`mise run msl:funnel`・`msl:sim` の本数が減らないこと）。

| # | 回避 | 外す条件 |
| --- | --- | --- |
| 02 | 許容誤差の下限（GM-W10003）、auto の dopri45 への切り替え（GM-I11002） | サンプラの判定に丸めの床が入り、`SimpleTriacCircuit` が `Tolerance = 1e-12` のまま通る |
| 04 | 初期化の解き直しと GM-W10004（ADR-0020） | Rumoca が関係式を評価したあと初期化を解き直す（`04` の `.mo` で `x` の初期値が正しい） |
| 05・06 | 複製した session に構造的な上書きを埋めて compile し直す（ADR-0022）、名前の形で断る（GM-E10006） | 実行時の上書きが同名の修飾の束縛に届く（05）。または `set_structural_overrides` がコンパイル後も使え、名前の誤りを知らせる（06） |
| 07・08 | 既知の限界の試験（直ると落ちる db の試験） | その試験が落ちたら、試験を「正しい値」の試験に書き換える |
| 09 | bdf・esdirk34 を勧める | 試しの点の射影の失敗から復帰する |
| 10 | SDIRK の `max_nonlinear_solver_failures = usize::MAX` | diffsol が 1 歩ごとに数える |
| 11 | ヤコビ行列を受理した状態で取る（`rk.rs` の `anchor`） | diffsol が段の未知数の点で取る |
| 12 | 最初の刻みを `min_timestep` の 10 倍に持ち上げる | diffsol が見積もりを `min_timestep` と比べる |
| 13 | auto は方向微分を作れないとき RK に切り替えない（GM-U0004） | RK の道が誤った結果を返さず失敗する |
| 14 | host の道（ADR-0014） | 型付きの道が wasm32 で時計を呼ばない |
| 15 | auto の dopri45 への切り替え（ADR-0015） | BDF の補間が rtol 1e-7 で左端の検査に通る |
| 16 | `Dae` の直列化の形の書き換え（ADR-0029） | Rumoca が `external "builtin"` の組み込み関数を実行する |

17〜32（01・03 を含む。v0.10.4 で直った 25・25b・25c・25g・26 を除く）は GlobeMode 側の回避が無い（33 は AixLib の誤りで対象外）（32 は `--library` の閉包の読み込みで一部避けている）。直った rev に上げれば、`GM-U` の振り分けと既知の一覧（`src/Modelica/msl-known-unsupported.txt`）を `msl:funnel` で見直すだけでよい。
