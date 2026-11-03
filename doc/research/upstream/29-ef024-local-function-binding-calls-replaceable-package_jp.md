# 下書き（未提出）29: モデルの中の関数で、protected の束縛が replaceable な package の関数を呼ぶと EF024

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [29-ef024-local-function-binding-calls-replaceable-package.md](29-ef024-local-function-binding-calls-replaceable-package.md)、再現は [.mo](29-ef024-local-function-binding-calls-replaceable-package.mo)、1 枚の解説は [_jp.html](29-ef024-local-function-binding-calls-replaceable-package_jp.html)。

判定（GlobeMode、S091。S113 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

`replaceable package Medium = P` を持つモデルが関数 `g` を宣言し、その protected の変数に束縛 `Real w = Medium.f(u)` を書く。`g` を呼ぶと次になる。

```text
[EF024] flat variable is missing structured identity: f
```

同じ呼び出しを algorithm に書く（`z := Medium.f(u)`）と通る。束縛で package の名前を直接書く（`Real w = P.f(u)`）と通る。ライブラリと同じく `extends M(redeclare package Medium = P)` で redeclare しても落ちる。

## 再現手順

```bash
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M    # EF024
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M1   # EF024（extends で redeclare）
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M0   # 対照: algorithm で呼ぶ
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M2   # 対照: 束縛で P.f
```

## 期待

4 本すべて通る。関数の protected の成分は束縛を持てて、algorithm の前に評価される（MLS 3.6 §12.4.4）。`Medium.f` は関数を囲むモデルからの通常の探索で見つかる（§5.3）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/functions.html>
- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S113） |
| --- | --- | --- |
| `M` | EF024 `f` | EF024 `f`（同じ） |
| `M1` | EF024 `f` | EF024 `f`（同じ） |
| `M0`・`M2`（対照） | 通る | 通る |

- `globe check` は `M` で GM-U0002（S4）を出す。
- OpenModelica 1.27.2 の `checkModel` は 4 本とも completed successfully。

## ライブラリでの影響

run 41365960542（2026-11-02）で 18 本が `flat variable is missing structured identity: temperature` で止まる。IBPSA・Buildings・AixLib のそれぞれの `Media.Examples` の 6 本（`AirProperties`、`SteamProperties`、`WaterProperties`、`PropyleneGlycolWaterProperties`、`Specialized.Water.Examples.TemperatureDependentDensityProperties`、`Specialized.Water.Examples.WaterProperties_pT`）。これらは `Media.Examples.BaseClasses.PartialProperties` を継ぎ、その protected の関数 `checkState` が `Real TErrAbs = abs(Medium.temperature(state1) - Medium.temperature(state2))` を持つ。IBPSA の写しで 2 つの束縛を `checkState` の algorithm に移すと、`IBPSA.Media.Examples.WaterProperties` は GlobeMode の検査をすべて通る。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S113）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main（`91d6e936`、10 コミット後）は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。ライブラリの側では、呼び出しを束縛から algorithm に移す。

## 備考

- 31（replaceable な record を通して成分を読む束縛）も関数の束縛の形。29 の再現には replaceable な record が無く、原因が同じかは追っていない【未確認】。31 と続けて出してもよい。
