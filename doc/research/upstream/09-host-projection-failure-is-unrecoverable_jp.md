# 下書き（未提出）09: 試しの点での代数射影の失敗を、復帰不能な失敗として記憶する

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [09-host-projection-failure-is-unrecoverable.md](09-host-projection-failure-is-unrecoverable.md)、1 枚の解説は [_jp.html](09-host-projection-failure-is-unrecoverable_jp.html)。再現用の `.mo` は無い（MSL の例題と CLI のコマンド）。

判定（GlobeMode、S021〜S024。S162 で v0.10.4 で再確認）: **Rumoca の設計の穴**（陽的 RK の限界の一部でもある。要望は記憶の仕方だけ）。v0.10.4 でも再現する。

## 概要

モデル交換のホストの導関数の評価で、積分器が**試しの点**（棄却される RK の段、Newton 法の反復、刻みを半分にしての再試行）で右辺を評価し、そこで代数射影が発散すると、ホストはそれを復帰不能として記録する（`crates/rumoca-solver/src/fmi_me/integrator/derivative.rs` の `fail`）。`discards_failed_trials()` が真の部品（縮約した chart を切り替えるもの）だけが「この試しの点は捨てる」と扱う。ほかでは、失敗の後で刻みを縮める部品も最後は誤りで終わる。結果として、剛い回路が陽的 RK（`dopri45`・`tsit45`）と陰的 RK（`esdirk34`）で `algebraic projection did not establish coordinate convergence` で落ちる。BDF なら易しい軌道でも。

## 再現手順

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.Analog.Examples.SimpleTriacCircuit --source-root <MSL のルート> --solver rk-like --t-end 2e-5
```

下書きでは旗の名前が【未確認】だった。CLI の陽的 RK は `--solver rk-like`（`dopri45` は無い）、終了時刻は `--t-end`。陰的 RK の例は GlobeMode の `--solver esdirk34` で `ThyristorBehaviourTest`（t が (5.3e-5, 5.5e-5] で落ちる）。CLI には無いので S162 では回していない。

## 期待

試しの点で射影が発散したら、積分器がその試しを棄却して刻みを縮める。刻みが最小を下回ったときだけ失敗する。

## 実際

| Rumoca | `--solver rk-like --t-end 2e-5` |
| --- | --- |
| v0.10.2 | `[EX001] algebraic projection did not establish coordinate convergence: worst scaled residual row=36 target=simpleTriac.thyristor1.Anode.i value=7.154881e-1 ratio=7.154881e9 ...` |
| v0.10.4（S162） | 同じ文、同じ数値 |

ソースでは `fail` が、試しの点を捨てる部品でなければ失敗を `refuse` する。v0.10.4 の `derivative.rs` の差は、その旗を構造体に持たせ直しただけで、動作は同じ。

## 影響

MSL 2 本以上。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

一部。剛い回路には `bdf` か `esdirk34` を勧める。`ThyristorBehaviourTest` は `esdirk34` でもまだ落ちる。受理した点から遠い試しの点で NaN を返す保護は、1 件で効いたが閾値が恣意的なので採用していない。

## 備考

- 陽的 RK では剛さの限界の一部なので、GlobeMode は plugin のバグとは扱わない。要望は「復帰不能の記憶」だけ。
- ホストの射影が前の解から始まるので受理点の近くで失敗し続ける、という仮説は 1 回の測定から【未確認】。
