# 下書き（未提出）25f: `b[:]` の束縛が後で宣言する定数を読むと ED019

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [25f-ed019-function-shape-proof-binding-reads-later-constant.md](25f-ed019-function-shape-proof-binding-reads-later-constant.md)、再現は [.mo](25f-ed019-function-shape-proof-binding-reads-later-constant.mo)、1 枚の解説は [_jp.html](25f-ed019-function-shape-proof-binding-reads-later-constant_jp.html)。

判定（GlobeMode、S083。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を未対応として断る）。omc は通す。v0.10.4 でも再現する。

## 概要

関数の中で `protected constant Real b[:] = {c, 2}` を `constant Real c` より先に宣言すると、次で落ちる。

```text
[ED019] unsupported Flat semantic owner 'function shape proof': 'b': axis 1 is variable-size and neither a binding nor an assignment sizes it
```

`c` を先に宣言する、または束縛をリテラルにすると通る。

## 再現手順

```bash
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M    # 落ちる
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M0   # 対照: 宣言の順を入れ替え
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M1   # 対照: リテラルの束縛
```

## 期待

3 本とも通る。関数の束縛は、循環が無い限り宣言の順でなく依存の順に評価される（MLS 3.6 §12.4.4）。`b` の大きさは束縛 `{c, 2}` で 2 に決まる。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep32.M` | ED019 `'b': axis 1 is variable-size` | ED019（同じ） |
| `Rep32.M0`・`Rep32.M1`（対照） | 通る | 通る |

- OpenModelica 1.27.2 の `checkModel` は全部通す（2026-11-02）。

## ライブラリでの影響

3 本。IBPSA・AixLib・Buildings の `Media.Steam.temperature_ph`（`b[:]` が後で宣言する `TMean`・`TSD` を読む）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。`GM-U0001`（未対応）として数える。

## 備考

- 25d・25e・25h と 1 つの issue にまとめて出してもよい。
