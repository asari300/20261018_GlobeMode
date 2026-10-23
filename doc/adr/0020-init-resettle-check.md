# ADR-0020 初期化の結果を初期化系にもう一度入れて確かめる（GM-W10004）

- 状態: Accepted（2026-10-22、S018）。「全部のモデルで解き直す」は [ADR-0021](0021-resettle-only-when-relations-flip.md)（S019）で、関係式の記憶が変わったときだけに改めた
- 関係: [ADR-0016](0016-preflight-on-build-lowering.md)（プリフライトは組み立ての lowering の結果で行う）の上に足す。[ADR-0018](0018-no-preflight-limit.md)（プリフライトに上限を掛けない）は変えない。調べた経緯は [research/msl-sim-failures.md](../research/msl-sim-failures.md) の節「S016 で調べたこと」の EngineV6 と「S018 で足したもの」

## 背景

- S016 で、MSL の `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6` の `filter.x` が t = 0 から参照結果と外れる原因が、Rumoca の初期化にあると分かった。初期化方程式（`CriticalDamping` の `initType = SteadyState`、`der(x) = 0`）が t = 0 で true になる関係式 `v_rel < 0` を読むとき、Rumoca は関係式の既定値 false の枝で初期化方程式を解き、初期化の後に関係式が反転しても解き直さない。小さいモデルで再現し、omc は正しい。
- GlobeMode は初期化の後に初期化方程式の残差を見ていないので、誤った初期値のまま黙って積分する。管の外れとして数えるだけで、使う人には何も知らせていなかった。
- Rumoca（rev 276e82bd）の公開 API を調べた（2026-10-22）。
  - 初期化方程式の残差を評価する `eval_initial_residual` は、crate の中だけの trait `AlgebraicProjectionModel`（`crates/rumoca-solver/src/runtime/projection.rs`、`pub(crate) mod projection`）の実装で、外から呼べない。公開されているのは方向微分 `SolveRuntime::eval_initial_residual_jacobian_v` だけ。
  - 初期化そのものを解く `SolveRuntime::settle_initialization_system`、関係式の記憶を状態から更新する `update_relation_memory_from_state`、代数変数を合わせる `refresh_algebraic_and_output_slots` は公開されている。Rumoca の FMI ME の kernel の `exit_initialization_mode`（`crates/rumoca-solver/src/fmi_me/kernel/component.rs:1184`）と rumoca-sim の `refresh_prepared_vectors`（`crates/rumoca-sim/src/prepared_vectors.rs:99`）は、この 3 つをこの順に呼ぶ。GlobeMode のプリフライトは、同じ順で呼べば実行が始まる点を再現できる。
  - 関係式の記憶はパラメータの枠にある（`root_relation_memory_targets` は `ScalarSlot::P`）。初期化の中で `update_root_relation_memory_from_values` が書き換え、その戻り値（変わったか）は捨てられる（`initial_projection.rs:784`）。記憶は `p` に残るので、同じ `p` からもう一度初期化系を解くと、反転した後の枝で解ける。
- プリフライト（S10 の数値の部分、`globe-mode-exec/src/rumoca.rs`）は、すでに `SolveRuntime::new(model)` で実行時オブジェクトを作って GM-W10002 の評価をしている（[ADR-0016](0016-preflight-on-build-lowering.md)）。初期化系を解く費用は、そこに足せる。

## 決定

**プリフライトで初期化系を 2 回解き、2 回目で動いた変数のうち、実行が実際に始まった t0 の値も 2 回目と違うものを警告 GM-W10004 で知らせる。実行は変えない。**

- 1 回目は Rumoca の初期化と同じ順（`settle_initialization_system` → `update_relation_memory_from_state` → `refresh_algebraic_and_output_slots`。許容誤差は実行の設定の許容誤差、反復の上限は rumoca-sim と同じ 64）で解き、`y1` を得る。
- 2 回目は `y1` と同じ `p` からもう一度 `settle_initialization_system`（と代数変数の合わせ直し）を行い、`y2` を得る。
- `|y2 - y1| > eps`（`eps = max(10 × 許容誤差, 1e-6) × max(1, |y1|, |y2|)`）の変数を「動いた」とする。
- **実行が終わったあと**、動いた変数ごとに結果の最初の行（t0）の値 `y0` を引き、`|y2 - y0| > eps` のものだけを本文に書く（名前と `y0 → y2`、最大 10 個）。Rumoca の FMI ME の host は初期化のあとに初期イベントの反復（`update_discrete_states_to_completion`）を行い、離散変数が変わるモデルではそこで初期化系を解き直すので、実行は `y1` ではなく `y2` の値で始まることがある（MSL の `IMC_Initialize`。169 変数が動くが、実行は `y2` から始まり参照結果と合う）。`y1` との差で警告すると、こうしたモデルで誤って出る。
- 1 回目が失敗したら黙る（実行の側が GM-E11001 にする）。2 回目が失敗しても黙る（homotopy のあるモデルでは、解いた点から λ = 0 の掃引をやり直すと収束しないことがある。MSL の `TanksWithOverflow`。失敗は実行が始まる点の誤りを示さない）。実行が失敗したときも出さない（t0 の値が無い）。実行時オブジェクトが作れないときも黙る（これまで通り）。
- 単独の `preflight`（`globe_mode_exec::preflight`）は GM-W10002 だけを出す。GM-W10004 は `run`（`checked_simulate`）だけが出す。
- 警告にとどめる。実行は Rumoca の初期化の結果で始まる。補足に「右（もう一度解いた値）のほうが正しい見込み」と回避（初期化方程式が関係式を読まないようにする）を書く。

比べた案と、採らなかった理由は次の通り。

| 案 | 採らなかった理由 |
| --- | --- |
| A. 初期化方程式の残差を直接評価する | Rumoca が公開していない（上）。方向微分だけでは残差は分からない |
| A'. 1 回目と 2 回目の差だけで警告する（最初の実装） | 初期イベントの反復で解き直されるモデル（`IMC_Initialize`）で誤って出る。実行の t0 の値と比べれば、実際に誤った値で始まったものだけになる |
| B. 2 回目の値で実行を始める（GlobeMode の側で直す） | FMI ME の host（`MeComponentHost`）にも `PreparedSimulation` にも、初期化の後の状態を差し替える公開の入口が無い。Rumoca の CLI と結果が食い違うことにもなる |
| C. 上流に報告するだけにする | 直るまで使う人に知らせる手段が無い。報告は別に続ける（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の候補 11） |
| D. 根の条件（関係式）があるモデルだけで検査する | 関係式の記憶が原因の分だけを見ることになる。初期化が固定点でない（解き直すと動く）ほかの原因も拾えるように、全部のモデルで行う。費用は MSL で測る（結果の節） |

## 結果

- `globe-mode-core` に `codes::INIT_RESETTLE_MOVED`（GM-W10004、S10、警告）。`globe-mode-exec/src/rumoca.rs` の `probe_start`（GM-W10002 の評価と同じ実行時オブジェクトで `resettle` を行う）、`resettle_diagnostic`（実行の結果の t0 の値と比べる）。`run` のどの道（bdf・dopri45・auto・自前の積分器・session の道）でも、組み立ての lowering の結果で 1 回だけ行う。
- 検証用の最小のモデル `GlobeModeValidation.Simulation.InitRelationBranch`（S016 の R1 と同じ仕組み。1 次のフィルタの定常の初期化が `if v < 0` を読む）。試験 `an_initialization_that_reads_a_relation_flipped_at_t0_is_warned`: `run` が `x = 2 → 1` を警告し、実行は x(0) = 2（Rumoca の不具合の値）で始まる。Rumoca が直したら、この試験の期待を入れ替える。
- `cargo xtask msl-sim` の報告に「初期化の解き直しで値が変わる警告（GM-W10004）が出た」の本数を足した。MSL での本数と費用は [msl-sim.md](../msl-sim.md) と S018 の作業記録に書く。
- 費用（RPi、`msl:sim` の `stage_times`）: プリフライトの段は 356 本の合計 98.5 s（S015 は約 12 s）、中央値 0.014 s。最大は `LightningSegmentedTransmissionLine` の 33.6 s（実行の初期化そのものが 27.9 s のモデルで、2 回分が足される）、次いで `EngineV6` 15.0 s。全体（5845 s）の 1.7%。RPi では `EngineV6` が積分の段で 240 s の応答の上限に当たるようになった（S015 の x86_64 では 169 s で終わっていた。機械の速さが主で、プリフライトの 15 s も含む）。費用を下げたければ、根の条件の無いモデル（関係式の記憶が無い）で解き直しを省く案（上の D）に戻れる。
- 判定を直した版の `msl:sim`（RPi）: GM-W10004 は 0 本。プリフライトの段の合計は 93.6 s。`LightningSegmentedTransmissionLine` は 45.9 s になり（機械が全体に遅い回で、初期化そのものも 31.3 s）、240 s の応答の上限に当たって外から止まった（1 回目は 197.7 s で終了時刻まで）。RPi ではこの 1 本が上限ぎりぎりで、案 D に戻す理由になりうる（次回）。
- Rumoca が初期化方程式の残差の評価を公開するか、初期化が反転した関係式で解き直すようになったら、この検査は残差の評価に置き換えるか、外す。
