# 下書き（未提出）21: parameter の束縛の `getInstanceName()` と、成分配列を含む式の if 方程式で EF004

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [21-ef004-getinstancename-and-if-equation-branches.md](21-ef004-getinstancename-and-if-equation-branches.md)、再現は [.mo](21-ef004-getinstancename-and-if-equation-branches.mo)、1 枚の解説は [_jp.html](21-ef004-getinstancename-and-if-equation-branches_jp.html)。

判定（GlobeMode、S070。S115 で v0.10.4 で再確認）: **2 つとも Rumoca の穴**（正しい Modelica を断る）。原因は別々で、コードが同じ EF004。2 件に分けて出すか、1 件に 2 部で出す。omc は全部通す。v0.10.4 でも 2 つとも再現する。

## 概要

EF004 は Rumoca の flatten の「unsupported equation form」の受け皿。公開ライブラリの検査で 3 番目に多い（217 本）。

- **A. 方程式・algorithm の外の `getInstanceName()`**（119 本）。`final parameter String name = getInstanceName();` が `[EF004] unsupported equation form: getInstanceName() requires a model/block instance scope` で落ちる。equation・initial equation の `assert` の中なら通る。
- **B. 成分配列の成分とベクトルの積を含む if 方程式**（Chemical、41 本）。`hMix*(s.n_flow*ones(n)) = 0`（`s` は record の配列）を if の片方の枝に書くと、`n` が 2 以上で `[EF004] ... dynamic if-equation branches have different scalar row counts`。if の外に書くと `ED001 unbalanced model: 4 equations, 3 unknowns` になり、スカラの積を `n` 行と数えているように見える【推測】。

## 再現手順

```bash
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A    # A: 落ちる
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A2   # 対照: equation の assert
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A3   # 対照: initial equation の assert
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B    # B（n = 2）: 落ちる
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B3   # B（n = 3）: 落ちる
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B1   # 対照: n = 1
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B4   # if の外: ED001（4 方程式・3 未知数）
```

## 期待

- A: 3 本とも通る。`getInstanceName()` はモデル・ブロックの中で呼べば、そのインスタンスの名前を返す（MLS 3.6 §3.7.4.6）。モデルの中の parameter の束縛は「モデルの中」。
- B: すべて通り、釣り合う。ベクトルとベクトルの積はスカラ（§10.6.4 の表 10.11）なので、式は `n` によらず 1 行。`n = 3` で 5 方程式・5 未知数（omc）。

確認日 2026-11-01。

- <https://specification.modelica.org/maint/3.6/operators-and-expressions.html>
- <https://specification.modelica.org/maint/3.6/arrays.html>

## 実際

| モデル | v0.10.2 | v0.10.4（S115） |
| --- | --- | --- |
| `Rep21A` | EF004 `getInstanceName()` | EF004（同じ） |
| `Rep21B`・`Rep21B3` | EF004 `different scalar row counts` | EF004（同じ） |
| `Rep21B4` | ED001（4 方程式・3 未知数） | ED001（同じ） |
| `Rep21A2`・`Rep21A3`・`Rep21B1`（対照） | 通る | 通る |

- `globe check` は A・B とも S4 で GM-U0001、`Rep21B4` は S5 で GM-E5002。
- OpenModelica 1.27.2 の `checkModel` は `Rep21A`・`Rep21B`・`Rep21B3`・`Rep21B4` とも completed successfully（`Rep21B4` は 3 方程式・3 変数）。

## ライブラリでの影響

S063 で EF004 は 217 本。A が 119（ThermofluidStream 93、Buildings 11、IBPSA 8、AixLib 7）、B が Chemical 41（`Processes.mo` の 1174 行）。v0.10.4 で例題そのものは回していない【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-03（S115）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、配布バイナリ）で再確認。main は【未確認】。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。S4 の `GM-U0001`（Rumoca の未対応）として数える。

## 備考

- A は宣言の束縛を下げる文脈にインスタンス名が渡っていないように読める【読んだだけ】。B は if の枝の行数の推定の誤りに見える【推測】。
- Chemical のもう 1 つの失敗 `EF016`（21 本）は別の原因で、この下書きに含めない。
- 提出の順は 4 位。
