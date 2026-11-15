# S297 `flat-redeclare`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s297-flat-redeclare` |
| worktree | `.git-worktree/s297-flat-redeclare` |
| Phase | Track B（B3 の 3 つ目） |
| セッション文書 | `doc/sessions/S297-flat-redeclare/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

S284 の `globe-mode-flat` が断る理由の最上位だった `redeclare`・`replaceable`・`class extends` を平坦化で扱う。部品の `redeclare T x`・`redeclare package Medium = X`・`redeclare model M = X`・`extends B(redeclare …)`・`replaceable` の既定の型を、インスタンス化のときの型の差し替えとして修飾の伝播と同じ順（外側が勝つ）で当てる。`Modelica.Media` の関数は評価しない。`globe-mode-flat` の中だけを変える。

## 決めたこと

- `redeclare …` は syntax では字句の並び（`Redeclaration` 節点に節点の子が無い）なので、`model R <本文>; end R;` に包んで読み直して宣言にする。syntax・resolve を変えない約束を守るため。名前は書いた場所のクラス（`Library::class_at`）から `resolve_head` で引き、仮のファイル番号の表に入れて、既存の `res_of` と `cref`・`eval` がそのまま使えるようにした。表は `Box::leak` で `&'static` にした（節の数は `Redeclaration` の数で頭打ち。長く動かすなら見直す）。
- 型の差し替えは、書いた場所で型を解決しておく（`Mod::redecl`・`Ctx::cenv`）。入れ子へは `redeclare package M = M` が運ぶ。
- `replaceable` は既定の型で平らにする。`constrainedby` は見ない。
- `Medium.BaseProperties` のように差し替え後のパッケージを通した型・定数は、そのパッケージの `extends` の修飾も合わせて引く（`pkg_env`）。受け継いだクラスの中の `ThermodynamicState`・`nXi` も差し替え後で引き直す（`Ctx::heir`）。別名 `package Medium = MoistAir`・パッケージの中のモデルを直接平らにするとき（`TwoPhaseWater.BaseProperties`）も同じ。関数の呼び出しは式のまま。
- 範囲を少し広げたもの（redeclare を通すと次の理由で止まったので）: 長い形の `type T extends Real;`、`type Axis = Real[3]`、`a + b = zeros(n)` の方程式の数。
- `protected` のコネクタは親から見て「つながっていない内側のコネクタ」の flow = 0 の対象にしない。最上位は `protected` も対象にする（`CauerLowPassOPV` が Rumoca の flat と合うのはこの形）。

## やったこと

- `src/Rust/mod/globe-mode-flat/src/lib.rs`: `Project::snippet`（読み直しと名前の解決）、`class_ext_base`（`lineage` にも入れる）。
- `src/Rust/mod/globe-mode-flat/src/flatten.rs`: `Redecl`・`Cenv`・`Ctx::{cenv,heir}`、`redeclaration`・`class_res`・`member_class`・`heir_member`・`alias_heir`、`pkg_env`・`pkg_binding`・`dyn_const`、`type_of` の長い形の型と型の配列、`Item::Decl::public`、`collect` の `class extends`。
- `src/Rust/mod/globe-mode-flat/tests/flat.rs`: 単体 6 本を足し（部品の redeclare、package の redeclare と class extends、heir、別名のパッケージ、extends の中の redeclare record、型の配列と `type extends`）、`declined_forms` の 2 か所を新しい動きに合わせた。`tests/compare.rs`: `cmp`（`GLOBE_FLAT_MODEL=A,B` で Rumoca の flat と 1 本ずつ比べる）を足した。
- doc: ADR-0039 の付記（S297）、roadmap（Track B の 2 か所）、architecture の crate 表、features、セッションの索引、セッション文書。

## 検証

- `cargo test -p globe-mode-flat`: 16 pass・5 ignored（S284 は 10 pass）。`cargo clippy -p globe-mode-flat --all-targets -- -D warnings` 警告 0、`cargo fmt` 済み。依存の追加なし（`cargo deny` は対象外）。
- `GlobeModeValidation` の 75 本: 自前で平らにできた 72 → **74**（断ったのは `stream` 1）。両方で通った 73 本で名前 **73/73**・方程式の数 **73/73**（S284 は 71/71）。
- MSL 4.1.0 の例題 566 本（debug、読み込み約 20 秒・flatten 約 7 秒）: 自前で平らにできた 192 → **279**。Rumoca の flat まで通った 273 本で名前 **273/273**・方程式の数 **254/273**（S284 は 189/189・186/189）。合わない 19 本は `Clocked` 4（`UniformNoise` 系・`SampleWithADeffects`・`ClockedWith_AD_DA_Effects`）、`QuasiStatic` 11（`Electrical.QuasiStatic` 2・`Magnetic.QuasiStatic.FluxTubes` 9。`Complex` と `Connections.isRoot` の数え方と見ているが【未確認】）、DC 機械 2（`DCPM_Drive` は 597 対 601、`DC_CompareCharacteristics` は 544 対 548。`protected` のコネクタの数え方が違うと見ているが【未確認】）、`SuperCapDischargeCharge`（71 対 72）、`GenerationOfFMUs`（S284 から）。
- 断った理由の内訳（最初の理由だけ。S284 → S297）: `redeclare` 263 → 0、`replaceable` 9 → 0、`class extends` 11 → 0、配列の型 13 → 0、レコードの束縛 41 → 155、配列のコネクタの一括接続 7 → 72、`[:]` 13 → 20、条件つきの宣言 6 → 19、`connect` 7 → 8、配列の修飾 0 → 8、`non-constant-dim` 2、`expandable` 2、`stream` 1。先へ進んだ分が別の理由に当たるので、増えた行もある。
- 注意: 共有 target に別の worktree の core・syntax・resolve のビルドが混ざり、同じコードで `globe-mode-frontend` がコンパイルできない（`ModelParameter` の `enum_type`）・結果が戻ることが何度もあった。`touch src/Rust/mod/globe-mode-{core,fmi,fmt,ir,syntax,resolve,flat,frontend}/src/lib.rs` のあとに回した数字だけを載せた。MSL の計測は 3 GB の RAM の WSL で落ちたので `setsid nohup` で端末から切り離して回した。
- `check_docs.py`・`render_slides.py`・`mise run check`: 下の「締め」を参照。
- 漏洩の検査: 下の「締め」を参照。

## 残したこと

- 後ろで宣言したレコードを欄ごとの修飾にする（`frictionParameters = dcpmData.frictionParameters`、46 本）。宣言を入れ替えて再試行する（途中までの変数・方程式を戻す）作りが要る。
- 関数の戻り値のレコード（`R = Frames.nullRotation()`、27 本）・外部オブジェクト（`tableID`、29 本）、配列のコネクタの一括接続（72 本）、`Connections.isRoot` の評価、方程式の数が合わない 19 本の原因。
- `xtask diff-frontend` への組み込み、B4（DAE）。
