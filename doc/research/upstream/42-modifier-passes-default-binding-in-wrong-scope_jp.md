# 下書き（未提出）42: `vol(massDynamics = massDynamics)` が囲むモデルの parameter の既定を `vol` の側で評価し、その parameter を条件にした `if` 方程式が違う枝になる

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [42-modifier-passes-default-binding-in-wrong-scope.md](42-modifier-passes-default-binding-in-wrong-scope.md)、再現は [.mo](42-modifier-passes-default-binding-in-wrong-scope.mo)。

判定（GlobeMode、S373）: **バグ**。`Boiler b(energyDynamics = Dyn.Steady)` の `Boiler` は `Vol vol(massDynamics = massDynamics)` で `massDynamics` を下へ渡す。`b.massDynamics` の既定は `energyDynamics` なので、`b.vol.massDynamics` は `Steady` になるはず（MLS 3.7 の 7.2: 修飾の右辺は囲むクラスの中で評価する）。Rumoca は `vol` の `if massDynamics == Dyn.Steady then 0 = f else der(m) = f` を `else` の枝で平らにする。`b.massDynamics` を `energyDynamics` の既定の `Dynamic` と見ているように見える。omc 1.27.2 は `0.0 = b.vol.f`。

状態（S373、2026-11-16）: v0.10.2（GlobeMode の固定）と v0.10.4（最新のリリース）で再現。Rumoca の main はビルドしていない。

## 概要

`Decl` が `parameter Dyn energyDynamics = Dyn.Dynamic;` と `parameter Dyn massDynamics = energyDynamics;` を宣言する。`Boiler` も `Vol` も `Decl` を継ぐ。`Boiler` は `Vol vol(massDynamics = massDynamics)` を持ち、使う側は `Boiler b(energyDynamics = Dyn.Steady)`。`Vol` には `if massDynamics == Dyn.Steady then 0 = f else der(m) = f` がある。`A` の flat（`--emit flat-mo`）は、`0 = b.vol.f;` のはずが `der(b.vol.m) = b.vol.f;` になる。対照は通る: `Vol5` が `Decl` を継がずに 2 つの parameter を自分で宣言する形（`B`）、`energyDynamics` だけを渡す形（`C`）、`b` に直接 `massDynamics` を書く形（`D`）。

## 再現手順

```bash
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.A --emit flat-mo   # 違う枝
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.B --emit flat-mo   # 対照: Vol5 が parameter を自分で宣言
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.C --emit flat-mo   # 対照: energyDynamics だけ渡す
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.D --emit flat-mo   # 対照: b に massDynamics を書く
```

`b.vol`（対照では `v`）の方程式が `0 = ...` か `der(...) = ...` かを見る。

## 期待

`A` は `B`・`C`・`D` と同じ `0 = b.vol.f;`。

## 実際

| モデル | `vol` の方程式（v0.10.2） | `vol` の方程式（v0.10.4） |
| --- | --- | --- |
| `A`（`vol(massDynamics = massDynamics)`） | `der(b.vol.m) = b.vol.f;` | `der(b.vol.m) = b.vol.f;` |
| `B`（対照: `Vol5` が自分で宣言） | `0 = b.vol.f;` | `0 = b.vol.f;` |
| `C`（対照: `energyDynamics` だけ渡す） | `0 = b.vol.f;` | `0 = b.vol.f;` |
| `D`（対照: `b` に `massDynamics`） | `0 = b.vol.f;` | `0 = b.vol.f;` |

omc 1.27.2 の `instantiateModel(Rep42.A)` は `final parameter ... b.vol.massDynamics = Rep42.Dyn.Steady;` と `0.0 = b.vol.f;`。

外れるのは、囲むモデルと部品が同じ基底から既定 `massDynamics = energyDynamics` を継ぎ、囲む側がその parameter を下へ渡す組み合わせだけ。`b.massDynamics` の既定の束縛 `energyDynamics` を `vol` の側（`vol.energyDynamics` は `Dynamic`）で評価しているように見える。

## 環境

- Rumoca v0.10.2（配布の CLI `rumoca 0.10.2`。GlobeMode は rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` に固定）と v0.10.4（配布の `rumoca 0.10.4`、タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`、<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>。2026-11-16 でも最新のリリース。S359 で確認）。Rumoca の main はビルドしていない。
- Linux x86_64（WSL2、Ubuntu）。omc 1.27.2。

## GlobeMode 側の回避

要らない。GlobeMode の自前の flat（`globe-mode-flat`）は `b.vol.massDynamics = b.massDynamics = b.energyDynamics` の連鎖を評価して `0 = f` を残す。下の例題の DAE では状態が 6 対 7 で食い違い、`cargo xtask diff-frontend` が報告する。Rumoca の挙動には合わせない。

## 補足

- Buildings 14.0.0（コミット `ad48324b`）の `Buildings.DHC.Plants.Steam.BaseClasses.Examples.BoilerPolynomial` で見つけた（S373。S369 の `libraries.yml` の run から）。`boiSte` は `energyDynamics = SteadyState` の `BoilerPolynomial`。`massDynamics` の既定は `energyDynamics`（`LumpedVolumeDeclarations`・`PartialTwoPortTwoMedium` が宣言）で、体積 `vol`（`ControlVolumeEvaporation`。`PartialSaturatedControlVolume` を継ぎ、これも `PartialTwoPortTwoMedium` を継ぐ）には `final massDynamics = massDynamics` が渡る。体積の `if massDynamics == SteadyState then 0 = mWat_flow + mSte_flow else der(m) = mWat_flow + mSte_flow` が Rumoca では `else` の枝になる。方程式と未知数の数は同じ（161 対 161。どちらの枝も 1 本）だが、Rumoca は `der(boiSte.vol.m)` を持ち、状態が 7 本（GlobeMode は 6 本）になる。Rumoca の `boiSte.vol` の構造の行は、この方程式に `der(m)`・`mWat_flow`・`mSte_flow` を並べる（定常の枝は 2 つの流量だけ）。
- 同じ形は多い（`Buildings.Fluid.Interfaces.LumpedVolumeDeclarations` の `massDynamics = energyDynamics`、Buildings と IBPSA の多くのモデルが `final massDynamics = massDynamics` で渡す）。変わる例題の本数は分からない。7 ライブラリの突き合わせで見えたのは `BoilerPolynomial` だけ（状態は Rumoca が釣り合いまで通した例題でしか比べない。Buildings は 351 本のうち 116 本）【ほかの例題の本数は未確認】。
- 出典: GlobeMode の S373、MLS 3.7 <https://specification.modelica.org/maint/3.7/inheritance-modification-and-redeclaration.html>（7.2。節の見出しを読んだだけで、評価の範囲の文言は読み直していない【未確認】）。
