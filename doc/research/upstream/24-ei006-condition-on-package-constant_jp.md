# 下書き（未提出）24: 条件付き成分の条件が別の package の定数を読むと EI006 で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [24-ei006-condition-on-package-constant.md](24-ei006-condition-on-package-constant.md)、再現は [.mo](24-ei006-condition-on-package-constant.mo)、1 枚の解説は [_jp.html](24-ei006-condition-on-package-constant_jp.html)。

判定（GlobeMode、S072。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。条件は package の定数からできた parameter 式。omc は通す。v0.10.4 でも再現する。

## 概要

`Real X_w if M.nXi > 0;`（`M` は package、`nXi` はその定数）が次で落ちる。

```text
[EI006] conditional component 'X_w' requires parameter expression
```

package の定数に束縛した `final parameter` を条件に使うとき（`single = (Medium.nX == 1)`、`x if not single`）と、package の定数をじかに書くとき（`x if Medium.nX > 1`）も同じ。モデル自身の定数・parameter なら通る。

## 再現手順

```bash
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.A    # connector、replaceable な package の定数
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.B    # package の定数に束縛した final parameter
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C    # package の定数をじかに
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C0   # 対照: モデルの定数
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C1   # 対照: モデルの parameter
```

## 期待

5 本とも通る。条件は Boolean のスカラの parameter 式でなければならない（MLS 3.6 §4.4.5）。定数式・parameter・それらの演算は parameter 式（§3.8.3）、定数は定数式（§3.8.2）。package の定数とリテラルの比較は定数式。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/class-predefined-types-and-declarations.html>
- <https://specification.modelica.org/maint/3.6/operators-and-expressions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep24.A` | EI006 `X_w` | EI006（同じ） |
| `Rep24.B`・`Rep24.C` | EI006 `x` | EI006（同じ） |
| `Rep24.C0`・`Rep24.C1`（対照） | 通る | 通る |

- `globe check` は S3（instantiate）で GM-E3001。
- OpenModelica 1.27.2 の `checkModel` は 5 本とも completed successfully（`Rep24.A` は再現の connector が釣り合わない警告だけ）。

## ライブラリでの影響

S063 で 117 本（Buildings 54、IBPSA 30、残りは AixLib）。40 本が `X_w`（`IBPSA.Fluid.FMI.Interfaces.FluidProperties`）、27 本が `x_pTphi`（`MassFlowSource_WeatherData`）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。S3 の失敗（GM-E3001）として数える。

## 備考

- 条件の評価器が、import していない package の定数の点つきの参照（`Medium.nX`）を解決できていないように見える【推測】。
- ライブラリのほかの条件の名前（`solRad`・`capFil1` など）は縮めていない。別の原因かもしれない。
- 提出の順は 5 位（S115 で 20 が下がったので 6 位から上がった）。
