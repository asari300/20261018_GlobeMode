# 下書き（未提出）25e: record の parameter の成分を次元にした record を関数の局所変数にすると ED019

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [25e-ed019-function-shape-proof-record-local-dimension.md](25e-ed019-function-shape-proof-record-local-dimension.md)、再現は [.mo](25e-ed019-function-shape-proof-record-local-dimension.mo)、1 枚の解説は [_jp.html](25e-ed019-function-shape-proof-record-local-dimension_jp.html)。

判定（GlobeMode、S083。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を未対応として断る）。omc は通す。**v0.10.4 で一部が直った**: 成分が `constant` の形（`MC`）は通るようになり、`parameter` の形（`M`）は再現する。出すなら `parameter` の形だけ。

## 概要

`parameter Integer nc = 2; parameter Real[nc] a` を持つ record を、関数の protected の局所変数にすると次で落ちる。

```text
[ED019] unsupported Flat semantic owner 'function shape proof': dependent extent is not an exact Integer expression over proven shape axes
```

次元をリテラルにする、または同じ record をモデルの成分にすると通る。

## 再現手順

```bash
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M    # 落ちる（parameter の成分）
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.MC   # v0.10.2 は落ちる、v0.10.4 は通る（constant の成分）
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M0   # 対照: リテラルの次元
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M1   # 対照: モデルの成分
```

## 期待

4 本とも通る。関数の成分の配列の次元は、定数と parameter に依ってよい（MLS 3.6 §12.4.5）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep31.M`（parameter） | ED019 `function shape proof` | ED019（同じ。`parameter Real[nc] a` を指す） |
| `Rep31.MC`（constant） | ED019 | **通る** |
| `Rep31.M0`・`Rep31.M1`（対照） | 通る | 通る |

- OpenModelica 1.27.2 の `checkModel` は全部通す（2026-11-02）。

## ライブラリでの影響

6 本・MSL 2 本。MSL 4.1.0 の `Media.R134a`（`Common.EOSIdealCoeff` の `parameter Integer nc=5; parameter Real[nc] a`。`Media/R134a.mo` の 52〜53 行）と ThermofluidStream の写し。MSL の形は `parameter` なので、v0.10.4 でも残る形にあたる。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。`GM-U0001`（未対応）として数える。

## 備考

- MSL の写しで record の次元をリテラルにすると、MSL の `R134a1`・`R134a2` はこの誤りを抜ける（後で別の ED008 `state_h` で止まる）。
- 25d・25f・25h と 1 つの issue にまとめて出してもよい。
