# 下書き（未提出）02: 許容誤差 1e-12 で、サンプラの左端の検査に丸めの床が無い

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [02-sampler-endpoint-roundoff.md](02-sampler-endpoint-roundoff.md)、1 枚の解説は [_jp.html](02-sampler-endpoint-roundoff_jp.html)。再現用の `.mo` は無い（MSL の例題と CLI のコマンド）。

判定（GlobeMode、S011。S162 で v0.10.4 で再確認）: **一部は v0.10.4 で直った**。`ChopperBuckBoost_DutyCycle` は直り、`SimpleTriacCircuit` を `Tolerance = 1e-12` で回す形（丸めの床が無い）が残る。出すなら後者だけ。

## 概要

モデル交換のホストが、受理した区間の左端でサンプラの値と積分器の値が食い違うと契約の誤りで止める。食い違いの上限に丸めの床が無い。

- `SimpleTriacCircuit`: モデルが `Tolerance = 1e-12` を指定する。食い違いは 1.0e-12、上限は 1e-12 で、わずかに超える。上限は機械イプシロンと値の大きさの積の数倍を下回らないのがよい。
- `ChopperBuckBoost_DutyCycle`: BDF で食い違い 1.8e-4、上限 1.2e-5（RK なら通る）。**v0.10.4 で直った。**

## 再現手順

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.Analog.Examples.SimpleTriacCircuit --source-root <MSL のルート> --solver bdf --rtol 1e-12 --atol 1e-12
```

下書きの時点では CLI で確かめていなかった。S162 で CLI で確かめた。

## 期待

1e-12 の許容誤差（珍しいが正しい注釈）が通るか、倍精度で解像できる値に丸められる。

## 実際

| モデルと指定 | v0.10.2 | v0.10.4（S162） |
| --- | --- | --- |
| `ChopperBuckBoost_DutyCycle`（既定、`--solver bdf` でも同じ） | `[EX001] accepted-interval left endpoint sampler state 0 is 11.732514634476857, but the checked point carries 11.732335121155664; the disagreement exceeds 0.000012` | 通る（16017 点） |
| `SimpleTriacCircuit`（既定） | 通る | 通る |
| `SimpleTriacCircuit`（`--rtol 1e-12 --atol 1e-12`） | 食い違い 1.005e-12、上限 1e-12 で `[EX001]` | 食い違い 1.13e-12、上限 1e-12 で `[EX001]`（state 1） |

- CLI の既定はモデルの `Tolerance = 1e-12` をそのまま使わない（既定の実行は通る）ので、指定して試す。
- `ChopperBuckBoost_DutyCycle` が直ったのは、v0.10.4 が同梱する diffsol の BDF の修正（`vendor/diffsol/RUMOCA_PATCH.md` の "The fifth correction"。受理した区間の連続拡張を保つ）による。下書き 15 と同じ原因。

## 影響

MSL 2 本（S011）。GlobeMode の回避後は 0 本。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）と配布バイナリ。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、配布バイナリ。2026-11-05 でも最新）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

あり。(1) 許容誤差が 1e-10 より小さいときは 1e-10 に上げて `GM-W10003` を出す（S012）。(2) `--solver auto` はサンプラの食い違いで dopri45 に回し直す（`GM-I11002`）。症状を隠すだけで、上限そのものは変わらない。

## 備考

- 該当のコードは英語の本文の Notes にある（`fmi_me/root.rs` の `state_consistency_bound` の周り）。
- 提出の順は「残り（02〜15）」の群。出すなら `SimpleTriacCircuit` の 1 点に絞る。
