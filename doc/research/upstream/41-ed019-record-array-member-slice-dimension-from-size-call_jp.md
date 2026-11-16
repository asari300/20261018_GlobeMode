# 下書き（未提出）41: レコードの配列の成分（`aw.re`・`aw[1].re`・`aw[i].re`）が、配列の大きさを `size(...)` で決めると ED019 `record-array member slice` で止まる（例 `Complex aw[size(a, 1)]`）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [41-ed019-record-array-member-slice-dimension-from-size-call.md](41-ed019-record-array-member-slice-dimension-from-size-call.md)、再現は [.mo](41-ed019-record-array-member-slice-dimension-from-size-call.mo)。

判定（GlobeMode、S371）: **バグ**。レコードの配列の要素の成分は普通の成分参照で、レコードの配列への `a.m` は「配列 `{a[1].m, ...}` を返す」スライド操作（MLS 3.7 の 10.6.9: 「レコードの配列にメンバーアクセス演算子を使うと、各レコードのメンバーを選んで配列を作る」）。Rumoca は配列の大きさがリテラルのとき、または `n = 3` のような `Integer` の parameter のときはこれらの参照を通し、大きさを `size(...)` で計算する形（`ndims(...)`・`integer(...)` も）では `[ED019] unsupported Flat semantic owner 'record-array member slice': reference scope is not an instantiated class occurrence` で止まる。flat のモデルはできていて、DAE を作るところで出る。omc 1.27.2（`checkModel`）はこれらのモデルをすべて通す。MSL の `Modelica.ComplexBlocks.Examples.ShowTransferFunction` は、この理由で Rumoca がコンパイルできない。

状態（S371、2026-11-16）: v0.10.2（GlobeMode の固定）と v0.10.4（最新のリリース）で再現。Rumoca の main（`b55eafce`）はビルドしていない。ソースには同じ文言がある（補足）。

## 概要

`R aw[size(a, 1)]`（`a` は parameter の配列、`R` は `Real` の成分が 2 つのレコード）を持つモデルで、`aw` の成分を 1 つでも参照すると断られる。

```text
[ED019] unsupported Flat semantic owner `record-array member slice`:
  reference scope is not an instantiated class occurrence
```

3 種類の参照のどれでも出る。スライド `sum(aw.re)`、1 つの要素の成分 `aw[1].re`、方程式の中の要素の成分 `aw[i].re = a[i]`（最初の参照を指す）。モデルを部品として使うかには依らない。最上位のクラスでも、別のモデルの部品でも同じ。

決めるのは大きさの書き方。モデルの本体を同じにして、`R aw[3]`、`parameter Integer n = 3` の `R aw[n]`、`constant Integer n = 3`、`n = 2 + 1`、`n = max(1, 3)`、`n = min(3, 4)`、`n = abs(-3)`、`n = div(7, 2)`、`n = mod(7, 4)` は通る。`R aw[size(a, 1)]`（`a` は parameter・constant・大きさ固定の parameter の配列）、`size({1, 2, 3}, 1)`、`size(ones(3), 1)`、`n = size(a, 1)`（`Integer` の parameter）、`ndims({1, 2, 3}) + 2`、`integer(3.2)` は通らない。これらの変種は、下の `A` のようなモデルで大きさだけを変えて見つけた。`.mo` に入れたのは失敗する形の `A`・`E` と、対照の `F`・`G`。

成分を 1 つも参照しない形（`for` 文の `aw[i] = R(a[i], 0)` だけ）は、下書き 39 の `ED001`（方程式 3 本・未知数 6 個）になる。

flat のモデル（`--emit flat-json`）はどちらもできる。通る大きさでは変数が `aw[1].re`・`aw[1].im`…で、`instance_relations` には要素ごとに `Aggregate` の実体（`indices` が `[1]`・`[2]`・`[3]`）があり、それぞれに `R` の `Class` の実体がある。`size(a, 1)` では変数が `aw.re`・`aw.im`（`dims` は `[3]`）で、`Aggregate` の実体は 1 つ（`indices` は `[]`）、`R` の `Class` の実体も配列全体で 1 つ。`--emit dae-json` と `--emit dae-mo` は ED019 で止まる。

MSL では `Modelica.ComplexBlocks.ComplexMath.TransferFunction` が `Complex bw[size(b,1)]; Complex aw[size(a,1)];` を持ち、31 行目の `bSum = Complex(sum(bw.re), sum(bw.im));` で出る。

## 再現手順

自己完結（普通のレコード。このファイルだけを置いたディレクトリで回す。同じ場所の `.mo` も読むため）。

```bash
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.A --emit dae-mo   # size(a, 1)、スライド sum(aw.re)
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.B --emit dae-mo   # size(a, 1)、1 つの要素の成分 aw[1].re
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.C --emit dae-mo   # A を部品として使う
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.D --emit dae-mo   # size(a, 1)、for 文の中の aw[i].re = ...
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.E --emit dae-mo   # n = size(a, 1)（Integer の parameter）、スライド
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.F --emit dae-mo   # 対照: リテラルの大きさ 3
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.G --emit dae-mo   # 対照: n = 3
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.H --emit dae-mo   # 対照: size(a, 1) で大きさを決めた Real の配列
```

MSL の `Complex`（4.1.0。ソースの根は `Complex.mo` だけでよい）で。`ComplexSlice.mo` として保存する。

```modelica
model Q "size from size(a, 1), the field of all elements read as a slice"
  parameter Real a[:] = {1, 2, 3};
  Complex aw[size(a, 1)];
  Real s;
equation
  for i in 1:size(a, 1) loop
    aw[i] = Complex(a[i], 0);
  end for;
  s = sum(aw.re);
end Q;

model K "control: literal size 3"
  Complex aw[3];
  Real s;
equation
  for i in 1:3 loop
    aw[i] = Complex(i, 0);
  end for;
  s = sum(aw.re);
end K;
```

```bash
rumoca compile ComplexSlice.mo -m Q --source-root <MSL 4.1.0 のパス>/Complex.mo --emit dae-mo
rumoca compile ComplexSlice.mo -m K --source-root <MSL 4.1.0 のパス>/Complex.mo --emit dae-mo
```

MSL の例題（`<MSL root>` は `Modelica 4.1.0` を含むフォルダ）。

```bash
rumoca compile "Modelica 4.1.0/package.mo" -m Modelica.ComplexBlocks.Examples.ShowTransferFunction --source-root <MSL root> --emit dae-mo
```

## 期待

`A`〜`E` は対照の `F`・`G`・`H` と同じく通る。`a = {1, 2, 3}` のときの `size(a, 1)` は 3 で、`R aw[3]` と同じ配列になり、`aw.re` は配列 `{aw[1].re, aw[2].re, aw[3].re}` になる（MLS 3.7 の 10.6.9）。`Q` は `K` と同じく通り、`Modelica.ComplexBlocks.Examples.ShowTransferFunction` も通る。

## 実際

`rumoca compile ... --emit dae-mo` の最初のエラー行（v0.10.2 と v0.10.4 で同じ）。

| モデル | 結果 | 最初のエラーの位置 |
| --- | --- | --- |
| `A` `size(a, 1)`、`sum(aw.re)` | `ED019` record-array member slice | `sum(aw.re)` の `aw.re`（21:13） |
| `B` `size(a, 1)`、`aw[1].re` | `ED019` record-array member slice | `aw[1].re`（36:9） |
| `C` `A` を部品として使う | `ED019` record-array member slice | `sum(aw.re)` の `aw.re`（21:13） |
| `D` `size(a, 1)`、`aw[i].re = a[i]` | `ED019` record-array member slice | `aw[i].re`（54:7） |
| `E` `n = size(a, 1)`、`sum(aw.re)` | `ED019` record-array member slice | `sum(aw.re)` の `aw.re`（72:13） |
| `F` リテラルの大きさ 3（対照） | 通る | |
| `G` `n = 3`（対照） | 通る | |
| `H` `size(a, 1)` で決めた `Real` の配列（対照） | 通る | |
| `Q` MSL の `Complex`、`size(a, 1)`、`sum(aw.re)` | `ED019` record-array member slice | |
| `K` MSL の `Complex`、リテラルの大きさ 3（対照） | 通る | |
| MSL の `ShowTransferFunction` | `ED019` record-array member slice | `TransferFunction.mo:31:22`、`sum(bw.re)` の `bw.re` |

`行:列` はこのフォルダの `.mo` のもの。MSL の例題のメッセージも同じ文面。

```text
[ED019] unsupported Flat semantic owner `record-array member slice`:
  reference scope is not an instantiated class occurrence
  ...TransferFunction.mo:31:22
  30 |   aw = {a[i]*(w)^(size(a,1)-i)*powerOfJ(size(a,1)-i) for i in 1:size(a,1)};
  31 |   bSum = Complex(sum(bw.re), sum(bw.im));
```

omc 1.27.2 の `checkModel`（最後の 3 つは `loadModel(Modelica, {"4.1.0"})` のあと）: `A` は方程式 7・変数 7、`B` 7 と 7、`C` 7 と 7、`D` 6 と 6、`E` 7 と 7、`F` 7 と 7、`G` 7 と 7、`H` 7 と 7。`Q` 7 と 7、`K` 7 と 7。`Modelica.ComplexBlocks.Examples.ShowTransferFunction` は 29 と 29。

## Rumoca v0.10.4 での再確認（S371、2026-11-16）

どちらの版も配布の CLI（`rumoca 0.10.2`・`rumoca 0.10.4`）で回した。上の表がそれ。v0.10.4（タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、2026-11-02 公開）は 2026-11-16 でも最新のリリース（releases API）。`.mo` のどのモデルも、2 つの版で結果が変わらない。

2026-10-31 の GlobeMode の funnel の結果（v0.10.2、MSL の例題 566 本のうち 2 本）で同じメッセージが出ていたもう 1 本は `Modelica.Electrical.Batteries.Examples.ShowImpedance` で、`Electrical/Batteries/Utilities/Impedance.mo` の 12 行目 `cellData.rcData[k].R` で出る。v0.10.2 は ED019 で止まり、v0.10.4 は通り抜ける（`--emit dae-json` は終了コード 0。`--emit dae-mo` はそのあとコード生成で列挙型の `EC002` で止まるが、これは別の限界）。これはパラメータのレコードの中のレコードの配列（`Electrical/Batteries/ParameterRecords/TransientData/CellData.mo` の `parameter RCData rcData[nRC]`、`nRC` は `Integer` の parameter）で、この下書きの `size(...)` の大きさではない。v0.10.2 がそこで止まり v0.10.4 が止まらない理由は追っていない。

## 環境

- Rumoca v0.10.2（配布の CLI `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布の `rumoca 0.10.4`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）。Rumoca の main（`b55eafce`、2026-11-13）はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。MSL 4.1.0。omc 1.27.2。

## GlobeMode 側の回避

自前のフロントエンドでは要らない。自前の flat（`globe-mode-flat`）は、大きさを評価できるレコードの配列を要素に展開して（`aw[1].re`…）成分の参照を残すので、MSL の例題も平坦化できる（S337、[ADR-0039](../../adr/0039-native-cst-parser.md)）。Rumoca を通す検査（`globe check`・`msl:funnel`）では、この例題は S5 で止まり、Rumoca のコード `ED019` で `GM-U0001`（未対応。誤りではない）に振り分けられる（[msl-funnel.md](../../msl-funnel.md) の 109 行目）。自前の flat と Rumoca の flat はここで違う。Rumoca は 1 つの変数 `transferFunction.aw.re`（大きさ 3）で、自前は `transferFunction.aw[1].re`…（名前の食い違いとして `diff-frontend` に出る。ADR-0039 の S334・S337）。

## 補足

- 追っていないこと: 大きさの書き方で結果が変わる理由。原因は DAE を作る段にあるように見える。Rumoca の main（`b55eafce`。GitHub の contents API でソースを読んだ。ビルドしていない）では、メッセージ「reference scope is not an instantiated class occurrence」は `crates/rumoca-phase-dae/src/construction/analysis/record_array_fields.rs` の `component_ancestry` が、スコープの実体の種類が `Class` でないときに返す。`A` でスコープがどの実体かは分からない。main の成分のスライドの回帰試験（`crates/rumoca/tests/suite_core/record_array_member_slice_test.rs`。`Pin pin[3]` の `pin[:].v`）には、`size(...)` の大きさは無い【ファイルを読んでの推測。回していない】。
- funnel の結果でこのメッセージが出る MSL の例題 2 本のうち、`size(...)` の形で v0.10.4 でも止まるのは 1 本（`ShowTransferFunction`。`Complex bw[size(b,1)]`）。両版で確かめたのはこの例題だけで、ほかのライブラリの例題は見ていない【未確認】。GlobeMode の自前の flat と Rumoca の flat の突き合わせ（`diff-frontend`、S351）で `ShowTransferFunction` は方程式の数が 0/1 だったが、ED019 は数える前に出る。下書き 39（レコードの配列の方程式の数）は、これが直ってから初めてこの例題に届く。
- `D` は、参照がスライドでなくてもよいことを示す。方程式の左辺の要素の成分でも同じエラーが出るので、`member slice` という名前は MLS の 10.6.9 のスライドより広い。
- 出典: GlobeMode の S359（下書き 39 の `ShowTransferFunction` で見つけた。その備考は「同じモデルを最上位のクラスにすると通る」と書いたが、この形では通らない。`A` と、それを最上位で使う形はどちらも落ちる）と S371（この下書き）。MLS 3.7 <https://specification.modelica.org/maint/3.7/arrays.html>（10.6.9 Slice Operation。節の本文をページで読んだ。2026-11-16 確認）。Rumoca の main のソース: <https://github.com/CogniPilot/rumoca/blob/b55eafce/crates/rumoca-phase-dae/src/construction/analysis/record_array_fields.rs>（2026-11-16 に読んだ）。
