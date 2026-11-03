# 下書き（未提出）25h: `Real[:]` の局所変数を呼び出しの代入だけで大きさを決めると ED019

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [25h-ed019-function-shape-proof-flexible-array-from-call.md](25h-ed019-function-shape-proof-flexible-array-from-call.md)、再現は [.mo](25h-ed019-function-shape-proof-flexible-array-from-call.mo)、1 枚の解説は [_jp.html](25h-ed019-function-shape-proof-flexible-array-from-call_jp.html)。

判定（GlobeMode、S083。S115 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を未対応として断る）。omc は通す。v0.10.4 でも再現する。

## 概要

関数の中の `protected Real[:] X;` を `X := g(x)` だけで大きさを決める（束縛 `= g(x)` でも同じ）と、次で落ちる。

```text
[ED019] unsupported Flat semantic owner 'function shape proof': 'X': axis 1 is variable-size and neither a binding nor an assignment sizes it
```

`Real[2] X` なら通る。メッセージは「代入が大きさを決めない」と言うが、代入がある。

## 再現手順

```bash
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M    # 落ちる（代入）
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M1   # 落ちる（束縛）
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M0   # 対照: X[2]
```

## 期待

3 本とも通る。関数の protected の配列は `:` で宣言でき、代入で大きさが決まる（MLS 3.6 §12.4.5）。`g` の出力は `X[2]`。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep34.M`・`Rep34.M1` | ED019 `'X': axis 1 is variable-size` | ED019（同じ） |
| `Rep34.M0`（対照） | 通る | 通る |

- OpenModelica 1.27.2 の `checkModel` は全部通す（2026-11-02）。

## ライブラリでの影響

2 本・MSL 2 本。MSL 4.1.0 の `Media.Air.ReferenceMoistAir.Xsaturation`（`MassFraction[:] X; X := massFractionSaturation(state)`）と ThermofluidStream の写し。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。`GM-U0001`（未対応）として数える。

## 備考

- 25d・25e・25f と 1 つの issue にまとめて出してもよい。
