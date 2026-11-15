# 下書き（未提出）39: レコードの配列と、レコードの配列式（`aw = {R(1, 0), ...}`）の方程式を、フィールドごとでなく要素ごとに数える（v0.10.2 は ED008、v0.10.4 は ED001 の釣り合い）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [39-array-equation-of-record-array-constructor-counts-elements.md](39-array-equation-of-record-array-constructor-counts-elements.md)、再現は [.mo](39-array-equation-of-record-array-constructor-counts-elements.mo)、1 枚の解説は [_jp.html](39-array-equation-of-record-array-constructor-counts-elements_jp.html)。

判定（GlobeMode、S359）: **バグ**（方程式の数え方。v0.10.2 はその前の ED008 で止まる）。`R aw[3]`（フィールド 2 つ）に対する `aw = {R(1, 0), R(2, 0), R(3, 0)};` はスカラー方程式 6 本（MLS 3.7 の 10.6.1: レコードの等式は成分ごとに適用する。4.8: 方程式はプリミティブ型のスカラーに展開してから数える）。Rumoca は 3 と数える。同じモデルを `for` 文で書いた形や、もう一方の配列を別の書き方で決める `aw = bw` は正しく数える。omc 1.27.2（`checkModel`）はどれも「6 equation(s) and 6 variable(s)」。

状態（S359、2026-11-16）: **v0.10.4 で一部直った**。MSL の `Complex`（operator record）で大きさが書いた数のとき（`Complex aw[3]`）は v0.10.4 で通る（v0.10.2 は ED008）。普通のレコードと、大きさがパラメータの配列から来る `Complex`（`Modelica.ComplexBlocks.ComplexMath.TransferFunction` の形）は、v0.10.4 でも下の ED001。Rumoca main はビルドしていない【main は未確認】。

## 概要

レコードの配列があり、配列全体をレコードのコンストラクタ呼び出しの配列式（または配列の内包表記）で決める方程式が 1 本あると断られる。

```text
[ED001] unbalanced model: 3 equations, 6 unknowns (balance = -3)
```

flat のモデル（`--emit flat-json`）では、方程式 `aw = {R(1, 0), R(2, 0), R(3, 0)}` の `scalar_count` が 3、レコードの配列はスカラーの未知数 6（`aw[1].re`、`aw[1].im`、… 。大きさがパラメータの配列から来るときは大きさ 3 の配列変数 `aw.re`・`aw.im`）。両辺がレコードの配列（`aw = bw`）の方程式は v0.10.4 では 6 と数える。つまり配列式の側だけが要素ごとに数えられる。

v0.10.2 は、書いた数の大きさの同じモデルで `[ED008] unresolved Flat reference 'aw'`。パラメータの配列から来る大きさでは、上の ED001 が既に出る。

MSL では `Modelica.ComplexBlocks.ComplexMath.TransferFunction` の `Complex aw[size(a,1)]` と `aw = {a[i]*(w)^(size(a,1)-i)*powerOfJ(size(a,1)-i) for i in 1:size(a,1)};` がこの形。

## 再現手順

自己完結（普通のレコード。このファイルだけを置いたディレクトリで回す。コンパイラは同じ場所の `.mo` も読む）:

```bash
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.A --emit dae-mo   # 大きさ 3、配列式
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.B --emit dae-mo   # 大きさがパラメータの配列から、内包表記
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.C --emit dae-mo   # aw = bw、bw は配列式
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.D --emit dae-mo   # 対照: for 文
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.E --emit dae-mo   # 対照: aw = bw、bw は for 文
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.F --emit dae-mo   # 対照: Real の配列
```

MSL の `Complex`（4.1.0。ソースの根は `Complex.mo` だけでよい）。`ComplexForms.mo` として保存:

```modelica
model L "size 3, array constructor"
  Complex aw[3];
equation
  aw = {Complex(1, 0), Complex(2, 0), Complex(3, 0)};
end L;

model P "size from a parameter array, comprehension"
  parameter Real a[:] = {1, 2, 3};
  Complex aw[size(a, 1)];
equation
  aw = {Complex(a[i], 0) for i in 1:size(a, 1)};
end P;
```

```bash
rumoca compile ComplexForms.mo -m L --source-root <MSL 4.1.0 の場所>/Complex.mo --emit dae-mo
rumoca compile ComplexForms.mo -m P --source-root <MSL 4.1.0 の場所>/Complex.mo --emit dae-mo
```

## 期待

`A`・`B`・`C`・`L`・`P` が、対照の `D`・`E`・`F` と同じように通る。`aw = {R(1, 0), ...}` は要素ごとのレコードの方程式で、レコードの方程式 1 本はフィールドごとのスカラー方程式（`A` は 6、`C` は 12）。

## 実際

`rumoca compile ... --emit dae-mo` の最初のエラー行:

| モデル | v0.10.2 | v0.10.4 |
| --- | --- | --- |
| `A` 大きさ 3、配列式 | `ED008` unresolved Flat reference `aw` | `ED001` 方程式 3、未知数 6 |
| `B` 大きさ `size(a, 1)`、内包表記 | `ED001` 方程式 3、未知数 6 | `ED001` 方程式 3、未知数 6 |
| `C` `aw = bw`、`bw = {...}` | `ED008` unresolved Flat reference `bw` | `ED001` 方程式 9、未知数 12 |
| `D` for 文（対照） | 通る | 通る |
| `E` `aw = bw`、`bw` は for 文（対照） | 通る | 通る |
| `F` `Real` の配列（対照） | 通る | 通る |
| `L` MSL の `Complex`、大きさ 3、配列式 | `ED008` unresolved Flat reference `aw` | **通る** |
| `P` MSL の `Complex`、大きさ `size(a, 1)`、内包表記 | `ED001` 方程式 3、未知数 6 | `ED001` 方程式 3、未知数 6 |

`E` は v0.10.4 で、`aw = bw` を 6 と数えたときだけ釣り合う。だから `C` の `ED001`（9 と 12）の差は配列式の方程式だけから来る。

omc 1.27.2、`checkModel`: `A` は方程式 6・変数 6、`B` 6 と 6、`C` 12 と 12、`D` 6 と 6。`L`・`P`（MSL の `Complex.mo` と）は 6 と 6。

## v0.10.4 での再確認（S359、2026-11-16）

各リリースの配布バイナリの CLI（`rumoca 0.10.2`、`rumoca 0.10.4`）で回したのが上の表。v0.10.4（タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、2026-11-02 公開）は今も最新のリリース（`git ls-remote` と releases の API、2026-11-16）。

`L` は v0.10.4 で直った。`crates/rumoca-phase-flatten/src/operator_records.rs` と `operator_records/arrays.rs` は v0.10.4 にあり、v0.10.2（`276e82bd`）に無い。後者は「ベクトルへの operator record の演算子」を扱い、レコードのベクトル 2 つの間の方程式の要素の組を作る関数（`vector_equation_pairs`）を持つ。operator record の `Complex` が通り、普通のレコードが通らないのは、おそらくこのため【推測。追っていない】。演算子の無い（フィールドだけの）局所の `operator record Complex` は普通のレコードと同じ `ED001` になり、合う。

## 環境

- Rumoca v0.10.2（配布バイナリ `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布バイナリ `rumoca 0.10.4`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）。Rumoca main（`b55eafce`、2026-11-13）はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。MSL 4.1.0（`Complex.mo` だけ）。

## GlobeMode 側の回避

GlobeMode 自身の検査には要らない。DAE を作る段（`globe-mode-dae`）は方程式をフィールドごとに数え（6）、S351 の試験 `record_array_equation_counts_per_field` が持つ（`for` 文・内包表記・配列式の形が同じに数えられ 8 と 8、`aw = bw` は 12 と 12）。flat のモデルは要素ごとの数（3）のまま、Rumoca の flat と同じ。固定中の Rumoca（v0.10.2）は、これらのモデルで比べられない: 釣り合いの検査で落ちる（GlobeMode の `diff-frontend`、S351: `aw = {...}` と `aw = bw` の小さなモデル 2 本で 0/2、for 文の形は 2/2 で一致）。固定を v0.10.4 に上げて変わるのは、大きさが書いた数の `Complex` だけの見込み【`msl:diff-frontend` への効果は測っていない】。

## 備考

- 追っていないこと: 3 という数がどこから来るか。flat のモデルは方程式を `scalar_count` 3 の残差 `aw = {...}` 1 本で持つ。
- 途中で見つけた別の限界。S371 で別の下書き [41](41-ed019-record-array-member-slice-dimension-from-size-call_jp.md) にした: `Modelica.ComplexBlocks.Examples.ShowTransferFunction` は、どちらの版でも `TransferFunction.mo:31` の `sum(bw.re)` で `[ED019] unsupported Flat semantic owner 'record-array member slice': reference scope is not an instantiated class occurrence` で止まる。S371 で分かった原因は部品ではなく、レコードの配列の大きさ `size(a, 1)`（`Complex bw[size(b,1)]`）: 配列の成分（`aw.re`・`aw[1].re`）は、最上位のクラスでも v0.10.2・v0.10.4 で同じ ED019 になり、大きさがリテラルや `n = 3` のような `Integer` の parameter なら通る。（この備考の最初の版（S359）は「同じモデルを最上位のクラスとして compile すれば通る」と書いたが、S371 で試した形では成り立たなかった。）omc はどれも通す。だから GlobeMode の比較での `ShowTransferFunction` の失敗は、方程式の数だけでは説明できない。
- 出どころ: GlobeMode の S351（[解説書](../../sessions/S351-review6-fixes/解説書.md)、[作業記録](../../../log/work/2026-11-15-s351-review6-fixes.md)）、MLS 3.7 <https://specification.modelica.org/maint/3.7/arrays.html>（10.6.1）と <https://specification.modelica.org/maint/3.7/class-predefined-types-and-declarations.html>（4.8、Definition 4.4）、確認日 2026-11-16。節の番号と引いた規則は、ページの要約を通して読んだ（全文は読んでいない）。
