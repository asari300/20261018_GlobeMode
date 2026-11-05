# 下書き（未提出）13: BDF が誤りで止まる例題を、RK は定数の（誤った）温度で黙って終える

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [13-rk-silently-wrong-result-generationoffmus.md](13-rk-silently-wrong-result-generationoffmus.md)、1 枚の解説は [_jp.html](13-rk-silently-wrong-result-generationoffmus_jp.html)。再現用の `.mo` は無い（MSL の例題と CLI のコマンド）。

判定（GlobeMode、S012・S013。S162 で v0.10.4 で再確認）: **Rumoca のバグ**（黙って誤った結果を返す）。v0.10.4 でも再現する。

## 概要

`Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs` は、初期化で方向微分が作れないので BDF で失敗する。RK（`--solver rk-like`）では終了時刻まで走るが、`directCapacity.heatCapacitor.T` が全期間 293.15 K のままで、誤りも警告も出ない。結果が黙って誤る。RK も BDF と同じ理由で失敗するか、導関数を作るかのどちらかにしてほしい。

## 再現手順

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs --source-root <MSL のルート> --solver rk-like -o out.csv
```

`directCapacity.heatCapacitor.T` の列を読む。下書きでは旗の綴りが【未確認】だった。S162 で CLI の旗として確かめた。

## 期待

温度がモデルの熱流で変わる（参照結果は変わる）か、実行が失敗する。

## 実際

| Rumoca | `--solver rk-like` | `--solver bdf` |
| --- | --- | --- |
| v0.10.2 | 501 点で完了。`directCapacity.heatCapacitor.T` は 293.15 K で最小も最大も同じ。同じ容量の `directCapacity.T` は 293.15 から 365.49 K | `[EX001] directional derivative is unavailable: algebraic projection sensitivity matrix is singular` |
| v0.10.4（S162） | 同じ（最後の行まで同じ値） | 同じ文 |

同じ容量なのに 2 つの信号が食い違う。

## 影響

MSL 1 本。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

あり。`auto` は方向微分が作れないとき RK に切り替えず、誤った値を返さずに `GM-U0004` で止まる（S013、[ADR-0015](../../adr/0015-auto-and-reference-tolerance.md)）。

## 備考

- BDF が要る方向微分を RK の道が要らない理由は分かっていない【未確認】。
- 提出の順は「残り（02〜15）」の群。
