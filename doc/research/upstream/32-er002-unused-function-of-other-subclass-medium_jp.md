# 下書き（未提出）32: モデルが使わない別の派生クラスの Medium の、壊れた関数まで名前を解決して ER002

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [32-er002-unused-function-of-other-subclass-medium.md](32-er002-unused-function-of-other-subclass-medium.md)、再現は [.mo](32-er002-unused-function-of-other-subclass-medium.mo)、1 枚の解説は [_jp.html](32-er002-unused-function-of-other-subclass-medium_jp.html)。

判定（GlobeMode、S108。S113 で v0.10.4 で再確認）: **Rumoca の穴**（モデルが実体化しないクラスまで検査する）。omc はモデルを通す。壊れた関数そのものはライブラリの誤りで、AixLib 向けの下書き [33](33-aixlib-specific-entropy-of-ptx-missing_jp.md) に分けた。v0.10.4 でも再現する。

## 概要

partial model `PFP` が `replaceable package Medium1 = PM` を持つ。モデル `M` はそれを `redeclare package Medium1 = Good` で継ぎ、`Medium1.f(time)` を呼ぶ。別のモデル `Engine` が同じ `PFP` を `redeclare package Medium1 = Bad` で継ぐ。`Bad` の関数 `T_psX` の入れ子の関数は、どこにも宣言の無い `specificEntropyOfpTX` を呼ぶ。`T_psX` に届くのは `Bad.g` からだけ。`M` は `Engine`・`Bad`・`g`・`T_psX` のどれも使わないのに、`M` のコンパイルが次になる。

```text
[ER002] unresolved function call: 'specificEntropyOfpTX'
```

`M` が `Medium1` を通して呼ばない（`M0`）と通る。基底に `Medium1 = Bad` と redeclare する別の派生クラスが無い（`M1`）と通る。

## 再現手順

```bash
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M       # ER002
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M0      # 対照: Medium1 を通して呼ばない
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M1      # 対照: 同じ呼び出し、別の派生クラスの無い基底
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.Engine  # ER002（Engine も g を呼ばない）
```

## 期待

`M`・`M0`・`M1` は通る。`M` の平坦化で実体化するのは、`M`、`Medium1 = Good` の基底 `PFP`、呼ぶ関数だけ（MLS 3.6 §5.6、<https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>、確認日 2026-11-03）。`PFP` の別の派生クラスとその Medium は `M` の一部でない。`Engine` も `g` を呼ばないので、OpenModelica は `Engine` も通す。

## 実際

| モデル | v0.10.2 | v0.10.4（S113） |
| --- | --- | --- |
| `M` | ER002 | ER002（同じ） |
| `Engine` | ER002 | ER002（同じ） |
| `M0`・`M1`（対照） | 通る | 通る |

- `globe check` も同じ（S1 名前解決で停止、ER002）。
- OpenModelica 1.27.2 の `checkModel` は `M`・`M0`・`M1`・`Engine` とも completed successfully。

## ライブラリでの影響

run 41365960542（2026-11-02、Rumoca v0.10.2、MSL 4.1.0）で AixLib 3.0.1 の 39 本が `unresolved function call: 'specificEntropyOfpTX'` で止まる（`Airflow.Multizone.Examples` 6、`Fluid.HeatPumps` 7、`Fluid.Chillers` 5、`Fluid.Geothermal.Borefields` 5 ほか）。

- 呼び出しは `AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa.T_psX.f_nonlinear`（670 行）にあり、その `setState_psX` から届く。
- これらの例題は `AixLib.Fluid.Interfaces.PartialFourPortInterface` を継ぐ部品を使い、普通の媒質で `Medium1.setState_phX(...)` を呼ぶ。同じ基底を `AixLib.Fluid.BoilerCHP.ModularCHP.BaseClasses.ExhaustHeatExchanger` が `redeclare package Medium1 = CHPFlueGasLambdaOnePlus` で継ぐ。
- AixLib の写しで `ExhaustHeatExchanger.mo` を空のモデルにすると ER002 が消え、`extends ...PartialFourPort(redeclare package Medium1 = ...CHPFlueGasLambdaOnePlus)` だけにすると戻る（S108）。

v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S113）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main（`91d6e936`、10 コミット後）は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

一部。ネイティブと wasm の `--library` はモデルの閉包だけを読む（GlobeMode の ADR-0025）ので、上の例題では出ない。公開ライブラリの検査（全部読み）では出る。

## 備考

- ライブラリも正しくない。`CHPCombustionMixtureGasNasa` は MSL の `MixtureGasNasa.T_psX` を写したが、そこで呼ぶ protected の `specificEntropyOfpTX` を写していない。AixLib 向けは下書き 33。この下書きは「使わないモデルから Rumoca がそこまで届く」ことだけを扱う。
- Rumoca が `M` から `Bad` にどう届くか（読み込んだクラスの中の `PFP.Medium1` の redeclare をすべて解決しているように見える）はコードで追っていない【未確認】。
- AixLib が 33 を直すと、この 39 本は ER002 で止まらなくなる。Rumoca の穴そのものは残る。
