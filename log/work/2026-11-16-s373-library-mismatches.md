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

- コードの検査: `cargo fmt --check`（全体）、`cargo clippy -p globe-mode-dae -p globe-mode-frontend --all-targets --features globe-mode-frontend/native -- -D warnings`、`cargo test -p globe-mode-dae -p globe-mode-flat -p globe-mode-analysis`（dae 21 本・flat 107 本ほか全部通過）、`cargo test -p globe-mode-frontend --features native`（単体・結合 49＋1＋20＋19＋6＋3＋1＋1＋2＋1 本ほか通過。ドキュメントテストだけ共有 target の古い rlib で `E0460`。別の worktree のビルドのせいで、このコードとは関係が無い）。`mise run check` の全体（workspace の全 crate の試験・deny）は回していない（依存の追加なし。変えたのは `globe-mode-dae` の 1 関数と試験だけ）。
- 試験が直す前に落ちること: `record_vars` の変更を戻して `slice_of_a_component_array_field_touches_every_element` を回し、`is_balanced() && is_regular()` で落ちた。
- **全体の検査**（写したバイナリ `xtask-s373d` を `flock` の下で。S374 を取り込んだあとの main の `7aa4e1e9` を足した状態。基準は `bin-copies/frontend-diff-s374.json`）: 882 s、最大 RSS 6.0 GiB。**回帰 0・改善 0**。MSL 566 本: parse 566/566・resolve 566/566・flat 565/566・名前 529/532・方程式の数 517/532・DAE 565/565・釣り合い 382/382・数 381/382・状態 381/382・構造の正則 382/382。`GlobeModeValidation` 75 本: 75/75・Rumoca も通った 74 本で一致・正則 73/73。（S374 を取り込む前に S370 の基準 `frontend-diff-s370c.json` でも同じ結果で回帰 0・改善 0、992 s）
- **ライブラリ 7 つ 2132 本**（手元の release、`xtask-s373b`〜`s373d`・`data/libs` は `fetch-library` で取得）: 平らにできた 1188 本（AixLib 362・Buildings 396・IBPSA 287・ThermofluidStream 117・DeltaRobot 16・Chemical 10・ClaRa 0。S369 の run 43772634191 と同じ）、Rumoca も flat まで通った 572 本で名前 568・方程式の数 562、DAE 1188 本、Rumoca も釣り合いまで通った 335 本で釣り合い 335・方程式と未知数の数 335・状態 334・構造の正則 **335/335**（直す前は 334/335。`WindowSimple`）。食い違いの一覧は全部上の表の原因（新しい食い違いは 0 本）。Buildings の `CorrectedNFactors`（Rumoca が 49 分）と ClaRa（5 本、全部断る）は手元で回していない（S369 の run の結果を使った）。
- **S374 の関数の評価の予算（1 回の flat で 100 万）に当たった例題は 0 本**。手元だけの probe（`fneval.rs` の `tick_n` が上限を超えた最初の 1 回に stderr へ `FNBUDGET` を出し、`diff-frontend` が例題ごとに名前を出す。コミットせずに戻した）で 2126 本を回した: DeltaRobot 21・Chemical 76・ThermofluidStream 122・IBPSA 457・AixLib 642・Buildings 808（`CorrectedNFactors` を除く）。`FNBUDGET` は 0 回。probe の動作は、`for` を 60 万回回す関数で次元が決まる小さなモデルで確かめた（`FNBUDGET` が 1 回出る）。**範囲の反復数・配列の大きさが `MAX_ITERS` を超える形（`n <= MAX_ITERS` の絞り込み）は probe の対象外**（予算を数える `tick_n` を通らずに分からないになる）。S374 を取り込んだ後の平らにできた本数も同じ 1188 本で、断る理由に増えたものは無い。
- **`libraries.yml`（feature の push。S374 を取り込んだあとの `610ba2ca` で `gh workflow run libraries.yml --ref feature/s373-library-mismatches`、run 43789815281、全ジョブ success）**: `diff-frontend` の 11 分割を足した数字が手元と同じ: 平らにできた 1188 本・Rumoca も flat まで通った 572 本で名前 568・方程式の数 562・DAE 1188 本・釣り合い 335・数 335・状態 334・構造の正則 335/335。`too-large` で断った例題 0 本・自前の panic 0 本。食い違いの一覧（14 行）は上の表の原因だけ（`WindowSimple` は出なくなった）。Buildings の 4 分割目は diff-frontend だけで 3214 s（Rumoca が `CorrectedNFactors` に 49 分。S369 と同じ）。最初に S374 を取り込む前の `cd6641a8` で dispatch した run 43786102907 は、`libraries` の別の run（スケジュール、main の `b4019e6e`）が動いていたので待機中のまま、S374 の取り込みのあとで取り消して出し直した。スケジュールの run 43782125765（main の `b4019e6e`、この WP の修正なし）の `diff-frontend` の表は、自前で平らにできた本数・Rumoca との一致の本数が上と同じ（修正前の `WindowSimple` の構造の正則だけが違う）。
- `check_docs.py`: デッキ（S373 を含む全部）OK / リンク 文書 1363 件・相対リンク 5625 件の切れ 0 件。`render_slides.py`（自分のデッキだけ。4 枚）: デッキ 1 件 / 問題 0 件。上流の下書き 42〜44 のスライド（`_jp.html`）も `render_slides.py` で問題 0 件。`markdownlint-cli2`（MD013 を除く）: 追加・変更した `.md`（英語の下書きの裸の URL〔MD034〕は 40・41 と同じ書式のまま）で 0 件。

## 残したこと

- 42・43・44 は Rumoca の main で確かめ直していない（v0.10.2・v0.10.4 だけ）。出すかはユーザーが決める。
- 44 と 40 の根が同じかは、AixLib の例題を切り出していないので分からない（CLI が AixLib の木で止まる）。
- `libraries.yml` の `diff-frontend` の自前の flat が断る理由（`unsized-array`・`redeclare`・`connect-array`・`inner-outer`・`non-constant-dim` ほか）は自前の穴で、この WP の範囲外（S369 の run: `unsized-array` 371・`expandable` 266・`composite-binding` 141・`inner-outer` 63・`redeclare` 58・`connect-array` 58・`non-constant-dim` 53 ほか。S367・S370 のあとの本数は `libraries.yml` の報告を見る）。
- Rumoca の `rumoca compile`（v0.10.2）が Buildings で segfault する件は原因を調べていない。
