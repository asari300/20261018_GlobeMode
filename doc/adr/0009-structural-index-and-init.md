# ADR-0009 S7 は Pantelides 法で構造インデックスを見積もり、S8 は初期条件の数と属性で判定する

- 状態: Accepted（2026-10-18、S007）

## 背景

- [architecture.md §3.2](../architecture.md#32-遅延評価とシミュレーション前のアサーション) は、`assert_simulatable` の S7（インデックス）と S8（初期化）を決めていた。中身は未実装だった。
  - S005 の S7 は、der の形（状態を既知とみなす二部グラフ）に完全マッチングが無いときの情報 `GM-I7001`（「微分インデックスが 1 より大きい見込み」）だけ。何回微分すればよいか、どの状態が状態選択に回るかは分からなかった。
  - S8 は何も無かった。初期条件の過不足は、シミュレーションを始めるまで分からなかった。
- S6 は、`x` と `der(x)` を 1 つの列にまとめた二部グラフに完全マッチングがあるかで判定する（[S005 の解説書](../sessions/S005-check-queries/解説書.md)の「S6 の判定を直した」）。これは Pantelides 法が失敗しない条件と同じなので、S6 を通った構造には Pantelides 法を最後まで回せる。
- 入力に使える中立 IR は `DaeStructure`（der の形の `incidence` と、方程式が状態そのものを参照するときの `state_refs`）。`globe-mode-analysis` は Rumoca に依存しない（wasm32 でビルドでき、Track B の自前コアにもそのまま使える）。
- Rumoca v0.10.2 の DAE は、変数ごとに `fixed` / `start` / 束縛と、初期方程式（`initialization_owners()`）を公開している。調べた事実は [research/rumoca.md](../research/rumoca.md#s006-で調べた-apiv0102) にある。
- omc はこの環境に入っていない。OpenModelica のサイトが重くて取得できず、**omc との照合はユーザーの指示で後回し**にした（入れたら再開する）。

## 決定

### S7: Pantelides 法を `DaeStructure` の上で回す（`globe-mode-analysis/src/pantelides.rs`）

- **グラフの持ち方**: 未知数の「族」ごとに階数つきのノードを持つ。
  - 状態 `x` の族は階数 0（`x`）と 1（`der(x)`）から、代数・出力の族は階数 0 から始める。
  - 方程式 e は、`state_refs[e]` の状態を階数 0 で、`incidence[e]` の `der(x)` を階数 1 で、代数を階数 0 で参照する。e を d 回微分したものは、参照（族 f、階数 o）ごとに階数 o..=o+d を参照する。
  - マッチングの対象は各族の**最高階**のノードだけ。方程式 e と族 f は `o + d[e] == top[f]` のときに辺で結ばれる。
- **手順**（Pantelides 1988、Mattsson–Söderlind 1993 の説明に従う）:
  1. der の形の最大マッチング（Hopcroft–Karp）から始める。der の形は、最初の最高階のグラフそのもの。
  2. 対応の無い方程式ごとに、彩色つきの交互路を探す。探索は再帰ではなくスタックで行う（長い交互路でもスタックが溢れない）。
  3. 失敗したら、彩色した方程式をすべて 1 回微分し、彩色した族の最高階を 1 つ上げて、同じ方程式で探し直す。
- **結果**は `IndexReport { index, differentiations, orders, dummy_candidates, n_differentiated, dynamic_states, failed }`。
  - `index`（構造インデックスの見込み）= 方程式の微分の最大回数 + 1。微分が要らなければ 1。
  - `dummy_candidates`（状態選択の候補）= 1 回以上微分した方程式が参照する状態。
  - **`dynamic_states`（動的な状態の見込み）= 状態の数 + Σ（新しく作った微分のノード）− Σ（方程式の微分回数）**。拡張した系の「未知数 − 方程式」で、積分器の数にあたる。Pryce の Σ 法の自由度と同じ値になる。
- **S7 は情報にとどめ、実行を止めない**。
  - `GM-I7001`（Info）: インデックスが 2 以上。構造インデックスの見込みと、微分した方程式とその回数を補足に書く。
  - `GM-I7002`（Info）: 動的な状態 < 状態の数。状態選択（dummy derivative）が要る。候補の状態を補足に書く。
  - `GM-E7001`（Error）: 打ち切り（方程式数と未知数が違う、構造特異、微分の回数の上限）。S6 を通った構造では起きないはず。
- S7 は S6 が構造特異でない（regular）ときだけ回す。S6 の `StructuralReport::diagnostics` からは `GM-I7001` を外し、S7 の診断は `IndexReport` に一本化する。
- `dynamic_states` の最初の案「状態の数 − Σ（状態の階数 − 1）」は採らない。並列コンデンサ（`v1 = v2` を 1 回微分するだけで、階数の上がる族が無い）で 2 になり、正しい 1 にならない。

### S8: 初期条件の数と属性だけで判定する（`globe-mode-analysis/src/init.rs`）

- **入力**は `InitStructure { states, parameters, discretes, algebraics, initial_equations }`（スカラーごとの `fixed` / `has_start` / `has_binding` と位置）。フロントエンド（`globe-mode-frontend/src/init.rs`）が Rumoca の DAE から取り出す。
  - `has_start` は「**明示された** start」とする。Rumoca は start を書かなくても既定の start を補う（式の位置が宣言と同じか、由来が `DaeGeneration::DefaultStart`）ので、それを除く。型から受け継いだ start、式の start（`start = p`）、外からの修正（`Sub s(x(start = 2))`）は、明示されたものとみなす。
  - 初期方程式には、Rumoca が取り分ける定義も数える: Real でない `fixed = false` のパラメータを決める式（`initial_parameter_values()`）と、離散変数の初期化の時点の値を決める式や代入（`initial_discrete_values()`）。
- **数え方**:

  | | 数えるもの |
  | --- | --- |
  | 要る数 | 動的な状態（S7 の `dynamic_states`。S7 が無いか打ち切ったときは状態の数）+ 離散変数 + `fixed = false` で束縛の無いパラメータ |
  | 与えた数 | 初期方程式 + `fixed = true` の状態・離散変数・代数変数 |

  - **要る数には、宣言した状態の数ではなく S7 の動的な状態の数を使う**。インデックスが 2 以上の DAE では、状態の一部は微分した拘束で決まり、初期条件が要らないため。
  - `fixed = true` の状態の start は、初期方程式とは別に 1 つと数える。Rumoca v0.10.2 の ToDae は、それを初期方程式に変えないので 2 重には数えない。
  - 離散変数は `pre(d)` が初期化の未知数になる（MLS §8.6）ので、1 つずつ要る数に入れる。
- **診断**（Error は確かなときだけにする）:

  | 場合 | 診断 |
  | --- | --- |
  | 与えた数 > 要る数、離散変数が無く、動的な状態 = 状態の数 | `GM-E8001`（Error）。位置は最初の初期方程式（無ければ最初の `fixed = true` の宣言） |
  | 与えた数 > 要る数、離散変数がある、または動的な状態 < 状態の数 | `GM-W8004`（Warning。初期条件が多い見込みだが確定できない） |
  | 与えた数 < 要る数、不足 ≤ `fixed = false` の離散変数の数 | 柔らかい `GM-W8001`（「足りない見込み。離散変数の pre の初期値に start を使えば足りる。MLS 8.6」）。`GM-W8002` は出さない |
  | 与えた数 < 要る数、それより多く足りない | `GM-W8001`（不足分は start 値を推定値として使う見込み）と、start の無い状態ごとに `GM-W8002` |
  | 束縛も start も無い `fixed = true` のパラメータ | `GM-W8003`（数が合っていても出す） |

- **近似であることを受け入れる**: 数だけでは確定できない場合が 2 つある。どちらも Error にせず、警告にとどめる。
  - 離散変数が when の外の式で決まる（MLS §8.6 の暗黙の `pre(v) = v` で初期条件が要らない）か、when の中で決まるかは、`InitStructure` では区別できない。離散変数があるときの過剰は確定させず（`GM-W8004`）、不足が離散変数の数に収まるときは柔らかい `GM-W8001` にする。
  - インデックスの低減で状態が減る（動的な状態 < 状態の数）とき、余った `fixed = true` が拘束と矛盾しない（整合する冗長な初期条件）かは、数値でしか決まらない。このときの過剰も `GM-W8004` にする。
- S8 は S0〜S7 に Error が無いときだけ回す。

### 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| S7 を Rumoca の構造解析（`rumoca-phase-structural`）に任せる | `globe-mode-analysis` を Rumoca に依存させないため（wasm32、Track B）。S6 と同じ方針 |
| S7 を Error にする（インデックス 2 以上で止める） | インデックスが 2 以上なのはモデルの誤りではない（MSL の正しい例題にも多い）。インデックスの低減は P3 のシミュレーションの側で行う |
| S8 で初期方程式の incidence を持ち、初期化系をマッチングする（どの初期方程式がどの変数を決めるか） | `InitStructure` に初期方程式の incidence が無く、フロントエンドの抽出から足す必要がある。初期化系は、S7 で微分した拘束と合わせて組む必要もある。S007 では数と属性で多くの誤り（fixed と初期方程式の重複、初期条件の不足）を捕まえられるので、S008 以降の候補にした |
| S8 の要る数を宣言した状態の数にする | `Index.Pendulum`（状態 4、自由度 2）に「2 個足りない」、`Index.ParallelCapacitors`（状態 2、自由度 1）に「1 個足りない」と、誤った警告が出た（作業の途中で直した） |
| 離散変数があっても過剰を `GM-E8001` にする | 離散変数の初期条件の要否を区別できないので、正しいモデルを Error にしうる。Error は確かなときだけにする |
| インデックスの低減で状態が減るときの過剰も `GM-E8001` にする | 最初の全例題のファネルで、MSL の正しい例題 3 本が S8 の Error になった（下の結果）。余った fixed が整合する冗長な初期条件かは、構造の検査では決まらない |
| omc（`checkModel`、初期化）と照合してから決める | omc が未導入（ユーザーの指示で後回し）。入れたら S5 の照合と合わせて行う |

## 結果

- **S007 で実装した**（2026-10-18）。
  - `globe-mode-analysis`: `pantelides.rs`（`analyze_index`、`IndexReport`）と `init.rs`（`analyze_init`、`analyze_init_with_index`、`InitReport`）。
  - `globe-mode-ir`: `InitStructure`・`InitVariable`。
  - `globe-mode-frontend`: `init.rs`（Rumoca の DAE から `InitStructure` を取り出す）。
  - `globe-mode-db`: `index_report`（`dae_shape` → S7）、`init_shape`（位置を落とした `InitStructure`、防火壁）、`init_report`（`init_shape` と `index_report` の `dynamic_states` → S8）。`check_model` は S0〜S8。クエリの連鎖は [ADR-0005](0005-lazy-query-db.md) の結果にある。
  - `globe-mode-core`: `codes` に `GM-I7002`、`GM-E7001`、`GM-E8001`、`GM-W8001`〜`GM-W8004`。
  - CLI: `globe check` に S7・S8 の行と、`--json` の `s7` / `s8` / `init`。
- **テストで確かめたこと**:
  - Pantelides 法のテスト 17 本と funnel の 1 本。
  - 乱択 3000 件で「S6 の `index_reduction_needed`（der の形に完全マッチングが無い）⇔ `index >= 2`」。
  - 乱択 1916 件で、微分の回数が Pryce の Σ 法の最小のオフセットと一致した（証明はしていない）。
  - db の端から端までのテスト `tests/stages.rs` 11 本（S7・S8）。
- **検証用のモデル**（`src/Modelica/GlobeModeValidation`、`globe check` の出力で確かめた）:

  | モデル | S7 | S8 |
  | --- | --- | --- |
  | `Index.Pendulum`（直交座標の振り子） | インデックス 3。微分は拘束 `x ^ 2 + y ^ 2 = L ^ 2` を 2 回、`der(x) = vx` と `der(y) = vy` を 1 回ずつ。候補 x, y, vx, vy、動的な状態 2 / 4 | 2 / 2（fixed は x と vx） |
  | `Index.ParallelCapacitors` | インデックス 2。`v1 = v2` を 1 回。動的な状態 1 / 2 | 1 / 1（fixed は v1） |
  | `Invalid.HighIndexDuplicate` | インデックス 2。動的な状態 0 / 1 | 与えた 1 / 要る 0 → `GM-W8004`（インデックスの低減で状態が減るので確定させない。警告だけで通る） |
  | `Init.Balanced` | 1 | 2 / 2 |
  | `Init.OverDetermined` | 1 | 2 / 1 → `GM-E8001`（位置は `x = 2`） |
  | `Init.UnderDetermined` | 1 | 0 / 2 → `GM-W8001`、`GM-W8002`（h2） |
  | `Init.ParameterWithoutValue` | 1 | 1 / 1、`GM-W8003`（T） |
  | `Init.FreeParameter` | 1 | 2 / 2（`fixed = false` の p を定常状態の初期方程式で決める） |
  | `Connect.RealCircuit`、`GlobeModeExamples.RC` | 1 | `GM-W8001`・`GM-W8002`（fixed の無い RC。警告だけ） |

  - S8 の列は「与えた数 / 要る数」（`globe check` の表示と同じ順）。
  - 振り子の fixed を x と vx にしたのは、糸が水平（x = L）で x を固定すると ∂(x² + y²)/∂y = 2y = 0 で y が決まらないため。x = 0.6 から放す形にした。
- **MSL での S7・S8**（`globe check -L data/msl`。S8 は「与えた数 / 要る数」）:

  | モデル | S7 | S8 |
  | --- | --- | --- |
  | `Modelica.Electrical.Analog.Examples.ChuaCircuit` | 1（微分なし） | 3 / 3 |
  | `Modelica.Mechanics.Rotational.Examples.First` | インデックス 3、動的な状態 4 / 8 | 4 / 4 |
  | `Modelica.Blocks.Examples.PID_Controller` | インデックス 3、動的な状態 6 / 10 | 6 / 9（要る数は動的な状態 6 + 離散 3）。柔らかい `GM-W8001` だけ |
  | `Modelica.Blocks.Examples.DemonstrateContinuousSignalExtrema` | — | 21 / 21 |
  | `Modelica.Blocks.Examples.DemonstrateSignalExtrema` | — | 11 / 11 |

  - 「—」は S007 の報告に記録が無いもの。
  - 下の 2 本は、離散変数と代数変数の `fixed = true` を数えていなかったときに、誤って `GM-E8001` になった（`DemonstrateContinuousSignalExtrema` は要る 3 に対し与えた 9）。数えるように直して、上の数になった。
- **MSL の全例題**（`cargo xtask msl-funnel`、[doc/msl-funnel.md](../msl-funnel.md)）:
  - **最初の実行**（過剰の規則を直す前。S8 の過剰はすべて `GM-E8001`）: OK 366・NG 16（S5 1・S6 12・S8 3）・GM-U 184・panic 0。
    - S7 を回した 369 本: 構造インデックス 1 が 215、2 が 58、3 が 95、4 以上が 1（`Modelica.Blocks.Examples.FilterWithDifferentiation`。微分 14 本・最大 3 回）、打ち切り 0。状態選択が要るもの 154。
    - S8 まで来た 369 本: 合う 111・過剰 3・不足 255・入力なし 0。`GM-W8003` が出たもの 2（`Modelica.Fluid.Examples.InverseParameterization`、`Modelica.Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter`）。
    - S6（der の形）と Rumoca の照合 381 / 381。検査 881.8 s（1 本 1.56 s）、最大 RSS 7.1 GiB。
    - **S8 の Error になった 3 本は MSL の正しい例題**だった: `Modelica.Electrical.Analog.Examples.GenerationOfFMUs`（与えた 8 / 要る 6）、`Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks`（60 / 57）、`Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs`（4 / 3）。どれもインデックス 2 で、拘束でつながった状態すべてに `fixed = true` がある。これを受けて、過剰を `GM-E8001` にするのは「離散変数が無く、動的な状態 = 状態の数」のときだけにした（上の決定）。
  - **最終の実行**（規則を直した後）: 566 本: OK 369（うち S7 の情報 154）・NG 13（S5 1・S6 12・S7 0・S8 0）・GM-U 184（82 / 52 / 50）・panic 0。OK・NG の本数は S005 と同じ。S7 を回した 369 本の構造インデックスは 1 が 215・2 が 58・3 が 95・4 以上が 1・打ち切り 0、状態選択が要るもの 154。S8 の 369 本は合う 111・過剰の見込み 3（`GM-W8004`。Error 0）・不足 255・入力なし 0、`GM-W8003` あり 2。S6 と Rumoca の照合 381 / 381。検査 833.0 s（1 本 1.47 s）、最大 RSS 7.2 GiB。
- **残した近似と、次にすること**:
  - 離散変数が when の中で決まるかどうかの区別。
  - 初期方程式の incidence（どの初期方程式がどの変数を決めるか）。数が合っていても、決まらない変数や重複した初期方程式は見つけられない。
  - S7 の結果を Rumoca 自身のインデックス低減（`rumoca-phase-structural/src/dae_transform.rs`）と照合することは、S007 ではしていない。
  - omc との照合（omc 未導入。入れたら再開する）。
