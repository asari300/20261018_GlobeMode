# ADR-0013 P3 の実行系は、Rumoca の実行系（rumoca-sim）を globe-mode-exec に閉じ込めて使う。結果は中立の型で返し、MAP-LIB の参照結果と矩形の管で比べる

- 状態: Accepted（2026-10-19、S011）

## 背景

- P3（シミュレーション）の完了条件は「MSL の例題 153 本以上が実行できる。そのうち 90% 以上が参照結果のファネルに収まる。bdf が wasm で（Node 上で）動く」（[roadmap.md](../roadmap.md)）。設計（[architecture.md](../architecture.md) §3.3）は、自前の `trait HybridSystem` と `trait Integrator` とハイブリッドの driver を持ち、Modelica のモデルと FMI-ME の FMU が同じ driver を使う形を描いていた。
- S011 で Rumoca v0.10.2（rev `276e82bd`）の実行系を読んで確かめた事実（[research/rumoca.md](../research/rumoca.md#s011-で調べた実行系v0102)）:
  - `rumoca-sim` の入口は `simulate_with_diagnostics(&Dae, &SimOptions)`（Auto / Bdf / RkLike）と、BDF だけの型付きの道（`build_simulation` → `check_prepared_initialization` → `run_prepared_simulation`。失敗の段階 `SimFailureStage` が分かる）。ソルバは diffsol 0.13 の BDF と、Rumoca の Dormand–Prince 5(4) の 2 つだけ（`DiffsolMethod` は `Bdf` だけ）。
  - Rumoca のモデルを外の積分器から回す FMI-ME 風の核（`SolveMeKernel`）の set_time・状態・導関数・イベント指標・イベント更新は `pub(crate)`。外から差し替えられるのは数値積分のプラグイン（`MeIntegratorBackend`）だけで、ハイブリッドの driver（初期化・イベント・出力）は Rumoca の `MeSimulationSession` が持つ。**公開の API では、Rumoca のモデルを自前の `HybridSystem` として包めない。**
  - 時間切れは協調型（`TimeoutBudget::check()` は積分の繰り返しと根探索でだけ見る）。初期化と積分器の 1 歩の内側では止まらない。Rumoca 自身の MSL のハーネスは、モデルごとに別プロセスのワーカーで回し、監視から止めている。
  - Rumoca は自分のワークスペースで crates.io の diffsol 0.13.2 に BDF の数値の修正を当てている（`vendor/diffsol/RUMOCA_PATCH.md`）。`[patch]` はワークスペースのルートでしか効かないので、git 依存では修正前の版が入る。
  - wasm 以外では Cranelift の JIT（`rumoca-exec-cranelift`）が必ず付いてくる。初期化系のヤコビ行列の階数を返す公開の API は無い（内部で SVD の数値の階数を tracing の debug に出すだけ）。
- MSL 4.1.0 の参照結果（[research/openmodelica-dymola.md](../research/openmodelica-dymola.md#s011-で調べた参照結果と比較の方法)）: `modelica/MAP-LIB_ReferenceResults` のブランチ `v4.1.0`（先頭 `dac67829`）。`Modelica/` の下に 534 本（CSV と `comparisonSignals.txt`）。全部で約 2 GB。比較の標準は csv-compare（BSD-3-Clause）の矩形の管で、許容差 0.002。OpenModelica（master、2026-10-17）は 533 本を実行し、510 本が参照結果と一致する。

## 決定

### 実行系（`globe-mode-exec`）

- **Phase A は Rumoca の実行系をそのまま使う**。`rumoca-sim` を `default-features = false, features = ["solver-diffsol", "solver-rk45"]` で、`globe-mode-exec` の feature `rumoca-exec` の中だけに入れる（cargo-deny の `wrappers = ["globe-mode-exec"]`）。umbrella crate の既定の feature（zenoh・gamepad・UDP・web viewer）は入れない。
- **diffsol は Rumoca と同じ修正版を使う**。GlobeMode のワークスペースのルートに `[patch.crates-io] diffsol = { git = "https://github.com/CogniPilot/rumoca", rev = "276e82bd…" }` を置く（cargo は git のリポジトリの中から `vendor/diffsol` の package を見つける。S011 で `Cargo.lock` の source が Rumoca の git になることを確かめた）。Rumoca の CLI と同じ数値になる。
- フロントエンドは DAE まで変換したモデルを `CompiledModel`（Rumoca の `Arc<Dae>` と、生の experiment annotation）として渡す。中身に手が届くのは `#[doc(hidden)] rumoca_dae()` だけで、使うのは `globe-mode-exec` だけ。db は `GlobeDb::compiled_model(model)`（salsa の外。`check` と同じ文書を渡すので、検査の直後なら Rumoca のモデルごとのキャッシュから返る）。db は wasm でビルドするので、exec には依存しない。
- 結果は中立の型（`globe-mode-ir` の `SimSettings` / `ResolvedSettings` / `Trajectory` / `SimRun`）で返す。Rumoca の型は外へ出さない。
- ソルバの名前は `globe-mode-sim::solvers` の登録表（[architecture.md](../architecture.md) §3.3）で引く。実装のあるのは `bdf`（diffsol の BDF。既定）・`dopri45`（Rumoca の Dormand–Prince 5(4)）・`auto`（Rumoca に選ばせる）。`dassl`・`lsodar` → `bdf`、`rk45`・`dopri5` → `dopri45` は別名（`GM-I10003`）。表にあってまだ実装の無いもの（`esdirk34`・`tsit45`・`euler` など）と、表に無い名前は `GM-E10005`。
- 設定は「利用者の指定 > experiment annotation > 既定値」（StartTime 0、StopTime は開始 + 1、Tolerance 1e-6、Interval は区間の 1/500）。許容誤差は相対と絶対の両方に使う（Rumoca の MSL のハーネスと同じ）。

### S10 の数値の部分と、実行の失敗

| 段階 | 検査 | コード |
| --- | --- | --- |
| S10（実行の直前） | start 値で評価して、t0 で代数変数と導関数が有限か（`rumoca_sim::eval_dae_at`）。`fixed = false` の状態などは初期化で変わるので、確定できず**警告**にとどめる | `GM-W10002` |
| S10 | ソルバが使えない / 別名を置き換えた | `GM-E10005` / `GM-I10003` |
| 実行 | 初期化に失敗 / 積分に失敗 / イベントの反復が収束しない / assert / 時間切れ / 結果に NaN・inf | `GM-E11001`〜`GM-E11006` |
| 実行 | `terminate()` で終了時刻の前に終わった（失敗ではない） | `GM-I11001` |
| 実行 | 実行が止まらない Rumoca の警告（warning レベルの assert。Rumoca の WX001） | `GM-W11001` |
| 実行 | 実行系が未対応（Rumoca の lowering・組み立てで止まった）/ 実行系の内部エラー（panic、契約違反） | `GM-U0004` / `GM-U0005` |

- 実行の失敗は検査段階ではないが、同じ診断の形で返すので `Stage::Run` を足した。
- **初期化のヤコビ行列の階数は後回し**にする（公開の API が無い。tracing の出力を拾うのは安定しない）。S009 の `Invalid.UngroundedLoop` と MSL の `AdvancedSolenoid` は、実行すると `GM-E11001` になるかを見る（結果は下）。

### 結果の置き場と、参照結果との比較（`globe-mode-sim`）

- `globe-mode-sim` は Rumoca にも実行系にも依存しない（wasm32 でビルドできる）。設定の決定、ソルバの登録表、CSV（MAP-LIB の参照結果と同じ形）、比較を持つ。MAT v4 と Arrow IPC は後で足す。
- 比較は csv-compare の既定（Rectangle）の管を、アルゴリズムの記述から書き直す（コードは写さない）。半幅は時間 δt = tol·max(t_N − t_0, |t_0|, 1e-12)、値 δy = tol·max(y_max − y_min, |y_min|, 0.001)、tol = 0.002。参照の区間の外の点は判定しない。試験側に無い信号は不合格にする（csv-compare は警告で飛ばす）。
- 参照結果はコミットしない。`cargo xtask fetch-msl-ref` が、要るモデルの分だけ固定したコミットの raw の URL から `data/msl-ref/` に取る（MAP-LIB のリポジトリには LICENSE が無く、再配布の条件は【未確認】。取得して試験に使うだけにする）。

### MSL の全例題の実行（`cargo xtask msl-sim`、`mise run msl:sim`）

- `msl:funnel` で OK になった例題を、**別プロセスのワーカー**で順に回す。ワーカーは MSL を一度だけ読み、モデルごとに `catch_unwind` で囲む。親はモデルごとの時間（`--kill-seconds`）を超えたらワーカーを止めて作り直す（Rumoca の時間切れは協調型のため）。協調型の予算（`--wall-seconds`）も渡す。
- 参照結果のあるモデルは管で比べ、報告 [msl-sim.md](../msl-sim.md) に、実行できた本数・失敗の内訳・管に収まった本数を書く。時間がかかるので CI には入れない。

### Modelica のモデルと FMU の driver

- [architecture.md](../architecture.md) §3.3 の「Modelica のモデルと FMI-ME の FMU は同じ driver を使う」は、Phase A では成り立たない（Rumoca の driver が Rumoca のモデルを回す）。自前の `HybridSystem` と driver は、FMU の Import（P5）と Track B の自前コアで作る。Modelica のモデルを自前の driver で回す道は、(a) Rumoca の FMU の出力（C のコンパイラが要る）、(b) 低い層の公開 API（`SolveRuntime`。安定でない）、(c) 上流に `SolveMeKernel` の公開を提案する、のどれか。決めるのは P5 の前にする。
- 自前の積分器（`esdirk34`・`tsit45`・固定刻みなど）は、Rumoca の数値積分のプラグイン（`MeIntegratorBackend`）として差し込める見込み。S012 以降で試す。

### 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| 実行系を自前で作る（Solve IR の評価器、driver、ソルバ） | P3 の 1 回目で MSL の例題を回せない。Rumoca の実行系は MSL の 216 本（Rumoca の基準値）を回せる。自前の driver は FMU（P5）で要るので、そこで作る |
| Rumoca のモデルを自前の `HybridSystem` で包む | 公開の API では包めない（上の背景）。`SolveRuntime` の低い層は安定な API ではない |
| diffsol を crates.io の版のまま使う | Rumoca の CLI と数値が変わる（BDF の確定解の修正など 4 件が入らない）。Rumoca の MSL の数字と比べられなくなる |
| 時間切れを同じプロセスの協調型の予算だけで守る | 初期化と積分器の 1 歩の内側で止まらない。MSL の全例題を回すと止まらないモデルが出る |
| Rumoca の比較の指標（正規化した平均の絶対誤差、`sim_trace_compare`）を使う | 平均した L1 で、0.2% の管よりずっと緩い。P3 の完了条件の「ファネル」は MSL の参照結果の管（csv-compare）で測る |
| 参照結果を zip で一度に取る | 約 779 MiB（展開して約 2.7 GB）。要るのは OK の例題の分だけ |

## 結果

- **MSL の全例題**（`mise run msl:sim`、S011、[msl-sim.md](../msl-sim.md)）: `msl:funnel` で OK の 369 本のうち partial の 12 本を除く 357 本を実行し、**303 本が終了時刻まで進んだ**（P3 の目標 153 本以上）。参照結果と比べた 302 本のうち **292 本（96.7%）が管に収まった**（目標 90% 以上）。P3 の完了条件の 3 つのうち 2 つを満たした（残りは bdf を wasm で動かすこと）。
  - 止まった 54 本: 実行系が未対応 `GM-U0004` 26（Rumoca の構造解析 19・Solve IR への変換 3・実行系の組み立て 2・方向微分の準備 2）、初期化 `GM-E11001` 6、積分 `GM-E11002` 6、協調型の時間切れ `GM-E11005` 5、実行系の内部エラー `GM-U0005` 4（実行時の契約違反）、外から停止（120 s）7（Fluid の 5 本と `EngineV6` の 2 本）。panic 0、ワーカーの異常終了 0。
  - 管から外れた 10 本は、イベントの時刻のずれ（Clocked の 2 本、PowerConverters の 3 本）、信号の極値の追跡（`DemonstrateSignalExtrema`）、発振の位相（`Multivibrator`）などの見込み（原因は確かめていない）。信号の数では 1713 本中 1687 本が合格。
  - 時間は全体 31 分 52 秒（コンパイル 183.7 s・実行系の準備 139.8 s・初期化と積分 159.5 s。残りの多くは外から止めた 7 本の 120 s と、協調型の時間切れの 30 s）。ワーカーの最大の常駐メモリ 3.5 GiB。
  - 最初の実行では 3 本が「参照に無い信号」で落ちた。MAP-LIB の `comparisonSignals.txt` には `//` で比べないことにした信号を書き残したものがあり、読み飛ばすように直した。`CauerLowPassSC` は協調型の予算 30 s のぎりぎり（1 回目 33 s で時間切れ、2 回目は通った）。
- **数値的に特異なモデルは実行で捕まらない**: S009 の `Invalid.UngroundedLoop`（接地の無い 2 本の抵抗。構造では見えない特異）は、`globe simulate` で終了時刻まで進み、値はすべて 0（Rumoca の初期化が特異な系から 0 の解を選ぶ）。`GM-E11001` にはならない。MSL の `AdvancedSolenoid` は Rumoca の構造解析で止まる（`GM-U0004`）。初期化のヤコビ行列の階数の検査（S10）は、この形を見つけるために要る（roadmap の S012 の候補）。
- Rumoca の基準値（rev `276e82bd` の `msl_quality_baseline.json`、試した 286 本で simulate 216、予算 12 s）とは、対象の選び方と予算が違うので並べるだけにする。OpenModelica（master）は 534 本中 533 本を実行し、510 本が参照結果と一致する。
- 検証用のモデル `GlobeModeValidation.Simulation.*` 12 本（解析解のある ODE、イベント、`sample`、assert、terminate、t0 で有限にならない式、初期化の失敗、イベントの反復が収束しない、状態の無いモデル、非線形のループ、警告の assert）。`globe-mode-exec` のテストで bdf と dopri45 の両方を回す。
