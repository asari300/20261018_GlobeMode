# 下書き（未提出）34: 2 つの関係式の根が丸めの雑音のとき、事象の反復が終わらない（`value == 0.0` が厳密な比べ方）

対象: **Rumoca**（<https://github.com/CogniPilot/rumoca>）。英語の本文は [34-event-iteration-exact-zero-test-on-rounding-noise.md](34-event-iteration-exact-zero-test-on-rounding-noise.md)、再現は [.mo](34-event-iteration-exact-zero-test-on-rounding-noise.mo)、1 枚の解説は [_jp.html](34-event-iteration-exact-zero-test-on-rounding-noise_jp.html)。

判定（GlobeMode、S158）: **バグ**（丸めの雑音の符号を見ている）。同じ書き方が Rumoca v0.10.2（rev `276e82bd`）、v0.10.4、main（`590ac739`、確認日 2026-11-05）にある。

## 概要

`rumoca-solver` の `runtime/solve_runtime/relation_memory.rs` は、関係式の根の側を 0 との厳密な比べ方で決める。

- `mode_is_consistent`（v0.10.4・main の 708 行）: `value == 0.0 || relation_memory_value_from_root(value) == *side`
- `release_reversed_relation_overrides`（949 行）: `|| roots[*index] == 0.0`

事象の時刻の連続の状態がちょうど釣り合うと、代数の射影が返す根は厳密な 0 でなく丸めの雑音（1e-29〜1e-35）になり、その符号は、いま固定した方式に応じて反転する。方式の探索は「一貫した方式」を見つけて固定するが、次の回で雑音の符号が変わって固定が外され、周期 2 の繰り返しが `fixed_point_cap`（32）まで続く。

```text
[EventIteration] event condition equations did not converge with fixed pre at t=13.040017098156884
```

繰り返しは検出される（`on_relation_surface`。`y` は回ごとに 1e-29 しか違わない）が、`resolve_relation_cycle` が毎回 `Found` を返すので、「関係式の面にいる」という出口には行かない。

## 再現手順

GlobeMode の実行で見つけた（積分器は GlobeMode 自前の `sdirk34hw`。Rumoca の CLI には無い）。添付の [.mo](34-event-iteration-exact-zero-test-on-rounding-noise.mo) は MSL の例題の experiment を決めるだけ。

```bash
globe simulate -L data/msl 34-event-iteration-exact-zero-test-on-rounding-noise.mo -m Repro34 --solver sdirk34hw
# error[GM-E11003] ... t = 13.040017098 ... [EventIteration] event condition equations did not converge with fixed pre
```

Rumoca の CLI（BDF）が同じ釣り合いの状態に着くかは試していない【未確認】。この状態は刻みの列の偶然で、同じモデルが `esdirk34` では 15 s まで通り（GitHub Actions の run 41666815770）、`sdirk34hw` でも `--stop-time 13.2` なら通る。

## 起きていること（S158 の追跡）

モデル `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive`、Tolerance 1e-7。1.2 マイクロ秒の窓に事象が 3 つ（13.04001588 の点弧のあと、13.0400170981568 の状態事象）。探し当てた事象の時刻で、2 つのインダクタの電流がビットまで正負逆になる: `dcpm.la.i = 19.95342803660142`、`lMains.i = -19.95342803660142`（和は 0.0）。

1. 候補の根（番号 1 と 4）で方式を探す。どの方式でも根は `(+4.1e-30, -4.1e-35)`。方式 `[0, 1]` を一貫したものとして上書きに固定する。
2. 次の回の射影で根が `(-3.3e-34, +3.3e-29)` になり、符号が両方反転する。上書きは「逆向き」とされて外される（残るのは厳密な 0 のときだけ）。
3. 次の探索が `[1, 0]` を選び、根はまた反転する。周期 2。`y` は回ごとに 1e-29 動き、`p` は 6 要素（サイリスタの `off` の記憶）が入れ替わる。

## 期待する動き

根が射影の雑音の範囲で 0 なら、関係式は今の側を保つ（MLS 付録 B。Rumoca のソースの注釈が事象の反復でこれを引いている）。すでにある「関係式の面」の規則が、検出した繰り返しに対してやっているのと同じ。被演算子の大きさに対する相対の許容誤差で足りる。GlobeMode は診断を確かめるためだけに絶対値 `1e-24` を使った。

## 確認（GlobeMode の手元の修正。そのままの提案ではない）

2 つの比べ方を `value.abs() <= 1e-24` にすると、同じ実行が最後まで進む（StopTime 15、出力 87,498 行、64.7 s）。ほかの MSL の例題は、この修正で回し直していない【未確認】。

## 環境

- Rumoca v0.10.2、git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`、MSL 4.1.0。上のコードの 2 行は v0.10.4 と main `590ac7398fc42a383dedf4bf10ccc023353f8c54` と GitHub API で突き合わせた（確認日 2026-11-05、2 つの比べ方は同じ）。実行そのものは v0.10.4・main で回していない【未確認】。
- Linux x86_64（WSL2、Ubuntu）、GlobeMode の `globe`（release）。

## GlobeMode 側の回避

なし。`sdirk34hw` はこの MSL の例題 1 本で `GM-E11003` になる。`esdirk34`・`auto`・ほかのソルバは当たらない。[msl-solvers.md](../../msl-solvers.md) に記録し、既知の限界の試験（`msl_sdirk34hw_thyristor_bridge_event_iteration_is_a_known_limit`、`#[ignore]`）で見張る。

## 補足

- 下書き 02（サンプラの端点の判定に丸めの床が無い）は、別の場所での同じ種類の抜け。
- `sdirk34hw` だけの問題ではない。補間した状態が釣り合いにビットまで一致する積分器なら当たりうる。`esdirk34` も t = 13.2000171 で `la.i + lMains.i == 0.0` の状態を通るが失敗しない。どちらに転ぶかは雑音で決まる。
