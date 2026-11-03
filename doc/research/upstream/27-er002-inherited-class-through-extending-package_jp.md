# 下書き（未提出）27: extends で継いだ入れ子のクラスを、継いだ package の名前で引くと ER002

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [27-er002-inherited-class-through-extending-package.md](27-er002-inherited-class-through-extending-package.md)、再現は [.mo](27-er002-inherited-class-through-extending-package.mo)、1 枚の解説は [_jp.html](27-er002-inherited-class-through-extending-package_jp.html)。

判定（GlobeMode、S083。S115 で v0.10.4 で再確認）: **Rumoca の穴**（名前解決で正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

package `Q` が `extends P` で、`P` が `model B` を宣言しているとき、成分の型に `Q.B`（または `package A = Q` の `A.B`）を書くと次で落ちる。

```text
[ER002] unresolved type reference: 'Q.B'
```

`P.B` は通る。`replaceable package A = Q` の `A.B` も通る。同じように継いだ関数（`Q.f(time)`）も引ける。

## 再現手順

```bash
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M    # 落ちる: Q.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M1   # 落ちる: package A = Q; A.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M0   # 対照: P.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M2   # 対照: replaceable package A = Q
```

## 期待

4 本とも通る。クラスの要素には extends で継いだ要素が入る（MLS 3.6 §7.1）。合成名 `Q.B` の残りは `Q` の要素の中で探す（§5.3.2）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html>
- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep35.M` | ER002 `Q.B` | ER002（同じ） |
| `Rep35.M1` | ER002 `A.B` | ER002（同じ） |
| `Rep35.M0`・`Rep35.M2`（対照） | 通る | 通る |

- `globe check` は S1（名前解決）で GM-E1001。
- OpenModelica 1.27.2 の `checkModel` は 4 本とも completed successfully。

## ライブラリでの影響

公開ライブラリの例題では 0 本（S091）。ほかの再現を縮める途中で見つけた（`package Medium = Modelica.Media.Incompressible.Examples.Glycol47; Medium.BaseProperties medium;` が ER002、`replaceable package` なら通る）。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。継いだ入れ子のクラスを使うモデルでは `replaceable package Medium = ...` と書く（または宣言した package の名前で引く）。

## 備考

- 根から修飾した `Rep35.Q.B`、`P`・`Q` を外の package に置く、`Q` に定数を足す、`extends P(k = 2)` でも落ちる。
- MSL の `Modelica.Media.Examples.SimpleLiquidWater` は修正付きの非 replaceable な別名で通る。修正があると探索が変わる理由は追っていない【未確認】。
- 例題で出ないので、出すかはユーザーが決める。
