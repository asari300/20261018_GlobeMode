# S373 `library-mismatches`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s373-library-mismatches`（main の `b4019e6e`〔S370 のマージ〕から切った） |
| worktree | `.git-worktree/s373-library-mismatches` |
| Phase | Track B（B4 DAE）・上流の下書き |
| セッション文書 | `doc/sessions/S373-library-mismatches/` |
| セッション記録 | 専用の記録が `log/local/` に無いので省いた（並行の担当のうち、この WP の記録は取っていない） |

## 依頼

S369 の `libraries.yml`（run 43772634191、main は S367 まで）で、Rumoca も通った例題の名前・方程式の数・DAE が自前と合わないもの（S369 の作業記録に 13 本 + DAE 2 本）を集め直して原因ごとに分け、自前の誤りなら直す。Rumoca の誤りなら直さず、根拠を作業記録と `doc/upstream.md` の書式で残す（上流には出さない）。各修正に小さな回帰試験。最後に MSL の全体の検査で回帰 0 を確かめ、直したライブラリの例題は `--class` で一致を確かめ、`libraries.yml` を feature で 1 回回して確かめる。

## 決めたこと

- **原因は 5 つで、自前の誤りは 1 つだけだった。** 13 本 + 2 本を、`--class … --dump` で部品ごとの方程式の数を Rumoca と並べ、食い違う部品の小さな再現を作って、`rumoca compile`（配布の CLI v0.10.2・v0.10.4）と omc 1.27.2 で確かめた。再現の置き場は `.git-worktree/target-shared/{tl,tl2,tl3,dump373}/`（コミットしない）。

  | 原因 | 例題（3 つのライブラリにまたがるものは 1 つと数える） | 判定 |
  | --- | --- | --- |
  | `Real h = f(X)`（`X` が配列、`f` の出力はスカラー）を読む方程式（`ports[1].h_outflow = h_internal`、`Boundary_pT`・`MassFlowSource_T`）を Rumoca が 2 本と数える | `EnthalpyFlowRate`・`MoistAirEnthalpyFlowRate`・`PrescribedOutlet_Water`（Buildings・IBPSA・AixLib） | **Rumoca の誤り。v0.10.4 で修正**（36 と同じ原因の別の形。36 に追記） |
  | `redeclare model OpeningArea = …(use_opnWidth_in = false, …)` の修飾が捨てられる | AixLib `VentilationFlowRateSashOpening`（`opnWidth_in` 2 変数） | Rumoca の誤り（40。44 と見分けていない。40 に追記） |
  | 差し替えたクラスが基底クラスの `final use_opnWidth_in = false` で旗を決めても、`if use_opnWidth_in` の成分が残る | AixLib `VentilationFlowRateSimpleOpening`（12 変数） | Rumoca の誤り（**新規 44**。v0.10.2・v0.10.4 で再現。omc は作らない） |
  | 組み込みの列挙 `AssertionLevel`（と `StateSelect`）の parameter を flat の変数にしない | ThermofluidStream `Exercise3OttoEngine.BaseModel`・`Exercise4DieselEngine.BaseModel`（`dropOfCommons.assertionLevel`） | Rumoca の軽い誤り（**新規 43**。omc は残す。自前は残す） |
  | 修飾 `vol(massDynamics = massDynamics)` で、既定の `massDynamics = energyDynamics` を `vol` の側で評価し、`if massDynamics == Steady` が `else` の枝になる | Buildings `DHC.Plants.Steam.BaseClasses.Examples.BoilerPolynomial`（DAE の状態 6 対 7） | Rumoca の誤り（**新規 42**。v0.10.2・v0.10.4 で再現。omc は `0 = f`） |
  | `s[:].h = h[:]`（部品の配列の欄の全要素）の左辺が DAE の行の incidence に入らず、構造が特異 | AixLib `ThermalZones.HighOrder.Components.Examples.WindowsDoors.WindowSimple` | **自前の誤り**。直した |

- **自前の直し方は 1 か所**（`globe-mode-dae` の `Ctx::record_vars` の範囲の枝）。`solRadOut[:].H` の `[:]` を展開した `solRadOut[1].H` は、レコードの欄の持ち主でなく**変数そのもの**なのに、`leaves_of` が「変数なら空」を返し、`take_while(非空)` が 1 つ目で止まっていた。変数ならその変数を返す。範囲は見ないので左辺は全要素に触れる（S362 の shortcut と同じ向き。正則は甘く出る側）。
- **AixLib の `PrescribedSolarRad` 以外にも、同じ形の `a[:].x = b[:]`・`a[1:n].x` が MSL にあるかは数えていない**。全体の検査（下）で、回帰 0 であることだけ確かめた。
- **42・43・44 は 4 点セット（英語 `.md`・`.mo`・日本語 `_jp.md`・スライド 1 枚 `_jp.html`）にし、36・40 に追記した**。出さない（出すのはユーザー）。36 の追記は、v0.10.4 の CLI で `Buildings.Fluid.Sensors.Examples.EnthalpyFlowRate` を回し、フラットな方程式の数が 61（自前と同じ）になることで確かめた（v0.10.2 の道は 63）。`PrescribedOutlet_Water` は v0.10.4 の CLI での数え直しをしていない（`sou1`〜`sou4`・`sin` の 5 つが `scalar_count` 2 になっていたことは v0.10.2 の `--dump` で確かめた）。
- **v0.10.2 の CLI は Buildings の例題で segfault（終了コード 139）する**（`rumoca compile … --source-root data/libs/Buildings`）。スタックと見られるが調べていない。ライブラリの API（`diff-frontend` が呼ぶ道）は通るので、そちらを使った。v0.10.4 の CLI は通る。AixLib は両版の CLI が `PKG-008`（`RadiatorTypes` の部分ディレクトリと `.mo` が並ぶ）で止まる。
- **`StateSelect` の parameter は自前の flat で落とすまま**（Rumoca に合わせる S 以前の決め）。`AssertionLevel` を落として Rumoca に合わせることはしない（omc と同じく残す方が正しいので、43 の「GlobeMode 側の回避」に書いた）。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `Ctx::record_vars` の範囲の枝。試験 `slice_of_a_component_array_field_touches_every_element`（直す前は `is_balanced() && is_regular()` で落ちることを、直しを戻して確かめた）。
- `doc/research/upstream/`: 42・43・44（各 4 ファイル）、36・40（英語と 40 の日本語）の追記、`README.md`（表の行 42〜44、節「ライブラリの食い違いの原因分け」）。
- `doc/upstream.md`: 英語の要約（48 → 49 件）、表の行 42〜44、36・40 の影響の欄、提出の順の「—」の行 3 つ、回避を外す条件。
- `doc/adr/0039-native-cst-parser.md`（付記 S373）・`doc/roadmap.md`（S373 の節と「今の数字」）・`CHANGELOG.md`・`doc/sessions/README.md`・`doc/sessions/S373-library-mismatches/`・この記録。`doc/frontend-diff.md`（`msl:diff-frontend` が書く）。

## 検証

TODO-VERIFY

## 残したこと

- 42・43・44 は Rumoca の main で確かめ直していない（v0.10.2・v0.10.4 だけ）。出すかはユーザーが決める。
- 44 と 40 の根が同じかは、AixLib の例題を切り出していないので分からない（CLI が AixLib の木で止まる）。
- `libraries.yml` の `diff-frontend` の自前の flat が断る理由（`unsized-array`・`redeclare`・`connect-array`・`inner-outer`・`non-constant-dim` ほか）は自前の穴で、この WP の範囲外（TODO-REASONS）。
- Rumoca の `rumoca compile`（v0.10.2）が Buildings で segfault する件は原因を調べていない。
