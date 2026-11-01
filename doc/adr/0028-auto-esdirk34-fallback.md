# ADR-0028 `auto` の回し直しは BDF → esdirk34 → dopri45 の順にする。歩が丸めの幅より進まない失敗も対象にする

- 状態: Accepted（2026-11-01、S064。全例題 356 本の CI（run 41287267439）で実行できた本数が 309 → 310、管に収まった本数が 301 のまま、減ったモデルは 0。結果は「CI の結果」の節）
- 関係: [ADR-0014](0014-session-route-wasm.md) の決定 (d)（`auto` は BDF が失敗したら dopri45 で回し直す）の切り替え先を拡げる。[ADR-0015](0015-auto-and-reference-tolerance.md) の (a)（方向微分が作れない失敗では切り替えない）と (b)（許容誤差の倍率）はそのまま。ADR-0015 は覆さないので Accepted のまま

## 背景

- `auto` は BDF が積分で失敗した（`GM-E11002`）か、Rumoca のサンプラが食い違った（`GM-U0005`）とき、dopri45 で 1 回だけ回し直す（S012、[ADR-0014](0014-session-route-wasm.md) (d)）。dopri45 は陽的なので、硬い系では刻みが安定限界に張り付いて予算内に終わらない（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の節「S055 で調べたこと」）。
- S055 で `bdf` 単独の `GM-U0005` 10 本を調べた。`auto` は 7 本を dopri45 で救い、2 本（`ThyristorBridge2Pulse_DC_Drive`・`ThyristorBridge2mPulse_DC_Drive`）は dopri45 でも時間切れ、1 本（`Dimmer_RL`）は回し直しの対象外の文言（「受け入れた歩が丸めの幅より進まない」。`MeIntegrationError::Contract`）だった。S050 の表では、この 3 本のうち `esdirk34` が終えるのは `Dimmer_RL` と `ThyristorBridge2mPulse_DC_Drive`（`ThyristorBridge2Pulse_DC_Drive` は `GM-E11003` で、表の読みは 1 本多かった）。
- 今の `auto` は、回し直しにも `max_wall_seconds` をそのまま渡す。BDF が 16 s かかって失敗しても dopri45 は 180 s 使えるので、最悪で予算の 2 倍近くになる（S064 の手元の測定: BDF 16 s + dopri45 180 s で 201 s）。msl-sim の外からの停止（予算 + 余白）が最後の歯止めだった。
- `esdirk34` は BDF で通る MSL の例題の一部で落ちたり遅かったりする（S050 の表: bdf が通って esdirk34 が落ちるのは 17 本。ほとんどは時間切れ）。ただし `auto` は BDF が通れば `esdirk34` を呼ばないので、この弱点には触れない。

## 決定

### (a) 回し直しの順序は BDF → esdirk34 → dopri45

- BDF が規則に当たる失敗（下の (b)）をしたら、まず `esdirk34`（diffsol の SDIRK。陰的）で回し直す。`esdirk34` が時間切れ以外の理由で止まったら、`dopri45` で回し直す。時間切れなら回し直さない（予算が残っていない）。
- **順序は 2 通りを比べて決めた**:
  - 案 A（BDF → dopri45 → esdirk34）: 剛性の系は dopri45 が予算の最後まで使うので、`esdirk34` に時間が残らない。S055 の 3 本（`Dimmer_RL`・`ThyristorBridge2mPulse_DC_Drive` ほか）は、この順だと今と同じく終わらない。dopri45 に予算の一部だけ渡す案も考えたが、`esdirk34` は `ThyristorBridge2mPulse_DC_Drive` で 94 s（ランナー）かかるので、分け方で当たり外れが出る。
  - 案 B（BDF → esdirk34 → dopri45）を採る。`dopri45` だけが救える系（`TwoMass`。`esdirk34` は `GM-E11002` で 3 秒以内に落ちる）は、`esdirk34` が早く失敗するので、そのまま dopri45 に回る。`esdirk34` が長く走って予算を使い切るのは、`dopri45` も終わらない硬い系だけ。
- `esdirk34` だけに置き換える案は採らない。`TwoMass` のように `dopri45` だけが終える系がある。
- 案 B の費用: `dopri45` で救っていた 7 本は `esdirk34` で救うことになり、数値の道が変わる（Rumoca の `dopri45` → 自前の diffsol の SDIRK）。結果は参照結果と比べて確かめる（`msl-sim` の管）。時間は手元の測定で 1.2〜2 倍（`ThyristorBridge2mPulse_RLV_Characteristic` 53.8 → 77.9 s ほか）。

### (b) 回し直しの対象に「歩が丸めの幅より進まない」を足す

- 規則に当たる失敗は、これまでの「積分で失敗（`GM-E11002`）」「サンプラの食い違い（`GM-U0005`）」に、`accepted step … made no progress beyond roundoff`（rumoca-solver の host の契約違反。`GM-U0005`）を足した。型に印が無いので文言で見分ける（サンプラの食い違いと同じ扱い）。
- `Dimmer_RL` の BDF がこれで止まる。`esdirk34` は 92.6 s（手元）で終える。
- 方向微分が作れない失敗（`GM-U0004`）で切り替えない規則（ADR-0015 (a)）は変えない。時間切れ・assert・初期化の失敗でも切り替えない。
- 2 回目の回し直し（`esdirk34` → `dopri45`）は、時間切れ以外の失敗ならすべて対象にする。`dopri45` が救える系を `esdirk34` の失敗の種類で取りこぼさないため。

### (c) 回し直しには元の予算をそのまま渡す（残りの時間を渡す案は採らなかった）

- 今の `auto` は、BDF にも回し直しにも `max_wall_seconds` をそのまま渡す。これを変えない。
- 最初は「元の予算から、これまでにかかった時間（組み立て + 実行）を引いた残り」を渡す形で実装し、CI（run 41282411061）で回したところ、`LightningSegmentedTransmissionLine` が BDF の `GM-U0005` のまま止まった（前は dopri45 で終了、管に収まる）。msl-sim の最初の実行は予算 30 s で、この系は BDF の組み立てと積分で 30 s を超えるので「残り 0 以下」で回し直されなかった。Rumoca の協調型の時間切れは、dopri45 の回し直しでは 30 s を超えて走る（S050 の記録で `run_seconds` 63.6 s）など、予算の測り方が積分器ごとに違い、残りの時間を渡す規則は今の結果を変えてしまう。
- 欠点は、`auto` の全体が最悪で予算の 3 倍近くになること（BDF + esdirk34 + dopri45）。外からの停止（予算 + 余白）が歯止めで、msl-sim では時間切れの回し直しとして数えられる。残りの時間を渡す規則は、予算の測り方を揃えてから別に決める。

### (d) 診断

- 切り替えるたびに情報を 1 件出す。dopri45 への切り替えは今までどおり `GM-I11002`、esdirk34 への切り替えは新しい `GM-I11003`。本文は「{前の積分器} が … ので、{次の積分器} で回し直した」で、前の失敗の診断を補足に付ける。
- `msl-sim` の `ソルバ` の欄は `auto → esdirk34`・`auto → dopri45`・`auto → esdirk34 → dopri45`。`solver_fallbacks` は両方の合計。

### 選ばなかった案

- `auto` の先頭を `esdirk34` にする: BDF が通る系で `esdirk34` は落ちたり遅かったりする（上の背景）。
- 回し直しを増やして `radau5` なども足す: 手元の 21 本で `esdirk34` と `dopri45` の 2 段で足りない系は `GM-E11002` で全部のソルバが落ちるもの（`HeatingNPN_NORGate` など）だけ。

## 結果

手元の測定（release、`globe simulate -L data/msl --solver auto`、許容誤差は S050 の設定、`--timeout 180`。機械は他のセッションのビルドで負荷が変わるので、秒数は目安）:

| モデル | 前 | 後 |
| --- | --- | --- |
| `Lightning…TransmissionLine`（予算 180 s） | dopri45 で終了 134 s | esdirk34 で終了 202 s（手元はランナーの約 1.7 倍遅い）。予算 30 s では前は dopri45 で終了 116 s、後は esdirk34 が時間切れ 121 s（msl-sim は時間切れを 180 s で回し直す） |
| `ThyristorBridge2Pulse_RLV_Characteristic` | dopri45 で終了 8.0 s | esdirk34 で終了 13.7 s |
| `ThyristorBridge2mPulse_RLV_Characteristic` | dopri45 で終了 53.8 s | esdirk34 で終了 77.9 s |
| `ThyristorCenterTap2Pulse_RLV_Characteristic` | dopri45 で終了 5.3 s | esdirk34 で終了 9.9 s |
| `ThyristorCenterTap2mPulse_RLV_Characteristic` | dopri45 で終了 19.6 s | esdirk34 で終了 39.8 s |
| `ThyristorCenterTapmPulse_RLV_Characteristic` | dopri45 で終了 14.4 s | esdirk34 で終了 22.7 s |
| `ChopperBuckBoost_DutyCycle` | dopri45 で終了 21.9 s | esdirk34 で終了 34.1 s |
| `Dimmer_RL` | `GM-U0005`（回し直さない）91.2 s | esdirk34 で終了 92.6 s |
| `ThyristorBridge2mPulse_DC_Drive` | 時間切れ 192.0 s | 手元は時間切れ（esdirk34 単独で 162 s。ランナーでは 94 s で、CI では終了） |
| `ThyristorBridge2Pulse_DC_Drive` | 時間切れ 201.3 s | 時間切れ（esdirk34 は `GM-E11003` で落ち、dopri45 も終わらない） |
| `TwoMass` | dopri45 で終了 3.1 s | esdirk34 が `GM-E11002`、dopri45 で終了 3.4 s |
| `HeatingNPN_NORGate`・`HeatingPNP_NORGate`・`InvertingSchmittTrigger`・`IMC_InverterDrive`・`TanksWithOverflow` | `GM-E11002`・`GM-E11003`（dopri45 も失敗） | 同じ失敗 |
| `SinCosEncoder`・`ThyristorBehaviourTest`・`IMC_DOL`・`SMPM_Braking`・`OneTank`（BDF で通る） | 回し直しなし | 回し直しなし（`GM-I1100x` なし） |

全例題 356 本の結果（CI の run）は下に書く。

## CI の結果

`gh workflow run msl-sim.yml -f 'solvers=["auto"]'`（`msl-sim` ジョブだけを見る。ジョブ `solvers` は `["auto"]` に絞って、`msl-sim` の完了後に取り消した）。比べる相手は S055 の run 41247200921（S050 の run と全モデルで同じ結果。Rumoca の rev も同じ）。

| 項目 | S055 の run | run 41282411061（残りの予算を渡す版） | run 41287267439（この ADR の版） |
| --- | ---: | ---: | ---: |
| 実行できた | 309 | 311 | **310** |
| 管に収まった / 比べた | 301 / 308 | 302 / 310 | **301 / 309** |
| 参照結果と比べた信号 | 1757 / 1773 | - | 1766 / 1784 |
| `auto` が回し直した（`GM-I11002`・`GM-I11003`） | 15 | 15 | 16 |
| ソルバの欄 | `auto → dopri45` 8 | `auto → esdirk34` 8、`auto → esdirk34 → dopri45` 1 | 同左 |
| `GM-U0005` の止まり | 1 | 1 | **0** |
| 時間切れの回し直し（実行できた） | 12（1） | 12（4） | 15（5） |
| ワーカーの時間 / 全体 | 2807 s / 3828 s | 2299 s / 3207 s | 2897 s / 4124 s（ランナーの速さが run ごとに違う） |

- **run 41282411061（残りの予算を渡す版）では `LightningSegmentedTransmissionLine` が 1 本減った**（前は dopri45 で終了して管に収まる → `GM-U0005` で止まる）。原因は (c)。この版は採らず、回し直しに元の予算を渡す版にした。この run の +2 は、`Dimmer_RL`（実行できるが管の外）、`ThyristorBridge2mPulse_DC_Drive`（管に収まる）、`HBridge_DC_Drive`（`auto` を通らない BDF 単独の時間切れが 166 s で通っただけ。次の run では 180 s で時間切れに戻った。境目の揺れ）の 3 本が増え、`Lightning` が 1 本減った結果。
- **run 41287267439（この ADR の版）の差**（両方にある 356 本）: 実行できた +1（`Dimmer_RL`）、管に収まった ±0、減ったモデル 0。`Dimmer_RL` は実行できたが `vH01.y_arg`・`iH01.y_arg`（`Dimmer_R` と同じ、第 1 高調波の位相）が管から外れた（`Dimmer_R` と同じ原因かは【未確認】）。
- `auto` が esdirk34 で終えたのは 8 本（`Lightning`・`ThyristorBridge2Pulse_RLV`・`ThyristorBridge2mPulse_RLV`・`ThyristorCenterTap2Pulse_RLV`・`ThyristorCenterTap2mPulse_RLV`・`ThyristorCenterTapmPulse_RLV`・`ChopperBuckBoost_DutyCycle`・`Dimmer_RL`）、esdirk34 → dopri45 で終えたのは 1 本（`TwoMass`）。dopri45 で終えていた 7 本は、すべて管に収まったまま。esdirk34 の失敗の後も dopri45 に回るので、`TwoMass` は取りこぼさない。
- 救えなかった 2 本の `ThyristorBridge2mPulse_DC_Drive` と `ThyristorBridge2Pulse_DC_Drive`: 後者は esdirk34 が `GM-E11003`、dopri45 が時間切れで、変わらない。前者は run 41282411061 では 101 s で終わって管に収まったが、run 41287267439 では 187 s で時間切れだった（esdirk34 単独の S050 の `run_seconds` は 94.3 s。ランナーの速さで境目を行き来する）。
- 回し直しに数える（時間切れ → 180 s）本数が 12 → 15 になった。esdirk34 の回し直しが最初の予算 30 s に収まらず、時間切れから 180 s で回し直されて終えた系が 3〜4 本増えたため（`Lightning` など）。dopri45 は 30 s の予算を超えて走っていた（S050 の記録で `run_seconds` 63.6 s）ので、前は回し直されなかった。
- ワーカーの時間はランナーによって 2299〜2897 s と揺れ、この変更の費用は切り分けられない。

## 残したこと

- 時間切れの回し直しの境目にある `ThyristorBridge2mPulse_DC_Drive`（esdirk34 単独で 94 s）。ランナーの速さで通ったり通らなかったりする。esdirk34 を先頭に近い位置で試す、予算を分ける、などは別の決定。
- `auto` の全体の予算（以前から BDF と回し直しがそれぞれ全部の予算を使える）。残りの時間を渡す規則は、Rumoca の予算の測り方（dopri45 が 30 s を超えて走る）を揃えてから。
- `Dimmer_RL` の `y_arg` の外れの原因。
