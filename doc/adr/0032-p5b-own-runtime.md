# ADR-0032 P5b の自前のランタイムは、FMI 2.0 ME の C ABI を自前の cdylib で出す。S095 はモデルを crate に固定する

- 状態: Proposed（2026-11-02、S095。載せ替えの道は S099 で道 2 を選んだ。下の追記）
- 関係: [ADR-0026](0026-fmi-import-placement.md)（Import の置き方。C ABI を自前で持つ案）、[architecture.md](../architecture.md) の「Export」（P5a と P5b）。P5a（Rumoca のテンプレート、S094）の ADR-0031 とは別の番号にした

## 背景

- roadmap の P5 の Export は、P5a（Rumoca の `fmi2` / `fmi3` テンプレートでソース FMU を作る）の後に P5b（自前のランタイム）。2026-11-02 にユーザーが「P5a を先行し、それを参考にしつつ P5b を並行して進める」と決めた。
- S057 の調査（[research/fmi-2026-11-01.md](../research/fmi-2026-11-01.md)）で、Rust で FMU を出す crate は `fmi-export` 0.3.0（MIT OR Apache-2.0、FMI 3.0 の Export のみ）だけ。
- `globe-mode-exec` の公開 API は `simulate` / `run` などの「最後まで回す」ものだけで、任意の時刻と状態で右辺（微分）を評価する API は無い。ME はこれが要る。
- S095 では exec・sim・`globe-mode-fmi`・CLI を変えない（S094 と並行のため）。

## 決定

### (a) 置き方: 自前の FMI 2.0 ME の C ABI（採用）と `fmi-export` 0.3.0 を比べた

| 案 | 長所 | 短所 |
| --- | --- | --- |
| (a) `fmi-export` 0.3.0 の derive / trait にモデルを載せる | 関数の一覧と型を crate が持つ | FMI 3.0 のみ（P5 の完了条件の FMI 2.0 が無い）。依存が増え、`cargo deny` と licenses.md の手間が増える。1.0 前で API が動く |
| (b) 自前で `fmi2*` の 35 関数を `extern "C"` で出す（採用） | 依存 0。FMI 2.0 は FMPy・fmusim・fmuCheck のどれでも回せる。`globe-mode-c`（S075）と同じ形 | 関数の署名を自分で規格に合わせる（FMPy で確かめる）。FMI 3.0 は別に書く |

- 決め手は「依存 0 で小さい」と「FMI 2.0 が P5 の完了条件にある」の 2 つ。
- 新しい crate `globe-mode-fmu`（cdylib + rlib）。workspace のどの crate にも依存しない。exec に右辺の API が無いので、S095 では `Decay`（`der(x) = -k*x`）の式を Rust で書いて固定した。
- `modelDescription.xml` は crate に手書きで置き、`guid` を `src/lib.rs` の `GUID` と突き合わせる試験を置く。
- FMUstate・方向微分・Integer / Boolean / String の値は `fmi2Error` の stub（`modelDescription.xml` の `can*` / `provides*` は既定の false）。ログの callback は呼ばない。
- FMI 2.0 の共有ライブラリの置き場は `binaries/linux64/<modelIdentifier>.so`（FMI 3.0 なら `binaries/x86_64-linux/`）。

### (b) 任意のモデルを載せ替える道（持ち越し。候補）

| 道 | 要るもの | 長所 | 短所 |
| --- | --- | --- | --- |
| 1. FMU ごとに cdylib をビルドする（Solve IR から Rust のソースを生成） | Export のときに cargo と rustc（または Rumoca の `rust-ode` テンプレート） | 速い。FMU が小さい | 利用者の手元にツールチェーンが要る。target ごとにビルド |
| 2. 1 つの cdylib が `resources/` の Modelica のソースを読み、Rumoca で compile する | exec に「右辺を評価する」公開 API。cdylib に Rumoca と diffsol を入れる | ビルドが Export のたびに要らない | cdylib が大きい。`fmi2Instantiate` のたびに compile（MSL を読むと秒単位）。ライブラリの同梱 |
| 3. 1 つの cdylib が `resources/` に置いた Solve IR（直列化）を解釈実行する | Solve IR の直列化と、Rumoca に依存しない評価器 | コンパイラが要らない（architecture.md の P5b の「解釈実行」）。ブラウザ・Android から Export できる | 直列化の形を自分で決めて保つ。評価が遅い |

- どれでも、exec の側に「状態・パラメータ・時刻 → 微分・出力」の公開 API が要る（今は無い）。次に P5b を進めるときに、まずこの API を exec に足して、S095 の手書きの式をそれで置き換える。
- `modelDescription.xml` は、載せ替えのときに Solve IR から作る（`globe-mode-fmi` の型を使う。P5a と共有する）。

### (c) P5a との分担

- P5a（S094、`globe-mode-fmi` の `export_me_sources` と `globe fmu export`）はソース FMU を作る。P5b はコンパイラの要らないバイナリの FMU を作る。
- 両方とも検証は FMPy（BSD-2-Clause）で x(1) を解析解と比べる。P5a で作った `modelDescription.xml` の生成を P5b でも使う（重複させない）。

## 結果

- `cargo test -p globe-mode-fmu` の `fmpy_simulates_decay` が、cdylib と `modelDescription.xml` を FMU に詰め、FMPy 0.3.32 の ME（既定のソルバ CVode）で回す。x(1) = 0.3678794150955906（e⁻¹ = 0.36787944117144233、差 2.6e-8）。uv か FMPy を使えない環境では skip する。
- 関数を直接呼ぶ試験（陽的 Euler、k = 2）で x(1) が e⁻² に 1e-4 以内。
- 依存の追加 0。

## 追記（2026-11-02、S099）: (b) は道 2（1 つの cdylib が resources の Modelica を compile する）を選んだ

### 決定

- `globe-mode-exec` に公開の型 `ModelInstance`（`src/rumoca/instance.rs`）を足した。`new(&CompiledModel, &パラメータの上書き, 開始時刻)`・`set_time`・`set_states`・`states`・`derivatives`・`n_states`・`state_names`・`parameters`。既存の `simulate` / `run` / session の道は変えていない（追加だけ）。
- 中身は Rumoca の FMI ME 風の host でなく、host と同じ部品の `rumoca_solver::SolveRuntime`（公開）を直接持つ。理由: host の set time / set states / get derivatives に当たる `SolveMeKernel` の関数は `pub(crate)` で、積分器に渡す `MeDerivativeHandle` は host の呼び出しの間しか使えない（rev 276e82bd の `rumoca-solver/src/fmi_me/kernel.rs`・`integrator/derivative.rs`）。lowering は `lower_for_simulation_with_overrides`（`tunable_parameters` と同じ）、初期化は `settle_initialization_system`、微分は `eval_state_derivatives`。
- 事象（根の条件・`sample`）のあるモデルは `ModelInstance::new` が GM-U0004 で断る（事象の表示関数と事象の処理は持ち越し）。
- `globe-mode-fmu` は exec と frontend に依存する。`fmi2Instantiate` が `resources/<完全修飾名>.mo`（1 ファイル、MSL は読まない）を読んで compile する。道 1（FMU ごとにビルド）は利用者の手元にツールチェーンが要り、道 3（Solve IR の解釈実行）は直列化の形を決める仕事が大きいので、最小で確かめられる道 2 にした。
- パラメータは `fmi2SetReal` で覚え、`fmi2ExitInitializationMode` で lowering からやり直して反映する（`SimOptions::param_overrides` と同じ道なので、依存するパラメータにも届く）。
- `modelDescription.xml` は手書きをやめ、`globe_mode_fmu::model_description` が `ModelInstance` の状態名・パラメータから作る。`guid` はモデル名と文面の FNV-1a で、`fmi2Instantiate` が resources の文面から同じ値を作って突き合わせる。`globe-mode-fmi`（P5a、S098 が触る）の型は使っていない。

### 結果

- FMPy 0.3.32 の `validate_fmu` は問題なし。ME（CVode）で Decay の x(1) = 0.3678794150955906（e⁻¹ と 2.6e-8 の差。S095 の手書きと同じ）。
- HarmonicOscillator（状態 2、事象なし）: FMPy の x(1) = -0.41614683955697307、v(1) = -1.818594856574809（解析解と 3.0e-9・2.9e-9 の差）。`globe_mode_exec::run`（既定の bdf）の x(1) = -0.4161468121809279、v(1) = -1.8185947093271577 と 2.7e-8・1.5e-7 の差で、試験は 1e-4 以内を確かめる。
- cdylib の大きさ: debug 702 MB、release 44.1 MB（44,128,856 B）、strip 後 32.8 MB、gzip で 12.4 MB（FMU の zip に入る大きさの目安）。
- 外部の依存の追加 0（workspace の crate への依存だけ）。

### 持ち越し

- 事象のあるモデル（event indicator・`fmi2NewDiscreteStates`）、出力（代数変数）の値参照、MSL を使うモデル（resources へのライブラリの同梱）。
- `modelDescription.xml` の生成を `globe-mode-fmi`（P5a）と共有する。CS、FMI 3.0、Windows・macOS の置き場。
- `resourceLocation` の `%xx` を戻していない（`shortcut:` の注記）。

## 追記（2026-11-03、S109）: 事象のあるモデルを `SolveRuntime` の公開 API で回す

### 決定

- `ModelInstance::new` は事象（根の条件・`sample`）のあるモデルを断らなくなった（`GM-U0004` をやめた）。足した API は `n_event_indicators`・`event_indicators`・`enter_event_mode`・`update_discrete_states`（結果は `DiscreteUpdate`: `states_changed`・`terminate`・`next_event_time`）。
- Rumoca の FMI ME 風の host（`SolveMeKernel`）の `get_event_indicators`・`enter_event_mode`・`update_discrete_states` は rev 276e82bd でも `pub(crate)` のままなので、S099 と同じく host の部品の公開の関数を直接呼ぶ（Rumoca は改変していない）。
  - 事象指示関数: `SolveRuntime::eval_root_conditions` に `orient_typed_root_zeros`（ちょうど 0 を関係式の側に寄せる）。
  - Event Mode に入る: `full_solver_y` で今の y とパラメータの枠を `pre` として控える。
  - 離散の状態の更新: host の `apply_discrete_event_updates` と同じ順で `apply_projected_event_update`（代数の射影は `refresh_event_dependency_slots_certified`、越えた根は `root_crossings_with_relation_memory` で直前の事象の後の値と比べて上書き）→ `commit_pre_params_after_event_at` → `SolveStopSchedule::advance_past`。`reinit` で状態が変われば `valuesOfContinuousStatesChanged`。
  - 次の時刻事象: `next_runtime_event_stop`（端は `f64::MAX`。FMI の ME は終了時刻を知らせなくてよく、`next_periodic_event_time` は有限の端を要る）。
- host が持つ多様体の射影（`MeAlgebraicProjectionPolicy` の manifold、非公開）と、状態事象と時刻事象が重なったときの扱い（`StateTimeCoincidence`）は真似ていない。
- `globe-mode-fmu` は `numberOfEventIndicators` を `n_event_indicators` から作り、`fmi2GetEventIndicators`・`fmi2EnterEventMode`・`fmi2NewDiscreteStates` を上の API につなぐ。`newDiscreteStatesNeeded` はいつも偽（Rumoca は 1 回の呼び出しで事象の反復を収束させる。host の `MeDiscreteStates` と同じ）。

### 結果

- FMPy 0.3.32 の ME（CVode）で BouncingBall（状態事象と `reinit`）: `reinit` で状態が変わった事象 3 回（exec の `run` の n = 3 と同じ）。Event Mode に入った回数は 6（床を下へ越えたとき 3 回と、跳ね返って上へ越えたとき 3 回。上へ越えたときは `when h < 0` が発火しない）。h(2) = 0.26073984246749315、v(2) = -0.16588584207712853 で、exec の 0.26074169044319206・-0.1658694813220666 と 1.8e-6・1.6e-5 の差（試験は 1e-3 以内）。
- SampleCounter（`sample(0.05, 0.1)` の時刻事象）: Event Mode に入った回数 10（exec の n = 10 と同じ）、x(1) = 1.0（exec と 2.2e-16 の差）。
- 外部の依存の追加 0。

### 持ち越し

- `initial()` の事象（`apply_projected_initial_event_boundary`。`shortcut:` の注記）、多様体の射影、状態事象と時刻事象が同時のとき。
- 離散・出力の変数の値参照（SampleCounter の n・xs、BouncingBall の n を FMU の外から読めない。今は FMPy で Event Mode の回数を数えて突き合わせる）。
- MSL を使うモデル、CS・FMI 3.0・Windows（S099 の持ち越しのまま）。

## 追記（2026-11-03、S112）: 代数・離散の変数を FMU の外から読む

### 決定

- `ModelInstance` に `variables()`（状態とパラメータ以外の変数。名前・役割 `VariableRole`（Algebraic・Output・Discrete）・型 `ValueType`（Real・Integer・Boolean））と `variable_values()`（今の時刻と状態での値、同じ順。f64）を足した。
- 値は exec の `run` が結果の列を作るときと同じ部品を使う: Rumoca の host の `record_outputs`・`get_outputs` が呼ぶ `SolveRuntime::visible_values` を、`full_solver_y` で求めた y とパラメータの枠の上で評価する（どれも公開。Rumoca は改変していない）。変数の並びは `SolveModel::visible_names`、役割と型は `variable_meta`（`convert_variable_meta` で `SimVariableMeta` にし、exec の結果と同じ `role` で振り分ける）。離散の値はパラメータの枠にあり、直前の事象の後の値になる。
- `globe-mode-fmu`: 値参照 `2n+p..` に載せる。`causality` は役割が Output なら `output`（`ModelStructure` の `Outputs` にも並べる）、ほかは `local`。`variability` は離散か Integer・Boolean なら `discrete`、ほかは `continuous`。`initial="calculated"` で start は書かない。`fmi2GetReal`・`fmi2GetInteger`・`fmi2GetBoolean` で読め、型の違う値参照と書き込みは `fmi2Error`。
- Enumeration・String の変数は載せない（`shortcut:` の注記）。値参照ごとに解き直す（`shortcut:` の注記）。

### 結果

- FMPy 0.3.32 の ME で SampleCounter の n = 10（`fmi2GetInteger`。exec の n と同じ）、xs = 0.9500000000000001（exec は 0.9499999999999997、差 4.4e-16）。BouncingBall の n = 3（exec と同じ）。NonlinearLoop の代数変数 y(1) = 1.9999999999999991・z(1) = 4.000000000000001（exec は 2.0・4.0、差 8.9e-16・8.9e-16。x(1) は 9.4e-9 の差）。
- HarmonicOscillator・BouncingBall には代数変数が無いので、代数変数の突き合わせは NonlinearLoop（大きさ 2 の非線形の代数ループ）で行った。
- 外部の依存の追加 0。

### 持ち越し

- 入力（`causality="input"`）と、代数・離散の変数の書き込み。Enumeration・String の変数。
- `ModelStructure` の `Outputs` の依存（`dependencies`）を書いていない（既定の「すべてに依存」）。
- `initial()` の事象、MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows（S109 の持ち越しのまま）。

## 追記（2026-11-03、S114）: 入力（`causality="input"`）を FMU の外から与える

### 決定

- `ModelInstance` に `set_input(name, value)` を足し、`variables()` に役割 `VariableRole::Input` の変数を含めた（値は `variable_values()` で読める。`visible_values` が入力も返す）。
- 入力の置き場はパラメータの枠の `SolveLayout::input_parameter_index(name)`（公開）。Rumoca の FMI ME の host（`SolveMeKernel::set_float64`、`pub(crate)`）が入力を書く所と同じで、host がそのあと行うのは計算の控えの消去と `delay` の点の記録だけ（`ModelInstance` はどちらも持たない）。
- lowering の前の値: `lower_for_simulation_with_overrides` は駆動されない入力を断るので、束縛の無い入力の `start` を `SimOptions::initial_inputs`（公開）で渡す。Rumoca の `host_driven_input_seeds`（FMI の export と GPU の準備が使う。`pub(super)` で crate の外から呼べない）と同じ処理を exec の `input_starts` に書いた（`numeric_attribute_values` は `rumoca_phase_solve::fmi` の公開関数）。Rumoca は改変していない。
- `globe-mode-fmu`: 入力は値参照 `2n+p..` の変数の 1 つとして `causality="input"`・`variability="continuous"`・`initial` なし・`start`（初期化の後の値）で載せ、`fmi2SetReal` で書く（Real の入力だけ。出力・代数・離散への書き込みは `fmi2Error` のまま）。出力（`initial="calculated"`）は `ModelStructure` の `InitialUnknowns` にも並べた（FMI 2.0。S112 では欠けていて、FMPy の validate が `Expected {der(x), y}` で落ちた）。
- パラメータを変えて `fmi2ExitInitializationMode` で作り直したときは、与えた入力を置き直す。初期化は入力の `start` で解く（`shortcut:` の注記）。

### 結果

- `Simulation/InputDecay`（`der(x) = -x + u`、`y = 2*x`、`u(start = 0)`）を FMPy 0.3.32 の ME（CVode）で、`input=` で t = 0.5 に 0 → 2 の階段を与えて回した: x(1) = 1.1548180654142353、y(1) = 2.3096361308284705。解析解 x(1) = 2 + (e^-0.5 − 2)e^-0.5 = 1.1548181217461755 と 5.6e-8、y(1) と 1.1e-7 の差（試験は 1e-5 以内）。`validate_fmu` は問題なし（`Outputs` を含む）。
- exec の `run` には入力を与える道が無く（`ResolvedSettings` に入力が無い）、駆動されない入力を断るので、突き合わせは解析解で行った。
- 外部の依存の追加 0。

### 持ち越し

- 初期化が入力に依るモデル（`ModelInstance::new` に入力を渡す）、代数・離散の変数の書き込み、Integer・Boolean の入力、exec の `run` に入力を与える道。
- `Outputs` の `dependencies`、Enumeration・String、`initial()` の事象、MSL を使うモデル、`modelDescription.xml` の生成を P5a と共有、CS・FMI 3.0・Windows。
