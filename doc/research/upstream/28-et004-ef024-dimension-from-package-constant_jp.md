# 下書き（未提出）28: replaceable な package を通して定数を読む配列の次元が失敗する（ET004 / EF024）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [28-et004-ef024-dimension-from-package-constant.md](28-et004-ef024-dimension-from-package-constant.md)、再現は [.mo](28-et004-ef024-dimension-from-package-constant.mo)、1 枚の解説は [_jp.html](28-et004-ef024-dimension-from-package-constant_jp.html)。

判定（GlobeMode、S091。S108 で形を追記、S113 で v0.10.4 で再確認）: **Rumoca の穴**（正しい Modelica を断る）。omc は通す。v0.10.4 では素直な形（`M`・`M2`）が通るようになり、replaceable な package を通す形（`M1`・`M3`・`M5`）だけが残る。

## 概要

配列の次元が別の package の定数のとき、v0.10.2 では次のように落ちた。

```text
[ET004] unevaluable array dimensions for 'x': dimension expression could not be evaluated: [Expression(P.n)]
```

同じ定数を `replaceable package Medium = P`（既定のまま）を通して読むと次になる。

```text
[EF024] flat variable is missing structured identity: n
```

モデルの定数を次元にすると通る。部品に `redeclare package Medium = P` を付けて渡しても通る。MSL の流体の部品（`Boundary_pT` の `X[Medium.nX]`、必ず redeclare される）が通るのはこのため。

## 再現手順

添付の `.mo`（自己完結）をコンパイルする。

```bash
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M    # v0.10.2 は ET004、v0.10.4 は通る: Real x[P.n]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M1   # EF024: replaceable package Medium = P; Real x[Medium.n]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M2   # v0.10.2 は ET004、v0.10.4 は通る: parameter Integer m = P.n; Real x[m]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M3   # ET004: 部品が replaceable な package の既定を使う
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M0   # 対照: モデルの定数
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M4   # 対照: 同じ部品に redeclare を付ける
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M5   # EF024: モデルの中で宣言した record の次元が Medium.n（package は redeclare 済み。S108）
```

## 期待

7 本すべて通る。package で宣言した定数は定数式で、配列の次元に使える（MLS 3.6 §3.8.1・§10.1）。`P.n` は通常の名前の探索で見つかり（§5.3.2）、replaceable な package は redeclare しなければ既定を使う（§7.3）。確認日 2026-11-02。

- <https://specification.modelica.org/maint/3.6/operators-and-expressions.html>
- <https://specification.modelica.org/maint/3.6/arrays.html>
- <https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S113） |
| --- | --- | --- |
| `M`（`Real x[P.n]`） | ET004 | **通る** |
| `M1`（replaceable な package、既定のまま） | EF024 `n` | EF024 `n`（同じ） |
| `M2`（`parameter Integer m = P.n`） | ET004 | **通る** |
| `M3`（部品が replaceable な package の既定を使う） | ET004 `c.x` | ET004 `c.x`（同じ。`[Expression(Medium.n)]`） |
| `M5`（モデルの中の record、package は redeclare 済み） | EF024 `n` | EF024 `n`（同じ） |
| `M0`・`M4`（対照） | 通る | 通る |

- `globe check` は `M` で GM-E2001（S2）、`M1` で GM-U0002（S4）を出す（Rumoca v0.10.2 を使う GlobeMode）。
- OpenModelica 1.27.2 の `checkModel` は 7 本とも completed successfully。
- v0.10.4 には「Evaluate constant dimensions by declaration identity in typecheck and flatten」というコミットが入っている。素直な形が通るようになったのはこれと見られる【推測】。

## ライブラリでの影響

GlobeMode の公開ライブラリの検査（run 41365960542、2026-11-02、Rumoca v0.10.2、MSL 4.1.0）で 4 本。

| 例題 | 形 | v0.10.4 での見込み |
| --- | --- | --- |
| `Buildings.HeatTransfer.Conduction.BaseClasses.Examples.Temperature_u`（`Real ud[Buildings.HeatTransfer.Conduction.nSupPCM]`） | `M` | 直る見込み【未確認】 |
| IBPSA と Buildings の `Fluid.HeatExchangers.Examples.WetCoilEffectivenessNTUMassFlow`（部品 `UAFroRated` が `MediumA` の既定を使う） | `M3` | 残る |
| `AixLib.Fluid.Storage.Examples.TwoPhaseSeparator`（モデルの中の record の `mXi[Medium.nXi]`） | `M5` | 残る |

ライブラリの例題そのものは v0.10.4 で回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と、その配布バイナリ `rumoca-linux-x86_64`。
- 2026-11-03（S113）に最新のリリース Rumoca v0.10.4（タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、2026-11-02 公開、配布バイナリ、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）で再確認した。
- Rumoca の main（`91d6e936648387d04648ec9c52237479eaec46d2`、v0.10.4 の 10 コミット後）はビルドしていない。10 コミットの題に名前の探索・次元・束縛に触れるものは無い。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。モデルの側では、次元を持つ部品で package を redeclare するか、モデル自身の定数を使う。

## 備考

- 定数を式でだけ使う（`Real y = P.n*time`）と通る。次元に使うときだけ落ちる。
- replaceable な package を通すと ET004 でなく EF024 になる理由は追っていない【未確認】。
- 出すなら、件名と本文を `M1`・`M3`・`M5` に絞り、`M`・`M2` は v0.10.4 で直った対照として書く。
