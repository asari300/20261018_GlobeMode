# 下書き（未提出）20: 2 つの Medium の別名で、修正付きの extends の package の関数を EF025 で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [20-ef025-function-selection-identity.md](20-ef025-function-selection-identity.md)（後半の「Minimal reproduction: two Medium packages」）、再現は [20b の .mo](20b-ef025-two-medium-aliases-modified-extends.mo)、1 枚の解説は [_jp.html](20-ef025-function-selection-identity_jp.html)。

判定（GlobeMode、S070・S080。S115 で v0.10.4 で再確認）: **一部は v0.10.4 で直った。** 前半の `loadResource` の形（extends が 2 つの関数。136 本）は v0.10.4 で通る。後半の 2 つの Medium の形（`20b`、2 本）は再現する。出すなら後半だけ。omc は全部通す。

## 概要

1 つのモデルに package の別名が 2 つあり、それぞれ別の package を指し、その 2 つが同じ基底の package を**修正付きで** extends する。基底の package の中のモデルが基底の関数を修飾なしで呼ぶ（`BaseProperties` の `y = f(x)`）と、次で落ちる。

```text
[EF025] missing exact function-selection identity for 'f': function source package has multiple exact override package selections
```

別名が 1 つ、修正なしの extends、同じ package を 2 回指す、のどれかなら通る。

## 再現手順

```bash
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M    # 落ちる
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M0   # 対照: Medium が 1 つ
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M1   # 対照: 修正なしの extends
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M2   # 対照: 同じ package を 2 回
```

v0.10.4 で直った前半の形（参考）:

```bash
rumoca compile 20-ef025-function-selection-identity.mo -m Rep20Load --source-root <MSL のルート>   # v0.10.4 で通る
rumoca compile 20-ef025-function-selection-identity.mo -m Rep20TwoExtends.M                         # v0.10.4 で通る
```

## 期待

4 本とも通る。`M1.BaseProperties` の中の `f` は囲む package `A` で探す（MLS 3.6 §5.3）。同じモデルのもう 1 つの別名は、この探索に関わらない。`b1.y = 2*time`、`b2.y = 3*time`。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep28.M`（20b） | EF025 `multiple exact override package selections` | EF025（同じ） |
| `Rep28.M0`・`M1`・`M2`（対照） | 通る | 通る |
| `Rep20Load`（MSL）・`Rep20TwoExtends.M` | EF025 `no unique exact extends implementation` | **通る** |

- `globe check` は `Rep28.M` で S4 の GM-U0002。
- OpenModelica 1.27.2 の `checkModel` は全部 completed successfully（`Rep28.M` は 4 方程式・4 変数）。

## ライブラリでの影響

- 後半（残る形）: 2 本。ThermofluidStream の `Media.myMedia.Examples.MixtureGases`（`molarMass`）と Buildings の `DHC.Plants.Steam.BaseClasses.Examples.ControlVolumeEvaporation`（`setState_pTX`）。
- 前半（直った形）: `loadResource` の 136 本（Buildings・AixLib・IBPSA の気象データと表の読み込み）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。MSL 4.1.0。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。S4 の `GM-U0002`（Rumoca の内部の誤り）として数える。

## 備考

- 候補の package を、呼ぶモデルを囲む package でなく、モデルの上書きの package の一覧から選んでいるように読める【読んだだけ】。
- 本数が 138 から 2 に減るので、提出の順は 5 位から下げた（S115）。
