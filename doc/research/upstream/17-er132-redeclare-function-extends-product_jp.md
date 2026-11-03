# 下書き（未提出）17: 組み込みと同じ名前の `redeclare function extends product` を ER132 で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [17-er132-redeclare-function-extends-product.md](17-er132-redeclare-function-extends-product.md)、再現は [.mo](17-er132-redeclare-function-extends-product.mo)、1 枚の解説は [_jp.html](17-er132-redeclare-function-extends-product_jp.html)。

判定（GlobeMode、S070。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

継いだ `replaceable partial function product` を `redeclare function extends product` で置き換えると、次で落ちる。

```text
[ER132] cannot verify ExternalObject lifecycle for 'product': resolved base DefId(78) is missing
[ER002] unresolved component reference: 'z'   （x・y も同じ）
```

名前を組み込みでないもの（`prod`）にすると通る。`sum`・`max`・`min`・`abs`・`sin`・`ceil` でも同じく落ちる。`extends product` の単純名が、継いだ要素でなく組み込みの関数に解決されているように見える【推測】。

## 再現手順

```bash
rumoca compile 17-er132-redeclare-function-extends-product.mo -m Rep17.M
```

`Rep17.One` に `product`（落ちる）と `sum2`（同じ形で普通の名前。通る）がある。`product` を `prod` に変えるとファイル全体が通る。

## 期待

通る。`redeclare class extends B` の `B` は継いだ要素を指す（MLS 3.6 §7.3.1）。単純名は囲むクラスの順に探す（§5.3.1）。`product` は予約語でない（予約語に近い扱いは `Real`・`Integer`・`Boolean`・`String` だけ）。確認日 2026-11-01。

- <https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html>
- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep17.M` | ER132 `product` と ER002 `z`・`x`・`y` | 同じ（`DefId(78)` も同じ） |

- `globe check` は S1（名前解決）で GM-E1001。
- OpenModelica 1.27.2 の `checkModel(Rep17.M)` は completed successfully（6 方程式・6 変数）。

## ライブラリでの影響

公開ライブラリ 7 つの検査（S063）で 267 本（Buildings 97、AixLib 85、IBPSA 85）。すべて `IBPSA.Electrical.PhaseSystems.OnePhase` の `redeclare function extends product` で、電気の部品を使う例題が全部ここで止まる。S1 の失敗で最大。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。S1 の失敗（GM-E1001）として数える。

## 備考

- 直す所は 2 つありうる（英語の本文の Notes）: `extends product` の探索が継いだ要素を返すこと、ExternalObject の寿命の検査が分類できない基底を ER132 にしないこと。
- 提出の順は 1 位（本数が最大）。
