# 下書き（未提出）40: `redeclare model M = Impl(show = true)` の修飾が捨てられ、`show` を条件にした成分が消える

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [40-redeclare-class-modification-dropped.md](40-redeclare-class-modification-dropped.md)、再現は [.mo](40-redeclare-class-modification-dropped.mo)。

判定（GlobeMode、S362）: **バグ**。`Holder h(redeclare model M = Impl(show = true))` の短いクラス定義 `M = Impl(show = true)` は、`M` が指す `Impl` を修飾する（MLS 3.7 の 7.2: 修飾は短いクラス定義にも書け、例は `redeclare type Voltage = Real(unit = "V")`。7.3: `redeclare` はクラスを差し替える）。Rumoca は `h.m.show` を `Impl` の既定の `false` で平坦化するので、`if show` で宣言した成分ができない。omc 1.27.2 と、同じ修飾を成分に書いた形は `show = true` になる。

状態（S362、2026-11-16）: v0.10.2（GlobeMode の固定）と v0.10.4（最新のリリース）で再現。Rumoca の main はビルドしていない。

## 概要

`Holder` の中に `replaceable model M = Impl;`、使う側に `Holder h(redeclare model M = Impl(show = true))`。`Impl` は `parameter Boolean show = false;` と `Real y = 2 if show;` を持つ `Base` を継ぐ。差し替えた形の flat のモデル（`--emit flat-json`）は `h.m.show` がリテラルの `false` に束縛され、変数 `h.m.y` が無い。対照の形、成分に書いた `Holder h(m(show = true))` と直接の `Impl m(show = true)` は、`show` が `true` で `y` が残る。

## 再現手順

```bash
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.A --emit flat-json   # redeclare model M = Impl(show = true)
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.B --emit flat-json   # 対照: h(m(show = true))
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.C --emit flat-json   # 対照: Impl m(show = true)
```

`variables` のキーと、`h.m.show` の `binding` を見る。

## 期待

`Rep40.A` は `B`・`C` と同じく `h.m.show = true` で、変数 `h.m.y` がある。

## 実際

| モデル | `show` | 変数 |
| --- | --- | --- |
| `A`（`redeclare model M = Impl(show = true)`） | `false` | `h.m.show`、`h.m.x` |
| `B`（`h(m(show = true))`、対照） | `true` | `h.m.show`、`h.m.x`、`h.m.y` |
| `C`（`Impl m(show = true)`、対照） | `true` | `m.show`、`m.x`、`m.y` |

v0.10.2 も v0.10.4 も同じ。omc 1.27.2 の `instantiateModel(Rep40.A)` は `final parameter Boolean h.m.show = true;` と `Real h.m.y = 2.0;`。

## 環境

- Rumoca v0.10.2（配布の CLI `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布の `rumoca 0.10.4`、タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>。2026-11-16 でも最新のリリース。S359 で確認）。Rumoca の main はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。omc 1.27.2。

## GlobeMode 側の回避

要らない。GlobeMode の自前の flat（`globe-mode-flat`）は修飾を適用して `h.m.y` を残すので、この形を使う例題では Rumoca と食い違う。Track B はその差を記録し（[ADR-0039](../../adr/0039-native-cst-parser.md) の付記 S355・S362）、Rumoca の挙動には合わせない。

## 補足

- MSL 4.1.0（`Fluid/Pipes.mo`・`Fluid/Examples/InverseParameterization.mo`・`TraceSubstances.mo` を読んだ。2026-11-16 確認）: `Modelica.Fluid.Examples.InverseParameterization` は、`pipe1` が `redeclare model FlowModel = Modelica.Fluid.Pipes.BaseClasses.FlowModels.NominalTurbulentPipeFlow(show_Res = true, …)`、`pipe2` が `redeclare model FlowModel = …FlowModels.NominalLaminarFlow(show_Res = true, …)`。`if show_Res` の条件つき宣言は、`PartialStaggeredFlowModel` の `Res[n]`・`m_flows_turbulent[n-1]`（2 本の管とも）、`NominalTurbulentPipeFlow` の `dps_fg_turbulent[n-1]`（`pipe1` だけ）、`NominalLaminarFlow` の `pathLengths_nominal[n-1]`（`pipe2` だけ。`NominalTurbulentPipeFlow` では条件なし）。`StaticPipe` は `n = 2`。自前の flat は変数が 6 本多く（513 対 507。`pipe1` は `Res`・`m_flows_turbulent`・`dps_fg_turbulent`、`pipe2` は `Res`・`m_flows_turbulent`・`pathLengths_nominal`）、DAE はスカラーの方程式と未知数が 8 本多い（287 対 279。管ごとに 4）。S355 の突き合わせで変数の名前が合わなかった残り 2 本、`Modelica.Fluid.Examples.TraceSubstances` の `RoomCO2`・`RoomCO2WithControls` は `…FlowModels.DetailedPipeFlow(show_Res = true)` を使い、増えるのは `Res`・`m_flows_turbulent` だけ（`DetailedPipeFlow` に `dps_fg_turbulent` は無い。変数は 293 対 291、748 対 744）。原因は同じ。Rumoca 自身の釣り合いがこれらの例題でどうなるかは分からない【未確認】。
- 出典: GlobeMode の S355（小さなモデルで見つけた）と S362（この下書き。`InverseParameterization` の DAE の突き合わせ）、MLS 3.7 <https://specification.modelica.org/maint/3.7/inheritance-modification-and-redeclaration.html>（7.2・7.3。見出しと 7.2 の例を読んだ。2026-11-16 確認）。
- AixLib 3.0.1（コミット `e5df9a8d`、S373）: `AixLib.Airflow.WindowVentilation.Examples.VentilationFlowRateSashOpening` は `vdi2078_2` と `vdi2078_3` に `redeclare model OpeningArea = AixLib.Airflow.WindowVentilation.OpeningAreas.OpeningAreaSashVDI2078 (use_opnWidth_in = false, prescribedOpnWidth = 0.1)` を渡す。差し替えたクラスの `use_opnWidth_in` が、成分の `final parameter Boolean use_opnWidth_in = openingArea.use_opnWidth_in` に届き、`opnWidth_in` は `if use_opnWidth_in` で宣言されている。自前の flat はここに `opnWidth_in` を作らず（旗が `false`）、Rumoca は 2 つとも残す（変数は 588 対 590）。この下書きの形（短いクラス定義の修飾が捨てられる）と見られるが、差し替えたクラスが基底クラスで旗を決める形でも、下書き 44 の条件付き成分が同じ結果になる。AixLib のモデルを切り出していないので、2 つの見分けはついていない【未確認】。同じ package の `VentilationFlowRateSimpleOpening` は下書き 44。
