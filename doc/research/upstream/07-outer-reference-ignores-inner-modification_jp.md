# 下書き（未提出）07: outer の参照が、inner の宣言の修正でなく型の既定を読む

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [07-outer-reference-ignores-inner-modification.md](07-outer-reference-ignores-inner-modification.md)、再現は [.mo](07-outer-reference-ignores-inner-modification.mo)、1 枚の解説は [_jp.html](07-outer-reference-ignores-inner-modification_jp.html)。

判定（GlobeMode、S009。S162 で v0.10.4 で再確認）: **Rumoca のバグ**（inner の修正を無視する）。v0.10.4 でも再現する。

## 概要

`outer System system` を参照する部品に対し、対応する `inner System system(energyDynamics = Dynamics.FixedInitial)` がパラメータを修正しているとき、Rumoca は outer の参照を inner の修正でなく型の既定（`Free`）で評価する。`if massDynamics == Dynamics.FixedInitial` で守られた初期方程式が DAE から消える。MSL の Fluid の例題の形で（11 本が `inner Modelica.Fluid.System system(energyDynamics = FixedInitial)` を使う）、`PartialDistributedVolume` の初期方程式（`mediums.p = ps_start` など）が無くなる。

## 再現手順

```bash
rumoca compile 07-outer-reference-ignores-inner-modification.mo -m OuterModification --emit dae-json
```

出力の `storage.initialization_equation_operations` の個数を数える。期待は 1 個（2 要素の配列方程式 `volume.p = volume.ps_start`）。MSL の形は、`inner Modelica.Fluid.System system(energyDynamics = FixedInitial)` を宣言する Fluid の例題。

## 期待

初期方程式が有効（if の枝を取る）。

## 実際

| Rumoca | `OuterModification` | 型の既定を `FixedInitial` にした対照 |
| --- | ---: | ---: |
| v0.10.2 | 0 個 | 1 個 |
| v0.10.4（S162） | 0 個 | 1 個 |

型の既定を `FixedInitial` に変えると 1 個になる（既定が使われているのが原因）。どちらの版も `warning[WD001]: the branch of this equation is selected at translation, so parameter volume.massDynamics cannot be set: its branches differ in structure (MLS 3.7 §8.3.4)` を出す。

## 影響

MSL Fluid の `FixedInitial` の例題【本数未確認】。GlobeMode の S8（初期化の釣り合い）で初期方程式が足りない。`NonCircularPipes` では omc が冗長と言う 2 本の方程式が Rumoca の DAE に無い。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

なし。既知の限界として記録する（検証モデル `GlobeModeValidation.Init.OuterModification` と、直ると落ちる db の試験。不足は `GM-W8001`）。

## 備考

- 下書き 08（修正の範囲）と同じ族だが、別のコードの道。
