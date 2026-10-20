# ADR-0018 プリフライトに上限を掛けない（DrumBoiler で詰まるのは Rumoca の実行時オブジェクトの組み立て）

- 状態: Accepted（2026-10-21、S016）
- 関係: [ADR-0017](0017-lowering-limit-and-hard-timeout-retry.md) の結果の「プリフライトは警告しか出さないので、GlobeMode の側で上限を掛けられる」を、調べた結果で取り下げる。ADR-0017 の決定（lowering の段の上限、外から止めたものの回し直し）は変えない。製品の実行系（`globe-mode-exec`）も変えない

## 背景

- S015 では、MSL の `Modelica.Fluid.Examples.DrumBoiler.DrumBoiler` は lowering が約 117 s で終わったあと、S10 のプリフライトの段に入ったまま 481 s を過ぎても終わらなかった。プリフライトは警告（`GM-W10002`）しか出さないので、評価に上限を掛ける案を S016 の候補にした（[roadmap.md](../roadmap.md)）。
- プリフライト（`globe-mode-exec/src/rumoca.rs` の `eval_at_start`）は、Rumoca の `SolveRuntime::new(model)` で実行時オブジェクトを作り、`SolveRuntime::eval_at(t0, start 値, params, 1e-10, 32)` を呼ぶ。
- S016 で DrumBoiler を 1 本ずつ 5 回回し、`perf record` の標本と一時的な計測で、時間の使い道を調べた（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の節「S016 で調べたこと」）。
  - **`eval_at` には入っていない。** `SolveRuntime::new` が戻らない。
  - `SolveRuntime::new` の中の `construct`（`crates/rumoca-solver/src/runtime/solve_runtime.rs:527`）は、根の条件の program の構造解析（`root_condition_rows: prepare(…root_conditions…)`、`:938`）で時間を使う。そこから `crates/rumoca-eval-solve/src/prepared/construction.rs:68` → `crates/rumoca-ir-solve/src/refresh/assignment_shape.rs:100-118` に進み、store の位置ごとに prefix の依存集合（`BTreeSet`）を作り直す。
  - DrumBoiler の lowering 済みのモデルは、状態 3・solver_y 135・最大の代数ブロック 11 変数と小さい。ところが根の条件 31 本は計 59.7 万演算（1 本あたり約 6.5 万演算）と桁外れに大きい。先頭の 1 本の解析が 900 s で終わらなかった。
  - **組み立ての段も同じ `construct` を通る。** FMI ME の部品を作る `SolveRuntime::new_with_execution_backend`（`crates/rumoca-solver/src/fmi_me/kernel/component.rs:547`）が、bdf・auto の型付きの道と session の道の両方から呼ばれる。プリフライトを飛ばした回（2 回）は、組み立ての段に 227 s・295 s いたところで止めた。初期化には入らなかった。
- MSL の残りの 355 本では、プリフライトの段は最大 1.9 s（`LightningSegmentedTransmissionLine`）で、合計は約 12 s（S015 の `target/msl-sim.json` の `stage_times`）。
- Rumoca には、評価を外から打ち切る手段が無い。
  - スレッドごとの締め切り（`SolverDeadlineGuard`）を確かめる関数 `panic_on_expired_solver_deadline` は、rev 276e82bd のどこからも呼ばれていない。
  - `max_wall_seconds` の予算を見るのは FMI ME のセッションの刻みと根の探索だけで、初期化と実行時オブジェクトの組み立ては予算の外にある（`crates/rumoca-solver/src/fmi_me/session.rs:164,184,486`）。

## 決定

**プリフライトに上限を掛けない。** DrumBoiler は記録し、上流への報告の候補（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の「上流に報告する候補」の 10）に回す。

比べた案と、採らなかった理由は次の通り。

| 案 | 採らなかった理由 |
| --- | --- |
| A. プリフライトを別のスレッドで回し、決まった秒数で待ちを打ち切る（ネイティブだけ） | DrumBoiler は次の組み立てで同じ `construct` に詰まるので、実行できる本数は増えない。打ち切ったスレッドは止められず、組み立てと CPU とメモリを取り合う。wasm32 ではスレッドが無い |
| B. Rumoca の締め切り（`SolverDeadlineGuard`）を使う | 締め切りを確かめる所が無く、何も起きない |
| C. 反復の上限や許容誤差を下げる | 時間は Newton の反復ではなく、反復の前の構造解析で使っている |
| D. プリフライトを組み立ての後に回す | 止まる段が「組み立て」になって見え方は正しくなるが、実行できる本数は増えない。組み立てで失敗したときに `GM-W10002` が出なくなる。プリフライトの `SolveRuntime::new` は組み立てのものと別（Rumoca の内部にあり、使い回せない）なので、組み立てが終わるモデルでは費用も変わらない |

- 原因が Rumoca の実行時オブジェクトの組み立ての計算量にあり、GlobeMode の側の工夫では DrumBoiler は実行できるようにならない。どの案もコードと危険が増えるだけで、ほかのモデルの得にもならない（プリフライトは最大 1.9 s）。
- `msl-sim` で DrumBoiler を止めるのは、これまで通り外からの停止（回し直しの 240 s）。報告の最後の段は「プリフライト」のままになる。

## 結果

- コードは変えない。`msl:funnel`・`msl:sim` も回していない（変えたものが無いため。S016 の機械はメモリ 3 GB で、全体の実行もできなかった）。
- DrumBoiler の止まる理由は「Rumoca の `SolveRuntime` の組み立てで、巨大な根の条件の構造解析が終わらない」に改めた。S015 の「GlobeMode の側で直せる」は誤りだった。
- 組み立ての段は、根の条件の Cranelift の JIT（`regalloc2`）でも長く止まる（標本で 92〜95%）。lowering の段の遅さは、assert の登録（`Box<[RegisteredAssertion]>::clone`）と関数呼び出しの範囲ごとのアクションの収集（`CallScopedActionCollector::insert`）が支配的だった。どちらも上流の候補に書いた。
- プリフライトが組み立てとは別にもう 1 つ実行時オブジェクトを作る費用は、DrumBoiler 以外では小さい（上の 1.9 s）。Rumoca が実行時オブジェクトを公開するか、プリフライトの評価を組み立ての部品の上でできるようになったら、見直す。
