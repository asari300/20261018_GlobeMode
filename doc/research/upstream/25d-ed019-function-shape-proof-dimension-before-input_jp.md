# 下書き（未提出）25d: 配列の入力が後で宣言する `Integer` の入力で次元を取ると ED019

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [25d-ed019-function-shape-proof-dimension-before-input.md](25d-ed019-function-shape-proof-dimension-before-input.md)、再現は [.mo](25d-ed019-function-shape-proof-dimension-before-input.mo)、1 枚の解説は [_jp.html](25d-ed019-function-shape-proof-dimension-before-input_jp.html)。

判定（GlobeMode、S083。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を未対応として断る）。omc は通す。v0.10.4 でも再現する。

## 概要

関数が `input Real x[n]` を `input Integer n` より先に宣言し、その関数を呼ぶブロックを部品として使うと、次で落ちる。

```text
[ED019] unsupported Flat semantic owner 'function shape proof': dependent extent is not an exact Integer expression over proven shape axes
```

`input Integer n` を先にする、または `x[:]` にすると通る。

## 再現手順

```bash
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M    # 落ちる: ブロックを部品に
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M2   # 落ちる: ブロックを extends
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M0   # 対照: n が先
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M1   # 対照: x[:]
```

## 期待

4 本とも通る。関数の既定値と束縛は、循環が無い限り宣言の順でなく依存の順に評価され、次元は入力に依ってよい（MLS 3.6 §12.4.1・§12.4.4・§12.4.5）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep30.M`・`Rep30.M2` | ED019 `function shape proof` | ED019（同じ。`input Real x[n];` を指す） |
| `Rep30.M0`・`Rep30.M1`（対照） | 通る | 通る |

- OpenModelica 1.27.2 の `checkModel` は全部通す（2026-11-02）。

## ライブラリでの影響

3 本。IBPSA・AixLib・Buildings の `Controls.SetPoints.OccupancySchedule.getOutput`（`occupancy[nRow]` を `nRow` より先に宣言）。IBPSA の写しで順を入れ替えると、`IBPSA.Controls.SetPoints.Examples.OccupancySchedule` のこの ED019 は消える（後で別の ED019 で止まる）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。`GM-U0001`（未対応）として数える。

## 備考

- ED019 の `function shape proof` の残り（25d・25e・25f・25h）は 1 つの issue にまとめて出してもよい。25b・25c は v0.10.4 で直った（S115）。
