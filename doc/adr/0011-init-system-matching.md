# ADR-0011 S8 は初期化系（DAE とその微分 + 初期条件）をマッチングして、冗長な初期条件と初期条件の無い未知数を名指しする

- 状態: Accepted（2026-10-19、S009）。[ADR-0009](0009-structural-index-and-init.md) の S8 の「数と属性だけで判定する」を精密にする（ADR-0009 の S7 と、数と属性の判定はそのまま残す）

## 背景

- [ADR-0009](0009-structural-index-and-init.md) の S8 は、要る初期条件の数（動的な状態 + 離散変数 + 初期化で決めるパラメータ）と与えた数（初期方程式 + `fixed = true`）を比べるだけだった。ADR-0009 自身が「数が合っていても、決まらない変数や重複した初期方程式は見つけられない」を残した近似に挙げていた。
- S008 の omc との照合（[ADR-0010](0010-omc-comparison.md)、[omc-compare.md](../omc-compare.md)）で、S8 の過剰の照合が 355 本中 354 本の一致にとどまった。食い違った `Modelica.Fluid.Examples.NonCircularPipes` は、globe が「不足 4 / 40」、omc が「冗長な初期方程式 2 本（`circular_pipe.mediums[1].p = 1e6` と `annulus_pipe.mediums[1].p = 1e6`）」。配管の最初の体積の圧力は境界の圧力と拘束で等しく（インデックス 2）、その状態に付いた `fixed = true` は拘束と重なる。数だけでは「不足 − 冗長 = 4」しか分からない。
- S008 で omc と食い違ったもう 1 本、`Magnetic.FluxTubes.Examples.SolenoidActuator.Components.AdvancedSolenoid`（globe は S6 を通し、omc は Pantelides の内部エラー）も S009 で調べた。これは S8 ではなく S6 の限界だった（下の結果）。
- Rumoca v0.10.2 の DAE は、初期方程式（`initialization_owners()`）と取り分けた定義（`initial_parameter_values()` / `initial_discrete_values()`）の式を公開していて、`rumoca_eval_dae::for_each_scalar_coordinate_cached` で依存するスカラーの座標（状態、`der`、代数、パラメータ、離散、`pre`）を列挙できる（S009 で確かめた。[research/rumoca.md](../research/rumoca.md#s009-で調べた-apiv0102)）。

## 決定

### 初期化系の組み方（`globe-mode-analysis/src/init_system.rs`、`match_init_system`）

初期化の時点 t0 で成り立つ方程式をすべて集めた二部グラフを作り、最大マッチング（Hopcroft–Karp）と粗い DM 分解（S6 と同じ道具）で過剰な部分と不足している部分を出す。考え方は Pantelides（1988）の consistent initialization と Pryce の Σ 法と同じ。

| | 中身 |
| --- | --- |
| 未知数（列） | 未知数の族（[`DaeStructure::unknowns`]）ごとに階数 0..=top。状態 `x` の族は `x`、`der(x)`、…、代数 `z` の族は `z`、`der(z)`、…。top は S7 の `IndexReport::orders`（微分で上がった最高階）。これに `fixed = false` で束縛の無いパラメータと、離散変数（`d` と `pre(d)` を 1 つの列にまとめる）を足す |
| 方程式（行） | (1) DAE の方程式 e と、それを 1..=d[e] 回微分したもの（d は S7 の `differentiations`。e を j 回微分した行は、元の参照（族 f、階数 o）ごとに階数 o..=o+j を参照する）。DAE は t0 でも成り立ち、その微分も成り立つ。(2) 初期方程式（フロントエンドが取り出した参照。`der(x)` は階数 1、`x` と `pre(x)` は階数 0、代数と `pre(z)` は階数 0、パラメータ、離散）。(3) `fixed = true` の状態・代数変数・離散変数の `v = start`（階数 0 の列を 1 つ参照） |
| 不変量 | 列 − 行 = 動的な状態（S7）+ 離散変数 + 初期化で決めるパラメータ − 与えた初期条件 = **要る数 − 与えた数**。だから数だけの判定と食い違わない（不足 − 過剰 = 要る数 − 与えた数）。数だけの判定は差しか分からず、マッチングは過剰と不足を別々に出す |

- **過剰な部分**（対応の無い行から交互路で届く行と列）: 行が列より `extra` 本多い。この部分にある初期条件（初期方程式と `fixed = true`）が**冗長の候補**。過剰な部分に DAE の行（拘束やその微分）が含まれる（`over_with_dae`）なら、初期条件が拘束と同じ変数を決めていて、整合する値なら冗長なだけ（omc はこのとき冗長な初期方程式を外して進む）。
- **不足している部分**（対応の無い列から交互路で届く列と行）: 列が行より `missing` 個多い。この部分の未知数には初期条件が無い（start 値を推定値として使う見込み）。
- 初期化で決めるパラメータ（`fixed = false`、束縛なし）は初期化の未知数なので、DAE の方程式からの参照も要る。フロントエンドは、そのパラメータが 1 つでもあるときだけ DAE の連続の方程式をもう一度歩いて `InitStructure::equation_parameter_refs` に残す（無ければ歩かない。MSL の例題のほとんどは無い）。
- **束縛を通した依存も追う**: `Sub s(k = c)` のように、`fixed = false` の `c` が部品の束縛を通してしか DAE に現れないことがある（MSL の `InitSpringConstant` の `spring.c` → `spring.spring.c`）。束縛が（推移的に）初期化の未知数を参照するパラメータも未知数にし、その束縛 `s.k = c` を行にする（`InitStructure::parameter_binding_refs`）。これが無いと `c` はどの行にも繋がらず、「決まらない」と「冗長」が 1 つずつ偽に出る（最初のファネルで起きた）。
- 離散変数の列は DAE の行から参照されない（[`DaeStructure`] は離散変数を既知として数える）ので、初期条件の無い離散変数は常に不足している部分に入る。when の外の式で決まる離散変数には初期条件が要らない（MLS §8.6）が、その区別は持っていない（S009 の候補 4 のまま）ので、ADR-0009 の柔らかい警告の約束をそのまま使う。

### 入力（`globe-mode-ir`、`globe-mode-frontend/src/init.rs`）

- `InitStructure` に初期方程式ごとの参照を足した: `initial_incidence`（`der(x)` と代数。同じ DAE の `DaeStructure::unknowns` の添字）、`initial_state_refs`（状態そのもの `x` と `pre(x)`。`DaeStructure::state_refs` と同じ約束）、`initial_parameter_refs`、`initial_discrete_refs`、`equation_parameter_refs`（上）、`has_incidence`（取り出せた）。位置ではなく構造なので、`without_ranges`（`init_shape` の防火壁）でも残る。
- 未知数の添字は `structure.rs` と同じ鍵（`unknowns_and_keys`）で写すので、`DaeStructure` と揃う。`match_init_system` は長さ（状態の数、代数の数、方程式の数）が合わなければ組まない（None）。
- 初期方程式の式を 1 つでも射影できなければ `has_incidence = false` にして、S8 は数と属性だけで判定する（S007 と同じ結果になる。モデルを止めない）。取り分けた定義（離散変数や Real でないパラメータの値）は、値の式を射影できなければ対象だけを参照する。

### 診断（`InitReport::diagnostics`。マッチングがあるとき）

| 場合 | 診断 |
| --- | --- |
| `extra > 0`、過剰な部分に DAE の行が無く、離散変数も含まない | `GM-E8001`（Error）。位置は最初の冗長の候補。補足に冗長の候補の一覧 |
| `extra > 0`、過剰な部分に DAE の行がある（拘束と重なる）か、離散変数を含む | `GM-W8004`（Warning。整合する値なら冗長なだけ）。補足に冗長の候補と理由 |
| `missing > 0`、不足 ≤ 不足している部分の `fixed = false` の離散変数の数 | 柔らかい `GM-W8001`（ADR-0009 と同じ約束） |
| `missing > 0`、それより多い | `GM-W8001`（補足に初期条件の無い未知数の一覧）と、不足している部分にある start の無い状態（階数 0）ごとに `GM-W8002` |
| 両方 | 両方を出す（数だけの判定では片方しか出なかった） |

- マッチングが無いとき（初期方程式の参照が無い、S7 が無いか打ち切り）は ADR-0009 の数と属性の判定のまま。
- `InitReport::is_balanced` は、マッチングがあれば「過剰も不足も無い」、無ければ「数が合う」。`extra()` / `missing()` も同じ規則。
- db のクエリ `init_report` は `init_shape` に加えて `dae_shape` と `index_report` を読む（どれも位置を落とした写しなので、位置だけの編集では再計算されない）。
- `msl-funnel` の S8 の列は「与えた / 要る」に「（冗長 k・決まらない m）」を添える。分布の区分は、冗長が 1 つでもあれば過剰、無ければ不足があるかで分ける（S007 までは与えた数と要る数の差で分けていた）。`omc-compare` の S8 の照合（過剰 ↔ omc の「over specified」）は、マッチングがあれば `extra > 0` を過剰とする。

### 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| 初期方程式と `fixed = true` だけで小さな二部グラフを作り、状態 `x` と対応づける | インデックスが 2 以上のモデルで、拘束（やその微分）で決まる状態を見られない。`NonCircularPipes` の冗長はまさにそれ。拡張した系（DAE とその微分）を含めないと、`Index.Pendulum`（状態 4、動的な状態 2）に「2 個足りない」と誤る（ADR-0009 の要る数の議論と同じ） |
| `DaeStructure` にパラメータの参照を足す | S6・S7 の入力を変えずに済むよう、初期化で決めるパラメータがあるときだけ `InitStructure` 側に持った（`equation_parameter_refs`）。MSL の例題ではほとんど空 |
| 離散変数の when の区別まで同時にやる | 入力（Rumoca の when の構造）の調査が別に要る。候補 4 のまま次に送る |
| 過剰な部分に DAE の行があっても `GM-E8001` にする | 整合する冗長（MSL の `GenerationOfFMUs` など 3 本、`NonCircularPipes`）を Error にしてしまう。omc も Warning で外して進む。ADR-0009 の「Error は確かなときだけ」を守る |
| 拡張した系を Pryce の Σ 法のオフセットから直接組む | S7 の Pantelides 法の結果（微分の回数と族の最高階）がそのまま使える。Σ 法のオフセットと一致することは S007 の乱択テストで確かめている |

## 結果

（S009 の実行結果。数字は [msl-funnel.md](../msl-funnel.md)、[omc-compare.md](../omc-compare.md)、[作業記録](../../log/work/2026-10-19-s009-init-matching.md)）

- **実装**（2026-10-19）: `globe-mode-analysis/src/init_system.rs`（`match_init_system`、`InitMatching`、`InitCondition`、`InitUnknown`）、`init.rs`（`analyze_init_system`、`InitReport::matching`、マッチングがあるときの診断）、`globe-mode-ir`（`InitStructure` の参照 6 項目）、`globe-mode-frontend/src/init.rs`（参照の抽出。`structure.rs` の `unknowns_and_keys` を共有）、`globe-mode-db`（`init_report` が `dae_shape` と `index_report` も読む）、CLI（`globe check` の S8 にマッチングの行）、`msl-funnel`・`omc-compare`（冗長・決まらないの数）。
- **検証用のモデル**: `Init.RedundantAndMissing`（数は 1 / 1 で合うが、冗長 1・決まらない 1。`GM-W8004` と `GM-W8001`・`GM-W8002`）、`Invalid.UngroundedLoop`（S6 の限界の記録。下）。[src/Modelica/README.md](../../src/Modelica/README.md)。
- **MSL の全例題**（[msl-funnel.md](../msl-funnel.md)）: 566 本: OK 369・NG 13・GM-U 184・panic 0（S008 と同じ本数）。S8 まで来た 369 本すべてでマッチングできた（初期方程式の射影の失敗 0）。S8 の分布は合う 111・過剰 6（すべて `GM-W8004`。Error 0）・不足 252（S008 は 111・3・255。冗長と不足を両方持つ 3 本が過剰に移った）。検査 723.8 s（1 本 1.28 s）、最大 RSS 7.1 GiB。
- **omc との照合**（[omc-compare.md](../omc-compare.md)。omc の結果は S008 のものを `--reuse`）: S8（過剰）の一致 352 / 355（S008 は 354 / 355）。不一致 3 本は下の 2 つの Rumoca の制約で入力が欠けるか誤るもので、マッチングの誤りではない。globe の `GM-W8004` の 3 本（`Electrical.Analog.Examples.GenerationOfFMUs` 冗長 2、`Lines.CompareLineTrunks` 冗長 3、`Thermal.HeatTransfer.Examples.GenerationOfFMUs` 冗長 1）は omc の「冗長な初期方程式」の本数と同じ。S5・S6 は S008 と同じ（382 / 382、362 / 363）。
- **最初のファネルで見つけて直したこと**: `InitSpringConstant` に偽の「冗長 1・決まらない 61」が出た。`fixed = false` の `spring.c` が部品の束縛 `spring.spring.c = c` を通してしか DAE に現れず、`c` がどの行にも繋がらなかった。束縛を通した依存を閉包で追い、束縛を行にして直した（上の決定）。直した後は冗長 0。
- **`AdvancedSolenoid` の食い違いは S6 の限界**: 電気側に接地が無い部品で、外に出したピン p・n が未接続（単体の検査では flow が 0）。枝（抵抗・コイル・コンデンサ）8 本に対して節点 7 つなので、二部グラフには完全マッチングがある（S6 を通る）が、p と n のキルヒホッフの電流則は各枝の電流の保存を通して数値的に従属で、電位は差しか決まらない。omc は記号的な簡約（alias の除去）の後に `0.0 = 0.0` の方程式を見つけて「structurally singular」で止める（`-d=bltdump` で確かめた）。globe が S6 で止めた 7 本（`Subsystem` のように枝 < 節点）は構造でも特異で、枝 ≥ 節点 なら構造では見えない。**構造の検査（二部グラフ）では捕まえられない数値的な特異**で、S6 の誤りではない。数値の検査（S10 のプリフライトで初期化のヤコビ行列の階数を見る）の候補にした。検証用のモデル `Invalid.UngroundedLoop` に記録した（omc 1.27.1 は「Circular Equalities Detected」で止まる）。
- **`NonCircularPipes` の冗長 2 本は globe の DAE に無い**: omc が冗長と言う `mediums[1].p = 1e6` は `PartialDistributedVolume` の `initial equation` の `if massDynamics == FixedInitial then mediums.p = ps_start` で、`massDynamics` は `outer system` から来る。Rumoca v0.10.2 は **`outer` の参照を `inner` の修正ではなく型の既定で評価する**（`IfInit4` の 5 通りで切り分けた。[research/rumoca.md](../research/rumoca.md#s009-で調べた-apiv0102)）ので、この初期方程式（2 本の配管で 8 本）が消え、globe は「決まらない 36」（与えた 4 / 要る 40）になる。マッチングの入力が欠けているのであって、マッチングの誤りではない。MSL の Fluid の例題 11 本が `inner Modelica.Fluid.System system(…)` でこの形。上流に報告する候補（roadmap の 7）。検証用のモデル `Init.OuterModification` と db のテスト `outer_modification_is_lost_by_the_compiler`（Rumoca が直ったら落ちる）に記録した。
- **`RollingWheelSetDriving` / `RollingWheelSetPulling` の「冗長 7」も Rumoca の入力の問題**: 例題が `wheelSet(theta1(fixed = true), …)` と修正すると、Rumoca は部品の中の `wheelSet.wheelSetJoint.theta1`（MSL は `wheelSetJoint(theta1(fixed = false))` で外している）にも `fixed = true` を付ける（`Init.ModifierScope` で再現）。別名の両方が `fixed = true` なので、マッチングは正しく冗長と言う。omc は過剰と言わない。上流に報告する候補。検証用のモデルと db のテスト `modifier_leaks_into_subcomponents_in_the_compiler`（Rumoca が直ったら落ちる）に記録した。
- **omc が手前で止めるモデル**（`GM-W8003` 相当。`OpAmpCircuits.*` の 5 本）は、globe では S8 の警告のまま S6 より前では報告しない。値の無いパラメータがあっても構造の検査は意味を持つ（S6・S7 は値を使わない）し、部品の単体の検査で値を与えないのは普通のため。omc との照合では「比べられない」のまま。
