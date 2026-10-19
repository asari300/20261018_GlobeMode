# S011 `exec-backend`

| | |
| --- | --- |
| 日付 | 2026-10-19 |
| ブランチ | `feature/s011-exec-backend` |
| worktree | `.git-worktree/s011-exec-backend` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S011-exec-backend/` |
| セッション記録 | `log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md`（S005・S007〜S010 と同じセッション） |

## 依頼

「P3の実行系の着手」（S010 の締めで挙げた S011 の候補 1。ロードマップの「P3 の始め（実行系 `globe-mode-exec` と driver）」と、S10 の数値の部分）。

## 決めたこと

ユーザーの指示は「P3の実行系の着手」だけ。ほかは既定のまま進めた（サブエージェントの effort は指定が無かったので既定のまま）:

- **範囲**: roadmap の S011 の候補 1（P3 の始め: 実行系と driver、S10 の数値の部分）。P3 の完了条件（MSL の例題 153 本以上が実行できる・そのうち 90% 以上が参照結果のファネルに収まる・bdf が wasm で動く）のうち、最初の 2 つを測れるところまで作る。
- **実行系は Rumoca の `rumoca-sim` をそのまま使う**（[ADR-0013](../../doc/adr/0013-exec-backend.md)）。調べると、Rumoca のモデルを自前の `HybridSystem` で包む公開の API が無い（FMI-ME 風の核の set_time・導関数・イベント更新は `pub(crate)`。外から差し替えられるのは数値積分のプラグインだけ）。自前の評価器と driver は P3 の 1 回目で MSL を回せないので選ばなかった。「Modelica と FMU は同じ driver」は Phase A では成り立たないと architecture に書いた。
- **diffsol は Rumoca と同じ修正版**: Rumoca は自分のワークスペースで crates.io の diffsol 0.13.2 に BDF の数値の修正を当てている。`[patch]` はルートでしか効かないので、GlobeMode のルートに Rumoca の git の `vendor/diffsol` を指す patch を置いた（Cargo.lock の source が Rumoca の git になるのを確かめた）。
- **t0 で有限にならないのは警告（GM-W10002）**: `eval_dae_at` は start 値で評価するので、`fixed = false` の状態など初期化で変わるものを誤って止めうる。最初は Error（GM-E10005）にしていたが、骨格の段階で警告に変え、ソルバが使えないのを GM-E10005 にした。
- **初期化のヤコビ行列の階数は送った**: Rumoca に公開の API が無い（SVD の数値の階数を tracing の debug に出すだけ）。
- **実行の失敗は `Stage::Run` の診断**: 検査段階ではないが、CLI と JSON で同じ形にするため。GM-E11001〜GM-E11006、GM-I11001（terminate）、GM-W11001（止めない警告）、GM-U0004 / GM-U0005（実行系が未対応 / 内部エラー）。
- **MSL の全例題は別プロセスのワーカーで回す**: Rumoca の時間切れは協調型で、初期化と積分器の 1 歩の内側では止まらない。親がモデルごとの時間（120 s）を超えたらワーカーを止めて作り直す。協調型の予算は 30 s（Rumoca のハーネスは 12 s）。
- **参照結果は MAP-LIB のブランチ v4.1.0 を固定したコミット（`dac67829…`）から、要るモデルの分だけ取る**: 全部だと約 2.7 GB。リポジトリに LICENSE が無いので、取得して試験に使うだけにした（コミットしない）。
- **比較は csv-compare の矩形の管（許容差 0.002）を書き直す**: Rumoca の比較（平均の誤差）は管よりずっと緩い。試験側に無い信号と、参照の区間を覆わない結果は不合格にした（csv-compare は警告で飛ばす）。
- **partial の例題のテンプレート 12 本は実行しない**（Rumoca のハーネスも除く）。
- **ソルバ**: `bdf`（既定）・`dopri45`（Rumoca の Dormand–Prince 5(4)）・`auto`。`dassl`・`lsodar` → `bdf`、`rk45`・`dopri5` → `dopri45` は別名。表にあって未実装のもの（`esdirk34`・`tsit45`・固定刻みなど）は GM-E10005。
- **experiment の StopTime = StartTime は開始 + 1 まで回す**（警告 GM-W10001。Rumoca と同じ）。指定した値が同じなら誤り。
- **並列の分け方**: 調べもの 2 つ（Rumoca の実行系、参照結果と比較の方法）を先に並べ、その結果で骨格（core のコード、ir の型、frontend の `CompiledModel`、exec と sim の空の関数、依存と patch、検証用のパッケージ `Simulation`）を固めてコミットした（`7ab4fb0`、`092b888`）。そのあと所有するファイルを分けた 3 つを同時に走らせた。
  - A: `globe-mode-exec`（Rumoca の実行系のアダプタ、失敗の振り分け、プリフライト、検証用の `Simulation.*`）
  - B: `globe-mode-sim`（ソルバの登録表、設定の決定、CSV、管の比較）
  - C: xtask の `fetch-msl-ref` と `msl-sim`（ワーカー、親の監視、報告）
  - 統合（db の `compiled_model`、CLI の `globe simulate`、MSL の全例題の実行、正典の文書）は私がやった。B・A・C には途中で追加の作業を頼んだ（覆う範囲の検査と StopTime = StartTime、失敗の型での振り分けと止めない警告と `SimReport::settings`、partial を除く）。
  - スライドと解説書は、数字がそろってから文書のエージェントに頼んだ。私の書いた文書の誤り（増えた package の数 102 → 101、diffsol の修正が 4 件、ADR の表の GM-W11001、README の「管の幅 0.2%」→ 許容差 0.2% など 7 か所）も直してもらった。

## やったこと

- 骨格（私）: workspace に `rumoca-sim`（`solver-diffsol` と `solver-rk45`）と diffsol の patch、後で `rumoca-solver`（失敗の型を名前で振り分けるため。推移的依存で外部の crate は増えない）。新しい crate `globe-mode-exec` と `globe-mode-sim`。core に `Stage::Run` と GM-W10002・GM-E10005・GM-I10003・GM-E11001〜GM-E11006・GM-I11001・GM-W11001・GM-U0004・GM-U0005。ir に `sim.rs`（`SimSettings`・`ResolvedSettings`・`Trajectory`・`SimRun` など）。frontend に `CompiledModel` と `Checker::compile_retaining`。deny.toml（`rumoca-sim` の wrappers、instant の勧告）、`.gitignore`（`data/msl-ref/`）、ci_plan（wasm に sim、`src/Modelica` が exec のテストに効く）、mise の `msl:sim`。
- A（`globe-mode-exec/src/{lib.rs,rumoca.rs}`、`tests/simulate.rs`、`Simulation/*.mo` 12 本）: bdf は Rumoca の型付きの道（build → 初期化の確認 → 実行）、dopri45 は `rk45::simulate`、auto は Rumoca の選び方をなぞる。失敗は段・種類・呼んだ位置の順に振り分け、`MeSessionError` などは型で match。プリフライトは `eval_dae_at`。止めない警告は GM-W11001。`SimReport::settings`。テスト 13 + 19 本。
- B（`globe-mode-sim/src/*`）: 登録表（`SOLVERS`）、設定の決定と検査、CSV の読み書き（往復で値が変わらない）、管（セグメント木で窓の最大・最小を取る。1 信号 O(n + m log n)）、覆う範囲の検査、`comparisonSignals.txt` の読み取り。テスト 49 本。
- C（xtask の `reference.rs`・`sim.rs`・`main.rs`）: `fetch-msl-ref`（固定したコミット、`.part` から rename、404 も記録）、`msl-sim`（JSON 行のワーカー、親の監視、partial を除く、報告と JSON）。テスト 37 本。
- 統合（私）: db の `GlobeDb::compiled_model`（テスト 3 本）、CLI の `globe simulate`（検査を通してから実行、`--force`、`--output`、`--json`、失敗しても使った設定を表示）。`comparisonSignals.txt` の `//` で始まる行（比べないことにした信号）を読み飛ばすように直した（最初の実行で 3 本が「参照に無い信号」で落ちた）。
- 文書: ADR-0013（新規）、architecture・features・licenses・roadmap・README・CLAUDE.md、research（rumoca.md・openmodelica-dymola.md）、`src/Modelica/README.md`、`doc/msl-sim.md`（生成）、スライドと解説書。

## 検証

- コードの検査: `mise run check` 通過（`cargo fmt --all --check` 差分 0、`cargo clippy --workspace --all-targets -- -D warnings` 0 件、`cargo test --workspace` 422 本通過・7 本 ignore（S010 は 324 本・6 本）、cargo deny は advisories・bans・licenses・sources とも ok、`uv sync --locked` ok）。最初の 1 回は fmt の差分（db の新しいテスト）で落ち、`cargo fmt --all` で直した。`cargo check --target wasm32-unknown-unknown`（core・ir・frontend・graph・db・analysis・sim）通過。`cargo check -p globe-mode-exec --no-default-features` 通過。
- テストの内訳（新しいもの）: exec 32 本（単体 13・結合 19。MSL の ChuaCircuit の 1 本は ignore で、A が 1 回回して成功: コンパイル 7.2 s・準備 0.25 s・初期化と積分 4.35 s、debug）、sim 49 本、xtask 37 本（うち新しい 14 本）、db の `compiled_model` 3 本。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。frontend に `compile_retaining` を足したので回した。
- `msl:funnel` は回し直していない: S0〜S10 の判定の規則は変えていない（frontend は関数を足しただけ）。`msl-sim` は S010 の `target/msl-funnel.json`（OK 369 本）を入力にした。
- `cargo xtask fetch-msl-ref`（release、4 並列）: OK の 369 本のうち参照結果のある 353 本の分、1010 MB を 66 s で取得。参照結果の無いもの 16 本（partial の 12 本と `BusUsage_Utilities.Part`・`AdvancedSolenoid`・`SpacePhasorLimiter`・`TwoPhaseWater.BaseProperties`）。
- `cargo xtask msl-sim`（release、壁時計の予算 30 s、外から止めるのは 120 s、50 本ごとに `Checker::reset`）: 357 本（partial 12 本は実行しない）。**終了時刻まで 303**・実行 NG 47（`GM-U0004` 26・`GM-E11001` 6・`GM-E11002` 6・`GM-E11005` 5・`GM-U0005` 4）・外から停止 7・ワーカーの異常終了 0・panic 0。`GM-W10002` は 3 本。**参照結果と比べた 302 本のうち管に収まった 292 本（96.7%）**、外れ 10、参照結果なし 1。信号の数では 1713 本中 1687 本。全体 1911.7 s（31 分 52 秒）、ワーカーの起動 8 回（MSL の読み込みは合計 18.9 s）、ワーカーの最大の常駐メモリ 3.5 GiB。
  - 1 回目（パーサを直す前）は 302 本・288 / 301 本（95.7%）。3 本（`HoldWithDAeffects1`・`HoldWithDAeffects2`・`IMS_Start`）が `comparisonSignals.txt` の `//` の行を信号と読んで落ちた。`CauerLowPassSC` は 1 回目が 33 s で時間切れ、2 回目は通った（予算 30 s のぎりぎり）。
  - Rumoca の基準値（rev `276e82bd` の json: 試した 286 本で simulate 216、予算 12 s）とは対象と予算が違う。
- `globe simulate` を手で確かめた: `Simulation.BouncingBall` は 3 回跳ねて h(2) = 0.26074（CSV 510 行）、`Simulation.RuntimeAssert` は `GM-E11004`（t = 0.5）、`--solver dassl` で `GM-I10003`、`--solver esdirk34` で `GM-E10005`、`Simulation.InitNoSolution` は `GM-E11001` と使った設定の行。
- `check_docs.py`: デッキ 17 件すべて OK（S011 は 17 枚）/ 文書 89 件・相対リンク 583 件中の切れ 0 件。`render_slides.py`（Playwright）: デッキ 17 件 / 問題 0 件。S011 のスライドは手元で最小の余白 122.7 px（S07。CI のフォントの伸び約 92 px に対して）
- セッション記録の `--check`: 検査 4 件 / 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（50 ファイル）

## 残したこと

- **bdf を wasm で動かす**（P3 の残りの完了条件）: `globe-mode-exec` は native だけ。Rumoca の BDF の道は `Instant::now()` を呼び、faer の乱数で getrandom の backend の指定も要る。
- **初期化のヤコビ行列の階数**（S10）: Rumoca に公開の API が無い。数値的に特異な `Invalid.UngroundedLoop` は、実行すると誤りにならず値 0 のまま終了時刻まで進む（docs のエージェントが見つけ、私も確かめた）。
- **自前の積分器**（`esdirk34`・`tsit45`・固定刻み）: 登録表では「未実装」（GM-E10005）。Rumoca の `MeIntegratorBackend` として差し込めるか試す。
- **止まった 54 本と管から外れた 10 本の調査**: 一覧は `doc/msl-sim.md`。外から止めた 7 本は、初期化が協調型の予算の外なのが主な見込み【未確認】。
- **dopri45 の準備の時間**: `rk45::simulate` は組み立てから積分までを 1 回で呼ぶので、`build_seconds` が 0 になる。
- **プリフライトは lowering を 1 回余計にする**（`eval_dae_at` が自分で lowering する）。MSL の大きなモデルでは無視できない見込み。
- **Rumoca の文言から読んでいるもの**: 公開の型や accessor が無い 4 つ（Evaluation と SolveIr の文言の時刻、有限でない値の変数名、文言の中の位置、rk45 の lowering の `[EX…]` の印）。Rumoca の rev を上げたら `globe-mode-exec` のテストで確かめる。
- **`msl:funnel` は回し直していない**（S0〜S10 の規則は変えていない）。次に Rumoca の rev か検査を変えたら、funnel → sim の順に回す。
- 上流への報告の候補に、外の driver から回す API（`SolveMeKernel`）が非公開であること、diffsol の修正の上流への取り込み、初期化のヤコビ行列の階数の API、wasm で `Instant::now()` を足した（roadmap の S012 の候補 9。報告はユーザーが出す）。
