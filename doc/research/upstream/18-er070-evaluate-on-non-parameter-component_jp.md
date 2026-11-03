# 下書き（未提出）18: parameter でない成分への `annotation(Evaluate = true)` を ER070 で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [18-er070-evaluate-on-non-parameter-component.md](18-er070-evaluate-on-non-parameter-component.md)、再現は [.mo](18-er070-evaluate-on-non-parameter-component.mo)、1 枚の解説は [_jp.html](18-er070-evaluate-on-non-parameter-component_jp.html)。

判定（GlobeMode、S070。S115 で v0.10.4 で再確認）: **Rumoca の穴**（仕様が「効果が無い」とだけ言う所を誤りにする）。omc は通す。メッセージの節番号も誤り。v0.10.4 でも再現する。

## 概要

ブロックの成分（parameter でない）に `Evaluate = true` を付けると、次で落ちる。

```text
[ER070] annotation Evaluate is only allowed on parameter or constant components; 'Dzero' is not parameter or constant (MLS §18.6)
```

元は `IBPSA.Controls.Continuous.LimPID` の `Modelica.Blocks.Sources.Constant Dzero(final k = 0) if not with_D annotation(Evaluate = true)`。Rumoca は関数の局所変数への同じ注釈を警告 `WR006`（効果が無い）にしているが、成分のときだけ誤りにしている。

## 再現手順

MSL（`Modelica.Blocks.Sources.Constant`）が要る。

```bash
rumoca compile 18-er070-evaluate-on-non-parameter-component.mo -m Rep18 --source-root <MSL のルート>        # 落ちる
rumoca compile 18-er070-evaluate-on-non-parameter-component.mo -m Rep18Plain --source-root <MSL のルート>   # 対照: 注釈なし
```

## 期待

2 本とも通る。MLS 3.6 §18.3 の `Evaluate`（Annotation 18.1）は「成分の宣言、その型の宣言、型の基底クラスに書ける」「階層のある成分では中のすべての成分に適用する」「parameter の接頭辞のある成分にだけ効果がある」と書き、誤りとは言わない。§18.6 は 3.6 では「Graphical Objects」で、メッセージの節番号は誤り。確認日 2026-11-01。

- <https://specification.modelica.org/maint/3.6/annotations.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep18` | ER070 `Dzero`（MLS §18.6） | ER070（同じ） |
| `Rep18Plain`（対照） | 通る | 通る |

- `globe check` は S1（名前解決）で GM-E1001。
- OpenModelica 1.27.2 の `checkModel` は 2 本とも completed successfully（2 方程式・2 変数）。

## ライブラリでの影響

S063 で 258 本（Buildings 173、AixLib 58、IBPSA 27）。258 本のうち 250 本が `Dzero`。`LimPID` を使う例題がすべて止まる。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。MSL 4.1.0。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし（ソースから注釈を外せば通るが、GlobeMode はソースを書き換えない）。

## 備考

- 直し方の案: parameter でない成分も関数の局所変数と同じく `WR006` の警告にして注釈を捨てる。クラスと `extends` への `Evaluate` も同じく断っているが、仕様は型の宣言と基底クラスにも書けると言う【未試験】。
- 提出の順は 2 位。
