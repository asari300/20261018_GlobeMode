# ADR-0014 wasm32 と自前の積分器は、Rumoca の FMI ME 風の host に数値積分のプラグインを差す「session の道」で回す。ネイティブの bdf・dopri45・auto は型付きの道のまま

- 状態: Accepted（2026-10-19、S012）
- 関係: [ADR-0013](0013-exec-backend.md)（実行系を `globe-mode-exec` に閉じ込める）を拡げる。ADR-0013 は覆さない

## 背景

- P3 の完了条件の 3 つ目は「bdf が wasm で（Node 上で）動く」（[roadmap.md](../roadmap.md#p3-の完了条件の進み具合)）。P3 の範囲には esdirk34・tsit45・固定刻みのソルバと ResultStore（CSV / MAT v4 / Arrow）、パラメータを変えた再実行も入る（[features.md](../features.md)）。
- S012 の調べもので確かめた事実（確認日 2026-10-19、Rumoca rev `276e82bd`。出典の行番号は [research/rumoca.md](../research/rumoca.md#s012-で調べた実行系の公開の道v0102)）:
  - S011 の型付きの道（`rumoca_sim::build_simulation` → `check_prepared_initialization` → `run_prepared_simulation`）と `rk45::simulate` は std の `Instant::now()` を呼ぶ。wasm32-unknown-unknown では Node で実際に `RuntimeError: unreachable … <std::time::Instant>::now … build_simulation` になった。
  - Rumoca の FMI ME 風の host（`rumoca_solver::fmi_me`）の生成・実行の関数（`lower_correlated_for_simulation_with_overrides` → `finish_fmi_component` → `MeModelArtifact::new` → `MeRetainedComponent::instantiate` → `into_lease`（初期化）→ `numerical_setup` → `into_session(Some(Box<dyn MeIntegratorBackend>))` → `run_to_stop` → `finish`）はどれも公開されている。数値積分のプラグイン `MeIntegratorBackend`（`initialize`・`advance`・`sample`・`truncate_reset`）を差せば、Rumoca に手を入れずに積分器を替えられる。ハイブリッドの driver（イベント・根探索・出力・時間切れ）は Rumoca の host が受け持つ。この道の時間切れは wasm32 では `instant` を使うので、wasm でも効く。
  - Rumoca の BDF と RK45 のプラグイン（`rumoca_solver_diffsol::model_exchange_integrator`、`rumoca_solver_rk45::model_exchange_integrator`）も公開されている。
  - Cranelift の JIT を使うかを決める `admitted_native_execution_backend` は `pub(crate)` なので、この道はネイティブでもインタプリタで回る。
  - getrandom 0.3（diffsol → faer → rand）は、`--cfg getrandom_backend="wasm_js"` に加えて getrandom の feature `wasm_js` も要る。
  - パラメータの上書きは `SimOptions::param_overrides`。DAE は使い回せる（再コンパイル不要）が、lowering と組み立てはやり直す。変えられるのは role が Parameter で `is_tunable` が真のものだけ。
- wasm の試作は、この host を `simulate_fmi_component`（失敗を文言で返す）で呼び、文言を読み直して段を推し量っていた。初期化と積分のどちらで止まったかが型で分からない。

## 決定

### (a) session の道を 1 つ作り、wasm32 と自前の積分器に使う

- `globe-mode-exec/src/rumoca/session.rs` に、上の公開の関数を順に呼ぶ道を 1 つ作る。差すプラグインは 3 種類（`RumocaBdf`・`RumocaRk45`・自前の積分器）。失敗は呼んだ位置（lowering・組み立て・初期化・積分）と Rumoca の型（`MeSessionError` など）で振り分ける（S011 と同じ診断のコード）。
- **wasm32 はすべてのソルバでこの道を通る**。ネイティブでは、自前の積分器 6 つだけがこの道を通る。
- **ネイティブの `bdf`・`dopri45`・`auto` は S011 の型付きの道のまま**にする。Rumoca の CLI と同じ数値で、MSL の数字を S011 と比べられる。
- 2 つの道が同じ数値になることを、`Simulation.*` 12 本の bdf で確かめた。型付きの道と session の道（`RumocaBdf`）は**ビット単位で同じ**（成功 8 本の行数・終わり方・全変数の最後の値、失敗 4 本の診断の全体）。dopri45・auto も一致した。これを wasm の正しさの根拠にする。

### (b) wasm は wasm32-unknown-unknown + wasm-bindgen

- 新しい crate `globe-mode-wasm`（`cdylib` と `rlib`）。JS からは `new Workspace(libraryJson?, name?)`（ライブラリを 1 回だけ読む）、`set_file`・`remove_file`・`files()`・`simulate(requestJson)`、関数の `simulate`・`solvers()`・`version()`・`memory_bytes()`・`last_panic()`。入出力は JSON の文字列。
- ネイティブの `globe simulate` と同じ道（`GlobeDb::check_model` → `compiled_model` → `globe_mode_exec::run`）を通る。ファイルシステムが無いので、ライブラリ（MSL）はファイルの組で渡し、Rumoca の in-memory の組（DurableExternal）として読む（`Checker::with_library_sources`、`GlobeDb::with_library_sources`）。文書として渡す道とビット単位で同じ結果になる。
- 時計は `performance.now()`。panic=abort なので `catch_unwind` は効かない。panic の文言は `set_hook` で残し、`last_panic()` で読む（依存を足さない）。
- `wasm-bindgen` は Cargo.toml で `=0.2.129` に固定し、CLI（`wasm-bindgen --target nodejs`）も mise で同じ版を入れる（`"github:wasm-bindgen/wasm-bindgen" = "0.2.129"`）。版がずれると糊が作れないため。
- getrandom の backend はルートの `.cargo/config.toml` の `[target.wasm32-unknown-unknown]` の rustflags と、getrandom の feature `wasm_js` で与える（`globe-mode-exec` が wasm32 のときだけ getrandom 0.3 を直接の依存にして有効にする）。
- 確かめるのは **Node**（`mise run wasm:node`・`wasm:compare`、CI のジョブ `wasm-node`）。ブラウザ（`--target web`、Worker）は P6 で確かめる。
- 大きさの最適化（`lto = "fat"`・`codegen-units = 1`・`opt-level = "s"`・`strip`）は `mise run wasm:build` の環境変数で与え、ワークスペースの `[profile.release]` は変えない（ネイティブの release に影響させない）。

### (c) 自前の積分器 6 つ

| 名前 | 中身 | 置き場所 |
| --- | --- | --- |
| `esdirk34` | diffsol の SDIRK（ESDIRK 3(4)）。陰的、刻み幅の制御あり | `globe-mode-exec/src/rumoca/plugins/rk.rs` |
| `trbdf2` | diffsol の SDIRK（TR-BDF2）。2 次、L 安定 | 同上 |
| `tsit45` | diffsol の Tsitouras 5(4)。陽的、刻み幅の制御あり | 同上 |
| `euler`・`heun`・`rk4` | 固定刻み（1・2・4 次）。格子は t0 + k·h、刻みの間は 3 次の Hermite 補間 | 係数と補間は `globe-mode-sim/src/integrators.rs`（Rumoca に依存しない）、プラグインは `plugins/fixed.rs` |

- `rk.rs` は Rumoca の `crates/rumoca-solver-diffsol/src/me_integrator.rs`（Apache-2.0）を写して改変した。ファイルの先頭に明記した（[licenses.md](../licenses.md)）。
- 固定刻みは、イベントや切り詰めの後も同じ格子に戻る。刻みの棄却（discard）は失敗にする（刻みを縮められないため）。刻み幅は `--step`、既定は出力の間隔。
- 陽的 RK も、ヤコビ行列つきの `rhs_implicit` で作る（diffsol の `ClosureNoJac` が非公開のため）。
- 計画の残り（`radau5`・`ida`・`cvode`・`esdirk23`・`esdirk45`・`sdirk34hw`・`dopri853`・`impeuler`・`trapezoid`・`cerk*`・`qss`）は持ち越す（`GM-E10005`）。

### (d) `auto` は BDF が失敗したら dopri45 で回し直す

- ネイティブと wasm32 で同じ判断（`fallback()`）を使う。切り替えるのは、組み立て・初期化で方向微分が作れない（S011 の規則。S011 は黙って切り替えていた）、積分の失敗（`GM-E11002`）、サンプラの食い違い（`MeSessionError::Contract` の文言）の 3 つ。切り替えたら情報 `GM-I11002` を出す。
- 積分の途中で方向微分が作れなくなった失敗では切り替えない（調査で、RK に切り替えると黙って誤った結果になる例があったため）。
- ただし S011 の規則（組み立て・初期化で方向微分が作れないときは RK。Rumoca の Auto と同じ）は残した。MSL の全例題では、この規則に当たったのは `GenerationOfFMUs`（`Thermal.HeatTransfer.Examples`）の 1 本だけで、RK で終了時刻まで進むが温度が動かず管から外れる（[research/msl-sim-failures.md](../research/msl-sim-failures.md)）。`GM-I11002` で知らせるので黙ってはいないが、この規則を見直すかは S013 の候補にした。
- `cargo xtask msl-sim` の既定のソルバを `auto` にする。

### (e) パラメータの上書き（再コンパイル無し）

- `SimSettings.parameters`（名前 → 値）を両方の道で `param_overrides` に渡す。DAE は使い回し、lowering と組み立てはやり直す。
- 変えられないもの（構造パラメータ・`final`・`Evaluate = true`・定数）、tunable なパラメータに無い名前、有限でない値は `GM-E10006`（S10）。Rumoca の `[EX003]` の 3 つの文言（structural or constant / not a tunable parameter / must be finite）で分ける。
- プリフライトも上書きを反映する。Rumoca の `eval_dae_at` は上書きを見ないので、`lower_for_simulation_with_overrides` と `SolveRuntime::eval_at` に置き換えた（許容誤差 1e-10、反復 32 回）。
- 変えられるパラメータの一覧は `globe_mode_exec::tunable_parameters(&CompiledModel)`（`globe simulate --list-parameters`）。結果の `SimRun.parameters` にも、使ったパラメータの値を入れる。

### (f) 結果の MAT v4。Arrow IPC は持ち越す

- `globe-mode-sim/src/mat.rs` に `write_mat4` / `read_mat4`。形は OpenModelica と Dymola の dsres（`Aclass`・`name`・`description`・`dataInfo`・`data_1`・`data_2`。配置は [research/openmodelica-dymola.md](../research/openmodelica-dymola.md#s012-で調べた結果の形式mat-v4-と-arrow)）。値がすべて同じ列は別名に、符号だけ反対の列は負の別名に畳む。依存は増やさない。
- `globe simulate --format csv|mat`（無ければ `-o` の拡張子）。
- **Arrow IPC は持ち越す**。OpenModelica の `arrow.modelica` は `modelica.format` 0.1 の開発版で、形が固まっていない。arrow-rs 60 を入れると通常の依存が 41 crate（GlobeMode に無いもの 25）増える。ライセンスはすべて許可の区分で、wasm32 でもビルドできることは確かめた。

### 設定の検査（S10）

| 条件 | コード |
| --- | --- |
| 許容誤差が 1e-10 未満（指定でも experiment でも）→ 1e-10 に上げる | `GM-W10003`（警告） |
| 固定刻みのソルバに許容誤差がある（使わない） | `GM-I10004`（情報） |
| 刻み幅が有限でない・0 以下・区間より長い | `GM-E10004` |
| 固定刻みでないソルバに `--step` | `GM-W10001` |
| パラメータの上書きが使えない | `GM-E10006` |

許容誤差の下限は、Rumoca のサンプラの照合（受け入れた歩の端の点）が丸めの床を持たず、1e-12 では丸めで違反するため（MSL の `SimpleTriacCircuit`）。

### 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| wasm の試作の「文言を読み直す道」（`simulate_fmi_component`） | 失敗を文言で返すので、初期化と積分のどちらで止まったかが型で分からない。session の道は同じ公開の部品を自分で順に呼ぶので、呼んだ位置で分かる |
| ネイティブもすべて session の道にする | session の道は Cranelift の JIT を使えない（`admitted_native_execution_backend` が `pub(crate)`）。型付きの道は Rumoca の CLI と同じ道で、S011 の MSL の数字と比べられる |
| wasm32-wasip1 | std の `Instant` が WASI の時計で動くので型付きの道も通る見込みだが、wasm-bindgen が使えず、ブラウザでは WASI の shim が要る。P6 の目標はブラウザの Worker |
| 上流の修正（`Instant` を `web-time` などに替える）を待つ | 本命の修正だが、P3 の完了条件をこのセッションで満たせない。上流への報告の候補に入れる |
| 結果を Arrow IPC で書く | OpenModelica の形式がまだ 0.1 の開発版。依存が 25 増える。MAT v4 は omc・Dymola・scipy・DyMat で読める |
| 自前の積分器を新しい crate `globe-mode-solver` に置く | Rumoca のプラグインの trait に結びつくので exec に置いた。Rumoca に依存しない部分（係数と補間）は `globe-mode-sim` に置き、wasm32 でもビルドできる |

## 結果

- **bdf が wasm で（Node 上で）動いた**（P3 の完了条件の 3 つ目）。Node v24.22.0 で、`GlobeModeValidation.Simulation` の 13 本 × 9 ソルバの突き合わせ（`mise run wasm:compare`）117 件のうち 114 件がネイティブと一致した。bdf・dopri45・auto は全件一致（最大 3 ulp）。一致しない 3 件はどれも `BouncingBall` のイベントの近くの自前の積分器（esdirk34 の相対差 9.4e-4、trbdf2 と tsit45 で行数が 1 違う）。MSL の `ChuaCircuit` も bdf で回り、t = 3686 s まで一致し、その後はカオスで分かれる。
- wasm の時間（`ChuaCircuit`、bdf、MSL を in-memory で読む）: Node は読み込み 10.5〜11.9 s（1 回だけ）・検査 2.8〜3.2 s・コンパイル 0.09 s・実行 0.45〜0.5 s。ネイティブの同じ道は 6.9 s・2.0 s・0.06 s・0.19 s。線形メモリは MSL を読んだ後 262 MiB、`ChuaCircuit` の後 513 MiB（CSV を返すと 634 MiB）、Node の RSS 886 MiB。
- wasm の大きさ: 既定の release 28.2 MB（gzip 7.07 MB）、最適化して 14.5 MB（gzip 4.48 MB）。`opt-level = "s"` は 3 より 10〜20% 遅い。
- 自前の積分器の精度（`Simulation.Decay`、Tolerance 1e-8 の x(1) の誤差）: euler（h 0.1）は 0.9^10 との差 −1.7e-16、tsit45 7.3e-10、esdirk34 −1.1e-7、trbdf2 −3.2e-7、rk4 は h 0.1 で 3.3e-7・h 1/500 で 4.8e-14。大域誤差の次数の実測は euler 1.03・heun 2.06・rk4 4.06。`BouncingBall` は 6 つとも跳ねた回数 3。
- MSL の `Electrical.Analog.Examples` 54 本（予算 30 s、回し直し無し）: bdf 50 本が実行・48 本が管、tsit45 40・36（時間切れ 7）、esdirk34 42・39（時間切れ 2）。esdirk34 は bdf で通る 5 本で diffsol の非線形の失敗か刻みが小さすぎる、tsit45・esdirk34 は 5 本で射影が収束しない（持ち越し）。
- MSL の全例題（`mise run msl:sim`、既定 `auto`、時間切れの回し直し 180 s）: 対象 356 本（partial の 12 本と例題でない `SpacePhasorLimiter` を除く）のうち **309 本が終了時刻まで進み**（S011 は 303 本、+6）、参照結果と比べた 308 本のうち **299 本（97.1%）が管に収まった**（S011 は 292 / 302 本、+7。信号の数で 1753 / 1772）。止まった 47 本（S011 は 54 本）は、実行系が未対応（GM-U0004）24・初期化 6・時間切れ 5・積分 4・イベント 1・外から停止 7（どれも実行系の段）。GM-U0005（サンプラの食い違い）は 4 → 0。時間切れの 6 本を予算 180 s で回し直して 1 本が通った。`auto` が dopri45 に切り替えたもの（GM-I11002）は 11 本で、うち 4 本が最後まで進んだ。管から外れた 9 本のうち 2 本は既知の外れ（`src/Modelica/msl-sim-known-deviations.txt`）。新しく実行できた 6 本のうち `GenerationOfFMUs` は、Rumoca と同じ規則（初期化で方向微分が作れない）で RK に切り替わって最後まで進むが、温度が動かず管から外れる（調査の上流への報告の候補 6「RK で黙って誤った結果」）。全体 2945.8 s（49 分。回し直しを含む。S011 は 31 分 52 秒）、ワーカーの最大の常駐メモリ 3.4 GiB（[msl-sim.md](../msl-sim.md)）。
- `auto` の切り替えで MSL の `TwoMass` が通る（bdf は `GM-E11002`、auto は dopri45 で管 5/5）。
- パラメータの上書きは `Simulation.Parameters` で確かめた（`k = 2` で x(1) = exp(−2)、`c = 2k` が追従。`n`・`e`・`f`・`g`・`x`・無い名前・NaN は `GM-E10006`）。
- MAT v4 は scipy と DyMat 0.7 で読め、omc 1.27.1 の実物と同じスクリプトで同じに読めた。omc との違いは、変数の並び、omc は最後の時点を 2 回書く、omc はイベントの 2 行目の時刻を 1.5e-10 ずらす、の 3 つ。
- 残した問題: プリフライトの lowering は 2 回のまま。サンプラの食い違いの判定は文言頼み。型付きの道で Cranelift の JIT が本当に使われているかは【未確認】（session の道（インタプリタ）とビット単位で同じだった）。panic の後の `Workspace` は使えない。ブラウザは未試験。
