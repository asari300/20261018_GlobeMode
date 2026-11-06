# 上流（Rumoca・diffsol）への報告の下書き

全体の一覧・提出の順・手順は正典の [../../upstream.md](../../upstream.md) にまとめた（S096）。

S045（計画書の WP-H）で書いた（15 は S055、16 は S068、17〜21 は S070、24〜26 は S072。20・25 の追記と `20b`・`25b` の再現は S080、25 の 2 回目の追記と `25c`〜`25h`・27 は S083、28〜30 と 25・27 の追記は S091、31・32 と 28 の追記は S108。28〜32 の v0.10.4 での再確認と日本語版（`_jp.md`・`_jp.html`）、AixLib 向けの 33 は S113。17〜27 の v0.10.4 での再確認と日本語版は S115。01〜16 の v0.10.4 での再確認（各件の「Re-check」の節）と日本語版は S162。25 を形ごとに 25b〜25h の `.md` に分けたのは S096）、[Rumoca](https://github.com/CogniPilot/rumoca) と [diffsol](https://github.com/martinjrobins/diffsol) に出す issue の下書き。元の記録は [../msl-sim-failures.md](../msl-sim-failures.md) の「上流に報告する候補」と、[../../roadmap.md](../../roadmap.md) の候補 8。

- **出すのはユーザー。エージェントは出さない**（`gh issue create` を使わない）。
- **出すときは、再現を Rumoca の最新（main）で確かめてから出す。** 下書きの環境は Rumoca v0.10.2（rev `276e82bd`）、MSL 4.1.0。01〜34 は最新のリリース v0.10.4（2026-11-05 でも最新）でも確かめてある（01〜16 は S162）。Rumoca の CLI がこの環境に無いまま書いた件は、本文に「[not verified]」と書いてある。
- 本文は英語で、issue にそのまま貼れる形（Summary / Steps to reproduce / Expected / Actual / Environment / Workaround in GlobeMode / Notes）。
- 再現ファイル（`.mo`）は、MSL の例題で再現する件では置かない（モデル名とコマンドを本文に書く）。`src/Modelica/` には置かない（MODELICAPATH として開けるパッケージだけを置く規則）。
- 再現用の `.mo` は `globe fmt` で整形する（空白が要点のものを除く。S194 で 34 本すべてを整形した。整形の前後で全クラスの診断のコードと重大度が同じことを確かめてある）。新しく足すときも整形し、本文に貼る診断の `行:列` は整形後のファイルのものにする。
- diffsol の件（10〜12）は Rumoca が同梱する 0.13.2 での観測。diffsol 本家で同じかは確かめていない（本文に "not verified against upstream diffsol main"）。単体の diffsol のプログラムに縮めてから出す。
- 状態は「下書き」のまま。出したら issue の URL と日付をこの表に足す。
- v0.10.4 で再現する 01・02・05〜09・12〜33 には日本語版（`NN-<slug>_jp.md`、英語と同じ章立て）とスライド 1 枚の解説（`NN-<slug>_jp.html`、`doc/assets/slides.css`・`slides.js` だけを使う）がある（28〜33 は S113、17〜27 は S115、01〜16 は S162）。v0.10.4 で直った 03・04・15（一部）・25・25b・25c・25g・26 と、diffsol の main で直った 10・11 には作っていない（15 は残る `Dimmer_RL` の分だけ作った）。ユーザーが読んで出すかを決めるためのもの。

| # | 件名 | 対象 | 再現 | GlobeMode 側の回避 | 状態 |
| ---: | --- | --- | --- | --- | --- |
| 01 | [インデックス 2 以上の DAE で EL005](01-el005-index2-structurally-singular.md) | Rumoca | MSL `FilterWithDifferentiation`（CLI） | なし | 下書き。v0.10.4 で再現（S162）。 [日本語](01-el005-index2-structurally-singular_jp.md)・[1 枚](01-el005-index2-structurally-singular_jp.html) |
| 02 | [サンプラの端点の判定に丸めの床が無い](02-sampler-endpoint-roundoff.md) | Rumoca | MSL `SimpleTriacCircuit` ほか | あり（許容誤差の下限、auto の切り替え） | 下書き。一部は v0.10.4 で直った（`ChopperBuckBoost_DutyCycle`。`SimpleTriacCircuit` の 1e-12 は再現。S162）。 [日本語](02-sampler-endpoint-roundoff_jp.md)・[1 枚](02-sampler-endpoint-roundoff_jp.html) |
| 03 | [Solve IR の変換と実行時オブジェクトの組み立てが終わらない](03-solve-runtime-construction-does-not-finish.md) | Rumoca | MSL `DrumBoiler`（CLI） | なし | **v0.10.4 で修正**（終わらない症状。S162。出さない） |
| 04 | [初期化方程式を関係式の既定値で解き、解き直さない](04-init-equations-solved-with-relation-default.md) | Rumoca | [04](04-init-equations-solved-with-relation-default.mo) | 検出だけ（GM-W10004） | **v0.10.4 で修正**（S162。出さない） |
| 05 | [実行時のパラメータ上書きが同名の修飾の束縛に届かない](05-parameter-override-not-reaching-inlined-bindings.md) | Rumoca | [05](05-parameter-override-not-reaching-inlined-bindings.mo) | あり（構造的な上書き、ADR-0022） | 下書き。v0.10.4 で再現（S162）。 [日本語](05-parameter-override-not-reaching-inlined-bindings_jp.md)・[1 枚](05-parameter-override-not-reaching-inlined-bindings_jp.html) |
| 06 | [構造的な上書き: コンパイル後の ED020、名前の誤りの黙殺、スカラだけ](06-structural-overrides-ed020-and-scalar-only.md) | Rumoca | [06](06-structural-overrides-ed020-and-scalar-only.mo) | あり（複製した session、名前の形で断る） | 下書き。v0.10.4 のソースを読んだ。コードは同じ、実行は【未確認】（S162）。 [日本語](06-structural-overrides-ed020-and-scalar-only_jp.md)・[1 枚](06-structural-overrides-ed020-and-scalar-only_jp.html) |
| 07 | [outer の参照が inner の修正でなく型の既定を読む](07-outer-reference-ignores-inner-modification.md) | Rumoca | [07](07-outer-reference-ignores-inner-modification.mo) | なし（既知の限界として記録） | 下書き。v0.10.4 で再現（S162）。 [日本語](07-outer-reference-ignores-inner-modification_jp.md)・[1 枚](07-outer-reference-ignores-inner-modification_jp.html) |
| 08 | [部品への修正が中の同名の変数に漏れる](08-modifier-leaks-into-subcomponent.md) | Rumoca | [08](08-modifier-leaks-into-subcomponent.mo) | なし（既知の限界として記録） | 下書き。v0.10.4 で再現（S162）。 [日本語](08-modifier-leaks-into-subcomponent_jp.md)・[1 枚](08-modifier-leaks-into-subcomponent_jp.html) |
| 09 | [試しの点の射影の失敗を復帰不能として記憶する](09-host-projection-failure-is-unrecoverable.md) | Rumoca | MSL `SimpleTriacCircuit`（dopri45） | 一部（bdf・esdirk34 を勧める） | 下書き。v0.10.4 で再現（S162）。 [日本語](09-host-projection-failure-is-unrecoverable_jp.md)・[1 枚](09-host-projection-failure-is-unrecoverable_jp.html) |
| 10 | [SDIRK が Newton 法の失敗を積分全体で累積して数える](10-diffsol-sdirk-newton-failures-accumulate.md) | diffsol | [10](10-diffsol-sdirk-newton-failures-accumulate.mo) | あり（上限を外す） | **diffsol main で修正**（未リリース。ソースを読んだだけ、S162。出さない） |
| 11 | [SDIRK がヤコビ行列を状態の点で取る](11-diffsol-sdirk-jacobian-at-wrong-point.md) | diffsol | MSL `OvervoltageProtection`（esdirk34） | あり（受理した状態で取る） | **diffsol main で修正の見込み**（未リリース。ソースを読んだだけ、S162） |
| 12 | [最初の刻みの見積もりが min_timestep を下回る](12-diffsol-initial-step-below-min-timestep.md) | diffsol | [12](12-diffsol-initial-step-below-min-timestep.mo) | あり（下限の 10 倍に持ち上げる） | 下書き。同梱の diffsol も diffsol main もコードは同じ（実行は【未確認】、S162）。 [日本語](12-diffsol-initial-step-below-min-timestep_jp.md)・[1 枚](12-diffsol-initial-step-below-min-timestep_jp.html) |
| 13 | [RK で黙って誤った結果になる](13-rk-silently-wrong-result-generationoffmus.md) | Rumoca | MSL `GenerationOfFMUs` | あり（auto は RK に切り替えない） | 下書き。v0.10.4 で再現（S162）。 [日本語](13-rk-silently-wrong-result-generationoffmus_jp.md)・[1 枚](13-rk-silently-wrong-result-generationoffmus_jp.html) |
| 14 | [wasm32 で型付きの道が `Instant::now()` で panic](14-wasm32-instant-now-in-typed-route.md) | Rumoca | 本文の手順 | あり（host の道） | 下書き。v0.10.4・main のソースに同じ呼び出し（wasm32 のビルドは【未確認】、S162）。 [日本語](14-wasm32-instant-now-in-typed-route_jp.md)・[1 枚](14-wasm32-instant-now-in-typed-route_jp.html) |
| 15 | [BDF の補間が host の左端の検査に通らない（rtol 1e-7、1e-6 でも）](15-bdf-sampler-left-endpoint-tight-tolerance.md) | Rumoca | MSL `ThyristorBridge2Pulse_RLV_Characteristic` ほか 9 本（`--solver bdf --tolerance 1e-7`） | 一部（auto が 7/10 を dopri45 で救う） | 下書き。契約の誤りの 9 本は v0.10.4 で直り、`Dimmer_RL` の丸めの幅の停止は再現（S162）。 [日本語](15-bdf-sampler-left-endpoint-tight-tolerance_jp.md)・[1 枚](15-bdf-sampler-left-endpoint-tight-tolerance_jp.html) |
| 16 | [`external "builtin"` が組み込み関数を呼ぶだけの関数を実行できない（MSL 4.0.0 の `Modelica.Math`）](16-external-builtin-math-functions.md) | Rumoca | [16](16-external-builtin-math-functions.mo) | あり（Dae を直列化の形で書き換える、ADR-0029） | 下書き。v0.10.4 で再現（メッセージは EL001、S162）。 [日本語](16-external-builtin-math-functions_jp.md)・[1 枚](16-external-builtin-math-functions_jp.html) |
| 17 | [`redeclare function extends product`（組み込みと同じ名前）を ER132 で断る](17-er132-redeclare-function-extends-product.md) — Rumoca の穴。267 本 | Rumoca | [17](17-er132-redeclare-function-extends-product.mo)（自己完結。`product` を `prod` に変えると通る） | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](17-er132-redeclare-function-extends-product_jp.md)・[1 枚](17-er132-redeclare-function-extends-product_jp.html) |
| 18 | [非 parameter の成分への `annotation(Evaluate = true)` を ER070 で断る](18-er070-evaluate-on-non-parameter-component.md) — Rumoca の穴（MLS §18.3 は誤りと言わない）。258 本 | Rumoca | [18](18-er070-evaluate-on-non-parameter-component.mo)（MSL の `Constant` だけ） | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](18-er070-evaluate-on-non-parameter-component_jp.md)・[1 枚](18-er070-evaluate-on-non-parameter-component_jp.html) |
| 19 | [`stateSelect` の parameter 式を EI032 で断る](19-ei032-statselect-parameter-expression.md) — Rumoca の穴（MLS §4.8.1 は parameter 属性）。235 本 | Rumoca | [19](19-ei032-statselect-parameter-expression.mo)（自己完結） | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](19-ei032-statselect-parameter-expression_jp.md)・[1 枚](19-ei032-statselect-parameter-expression_jp.html) |
| 20 | [extends が 2 つの関数（MSL `Files.loadResource`）の呼び出しと、2 つの Medium の別名の関数を EF025 で断る](20-ef025-function-selection-identity.md) — どちらも Rumoca の穴。138 本（`loadResource` 136・Medium 2） | Rumoca | [20](20-ef025-function-selection-identity.mo)（MSL だけ、自己完結の版も）、[20b](20b-ef025-two-medium-aliases-modified-extends.mo)（自己完結。Medium が 1 つ・修正の無い extends・同じ package なら通る） | なし | 下書き。v0.10.4 で `loadResource` の形（`Rep20Load`・`Rep20TwoExtends`）は直り、20b（`Rep28.M`）は再現（S115）。 [日本語](20-ef025-function-selection-identity_jp.md)・[1 枚](20-ef025-function-selection-identity_jp.html) |
| 21 | [`getInstanceName()` を parameter の束縛に書くと、成分配列を含む式の if 方程式で EF004](21-ef004-getinstancename-and-if-equation-branches.md) — どちらも Rumoca の穴（原因は 2 つ）。217 本 | Rumoca | [21](21-ef004-getinstancename-and-if-equation-branches.mo)（自己完結） | なし | 下書き。v0.10.4 で A・B とも再現（S115）。 [日本語](21-ef004-getinstancename-and-if-equation-branches_jp.md)・[1 枚](21-ef004-getinstancename-and-if-equation-branches_jp.html) |
| 24 | [条件付き成分の条件が別の package の定数を読むと EI006 で断る](24-ei006-condition-on-package-constant.md) — Rumoca の穴（MLS §4.4.5・§3.8）。117 本 | Rumoca | [24](24-ei006-condition-on-package-constant.mo)（自己完結。モデルの定数・parameter なら通る） | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](24-ei006-condition-on-package-constant_jp.md)・[1 枚](24-ei006-condition-on-package-constant_jp.html) |
| 25 | [Boolean の出力の関数で、if の枝が代入のあと for を回すと ED019 `function conditional`](25-ed019-function-conditional-boolean-loop.md) — Rumoca の未対応（MLS §11.2・§12.2 は制限しない）。ED019 125 本のうち同じメッセージ 44 本（S063） | Rumoca | [25](25-ed019-function-conditional-boolean-loop.mo)（自己完結。Real の出力なら通る） | なし | **v0.10.4 で修正**（S115。出さない） |
| 25b | [入力の次元が package の定数で、その定数を extends の修正で変えると ED019 `function shape proof`](25b-ed019-function-shape-proof-modified-package-constant.md) — Rumoca の穴（MLS §10.1・§7.2）。MSL 4.0.0 の `PartialSimpleMedium`、4.1.0 では 1 本 | Rumoca | [25b](25b-ed019-function-shape-proof-modified-package-constant.mo)（自己完結。修正の無い extends・`x[:]` なら通る） | なし | **v0.10.4 で修正**（S115。出さない） |
| 25c | [呼び出しで大きさの決まる package 定数の `size` を `Integer` の局所変数に束縛すると ED019](25c-ed019-function-shape-proof-size-of-call-sized-constant.md) — Rumoca の穴（MLS §12.4.5）。2 本 | Rumoca | [25c](25c-ed019-function-shape-proof-size-of-call-sized-constant.mo) | なし | **v0.10.4 で修正**（S115。出さない） |
| 25d | [配列の入力が後で宣言する `Integer` の入力で次元を取ると ED019](25d-ed019-function-shape-proof-dimension-before-input.md) — Rumoca の穴（MLS §12.4.1・§12.4.4）。3 本 | Rumoca | [25d](25d-ed019-function-shape-proof-dimension-before-input.mo) | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](25d-ed019-function-shape-proof-dimension-before-input_jp.md)・[1 枚](25d-ed019-function-shape-proof-dimension-before-input_jp.html) |
| 25e | [record の成分を次元にした配列の record を関数の局所変数にすると ED019](25e-ed019-function-shape-proof-record-local-dimension.md) — Rumoca の穴（MLS §12.4.5）。6 本 | Rumoca | [25e](25e-ed019-function-shape-proof-record-local-dimension.mo) | なし | 下書き。v0.10.4 で `MC`（constant）は直り、`M`（parameter）は再現（S115）。 [日本語](25e-ed019-function-shape-proof-record-local-dimension_jp.md)・[1 枚](25e-ed019-function-shape-proof-record-local-dimension_jp.html) |
| 25f | [`b[:]` の束縛が後で宣言する定数を読むと ED019 `axis 1 is variable-size`](25f-ed019-function-shape-proof-binding-reads-later-constant.md) — Rumoca の穴（MLS §12.4.4）。3 本 | Rumoca | [25f](25f-ed019-function-shape-proof-binding-reads-later-constant.mo) | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](25f-ed019-function-shape-proof-binding-reads-later-constant_jp.md)・[1 枚](25f-ed019-function-shape-proof-binding-reads-later-constant_jp.html) |
| 25g | [if の枝が for だけ（`splineDerivatives`）で ED019 `function conditional`](25g-ed019-function-conditional-loop-only-branch.md) — Rumoca の未対応（MLS §11.2）。25 と同じ検査。25 本 | Rumoca | [25g](25g-ed019-function-conditional-loop-only-branch.mo) | なし | **v0.10.4 で修正**（S115。出さない） |
| 25h | [`Real[:]` の局所変数を呼び出しの代入だけで大きさを決めると ED019 `axis 1 is variable-size`](25h-ed019-function-shape-proof-flexible-array-from-call.md) — Rumoca の穴（MLS §12.4.5）。2 本 | Rumoca | [25h](25h-ed019-function-shape-proof-flexible-array-from-call.mo) | なし | 下書き。v0.10.4 で再現（S115）。 [日本語](25h-ed019-function-shape-proof-flexible-array-from-call_jp.md)・[1 枚](25h-ed019-function-shape-proof-flexible-array-from-call_jp.html) |
| 26 | [package の record 定数の成分を読むと ED008 unresolved Flat reference](26-ed008-field-of-package-record-constant.md) — Rumoca の内部エラー。ED008 108 本のうち `Media.Air.dryair.cp` 47 本 | Rumoca | [26](26-ed008-field-of-package-record-constant.mo)（自己完結。スカラの定数・モデルの record 定数なら通る） | なし | **v0.10.4 で修正**（S115。出さない。`i` の 18 本は要再現のまま） |
| 27 | [extends で継いだ入れ子のクラスを、継いだ package の名前で引くと ER002](27-er002-inherited-class-through-extending-package.md) — Rumoca の穴（MLS §7.1・§5.3.2）。ライブラリの例題では 0 本（S091） | Rumoca | [27](27-er002-inherited-class-through-extending-package.mo)（自己完結。宣言した package・replaceable な別名なら通る） | なし（replaceable な別名で書く） | 下書き。v0.10.4 で再現（S115）。 [日本語](27-er002-inherited-class-through-extending-package_jp.md)・[1 枚](27-er002-inherited-class-through-extending-package_jp.html) |
| 28 | [配列の次元が別の package の定数だと ET004、redeclare しない replaceable な package を通すと EF024](28-et004-ef024-dimension-from-package-constant.md) — Rumoca の穴（MLS §3.8.1・§10.1）。4 本（S108 でモデルの中の record の形を追記） | Rumoca | [28](28-et004-ef024-dimension-from-package-constant.mo)（自己完結。モデルの定数・`redeclare package` なら通る） | なし（部品で redeclare する） | 下書き。v0.10.4 で `M`・`M2` は直り、`M1`・`M3`・`M5` は再現（S113）。[日本語](28-et004-ef024-dimension-from-package-constant_jp.md)・[1 枚](28-et004-ef024-dimension-from-package-constant_jp.html) |
| 29 | [モデルの中の関数で、protected の束縛が replaceable な package の関数を呼ぶと EF024](29-ef024-local-function-binding-calls-replaceable-package.md) — Rumoca の穴（MLS §12.4.4・§5.3）。18 本 | Rumoca | [29](29-ef024-local-function-binding-calls-replaceable-package.mo)（自己完結。algorithm で呼ぶ・package の名前で呼ぶなら通る） | なし（algorithm で呼ぶ） | 下書き。v0.10.4 で再現（S113）。[日本語](29-ef024-local-function-binding-calls-replaceable-package_jp.md)・[1 枚](29-ef024-local-function-binding-calls-replaceable-package_jp.html) |
| 30 | [`y[:]`・`eta[size(y, 1)]` の record の parameter を関数の呼び出しで束縛すると ET004](30-et004-record-size-dimension-bound-to-call.md) — Rumoca の穴（MLS §10.1・§12.4.5）。15 本 | Rumoca | [30](30-et004-record-size-dimension-bound-to-call.mo)（自己完結。成分を直接書けば通る） | なし | 下書き。v0.10.4 で再現（S113）。[日本語](30-et004-record-size-dimension-bound-to-call_jp.md)・[1 枚](30-et004-record-size-dimension-bound-to-call_jp.html) |
| 31 | [束縛が replaceable な record（成分・クラス）を通して成分を読むと EF024](31-ef024-binding-reads-field-of-replaceable-record.md) — Rumoca の穴（MLS §7.3・§5.3・§12.4.4）。8 本 | Rumoca | [31](31-ef024-binding-reads-field-of-replaceable-record.mo)（自己完結。`replaceable` を外せば通る） | なし | 下書き。v0.10.4 で再現（S113）。[日本語](31-ef024-binding-reads-field-of-replaceable-record_jp.md)・[1 枚](31-ef024-binding-reads-field-of-replaceable-record_jp.html) |
| 32 | [モデルが使わない、別の派生クラスの Medium の壊れた関数まで解決して ER002](32-er002-unused-function-of-other-subclass-medium.md) — Rumoca の穴（MLS §5.6）。呼ばれる関数そのものは AixLib の誤り。39 本 | Rumoca | [32](32-er002-unused-function-of-other-subclass-medium.mo)（自己完結。Medium を通して呼ばない・別の派生クラスが無いなら通る） | 一部（`--library` は閉包だけ読む） | 下書き。v0.10.4 で再現（S113）。[日本語](32-er002-unused-function-of-other-subclass-medium_jp.md)・[1 枚](32-er002-unused-function-of-other-subclass-medium_jp.html) |
| 33 | [AixLib の `CHPCombustionMixtureGasNasa.T_psX` が宣言の無い `specificEntropyOfpTX` を呼ぶ](33-aixlib-specific-entropy-of-ptx-missing.md) — **AixLib の誤り**（MSL の `MixtureGasNasa` から protected の補助関数を写していない）。omc も断る。32 の 39 本の元 | AixLib | [33](33-aixlib-specific-entropy-of-ptx-missing.mo)（自己完結。元の `T_psX` なら通る） | 対象外 | 下書き。AixLib main（`e5df9a8d`）でも同じ（S113）。[日本語](33-aixlib-specific-entropy-of-ptx-missing_jp.md)・[1 枚](33-aixlib-specific-entropy-of-ptx-missing_jp.html) |
| 34 | [2 つの関係式の根が丸めの雑音のとき、事象の反復が終わらない（`value == 0.0` が厳密な比べ方）](34-event-iteration-exact-zero-test-on-rounding-noise.md) — Rumoca のバグ。MSL 1 本（`sdirk34hw`） | Rumoca | [34](34-event-iteration-exact-zero-test-on-rounding-noise.mo)（MSL の例題の experiment を決めるだけ。`globe simulate --solver sdirk34hw`） | なし（記録と既知の限界の試験） | 下書き。v0.10.4・main のコードも同じ（S158）。[日本語](34-event-iteration-exact-zero-test-on-rounding-noise_jp.md)・[1 枚](34-event-iteration-exact-zero-test-on-rounding-noise_jp.html) |

## 公開ライブラリの失敗から（17〜21、S070）

S063 の失敗の分類（[../libraries-2026-11-01.md](../libraries-2026-11-01.md)）の上位 5 つの Rumoca のコードを、ライブラリを使わない最小の再現にし、MLS 3.6 の該当節と omc 1.27.2（`checkModel`）で「ライブラリの誤りか Rumoca の穴か」を判定した（確認日 2026-11-01）。**5 件とも Rumoca の穴で、ライブラリ側の誤りは無かった**（omc は全部通す）。再現は Rumoca v0.10.2 の CLI（`cargo xtask fetch-rumoca`）と `globe check` の両方で同じコードが出る。20 の Medium の関数の分は S080 で最小の再現 [20b](20b-ef025-two-medium-aliases-modified-extends.mo) を作った。これらの再現ファイルは同じフォルダの `.mo` を互いに読むので、モデル名は `Rep17`〜`Rep21` で始めて重ならないようにしてある。

## 公開ライブラリの失敗から（22〜26、S072）

S070 と同じ手順で、S070 の「次回」に挙げた 5 つのコード（ER023・ER003・EI006・ED019・ED008）を判定した（確認日 2026-11-02）。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出る（対照のモデルは両方で通る）。omc 1.27.2 の `checkModel` は ER003 の再現だけを断り、ほかは通す。モデル名は `Rep22`〜`Rep26` で始める。

| # | コード | 判定 | 根拠 |
| ---: | --- | --- | --- |
| 22 | ER023（ClaRa 3 本） | **正当**（MLS の規定上の拒否）。下書きは書かない | MLS 3.6 §4.6「record に直接宣言する成分は record か type に限る」。`class … extends ExternalObject`（TILMedia）は class。omc は緩く通す。Rumoca のメッセージの節番号 §4.7 は 3.6 では §4.6（Balanced Models が §4.7）。再現は [22](22-er023-record-component-externalobject.mo) |
| 23 | ER003（ClaRa 2・AixLib 1） | **正当**（無い基底クラス）。ClaRa の 2 本は依存の取得で避けられる（GlobeMode 側の回避。SMArtInt は取得していない）、AixLib の 1 本はライブラリの誤り。下書きは書かない | ClaRa は取得しない依存 `SMArtInt.Internal.ClaRaDelay`。AixLib は `Modelica.Media.Examples.Tests.Components.PartialTestModel2` を引くが、MSL 4.0.0 にも 4.1.0 にも無い（`Modelica.Media.Examples.Utilities.PartialTestModel2` にある）。omc も同じ再現を断る。再現は [23](23-er003-base-class-not-found.mo) |
| 24 | EI006 | **Rumoca の穴** | 別の package の定数を読む条件は parameter 式（MLS §4.4.5・§3.8.2・§3.8.3）。omc は通す |
| 25 | ED019 | **Rumoca の未対応**（`function conditional`。代表の形だけ） | 文の組み合わせに MLS の制限は無い。omc は通す。`function shape proof` は S080 で最小の再現 [25b](25b-ed019-function-shape-proof-modified-package-constant.mo) を作った（MSL 4.0.0 の `SensorVerification`）。MSL 4.1.0 で出るほかの 12 本と `splineDerivatives` は S083 で 25c〜25h にした |
| 26 | ED008 | **Rumoca の穴**（内部エラー） | package の定数は名前で引ける（MLS §5.3.2）。omc は通す |

## 残りの 2 つの形（20b・25b、S080）

S070・S072 で【未確認】だった 2 つを、ライブラリの例題から MSL の該当の関数を写して削り、自己完結の再現にした（確認日 2026-11-02）。新しい番号の下書きは作らず、20 と 25 に「Minimal reproduction」の節を足した（25 の分は S096 で 25b に移した）。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出て、対照のモデルは両方で通る。omc 1.27.2 の `checkModel` は全部通す。モデル名は `Rep27`（25b）と `Rep28`（20b）で始める。

| 再現 | コード | 判定 | 消える条件 |
| --- | --- | --- | --- |
| [25b](25b-ed019-function-shape-proof-modified-package-constant.mo) | ED019 `function shape proof` | **Rumoca の穴**（MLS §10.1・§7.2） | 関数の入力 `x[n]` を `x[:]` にする、または package の `extends P(n = 2)` の修正を外す。MSL 4.0.0 の `PartialSimpleMedium.specificEnthalpy_pTX`（`X[nX]`）が原因で、4.1.0 は `X[:]` |
| [20b](20b-ef025-two-medium-aliases-modified-extends.mo) | EF025 `multiple exact override package selections` | **Rumoca の穴**（MLS §5.3） | 同じモデルの Medium の別名を 1 つにする、extends の修正を外す、または同じ package を 2 回指す |

## ED019 の残り（25c〜25h・27、S083）

S080 で【未確認】だった、MSL 4.1.0 で出る ED019 を二分法で最小にした（確認日 2026-11-02。本数は libraries の run 41365960542 の `*-funnel.json`）。S083 では新しい番号は作らず 25 に節を足したが、S096 で形ごとの下書き 25b〜25h に分けた。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出て、対照は両方で通る。omc 1.27.2 の `checkModel` は全部通す（27 と合わせて 25 本）。モデル名は `Rep29`〜`Rep35`。

| 再現 | メッセージ | 判定 | 消える条件 | 本数 |
| --- | --- | --- | --- | ---: |
| 25b（S080） | `dependent extent ...` | 同じ形（ThermofluidStream の MSL 4.0.0 の写しの `X[nX]`） | `X[:]` | 1 |
| [25c](25c-ed019-function-shape-proof-size-of-call-sized-constant.mo) | `dependent extent ...` | **Rumoca の穴**（MLS §12.4.5）。別の形 | 呼び出しで大きさの決まる package 定数の `size` を関数の `Integer` の局所変数に束縛しない | 2 |
| [25d](25d-ed019-function-shape-proof-dimension-before-input.mo) | `dependent extent ...` | **Rumoca の穴**（MLS §12.4.1・§12.4.4）。別の形 | 次元の入力 `n` を配列の入力より前に宣言する | 3 |
| [25e](25e-ed019-function-shape-proof-record-local-dimension.mo) | `dependent extent ...` | **Rumoca の穴**（MLS §12.4.5。`constant` でも出る）。別の形 | record の配列の次元を数で書く | 6 |
| [25f](25f-ed019-function-shape-proof-binding-reads-later-constant.mo) | `` `b`: axis 1 is variable-size ... `` | **Rumoca の穴**（MLS §12.4.4）。別の形 | 束縛が読む定数を前に宣言する | 3 |
| [25h](25h-ed019-function-shape-proof-flexible-array-from-call.mo) | `` `X`: axis 1 is variable-size ... `` | **Rumoca の穴**（MLS §12.4.5）。別の形 | 局所の配列の大きさを宣言する | 2 |
| [25g](25g-ed019-function-conditional-loop-only-branch.mo) | `function conditional`（`splineDerivatives`） | **Rumoca の未対応**（MLS §11.2）。25 と同じ検査の別の入り口（Real の配列の出力） | 最初の for を `d := y` にする、枝に代入を書く | 25 |
| [27](27-er002-inherited-class-through-extending-package.mo) | ER002 | **Rumoca の穴**（MLS §7.1・§5.3.2）。S080 で見えた CLI の失敗 | 宣言した package か replaceable な別名で引く | 未確認 |

## EF024・ET004 と ER002 の本数（28〜30、S091）

S080 で見えた「モデルの配列の次元を package の定数にすると EF024・ET004」を二分法で最小にし、libraries の run 41365960542（2026-11-02）の `*-funnel.json` で同じコードの例題を数えた（確認日 2026-11-02）。EF024 は 27 本、ET004 は 18 本で、メッセージで 3 つの形に分かれた。再現は `globe check` と Rumoca v0.10.2 の CLI で同じコードが出て、対照は両方で通る。omc 1.27.2 の `checkModel` は 12 本とも通す。モデル名は `Rep36`〜`Rep38`。

| 再現 | コード・メッセージ | 判定 | 消える条件 | 本数 |
| --- | --- | --- | --- | ---: |
| [28](28-et004-ef024-dimension-from-package-constant.mo) | ET004 `dimension expression could not be evaluated: P.n`、EF024 `missing structured identity: n` | **Rumoca の穴**（MLS §3.8.1・§10.1）。S080 で見えた形 | 次元をモデルの定数にする、package を `redeclare package` で渡す | 3（ET004） |
| [29](29-ef024-local-function-binding-calls-replaceable-package.mo) | EF024 `missing structured identity: temperature` | **Rumoca の穴**（MLS §12.4.4・§5.3） | 束縛でなく algorithm で呼ぶ。IBPSA の写しで `WaterProperties` が S0〜S10 を通る | 18 |
| [30](30-et004-record-size-dimension-bound-to-call.mo) | ET004 `size(y, 1)`（`motorEfficiency_yMot_generic.eta`） | **Rumoca の穴**（MLS §10.1・§12.4.5） | record の成分を直接書く。ライブラリの写しは直していない【未確認】 | 15 |

- 残りの EF024 9 本（Buildings の `Fluid.DXSystems` の `nomVal` 7、AixLib の `T` 1・`nXi` 1）は最小にしていない【未確認】。
- ER002 は 68 本（AixLib 67・Buildings 1）で、27 の形（継いだ入れ子のクラスへの `unresolved type reference`）は **0 本**。型の参照は取得しない `Modelica_DeviceDrivers` の 1 本だけで、残りは `specificEntropyOfpTX` の関数呼び出し 39（追っていない【未確認】）、成分の参照 18、import 10。
- 25c の名前（S083 の持ち越し）: 8 通りの名前の組で、目的のメッセージは局所変数の名前が package の定数（`nc = n` の `n`）と同じときだけ出て、呼ぶ関数の入力の名前には依らない。束縛 `nc = n` を関数 `f` の範囲で引いている（MLS §5.3 は書かれた package で引く）と読めるが、コードは読んでいない【推測】。下書き 25 の Notes に書いた。

## EF024 の残りと ER002 の `specificEntropyOfpTX`（31・32、28 の追記、S108）

S091 の持ち越し。run 41365960542（2026-11-02）の `*-funnel.json` の EF024 の残り 9 本と ER002 の `specificEntropyOfpTX` 39 本を、取得した Buildings 14.0.0・AixLib 3.0.1 の写し（`.mo` だけ）と Rumoca v0.10.2 の CLI で最小にした（確認日 2026-11-03）。CLI は MSL と AixLib を 1 つのソースの根に並べて渡した（根を 2 つに分けると名前が引けない。S063 と同じ）。再現は `globe check` と CLI で同じコードが出て、omc 1.27.2 の `checkModel` は 12 本とも通す。モデル名は `Rep36`（追記）・`Rep39`・`Rep40`。

| 再現 | コード・メッセージ | 判定 | 消える条件 | 本数 |
| --- | --- | --- | --- | ---: |
| [31](31-ef024-binding-reads-field-of-replaceable-record.mo) | EF024 `missing structured identity: nomVal`・`m`・`a` | **Rumoca の穴**（MLS §7.3・§5.3・§12.4.4） | `replaceable` を外す。Buildings の写しで `DXCoil.m_flow_small` の束縛を定数にすると `SpeedSelect` が S0〜S10 を通る | 8（Buildings の `nomVal` 7、AixLib の `T` 1） |
| [28](28-et004-ef024-dimension-from-package-constant.mo) の `M5` | EF024 `missing structured identity: n` | **Rumoca の穴**（28 と同じ。package を redeclare しても、モデルの中の record の次元だと落ちる） | 次元を成分に直接書く | 1（AixLib の `nXi`） |
| [32](32-er002-unused-function-of-other-subclass-medium.mo) | ER002 `unresolved function call: 'specificEntropyOfpTX'` | **Rumoca の穴**（MLS §5.6。モデルが使わないクラスを解決する）。ただし呼ばれる関数は AixLib の誤り | 別の派生クラス（`ExhaustHeatExchanger`）の `Medium1` を消す。`globe check` は閉包だけ読むので出ない | 39（AixLib） |

- 31 の 2 つの形: Buildings の `DXCoil` の `m_flow_small = 0.0001*sta[nSta].nomVal.m_flow_nominal`（`sta`・`nomVal` が replaceable な record の成分）と、AixLib の R134a の媒質の関数の `SaturationProperties sat = setSat_T(state.T)`（`ThermodynamicState` は `PartialMedium` の replaceable な record）。CLI は後者の位置を `R134a_IIR_P1_395_T233_455_Horner.mo` の 1247 行と出した。AixLib の写しは直して確かめていない【未確認】。
- 32 の道筋: `PartialFourPortInterface` を継ぐ部品が `Medium1.setState_phX` を呼ぶと、同じ基底を継ぐ `ModularCHP.BaseClasses.ExhaustHeatExchanger` の `Medium1 = CHPFlueGasLambdaOnePlus` まで解決し、その `setState_psX` → `T_psX.f_nonlinear` の未宣言の `specificEntropyOfpTX` で止まる。`ExhaustHeatExchanger.mo` を空のモデルにすると通り、`extends ...PartialFourPort(redeclare package Medium1 = ...CHPFlueGasLambdaOnePlus)` だけにすると戻る。Rumoca のコードは読んでいない【推測】。
- `CHPCombustionMixtureGasNasa` は MSL の `MixtureGasNasa.T_psX` を写したが、そこで呼ぶ protected の `specificEntropyOfpTX` を写していない。AixLib の誤りで、AixLib に知らせるかはユーザーが決める（GlobeMode からは出さない）。
- 29 の再現には replaceable な record が無く、31 と原因が同じかは追っていない【未確認】。

## Rumoca v0.10.4 での再確認と AixLib 向けの 33（S113）

28〜32 の再現の `.mo` を、Rumoca v0.10.2（`data/rumoca/`）と最新のリリース v0.10.4（タグのコミット `9c8c56fc`、2026-11-02 公開。配布バイナリを `data/rumoca-latest/` に置いた。コミットしない）の CLI で回した（確認日 2026-11-03）。Rumoca の main（`91d6e936`、v0.10.4 の 10 コミット後）はビルドしていない【未確認】。

| # | v0.10.2 | v0.10.4 | 結果 |
| --- | --- | --- | --- |
| 28 | 失敗 5（ET004 3・EF024 2）、対照 2 が通る | `M`・`M2` が通り、失敗 3（`M3` ET004、`M1`・`M5` EF024） | **一部直った**。replaceable な package を通す形だけが残る |
| 29 | EF024 2、対照 2 | 同じ | 再現 |
| 30 | ET004 1、対照 1 | 同じ | 再現 |
| 31 | EF024 4、対照 3 | 同じ | 再現 |
| 32 | ER002 2、対照 2 | 同じ | 再現 |

- 再現する件のメッセージの文面は v0.10.2 と同じ（`grep` で `[E…]` の行を突き合わせた）。
- 28 の直った形は、v0.10.4 のコミット「Evaluate constant dimensions by declaration identity in typecheck and flatten」によると見られる【推測】。ライブラリの例題は v0.10.4 で回していない【未確認】。
- 33（AixLib）: `CHPCombustionMixtureGasNasa.mo` の 670 行は AixLib main（`e5df9a8d` = v3.0.1）でも同じ（GitHub の API で取得して突き合わせ、確認日 2026-11-03）。omc 1.27.2 は CHP の媒質の `setState_psX` を呼ぶモデルを `Function specificEntropyOfpTX not found in scope f_nonlinear.` で断る（AixLib 3.0.1・MSL 4.0.0）。AixLib は既にこのパッケージを CI のホワイトリストに載せている（issue #1291）。

## Rumoca v0.10.4 での再確認（17〜27、S115）

S113 と同じ手順で、17〜27（20b・25b〜25h を含む）の再現の `.mo` を Rumoca v0.10.2（`data/rumoca/`）と v0.10.4（`data/rumoca-latest/`、タグのコミット `9c8c56fc`）の CLI で回した（確認日 2026-11-03）。18・20 の `Rep18`・`Rep20Load` は MSL 4.1.0 を `--source-root` に渡した。62 モデル × 2 版のうち、v0.10.4 で通るようになったのが 11、両方で落ちるのが 20、両方で通る（対照）のが 31。v0.10.4 で落ちるようになったものは 0。

| # | v0.10.2 | v0.10.4 | 結果 |
| --- | --- | --- | --- |
| 17 | ER132 1 | 同じ | 再現 |
| 18 | ER070 1、対照 1 | 同じ | 再現 |
| 19 | EI032 2、対照 2 | 同じ | 再現 |
| 20 | EF025 3（`Rep20Load`・`Rep20TwoExtends.M`・20b `Rep28.M`）、対照 4 | `Rep20Load`・`Rep20TwoExtends.M` が通る。20b は EF025 | **一部直った**。2 つの Medium の形（20b）だけが残る |
| 21 | EF004 3・ED001 1、対照 3 | 同じ | 再現 |
| 24 | EI006 3、対照 2 | 同じ | 再現 |
| 25 | ED019 1、対照 2 | 通る | **直った** |
| 25b | ED019 1、対照 2 | 通る | **直った** |
| 25c | ED019 2、対照 2 | 通る | **直った** |
| 25d | ED019 2、対照 2 | 同じ | 再現 |
| 25e | ED019 2、対照 2 | `MC`（constant）が通る。`M`（parameter）は ED019 | **一部直った** |
| 25f | ED019 1、対照 2 | 同じ | 再現 |
| 25g | ED019 1、対照 2 | 通る | **直った** |
| 25h | ED019 2、対照 1 | 同じ | 再現 |
| 26 | ED008 3、対照 2 | 通る | **直った** |
| 27 | ER002 2、対照 2 | 同じ | 再現 |

- 再現する件のメッセージの文面は v0.10.2 と同じ（17 の `DefId(78)` まで同じ）。
- 直った件は本文に「Status (S115)」の行と再確認の節を足し、出さない記録として残した。日本語版は作っていない。
- ライブラリの例題そのものは v0.10.4 で回していない【未確認】。直った件の本数（`loadResource` 136、25 の 44、25g の 25、26 の 47 など）が実際に消えるかは、GlobeMode の Rumoca の rev を上げて `msl:funnel`・`libraries.yml` で確かめる。

## 下書きにしなかったもの

[msl-sim-failures.md](../msl-sim-failures.md) の候補のうち、次は再現の最小化か相手の特定がまだなので書いていない。

- 4（理想スイッチの射影の判定）、5（fixed = true の代数変数から状態を解かない）、7（RK の道で壁時計の予算が守られない）、8（関数呼び出しの Solve IR の契約）、9（同時イベントと clock。Rumoca の基準値にも載っている）、20（diffsol の Hermite の補間が硬い準静的な変数で外れる）。
- 公開の範囲の依頼（`admitted_native_execution_backend` を公開に、diffsol の `ClosureNoJac` を公開に）は 14 の Notes に 1 行だけ書いた。
