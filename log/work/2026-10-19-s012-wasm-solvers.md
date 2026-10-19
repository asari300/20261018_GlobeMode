# S012 `wasm-solvers`

| | |
| --- | --- |
| 日付 | 2026-10-19 |
| ブランチ | `feature/s012-wasm-solvers` |
| worktree | `.git-worktree/s012-wasm-solvers` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S012-wasm-solvers/` |
| セッション記録 | `log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md`（S005・S007〜S011 と同じセッション） |

## 依頼

「進めて。サブエージェントはOpusのeffort-medium」（S011 の報告の後）。roadmap の S012 の候補を進める。サブエージェントは Opus、effort medium で動かす。

## 決めたこと

- **範囲**（既定のまま進めた）: roadmap の S012 の候補のうち、1（bdf を wasm で Node 上で動かす。P3 の残りの完了条件）を中心に、3（自前の積分器）、4（パラメータを変えた再実行と結果の MAT v4）、2（止まった 54 本と管から外れた 10 本の調査）を並べる。5〜10 は持ち越す。
- **進め方**（S011 と同じ形。ユーザーの指示でサブエージェントは Opus・effort medium）: 調べもの 3 つ（wasm の試作、積分器・再実行・MAT の調査、止まった例題の調査）を先に並べ、その結果で骨格を固めてから、所有するファイルを分けた実装のサブエージェントを同時に走らせ、最後に文書のサブエージェントに渡す。
- wasm の試作は、他のビルドとロックを取り合わないよう `CARGO_TARGET_DIR=target/spike-wasm` でビルドさせた。
- **調べものの結論**（3 つとも確認日 2026-10-19、Rumoca rev 276e82bd）:
  - wasm: wasm32-unknown-unknown + wasm-bindgen で、検査 → コンパイル → bdf が Node v24 で動いた（`Simulation.*` 12 本、MSL の ChuaCircuit・First。ネイティブとの差は 1 ulp 以内、カオスの Chua はイベントの時刻 1e-13 のずれから後半で開く）。Rumoca の型付きの道（`build_simulation`）は std の `Instant::now` で panic する。getrandom 0.3 は cfg に加えて feature `wasm_js` も要る。
  - 自前の積分器: `MeIntegratorBackend` と FMI ME 風の host の生成・実行の関数はどれも公開されていて、Rumoca に手を入れずに差し込める（試しで euler・rk4・tsit45・esdirk34 が Decay・BouncingBall・HarmonicOscillator を解析解どおりに解いた）。この道は `Instant::now` を呼ばず、Rumoca の BDF と RK45 のプラグイン（`model_exchange_integrator`）も公開されている。
  - パラメータの再実行は `SimOptions::param_overrides`（再コンパイル不要、lowering はやり直し）。MAT v4 は omc 1.27.1 の実物で配置を確かめた。Arrow は OpenModelica の形式がまだ開発版で依存が 25 増えるので後に回す。
  - 止まった 54 本: Rumoca の CLI では通るのに GlobeMode で止まるものは 0 本。GlobeMode 側で直せるもので +5 本（`auto` の dopri45 への切り替え +3、許容誤差の下限 +1、時間切れの回し直し +1）、管の外れ 10 本のうち比べ方（イベントの行）で +2 本の見込み。
- **設計の決定**（調べものの結論から、既定のまま進めた）:
  - **session の道を 1 つ作り、wasm と自前の積分器の両方に使う**（`globe-mode-exec/src/rumoca/session.rs`）。wasm の試作の「文言を読み直す道」（`simulate_fmi_component`）は、初期化と積分の段が型で分からなくなるので捨てた。ネイティブの bdf・dopri45・auto は今の道（Cranelift の JIT、Rumoca の CLI と同じ数値）のまま。
  - 自前の積分器は `esdirk34`・`trbdf2`・`tsit45`（diffsol）と `euler`・`heun`・`rk4`（固定刻み）の 6 つを「使える」にした。残りの計画のもの（`radau5`・`ida` など）は持ち越す。
  - 新しいコード: `GM-E10006`（パラメータの上書きが使えない）、`GM-W10003`（許容誤差を下限 1e-10 に上げた）、`GM-I10004`（固定刻みは許容誤差を使わない）、`GM-I11002`（`auto` で BDF が積分に失敗したので dopri45 で回し直した）。
  - Arrow IPC は持ち越す（OpenModelica の `arrow.modelica` は 0.1 の開発版、依存が 25 増える）。

## やったこと

- 骨格（親）: ir の `sim.rs` に `SimSettings{step, parameters}`・`ResolvedSettings{step, parameters}`・`SettingOrigins.step`、`SolverId` の 6 つ（Esdirk34・Trbdf2・Tsit45・Euler・Heun・Rk4）と `is_fixed_step`、`TrajectoryVariable.description`、`SimRun.parameters`（`ResultParameter`）、`TunableParameter`。core に GM-E10006・GM-W10003・GM-I10004・GM-I11002。sim の登録表で 6 つを「使える」に。exec の `rumoca/session.rs` と `plugins.rs` の器。workspace の依存（`rumoca-phase-solve`・`rumoca-solver-diffsol`・`rumoca-solver-rk45`・`diffsol`・`self_cell`・`wasm-bindgen = "=0.2.129"`）。exec の新しい直接の依存はどれも Cargo.lock に既にあり、外部の package は増えていない。
- A 実行系の本体（`globe-mode-exec/src/{lib.rs,rumoca.rs,rumoca/session.rs}`、`tests/simulate.rs`、`Simulation/Parameters.mo`・`package.order`）: session の道を仕上げ、型付きの道とビット単位で同じことを 12 本で確かめた。`auto` の切り替え（`fallback()`、GM-I11002）。パラメータの上書き（両方の道で `param_overrides`、`[EX003]` → GM-E10006 で文言を 3 つに分ける）。プリフライトの上書き（`lower_for_simulation_with_overrides` と `SolveRuntime::eval_at`）。`tunable_parameters`、`#[doc(hidden)] simulate_via_session`、`SimRun.parameters`。積分の途中の方向微分の失敗の文言。
- C 自前の積分器（`globe-mode-exec/src/rumoca/{plugins.rs,plugins/rk.rs,plugins/fixed.rs}`、`tests/integrators.rs`、`globe-mode-sim/src/integrators.rs`）: esdirk34・trbdf2・tsit45（Rumoca の `me_integrator.rs` を写して改変、先頭に明記）と euler・heun・rk4（格子 t0 + k·h、3 次の Hermite 補間、discard は失敗）。
- B wasm（`globe-mode-wasm/`（`src/lib.rs`、`tests/decay.rs`、`node/{smoke,simulate,compare}.mjs`）、frontend の `Checker::with_library_sources`、db の `GlobeDb::with_library_sources`、exec の wasm32 の getrandom、`.cargo/config.toml`、`mise.toml`（wasm-bindgen の CLI、`wasm:build`・`wasm:node`・`wasm:compare`）、`ci.yml`（ジョブ `wasm-node`、wasm32 の check に exec と wasm）、`ci_plan.py`・`test_ci_plan.py`（`WASM_CRATES` と出力 `wasm_node`））。
- D sim と CLI（`globe-mode-sim/src/{mat.rs,settings.rs,compare.rs,solvers.rs,store.rs,lib.rs}`、`globe-mode-cli/src/{main.rs,simulate.rs}`、`tests/simulate.rs`）: MAT v4 の読み書き、許容誤差の下限と固定刻みの検査、`normalize_events`、`globe simulate` の `--param`・`--step`・`--list-parameters`・`--list-solvers`・`--format`。
- X xtask（`xtask/src/{sim.rs,main.rs}`、`src/Modelica/msl-sim-known-deviations.txt`、`mise.toml` の `msl:sim`）: 既定 `auto`、`--retry-wall-seconds 180`、`--known-deviations`、`--baseline-json`、ワーカーの段の知らせと `worker_stage`、補足（最大 4 つ・600 字）、例題でないものの規則。
- 調べもの 3（`doc/research/msl-sim-failures.md`、新規）: 止まった 54 本と外れた 10 本の原因、直せるもの、上流への報告の候補 9 つ。
- 文書（文書の担当）: ADR-0014（新規）と ADR-0013 の「結果」の追記、ADR の索引、architecture・roadmap・features・licenses、research（rumoca.md・openmodelica-dymola.md・msl-sim-failures.md の「S012 で直したもの」）、README、`doc/sessions/README.md`、スライドと解説書、この作業記録。

## 検証

- テスト: workspace 全体で 485 本が通過・10 本 ignore・0 失敗（S011 は 422 本・7 本）。clippy 0 件。
- `mise run check`（fmt・clippy・test・deny・`uv sync --locked`）: `test_ci_plan.py` 20 本 OK、fmt の差分 0、clippy 0 件、テスト 485 本が通過・10 本 ignore・0 失敗、`cargo deny check` は advisories・bans・licenses・sources とも ok、`uv sync --locked` ok（`msl:sim` の後に回した）。
- 2 つの道: `Simulation.*` 12 本の bdf で、型付きの道と session の道（`RumocaBdf`）はビット単位で同じ（成功 8 本の行数・終わり方・全変数の最後の値、失敗 4 本の Diagnostic 全体）。dopri45・auto も一致。
- wasm（Node v24.22.0、`mise run wasm:compare`）: 13 本 × 9 ソルバの突き合わせ 117 件中 114 件が一致。bdf・dopri45・auto は全件一致（最大 3 ulp）、自前の 6 つは最大 1588 ulp。一致しない 3 件はどれも `BouncingBall` のイベントの近くの自前の積分器（esdirk34 の相対差 9.4e-4、trbdf2 と tsit45 で行数が 1 違う）。`ChuaCircuit` は t = 3686 s まで一致。`Decay` の x(1) の誤差は bdf 2.7e-8、dopri45 1.9e-9。
- wasm の時間（`ChuaCircuit`、bdf）: Node・in-memory は読み込み 10.5〜11.9 s・検査 2.8〜3.2 s・コンパイル 0.09 s・実行 0.45〜0.5 s。ネイティブ・in-memory は 6.9 s・2.0 s・0.06 s・0.19 s。Node・文書は 1 本 24.0 s、ネイティブ・文書は検査 14.4 s・コンパイル 3.2 s。線形メモリ: MSL を読んだ後 262 MiB、`ChuaCircuit` の後 513 MiB（CSV を返すと 634 MiB）、Node の RSS 886 MiB。
- wasm の大きさ: 既定の release 28.2 MB（gzip 7.07 MB）、最適化して 14.5 MB（gzip 4.48 MB）。最適化した版を空からビルドして 283 s。Node の `Decay` の 1 本目 228〜293 ms（うち検査 164〜203 ms）、2 本目以降約 5 ms。
- 自前の積分器: `Decay`（Tolerance 1e-8）の x(1) の誤差は euler（h 0.1）−1.7e-16（0.9^10 と）、tsit45 7.3e-10、esdirk34 −1.1e-7、trbdf2 −3.2e-7、rk4 は h 0.1 で 3.3e-7・h 1/500 で 4.8e-14。次数の実測は euler 1.03・heun 2.06・rk4 4.06。`HarmonicOscillator` は tsit45 3.6e-10、rk4（h 0.01）2.4e-9。`BouncingBall` は 6 つとも跳ねた回数 3。
- MSL（予算 30 s、回し直し無し）: `Electrical.Analog.Examples` 54 本で bdf 50 実行・48 管、tsit45 40（時間切れ 7）・36、esdirk34 42（時間切れ 2）・39。`Rotational.Examples` 7 本は tsit45・trbdf2・rk4 で 7/7、heun（h = 出力の間隔）は 3/7。最後まで進んだ 39 本の積分時間の合計は bdf 10.7 s、esdirk34 10.9 s、tsit45 36.7 s。
- プリフライトの時間（release）: `ChuaCircuit` 0.020 s（組み立て 0.038 s）、`DoublePendulum` 0.94 s（2.07 s）、`Fluid.HeatingSystem` 5.86 s（組み立ては失敗）。
- MAT v4: scipy・DyMat 0.7 で読め、omc 1.27.1 の実物と同じスクリプトで同じに読めた。
- 比べる前の正規化: S011 の結果を比べ直すと管 292 → 294 本（`PolyphaseTwoLevel_R` 24/24、`_RL` 47/47）。
- `cargo xtask msl-sim --only`: `TwoMass` は auto → dopri45 で管 5/5、`ThyristorBridge2mPulse_RLV_Characteristic` は回し直し 133 s で 25/25、`SpacePhasorLimiter` は対象外、`FilterWithDifferentiation` は補足に EL005、`Multivibrator` は既知の外れ、`DrumBoiler` は実行系の段で外から停止。
- `mise run msl:sim`（MSL の全例題、既定 `auto`）: 対象 356 本（partial の 12 本と例題でない `SpacePhasorLimiter` を除く）のうち **309 本が終了時刻まで進み**（S011 は 303 本、+6）、参照結果と比べた 308 本のうち **299 本（97.1%）が管に収まった**（S011 は 292 / 302 本、+7。信号の数で 1753 / 1772）。止まった 47 本（S011 は 54 本）は、実行系が未対応（GM-U0004）24・初期化 6・時間切れ 5・積分 4・イベント 1・外から停止 7（どれも実行系の段）。GM-U0005（サンプラの食い違い）は 4 → 0。時間切れの 6 本を予算 180 s で回し直して 1 本が通った。`auto` が dopri45 に切り替えたもの（GM-I11002）は 11 本で、うち 4 本が最後まで進んだ。管から外れた 9 本のうち 2 本は既知の外れ（`src/Modelica/msl-sim-known-deviations.txt`）。新しく実行できた 6 本のうち `GenerationOfFMUs` は、Rumoca と同じ規則（初期化で方向微分が作れない）で RK に切り替わって最後まで進むが、温度が動かず管から外れる（調査の上流への報告の候補 6「RK で黙って誤った結果」）。全体 2945.8 s（49 分。回し直しを含む。S011 は 31 分 52 秒）、ワーカーの最大の常駐メモリ 3.4 GiB（[msl-sim.md](../../doc/msl-sim.md)）。
- `mise run wasm:node`（締めで回し直した）: Node v24.22.0 で `Decay` の bdf の x(1) の誤差 2.7e-8・dopri45 1.9e-9、MSL の `ChuaCircuit`（in-memory のライブラリ 13.6 s、検査 3.2 s・コンパイル 0.11 s・実行 0.52 s、線形メモリ 513 MiB）、panic の文言を `last_panic` で読めた。252 s（wasm のビルドを含む）。最適化した wasm は 14.5 MB。
- `mise run wasm:compare`（締めで回し直した）: 117 件中 114 件が一致、3 件は上の `BouncingBall` の既知の差。締めで、`compare.mjs` がこの 3 件を「既知の差」（成功と診断のコードが一致していれば CSV の差を数えない）にして、それ以外に差があるときだけ終了コード 1 にするようにした（直す前は 3 件で終了コード 1 だった）。
- `mise run msl:compare`（frontend に `with_library_sources` を足したので回す）: 20 本中 20 本が一致（両方とも成功 17 本）、199 s。
- `msl:funnel` は回し直していない（S0〜S10 の判定の規則は変えていない。S10 の新しいコードは実行の設定にだけ当たる）。
- `check_docs.py`: デッキ 18 件 OK、文書 94 件・相対リンク 655 件で切れ 0 件。`render_slides.py`: デッキ 18 件で問題 0 件（S012 の 17 枚のスクリーンショットを見て、はみ出し無し）。セッション記録の `--check`: 4 件で取りこぼし 0 件。ステージした差分の漏洩検査: 0 件。
- CI の `wasm-node`: Actions の上では未確認（push の後に確かめる）。

## 残したこと

- **止まった例題の残り**: Rumoca の構造解析の EL005（インデックスの低減が足りない）18 本、Solve IR への変換が終わらない Fluid の例題など。多くは上流の問題。
- **自前の積分器**: esdirk34 は bdf で通る 5 本で diffsol の非線形の失敗・刻みが小さすぎる。tsit45・esdirk34 は 5 本で射影が収束しない（GM-E11002）。陽的 RK もヤコビ行列つきの `rhs_implicit` で作る（diffsol の `ClosureNoJac` が非公開）。固定刻みの既定の刻み（出力の間隔）では heun の精度が足りない。
- **ブラウザ**: `--target web` と Worker は未試験。panic の後の `Workspace` は使えない（作り直す道が要る）。
- **プリフライトの lowering が 2 回**（上書きを反映するため、組み立てと別に lowering する）。
- **exec の中の段の知らせ**: `msl-sim` の段は exec の外の 3 つまで。
- **`auto` の初期化での切り替え**: 初期化で方向微分が作れないときに RK に切り替える規則（S011 から、Rumoca の Auto と同じ）は残した。MSL の全例題でこれに当たったのは `Thermal.HeatTransfer.Examples.GenerationOfFMUs` の 1 本だけで、RK で終了時刻まで進むが温度が動かず管から外れる（`GM-I11002` は出る）。切り替えをやめて GM-U0004 で止めるかを S013 で決める（全体の回し直しの後に気づいたので、S012 では規則を変えていない）。
- **管の外れの残り**: 9 本のうち既知の外れでない 7 本（`DemonstrateSignalExtrema`・`DemoPowerSupplyWithBuffer` は許容誤差を 1/10 にすると収まる見込み、Clocked の 2 本、`Dimmer_R`、`DoublePendulumInitTip`、`GenerationOfFMUs`）。
- **サンプラの食い違いの判定は文言頼み**（`MeSessionError::Contract` の文言）。
- **型付きの道で Cranelift の JIT が本当に使われているかは【未確認】**（session の道（インタプリタ）とビット単位で同じだった）。
- **Arrow IPC**（OpenModelica の形式が 0.1 の開発版）、**残りの計画のソルバ**（radau5・ida など）。
- **上流への報告の候補**（ユーザーが出す）に S012 の分を足した: 型付きの道の `Instant::now()`（wasm）、`admitted_native_execution_backend` と `ClosureNoJac` が非公開、`doc/research/msl-sim-failures.md` の 9 つ。
- 持ち越し（S011 の候補 5〜10）: S10 の初期化のヤコビ行列の階数、S9 を omc と照らす、閉包の切り替えの残り、`Checker` のメモリ、字面の 0 除算・S8・`class_index` の残り。
