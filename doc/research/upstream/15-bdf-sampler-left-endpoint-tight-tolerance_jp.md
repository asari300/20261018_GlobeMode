# 下書き（未提出）15: BDF の補間が、rtol = 1e-7（ときに 1e-6）でホストの左端の検査に通らない

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [15-bdf-sampler-left-endpoint-tight-tolerance.md](15-bdf-sampler-left-endpoint-tight-tolerance.md)、1 枚の解説は [_jp.html](15-bdf-sampler-left-endpoint-tight-tolerance_jp.html)。再現用の `.mo` は無い（MSL の例題と CLI のコマンド）。

判定（GlobeMode、S050・S055。S162 で v0.10.4 で再確認）: **ほとんど v0.10.4 で直った。** 左端の契約の誤りで止まっていた 9 本は、`--solver bdf` の rtol = atol = 1e-7 と 1e-6 で通る。残るのは `Dimmer_RL`（丸めの幅の誤り）だけ。出すなら `Dimmer_RL` の 1 本。

## 概要

既定の `bdf`（`rumoca-solver-diffsol`）で、MSL 4.1.0 の 9 本が相対許容誤差 1e-7 で `MeSessionError::Contract`（`accepted-interval left endpoint sampler state <i> is <a>, but the checked point carries <b>; the disagreement exceeds <bound>`）で止まっていた。食い違いは上限の 1.1 倍から 445 倍。10 本目の `Modelica.Electrical.PowerConverters.Examples.ACAC.Dimmer_RL` は `accepted step from t=... to t=... made no progress beyond roundoff` で止まる。原因は、BDF の 1 歩前の補間が、その歩の局所誤差程度にしか正確でないこと（diffsol は全状態の RMS で検査する）。ホストは成分ごとに検査するので、局所誤差が RMS 平均の数倍の成分は積分器の検査に通ってホストの検査に落ちる。

## 再現手順

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic --source-root <MSL のルート> --solver bdf --rtol 1e-7 --atol 1e-7
```

下書きは GlobeMode の `globe simulate`（ライブラリ API）での観測だった。S162 で Rumoca の CLI で確かめた（GlobeMode の `msl-sim` と同じ rtol = atol）。

## 期待

BDF のサンプラが自分の受理した端点と、ホストが求める上限の中で一致する。

## 実際

CLI での結果（「契約」は `[EX001] accepted-interval left endpoint sampler state ...`）。

| モデル | v0.10.2, 1e-7 | v0.10.2, 1e-6 | v0.10.4, 1e-7 | v0.10.4, 1e-6 |
| --- | --- | --- | --- | --- |
| `ThyristorBridge2Pulse_RLV_Characteristic` | 契約（1.1e-7） | 通る | 通る | 通る |
| `ThyristorCenterTap2Pulse_RLV_Characteristic` | 通る | 通る | 通る | 通る |
| `ThyristorCenterTap2mPulse_RLV_Characteristic` | 契約（1.0e-6） | 契約（5.0e-6） | 通る | 通る |
| `ThyristorCenterTapmPulse_RLV_Characteristic` | 契約（4.7e-5） | 通る | 通る | 通る |
| `ChopperBuckBoost_DutyCycle` | 契約（9.1e-6） | 契約（1.8e-4） | 通る | 通る |
| `ThyristorBridge2mPulse_DC_Drive` | 事象の条件の方程式が収束しない（t = 0.10166） | 未記録 | 通る | 通る（滑りモードの警告） |
| `ThyristorBridge2mPulse_RLV_Characteristic` | 契約（3.9e-6） | 通る | 通る | 通る |
| `ThyristorBridge2Pulse_DC_Drive` | 契約 | 契約 | 通る | 通る |
| `Dimmer_RL` | 丸めの幅（t = 5.0416） | 240 秒で終わらず | 丸めの幅（t = 5.0416） | 丸めの幅（t = 6.2014） |

下書きの表（GlobeMode）と CLI の数字は少しずれる（例: `ThyristorCenterTap2Pulse_RLV_Characteristic` は CLI の v0.10.2 では通る）。GlobeMode はライブラリ API でホストを動かすため。型は同じ。

直った理由: v0.10.4 が同梱する diffsol の BDF の修正（`vendor/diffsol/RUMOCA_PATCH.md` の "The fifth correction"）。受理した歩の連続拡張を、次の歩（はるかに小さい刻み）に合わせて再スケールした表でなく、受理の直後に写した表で評価する。再スケールは (h_old/h_new)^k / k! の倍率で丸めの誤差を増やしていた。

残る: `Dimmer_RL` は `accepted step from t=5.041640891166859 to t=5.041640891166959 made no progress beyond roundoff`（v0.10.4、1e-7）。下書きの最後の備考にある別の欠陥（BDF の刻みの下限が、ホストの丸めの幅より小さい）の可能性がある。この修正では変わらない。

## 影響

MSL 9 本（直った）と 1 本（`Dimmer_RL`）。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）。diffsol 0.13.2 と Rumoca の `vendor/diffsol` の修正。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

一部。`--solver auto` がサンプラのメッセージで回し直す（S012、`GM-I11002`。後に esdirk34、[ADR-0028](../../adr/0028-auto-esdirk34-fallback.md)）。契約の誤りの 9 本は Rumoca を v0.10.4 に上げれば回避が要らなくなる見込み（`msl:funnel`・`msl:sim` の本数で確かめる）。

## 備考

- 関連: 下書き 02（同じメッセージ、別のモデル。こちらも同じ修正で直った）。
- `Dimmer_RL` は `fallback()` がサンプラのメッセージにだけ一致するので回し直さない。陽的ソルバはどれも時間切れ。
