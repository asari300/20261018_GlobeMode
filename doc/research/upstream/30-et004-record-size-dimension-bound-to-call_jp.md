# 下書き（未提出）30: `eta[size(y, 1)]` を持つ record の parameter を関数の呼び出しで束縛すると ET004

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [30-et004-record-size-dimension-bound-to-call.md](30-et004-record-size-dimension-bound-to-call.md)、再現は [.mo](30-et004-record-size-dimension-bound-to-call.mo)、1 枚の解説は [_jp.html](30-et004-record-size-dimension-bound-to-call_jp.html)。

判定（GlobeMode、S091。S113 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

record `R` が `parameter Real y[:]` と `parameter Real eta[size(y, 1)]` を持つ。関数 `f` が `output R r(y = {0, 1}, eta = zeros(2))` を返す。モデルの `parameter R p = f(2)` で次になる。

```text
[ET004] unevaluable array dimensions for 'p.eta': dimension expression could not be evaluated: [Expression(FunctionCall { comp: size, args: [y, UnsignedInteger("1")], ... })]
```

同じ record の成分を直接書く（`parameter R p(y = {0, 1}, eta = {2, 2})`）と通る。

## 再現手順

```bash
rumoca compile 30-et004-record-size-dimension-bound-to-call.mo -m Rep38.M    # ET004
rumoca compile 30-et004-record-size-dimension-bound-to-call.mo -m Rep38.M0   # 対照
```

## 期待

2 本とも通る。`:` の次元は束縛から決まる（MLS 3.6 §10.1）。ここでの束縛は関数の出力で、その大きさは出力の修正で決まっている（§12.4.5）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/arrays.html>
- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S113） |
| --- | --- | --- |
| `M` | ET004 `p.eta` | ET004 `p.eta`（同じ） |
| `M0`（対照） | 通る | 通る |

- `globe check` は `M` で GM-E2001（S2）を出す。
- OpenModelica 1.27.2 の `checkModel` は 2 本とも completed successfully。

## ライブラリでの影響

run 41365960542（2026-11-02）で 15 本。IBPSA・Buildings・AixLib のそれぞれの `Fluid.Movers.Examples.MoverContinuous`、`MoverParameter`、`MoverStages`、`PumpsSeries`、`SpeedControlled_y_pumpCurves`。`Fluid.Movers.Data.Generic` が `final parameter efficiencyParameters_yMot motorEfficiency_yMot_generic = motorEfficiencyCurve(...)` を持ち、record は `y[:]` と `eta[size(y, 1)]`、関数は `output efficiencyParameters_yMot motorEfficiency_yMot(y = {...}, eta = zeros(nPoi))` を宣言する。ライブラリを直して確かめてはいない【未確認】。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S113）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main（`91d6e936`、10 コミット後）は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。

## 備考

- `y[2]`（固定）と `eta[size(y, 1)]` に同じ呼び出しだと `[ED008] unresolved Flat reference 'y'`、`y[:]` と `eta[2]` だと `unresolved component dimension for p.y: :` に変わる。どちらも「呼び出しで束縛した record の大きさ」の同じ欠けに見えるが、追っていない【未確認】。
