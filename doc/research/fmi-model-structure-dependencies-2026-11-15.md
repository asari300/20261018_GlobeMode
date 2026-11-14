# FMI の ModelStructure の `dependencies` の調査（S266）: 何を既知として並べるか

- 確認日: 2026-11-15
- 目的: [ADR-0031](../adr/0031-fmi-export-p5a-first.md) の S224 の持ち越し「出力・導関数ごとの `dependencies`」を書くための規則を、仕様で確かめる。結果は ADR-0031 の S266 の追記。

## 1. 出典

| 版 | 取得した所 | 確認日 |
| --- | --- | --- |
| FMI 2.0.x | `https://raw.githubusercontent.com/modelica/fmi-standard/v2.0.x/docs/2_2_common_schema.adoc`（ブランチ `v2.0.x` の先頭は `4196862d`）。`Unknown` の属性の表（1505 行目付近） | 2026-11-15 |
| FMI 3.0.1 | `https://fmi-standard.org/docs/3.0.1/`（組版後の HTML）の「2.4.8 Model Structure」、表 24（`<Output>`・`<ContinuousStateDerivative>`・`<ClockedState>`・`<InitialUnknown>`・`<EventIndicator>` の属性） | 2026-11-15 |

## 2. 結論

| 問い | FMI 2.0 | FMI 3.0 |
| --- | --- | --- |
| `dependencies` を省くと | 「すべての Knowns に依存」 | 同じ |
| 空のリスト | どの Knowns にも依らない | 同じ |
| 値 | `ScalarVariable` の索引（1 始まり）を昇順 | 変数の `valueReference` |
| `Outputs`・`Derivatives` の Knowns（Event・Continuous-Time Mode、CS は通信点） | 入力、連続状態、独立変数（`causality="independent"`。`ScalarVariable` に定義が無ければ**暗黙に依存すると仮定**） | 入力、連続状態と離散時間の状態、**パラメータ（`causality="parameter"`）**、独立変数。「すべての依存を列挙しなければならない」 |
| `InitialUnknowns` の Knowns | 入力、`initial="exact"` の変数（独立パラメータ・初期状態）、独立変数 | 入力、`initial="exact"` の変数、独立変数 |
| `dependenciesKind` | 省略可（省略すると「構造の特別なもの無しに依存」）。あれば `dependencies` が必須で、要素の数が同じ | 同じ。値は `dependent`・`constant`・`fixed`・`tunable`・`discrete`（`InitialUnknown` では `fixed`・`tunable`・`discrete` は使えない） |

2.0 でパラメータを並べないのは、固定の因子（`v_freeze`）で Continuous-Time Mode の既知ではないため。3.0 は表のとおりパラメータも既知なので、列挙が要る。

## 3. GlobeMode の書き方（S266）

- `dependenciesKind` は省略する。構造を言い切らない値（`dependent`）と同じ意味で、仕様が省略を許す。
- `InitialUnknown`・`InitialUnknowns` の `dependencies` は書かない（＝すべてに依存）。初期化の既知（`initial="exact"` のパラメータ・初期状態）と初期方程式の依存を別に求める必要があり、S266 では扱わない。省略は仕様どおり安全。
- `Outputs`・`Derivatives`（3.0 は `Output`・`ContinuousStateDerivative`）は、BLT の解く順の閉包（`globe_mode_analysis::analyze_dependencies`）で、推移的に依る状態・入力（3.0 はパラメータと `time` も）を昇順に書く。
- 求められないときは属性を書かない（＝すべてに依存）。離散変数・`pre()`・条件を介するもの、インデックス 2 以上（状態選択の前）、変数の表に無い名前。
