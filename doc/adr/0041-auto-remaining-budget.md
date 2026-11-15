# ADR-0041 `auto` の回し直しには予算の残りを渡し、残りが無ければ時間切れで返す（ADR-0028 (c) を改める）

- 状態: Accepted（2026-11-15、S344。全例題の CI の結果は「結果」の節）
- 関係: [ADR-0028](0028-auto-esdirk34-fallback.md) の決定 (c)（回し直しには元の予算をそのまま渡す）を改める。(a)・(b)・(d) は変えない。ADR-0028 は (c) 以外が生きているので Accepted のまま、(c) の節に付記を足した。[ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md)（msl-sim の lowering の上限と外からの停止）の前提を狭める（付記を足した）。[ADR-0033](0033-progress-and-cancel.md)（中断）の型付きの道（#43）の続き

## 背景

- issue #6: `auto` は BDF が失敗すると、esdirk34・dopri45 に同じ `max_wall_seconds` を渡し直す。`--timeout 10` で BDF が 9.5 s で失敗すれば、次の積分器がさらに 10 s 走って約 19.5 s、最悪で BDF + esdirk34 + dopri45 の 3 倍近くになり、`--timeout` が本当の上限にならない。msl-sim の回し直し（予算 180 s、外から止めるのは 240 s）では、BDF が 100 s で失敗すると次の積分器が外からの停止にかかり、普通の結果でなく「外から停止」として記録される。
- issue #46: lowering（式を Solve IR に直す段）は Rumoca の予算の外で、トークンでも止められない。`auto` は回し直すたびに lowering をやり直す（BDF・esdirk34・dopri45 の最大 3 回）。ネイティブの dopri45 は単独のプリフライトと `rk45::simulate` で 2 回やる。
- ADR-0028 (c) は「残りの予算を渡す版」を CI（run 41282411061）で回し、`LightningSegmentedTransmissionLine` が 1 本減ったので採らなかった。msl-sim の最初の予算は 30 s で、この系は BDF の組み立てと積分で 30 s を超えるので「残り 0 以下」になり、回し直されなかった。そのとき BDF の失敗（`GM-U0005`）のまま返したとみられ、時間切れ（`GM-E11005`）でなかったので msl-sim の時間切れの回し直し（180 s）にも乗らなかった【未確認。その版の差分は残っていない】。
- Rumoca の予算（`SimOptions::max_wall_seconds`）は、組み立ての後（積分器の host を作ったとき）から測り、lowering を含まない（rev `276e82bd`・`9c8c56f` のどちらも `rumoca-solver` の `runtime/timeout.rs`）。0 以下・有限でない値は「予算なし」になる。

## 決定

1. **回し直しには予算の残りだけを渡す。** 残り = `max_wall_seconds` − それまでの組み立てと実行の時間の合計（`Outcome` の `build_seconds + run_seconds`。lowering の時間も引く。保守的）。BDF には元の予算を渡す。
2. **残りが 0 以下なら回し直さず、時間切れ（`GM-E11005`）で返す。** 回し直す前に判定するので、その回し直しの lowering も始めない（#46 の「回し直しのたびの lowering」のうち、予算が尽きたあとの分）。時間切れで返すので、msl-sim の「時間切れを予算 180 s で回し直す」にそのまま乗る。0 以下を Rumoca に渡すと「予算なし」になるので、渡さずにここで止める。
   - 回し直さなかったことと、その前の失敗（BDF が積分で失敗した）は診断に残らない。最終の診断は時間切れだけ（予算を使い切ったことは事実なので）。
3. 予算なし（`None`）はそのまま渡す。wasm32 は壁時計を測らない（`Stopwatch` が 0 秒）ので、渡す予算は変わらない。
4. **放置した型付きの道のスレッドも、トークンを見る（#46）。** `checked_detached` は呼び手へすぐ返したあと、別のスレッドの実行を放置する。そのスレッドの `Observer` にもトークンを渡し、段の境目と auto の回し直しの前で止まるようにした。lowering・積分の途中は止められない。

## 選ばなかった案

- 元の予算をそのまま渡す（ADR-0028 (c)）: #6 のとおり `--timeout` が上限にならない。
- 残りが無ければ前の失敗のまま返す: 時間切れにならず、msl-sim の回し直しに乗らない（ADR-0028 の CI の失敗の原因とみられる）。
- 積分器ごとに予算を等分する: ADR-0028 の案 A の途中で検討した。esdirk34 の所要が系で大きく違う（`ThyristorBridge2mPulse_DC_Drive` で 94 s）ので、分け方で当たり外れが出る。

## 結果

- 単体試験 `auto_hands_the_remaining_budget_to_each_retry`: 予算 40 s・BDF 10 s・esdirk34 20 s なら esdirk34 に 30 s、dopri45 に 10 s を渡す。予算 30 s なら dopri45 は（lowering も）始めず時間切れ。BDF だけで使い切れば esdirk34 も始めない。予算なしはそのまま。
- 実際のモデルでの直前・直後（`TwoMass`、`--solver auto --tolerance 1e-10 --stop-time 100 --timeout 1`）: 【CI・手元の結果は作業記録の「検証」に書く】
- 全例題の CI（`msl-sim.yml`）: 【実行後に書く】
- 残るもの:
  - lowering は予算の外のまま。回し直しは最大 3 回の lowering のままで、**残りが 0 を超えている間は回し直す**ので、予算が残っているモデルの lowering の重複は減らない。ネイティブの dopri45 の lowering 2 回（単独のプリフライトと `rk45::simulate`）は、`rk45::simulate_artifact` が `pub(crate)` で、lowering 済みを渡す口が Rumoca に無い（rev `276e82bd`・`9c8c56f` で確かめた）ので、上流の口を待つ。session の道に替えて 1 回にする案は #43 で測って取り消した（EngineV6 の bdf が時間切れになるほど遅かった。ADR-0013 の付記）。
  - Rumoca の予算が lowering を含まないので、最後の回し直しは「残り」の外で lowering の時間だけ長くなりうる。1 回目の lowering が 100 s の系では、残りの計算に lowering を引いてあるので、回し直しの lowering が重ければ全体は予算を超える。上限は ADR-0017 の外からの停止（msl-sim）か利用者の中断。
