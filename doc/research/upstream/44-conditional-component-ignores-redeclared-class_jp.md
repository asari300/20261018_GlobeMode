# 下書き（未提出）44: 差し替えたクラスが `use_in = false` を決めても、`if area.use_in` の条件付き成分が残る

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [44-conditional-component-ignores-redeclared-class.md](44-conditional-component-ignores-redeclared-class.md)、再現は [.mo](44-conditional-component-ignores-redeclared-class.mo)。

判定（GlobeMode、S373）: **バグ**。`Holder h(redeclare model Area = AreaFinal)` は replaceable な `Area` を `AreaFinal` に差し替える。`AreaFinal` は基底クラスの修飾で `use_in = false` を決める（`extends PartArea(final use_in = false)`）。`Holder` は `Real u = 1 if area.use_in;` を宣言する。条件は `false` なので `h.u` は無いはず（MLS 3.7 の 4.4.5: 条件が偽なら、成分は修飾も含めて平らにした DAE から取り除かれる。2026-11-16 に <https://specification.modelica.org/maint/3.7/class-predefined-types-and-declarations.html> で読んだ。7.3: `redeclare` はクラスを差し替える）。Rumoca は `h.u` を残す。flat のモデルに `h.area.use_in` と `h.u` がある。omc 1.27.2 は `h.area.use_in = false` だけ。

状態（S373、2026-11-16）: v0.10.2（GlobeMode の固定）と v0.10.4（最新のリリース）で再現。Rumoca の main はビルドしていない。

## 概要

`PartArea` は `parameter Boolean use_in = true;`。`AreaFinal` はそれを `final use_in = false` で継ぐ。`Holder` は `replaceable model Area = PartArea; Area area;` と `Real u = 1 if area.use_in;`。`Holder h(redeclare model Area = AreaFinal)`（`A`）の flat のモデル（`--emit flat-json`、`variables` のキー）に `h.u` が出る。対照は正しく取り除かれる: 成分が replaceable でなくクラスそのもの（`B`）、`use_in` を成分への修飾で決める形（`C`）、モデル自身の旗（`D`）。`h.area.use_in` の値は正しく `false` と評価される。条件付き成分だけが、replaceable なクラスの既定（`true`）で決まる。

## 再現手順

```bash
# このファイルだけがあるディレクトリで回す（コンパイラは隣の .mo も読む）
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.A --emit flat-json   # redeclare model Area = AreaFinal
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.B --emit flat-json   # 対照: AreaFinal area
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.C --emit flat-json   # 対照: h(area(use_in = false))
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.D --emit flat-json   # 対照: モデルの旗
```

`variables` のキーを見る。

## 期待

`Rep44.A` は `B`・`C`・`D` と同じく、`h.area.use_in` だけ。

## 実際

| モデル | `variables` のキー（v0.10.2） | `variables` のキー（v0.10.4） |
| --- | --- | --- |
| `A`（`redeclare model Area = AreaFinal`） | `h.area.use_in`、`h.u` | `h.area.use_in`、`h.u` |
| `B`（対照: `AreaFinal area`） | `area.use_in` | `area.use_in` |
| `C`（対照: `h(area(use_in = false))`） | `h.area.use_in` | `h.area.use_in` |
| `D`（対照: モデルの旗） | `flag` | `flag` |

omc 1.27.2 の `instantiateModel(Rep44.A)` は `final parameter Boolean h.area.use_in = false;` で、`u` は無い。

## 環境

- Rumoca v0.10.2（配布の CLI `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布の `rumoca 0.10.4`、タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>。2026-11-16 でも最新のリリース。S359 で確認）。Rumoca の main はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。omc 1.27.2。

## GlobeMode 側の回避

要らない。GlobeMode の自前の flat（`globe-mode-flat`）は差し替えたクラスで条件を評価して `h.u` を作らない。この形を使う例題では Rumoca と食い違い、`cargo xtask diff-frontend` が変数の名前の差として報告する。Rumoca の挙動には合わせない。

## 補足

- AixLib 3.0.1（コミット `e5df9a8d`）の `AixLib.Airflow.WindowVentilation.Examples.VentilationFlowRateSimpleOpening` で見つけた。経験式のモデル（`ASHRAE`・`Caciolo` ほか）は `replaceable model OpeningArea = AixLib.Airflow.WindowVentilation.OpeningAreas.OpeningAreaSimple` を差し替え、`OpeningAreaSimple` は `PartialOpeningArea(final use_opnWidth_in = false, ...)` を継ぐ。部分モデルは `final parameter Boolean use_opnWidth_in = openingArea.use_opnWidth_in;` と `RealInput opnWidth_in if use_opnWidth_in` を宣言する。自前の flat に `opnWidth_in` は無く、Rumoca は 12 個の部品すべてに残す（変数は 353 対 365）。AixLib のモデルは CLI で回していない（CLI は AixLib の木で `PKG-008` で止まる。同じ名前 `RadiatorTypes` の部分ディレクトリと `.mo` が並ぶため）。AixLib の例題の原因は、S369 の突き合わせ（ライブラリの API、v0.10.2）から読んだ【v0.10.4 の CLI での AixLib のモデルは未確認】。
- 同じ AixLib の package の sash の形（`VentilationFlowRateSashOpening`、`redeclare model OpeningArea = …OpeningAreaSashVDI2078 (use_opnWidth_in = false, …)`）は下書き 40（クラスの修飾が捨てられる）。40 とこの下書きの根が同じかは分からない。
- 出典: GlobeMode の S373、MLS 3.7 <https://specification.modelica.org/maint/3.7/class-predefined-types-and-declarations.html>（4.4.5。2026-11-16 に読んだ）と <https://specification.modelica.org/maint/3.7/inheritance-modification-and-redeclaration.html>（7.3。見出しだけ【それ以上は未確認】）。
