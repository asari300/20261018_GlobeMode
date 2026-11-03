# 下書き（未提出）31: 束縛が replaceable な record（成分・クラス）を通して成分を読むと EF024

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [31-ef024-binding-reads-field-of-replaceable-record.md](31-ef024-binding-reads-field-of-replaceable-record.md)、再現は [.mo](31-ef024-binding-reads-field-of-replaceable-record.mo)、1 枚の解説は [_jp.html](31-ef024-binding-reads-field-of-replaceable-record_jp.html)。

判定（GlobeMode、S108。S113 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 でも再現する。

## 概要

`replaceable` と宣言したものを通して成分を読む束縛が `[EF024] flat variable is missing structured identity: <名前>` になる。形は 2 つ。

1. **replaceable な record の成分。** record `Stage` が `replaceable parameter NV nomVal` を持つ。record `Coil` が `parameter Stage sta` と `parameter Real mSmall = sta.nomVal.m` を持つ。モデルの `parameter Coil c(sta = Stage(nomVal = NV(m = 2)))` で `m` の EF024。`sta` も `replaceable` にする（ライブラリと同じ）と `nomVal` の EF024。
2. **replaceable な record のクラス。** package の中の `replaceable record ST Real a = 2.5; end ST;` と、`ST` の成分を読む protected の束縛を持つ関数（`protected ST st; Real da = st.a;`、または `input ST s; ... Real t = s.a;`）。関数を呼ぶと `a` の EF024。

`replaceable` を外すとどちらも通る。同じ成分をモデルで読む（形 1）、または algorithm で読むと通る。

## 再現手順

```bash
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M    # EF024: m       （形 1）
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M2   # EF024: nomVal  （形 1、sta も replaceable）
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F    # EF024: a       （形 2、protected の record 変数）
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F1   # EF024: a       （形 2、input）
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M0   # 対照: nomVal が replaceable でない
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M1   # 対照: 成分をモデルで読む
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F0   # 対照: record のクラスが replaceable でない
```

## 期待

7 本すべて通る。`replaceable` は後で redeclare できるようにするだけ（MLS 3.6 §7.3）。`sta.nomVal.m` や `st.a` は通常の成分の参照（§5.3）。関数の protected の変数は束縛を持てる（§12.4.4）。確認日 2026-11-03。

- <https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html>
- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>
- <https://specification.modelica.org/maint/3.6/functions.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S113） |
| --- | --- | --- |
| `M` | EF024 `m` | EF024 `m`（同じ） |
| `M2` | EF024 `nomVal` | EF024 `nomVal`（同じ） |
| `F`・`F1` | EF024 `a` | EF024 `a`（同じ） |
| `M0`・`M1`・`F0`（対照） | 通る | 通る |

- `globe check` も同じコードを出す（`M` で GM-U0002、S4）。
- OpenModelica 1.27.2 の `checkModel` は 7 本とも completed successfully。

## ライブラリでの影響

run 41365960542（2026-11-02、Rumoca v0.10.2、MSL 4.1.0）で 8 本。

- 形 1、Buildings 14.0.0 の 7 本（`nomVal`）: `Fluid.DXSystems.BaseClasses.Examples.DryCoil`、`Fluid.DXSystems.Cooling.BaseClasses.Examples` の `ApparatusDewPoint`・`ApparatusDryPoint`・`DXCooling`・`SpeedSelect`・`SpeedShift`・`WetCoil`。record `DXCoil` の `m_flow_small = 0.0001*sta[nSta].nomVal.m_flow_nominal` が原因。Buildings の写しでこの束縛を `0.0001` にすると `SpeedSelect` は S0〜S10 を通り、`DryCoil`・`WetCoil` は別の EF004 に進む。
- 形 2、AixLib 3.0.1 の 1 本（`T`）: `AixLib.Media.Refrigerants.Examples.BranchingDynamicPipes`。R134a の媒質の `thermalConductivity` の `SaturationProperties sat = setSat_T(state.T)`（`ThermodynamicState` は `PartialMedium` の replaceable な record）。AixLib の写しを直して確かめてはいない【未確認】。

v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S113）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main（`91d6e936`、10 コミット後）は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。ライブラリの側では、関数なら algorithm で読む、または束縛が通る record の `replaceable` を避ける。

## 備考

- 形 2 に継承は要らない（関数と record は誰も継がない 1 つの package にある）。
- 形 1 に record の配列（`sta[nSta]`）は要らない。スカラの `sta` でも同じに落ちる。
- 29 と原因が同じかは追っていない【未確認】。29 と続けて出してもよい。
