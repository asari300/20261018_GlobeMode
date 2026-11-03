# 下書き（未提出）19: `stateSelect` の parameter 式を EI032 で断る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [19-ei032-statselect-parameter-expression.md](19-ei032-statselect-parameter-expression.md)、再現は [.mo](19-ei032-statselect-parameter-expression.mo)、1 枚の解説は [_jp.html](19-ei032-statselect-parameter-expression_jp.html)。

判定（GlobeMode、S070。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

`Real` の属性 `stateSelect` を、Rumoca はインスタンス化のときに列挙の値まで畳もうとし、畳めないと次で止まる。

```text
[EI032] invalid value for type attribute 'stateSelect': <式>
```

普通の 2 つの形が落ちる。

- **A**: 兄弟の部品の parameter を読む if 式。`T(stateSelect = if medium.preferredMediumStates then StateSelect.prefer else StateSelect.default)`。IBPSA の `ConservationEquation` と Media の形で、`Modelica.Media` の BaseProperties も同じ。
- **B**: 修正で渡した parameter。`Joint j(stateSelect = ss)` で、`Joint` の中は `Real phi(stateSelect = stateSelect)`。MSL の `Revolute` がこの作りで、親が値を渡すと落ちる（DeltaRobot）。

リテラル（`StateSelect.prefer`）と、B の既定のままは通る。

## 再現手順

```bash
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19A    # 落ちる
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19B    # 落ちる
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19A0   # 対照: リテラル
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19B0   # 対照: リテラル
```

## 期待

4 本とも通る。`stateSelect` は `parameter StateSelect stateSelect` として定義された属性（MLS 3.6 §4.8.1・§4.8.7.1）で、parameter 式を取れる（§3.8.3）。parameter の参照と、parameter の上の if 式は parameter 式。リテラルに限る規定は無い。Rumoca のメッセージが挙げる §4.4.4 は 3.6 では変数の種類の接頭辞の節。確認日 2026-11-01。

- <https://specification.modelica.org/maint/3.6/class-predefined-types-and-declarations.html>
- <https://specification.modelica.org/maint/3.6/operators-and-expressions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep19A` | EI032（if 式） | EI032（同じ） |
| `Rep19B` | EI032 `stateSelect` | EI032（同じ） |
| `Rep19A0`・`Rep19B0`（対照） | 通る | 通る |

- `globe check` は S3（instantiate）で GM-E3001。
- OpenModelica 1.27.2 の `checkModel` は `Rep19A`・`Rep19B` とも completed successfully（1 方程式・1 変数）。

## ライブラリでの影響

S063 で 235 本（Buildings 87、IBPSA 61、AixLib 約 80、DeltaRobot 7）。A が 228、B が 7（DeltaRobot）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。S3 の失敗（GM-E3001）として数える。

## 備考

- `stateSelect` は状態の選び方の指示だけなので、評価を後に回す、または畳めないときは `StateSelect.default` にして警告する、という直し方もある【提案】。
- 提出の順は 3 位。
