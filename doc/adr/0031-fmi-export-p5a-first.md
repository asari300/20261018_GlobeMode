# ADR-0031 FMI Export は P5a（Rumoca のテンプレート）を先行し、P5b（自前のランタイム）を並行して進める

- 状態: **Accepted**（2026-11-02、S094。ユーザーの判断 2026-11-02）
- 関係: [ADR-0002](0002-rumoca-adapter.md)（Rumoca はアダプタからだけ使う）、[ADR-0026](0026-fmi-import-placement.md)（Import の置き方。Proposed のまま、この ADR では触らない）、[architecture.md §3.4](../architecture.md#34-fmi-20--30) の「Export」
- 調査: [research/fmi-2026-11-01.md](../research/fmi-2026-11-01.md)、S094 の [作業記録](../../log/work/2026-11-02-s094-fmi-export-rumoca.md)

## 背景

- P5 の完了条件の 1 つは「Export した FMU が FMPy・fmusim・fmuCheck を通る」（[roadmap.md](../roadmap.md)）。Export は P5a（Rumoca のテンプレート）と P5b（自前のランタイム）の 2 段で計画していた。
- S094 で Rumoca の rev（`276e82bd`、v0.10.2）を読んだ。`rumoca-phase-codegen` に組み込みのターゲット `fmi2`・`fmi3`（ME と CS のソースの FMU。`modelDescription.xml` と `sources/model.c` ほか C 7 ファイル）があり、`fmi2` の README は「公式の XSD・FMPy・FMI-VDM・C の ABI の試験で確かめた」と書く。入口は umbrella crate `rumoca` の `target_manifest.rs` だが、使う部品（`TargetBundle`・`validate_*_target_capabilities`・`SolveTemplateRenderer::new_owned_with_fmi`、`rumoca_phase_solve::fmi::lower_to_fmi_component`）はすべて `rumoca-compile` と `rumoca-phase-solve` の公開 API にある。
- Rumoca の `fmi2` は事象（イベント）・外部関数・表・乱数・クロックを扱わない（manifest の `[capabilities]`）。P5b はそれを補い、コンパイラの要らない FMU（ブラウザ・Android から出せる）を目指す。

## 決定

- **P5a を先に進める**。Rumoca の組み込みターゲット `fmi2` のテンプレートを改変せずに使い、FMI 2.0 のソースの FMU（ME と CS）を出す。
  - Rumoca を呼ぶのは `globe-mode-frontend` の `fmi_export.rs`（feature `rumoca` の中。ADR-0002 の「アダプタからだけ」を守る）。Rumoca の CLI と同じ順（DAE と Solve の能力の検査 → FMI の部品 → C の射影 → テンプレートを描く）。
  - FMU の形にする（展開したディレクトリと zip、`cc` で `binaries/linux64/<modelIdentifier>.so`）のは `globe-mode-fmi` の feature `export`（`export_me_sources`・`write_fmu`）。CLI は `globe fmu export`。
  - 版は FMI 2.0 にした。fmuCheck が 2.0 までしか検査できず、Reference-FMUs にも 2.0 がある。3.0 は同じ道で `"fmi3"` を渡せば出る（試していない）。
  - FMI のヘッダは同梱しない。`--build` には `--fmi-headers <DIR>`（例: FMPy の `c-code`）を渡す。
- **P5b を並行して進める**（WP S095、新 crate `globe-mode-fmu`）。P5a の FMU を P5b の振る舞いの参照に使う。
- Import の A/B（ADR-0026）はこの ADR では決めない。

## 結果

- Decay の FMU を FMPy 0.3.32 が `validate`（問題なし）と ME・CS の `simulate` で回し、x(1) は ME（相対許容誤差 1e-8）で 0.36787942、CS で 0.36787944（e⁻¹ = 0.36787944）。試験は `globe-mode-cli/tests/fmu_export.rs`（cc・uv・FMPy が無ければ飛ばす）。
- 限界（P5a のまま残るもの）: 事象のあるモデル・外部関数・表は Rumoca が断る（`Err` で返る）。入力（`input`）の start を種にしない（Rumoca の `host_driven_input_seeds` は crate の外から呼べない）。guid は名前から作る（Rumoca の CLI は毎回 UUID v4）。生成物の C は Rumoca のテンプレート（Apache-2.0）の文面を多く含む。FMU を配るときの表示の要否は確かめていない【未確認】。
- fmusim・fmuCheck での検査は持ち越し（S094 では fmusim のバイナリを取得したが、手元で実行する許可が無かった）。

## 追記（S098、2026-11-02）: FMI 3.0・fmusim・ライセンスの表示

- **FMI 3.0 も出す**。`globe fmu export --fmi-version 3`（既定 2）で Rumoca の `fmi3` を描く。`--build` は `fmi3Functions.h` で `binaries/x86_64-linux/` に置き、`sources/buildDescription.xml` の `PreprocessorDefinition`（Rumoca の `fmi3` は `FMI3_OVERRIDE_FUNCTION_PREFIX=1`）を `-D` で渡す。渡さないと関数が `<modelIdentifier>_fmi3*` の名前で出て、FMPy が `fmi3ActivateModelPartition` を引けない。
- Decay の x(1) − e⁻¹（FMPy 0.3.32、相対許容誤差 1e-8）: 2.0・3.0 とも ME −2.6e-8、CS 3.3e-16。S094 の「ME 2.6e-9」は 0.3678794151 − 0.3678794412 の桁の書き誤りで、正しくは 2.6e-8。
- **fmusim は GitHub Actions で回す**（手動の `fmu-export.yml`。fmusim v0.15.0 の release の zip、sha256 固定）。run 41445403219 で 2.0・3.0 とも `fmusim validate` が通り、`fmusim simulate`（既定の設定）で ME −1.6e-4、CS 3.3e-16。fmuCheck は持ち越し。
- **ライセンスの表示**: FMU に `documentation/licenses/Rumoca-LICENSE.txt`（Rumoca の著作権表示と Apache-2.0 の全文）を同梱する（[licenses.md](../licenses.md) の「生成コード」）。上の「表示の要否【未確認】」はこれで閉じる。

## 追記（S105、2026-11-03）: Windows・macOS 向けのビルド

- **`--build --target <triple>`（何度でも）で `zig cc -target <triple>` を呼ぶ**。省略すると今までどおり host（x86_64 の Linux）を `cc` で作る。zig は mise の道具で、新しい依存は無い。置き場は FMI 2.0 が `linux64`・`win64`・`darwin64`（`darwin64` は arch を区別しない）、3.0 が `<arch>-linux`・`<arch>-windows`・`<arch>-darwin`。1 つの FMU に複数の置き場を同梱する。zig の Windows 向けが置く `.pdb` と import library は消す。
- 検査は `fmu-export.yml` のジョブ `fmu-cross`。Ubuntu で `--target x86_64-linux-gnu --target x86_64-windows-gnu --target aarch64-macos` の FMU を作り、ubuntu-24.04・windows-latest・macos-latest（arm64）の FMPy 0.3.32 で validate と ME・CS を回す。run 41531879481 で 3 つとも 2.0・3.0 の validate が通り、x(1) − e⁻¹ は ME −2.608e-8、CS 3.331e-16（[S105 の解説書](../sessions/S105-fmu-export-cross/解説書.md)）。Windows・macOS の fmusim は回していない。

## 追記（S234、2026-11-08）: Windows・macOS 向けの FMU と fmusim

- **P5a の `--target` は `x86_64-macos` も受ける**（`platform()` は元から arch を見ていた）。zig が書いた Mach-O と PE の先頭の magic を `globe-mode-cli` の試験が確かめる。FMI 2.0 の `darwin64` は arch を区別しないので、`x86_64-macos` と `aarch64-macos` を両方付けると後のものが前を上書きしていた。`write_fmu` が置き場の名前の重なりを先に断る。3.0 は `x86_64-darwin` と `aarch64-darwin` に分かれるので両方入れられる。
- **fmusim を 3 OS で回す**: `fmu-cross` が fmusim v0.15.0（Linux・Windows は x86_64、macOS は aarch64-darwin。x86_64-darwin の release は無い）で validate と ME・CS を回す。macOS の x86_64 向けの FMU は Mach-O の magic の確認までで、実行はしない（ランナーが arm64 で、Rosetta 上の fmusim も無い）。
- **P5b は cross しない**: `globe-mode-fmu` の cdylib は rumoca と exec を静的に含むので zig で組まず、ランナーの OS でネイティブにビルドする（`fmu-native`）。`tests/fmpy.rs` と `fmpy_run.py` は cdylib の名前（`.so`・`.dll`・`.dylib`）と `binaries/` の置き場を OS で決める。
- **fmusim の macOS(arm64) 版は FMI 2.0 の `darwin64` を見つけない**: v0.15.0 は 2.0 の FMU でも `binaries/aarch64-darwin/` を探して `dlopen failed` で落ちる（P5a・P5b とも、run 42471079551 のログ）。Linux・Windows の fmusim は 2.0 の `linux64`・`win64` を見つける。FMI 2.0 の規格の名前は `darwin64` なので FMU は直さず、macOS・FMI 2.0 の fmusim の simulate だけ飛ばす（validate は通る。FMPy の ME・CS は通る）。上流への報告は未提出。
- 結果（`fmu-export.yml`、run 42472525126 success）: 3 OS とも FMPy の ME・CS が x(1) − e⁻¹ = −2.608e-08（ME）・3.331e-16（CS）、fmusim が −2.282e-08（ME）・3.331e-16（CS）。P5b の FMPy CS と fmusim CS は 3.886e-15、fmusim ME は −2.282e-08。`tests/fmpy.rs` の 16 件は Ubuntu・Windows・macOS で通った。
- 道具: github.com の release の直リンクが 504 を返し続けた（mise の `packslip.sigstore.json` など。2026-11-08）。`fmu-cross`・`fmu-native` は mise を使わずランナーの rustup と `pipx install uv` にし、fmusim は API 経由の `gh release download` で取る。
- 1 つの FMU に P5b の複数 OS の置き場を同梱する道は持ち越し（各 OS のビルドを 1 つの zip に集める手順が要る）。

## 追記（S117、2026-11-03）: `modelDescription.xml` の型と書く側の共有

- **P5a は Rumoca のテンプレートが描く XML をそのまま FMU に入れる（置き換えない）**。`description`・`unit`・`SourceFiles`・`UnitDefinitions` などテンプレートだけが持つものを落とさないため。型 `ModelDescription` と読み（`parse_model_description`）・書き（`write_model_description_2`、FMI 2.0 のみ）は `globe-mode-fmi` の既定の feature が持ち、P5b（`globe-mode-fmu`）と共有する。Decay の 2.0 の XML は読み→書き→読みで同じ `ModelDescription` になる（`globe-mode-cli` の `tests/fmu_export.rs`）。3.0 の書く側は持ち越し。

## 追記（S124、2026-11-03）: FMI 3.0 の書く側

- **`write_model_description_3` を `globe-mode-fmi` に足した**。2.0 と同じ型・同じ範囲（Real・Integer・Boolean、3.0 の要素名は `Float64`・`Int32`・`Boolean`）で、変数は型の要素に属性をまとめ、`ModelStructure` は `Output`・`ContinuousStateDerivative`・`InitialUnknown` の `valueReference` で書く。3.0 には `numberOfEventIndicators` が無いので、読む側は `<EventIndicator>` の数を `number_of_event_indicators` に入れ、書く側は 1 つ以上なら `Err`（valueReference の欄を足すのは P5b が 3.0 を出すとき）。Reference-FMUs の Dahlquist 3.0 と P5a の Decay 3.0 で読み→書き→読みが同じ `ModelDescription` になる。P5a は引き続き Rumoca のテンプレートの XML を置き換えず、`globe-mode-fmu` は 2.0 のまま（[S124 の解説書](../sessions/S124-fmi3-writer/解説書.md)）。

## 追記（S144、2026-11-04）: ブラウザで FMU を書き出す

- **ブラウザ・Android でも出来る**（ソースの FMU だけ）。Rumoca のテンプレートを描く `export_me_sources` は文字列の組を返すだけで、`globe-mode-fmi` の `export` feature（frontend と `zip`）は `cargo check --target wasm32-unknown-unknown` が通った。ファイルに触る `write_fmu`・`cc` は使わず、zip をメモリ上で組む `fmu_bytes`（中身は `write_fmu` の `build` 無しと同じ）を足し、facade の `Workspace::export_fmu_bytes` と wasm の `Workspace.export_fmu`（`Uint8Array`）で返す。UI は Blob にしてダウンロードする。`--build`（`cc` / `zig cc`）はブラウザでは出来ない（プロセスもファイルも無い）ので UI に持たない。Desktop は LSP の `globe/exportFmu` が facade の `export_fmu` で `outDir` に書く。代償として lsp と wasm が facade の feature `fmu`（frontend のテンプレートと `zip`）を引き、wasm が大きくなる【増え分は未測定。release で 18.6 MB】。

## 追記（S137、2026-11-03）: `EventIndicator` の valueReference

- `globe-mode-fmi` の `ModelStructure` に `event_indicators`（3.0 の `EventIndicator` の valueReference）を足した。読む側は 3.0 で `<EventIndicator>` の valueReference を入れ（2.0 は空のまま）、`write_model_description_3` はそれを書く（S124 の「イベント指標が 1 つ以上なら `Err`」を外し、数が `number_of_event_indicators` と違うときだけ `Err`）。2.0 の読み書きと P5a（Rumoca のテンプレートの XML をそのまま使う）は変わらない。P5b（`globe-mode-fmu`）が 3.0 の ME を出すときに使う（[ADR-0032](0032-p5b-own-runtime.md) の追記）。

## 追記（S231、2026-11-08）: P5b の Co-Simulation

- **P5b（`globe-mode-fmu`）が Co-Simulation を出す**。`fmi2Instantiate(CoSimulation)`・`fmi3InstantiateCoSimulation` の instance は `fmi2DoStep`・`fmi3DoStep` で進む。中は古典 RK4（上限 1e-3 の刻み）と、ME と同じ事象処理（領域が変わった状態事象は二分法で詰め、時刻事象は刻みを区切って `enter_event_mode` → `update_discrete_states`）。入力は刻みの間 0 次ホールド。
- **積分器は `globe-mode-fmi-import` の `ode.rs` を再利用できなかった**。`ode.rs` は `pub(crate)` で `Error` が Import の型であり、何より Import の crate は共有ライブラリの読み込み（`libloading`）・zip・tempfile を引くので、FMU の cdylib が依存すると本末転倒（Export のランタイムが Import の読み込みを背負う）。`globe-mode-exec` には `ModelInstance` を 1 歩ずつ進める道が無い（`run` は Rumoca の積分器で全区間を回す）。そのため RK4 を 20 行ほど crate の中に置いた（shortcut。誤差の制御なし）。`ode.rs` を `globe-mode-fmi` のような低い crate に移して共有する案は Import の内部の変更になるので、重なりが増えたら別のセッションで。
- **CS の入口**: 対応は `fmi2DoStep`・`fmi3DoStep`・`fmi3EnterStepMode`（何もしない）。未対応（`Error`）は `fmi2SetRealInputDerivatives`・`fmi2GetRealOutputDerivatives`・`fmi2CancelStep`・`fmi2GetStatus` / `GetRealStatus` / `GetIntegerStatus` / `GetBooleanStatus` / `GetStringStatus`（Discard を返さないので不要）・`fmi3GetOutputDerivatives`。3.0 の `eventHandlingNeeded`・`earlyReturn` はいつも偽、`lastSuccessfulTime` は到達した時刻、`terminateSimulation` はモデルの `terminate()` のとき真。2.0 は `terminate()` を伝える道（`fmi2Discard` + `fmi2GetBooleanStatus`）を持たないので `Error`。`currentCommunicationPoint` が今の時刻と違う（巻き戻し）も `Error`。
- **`modelDescription.xml`**: `model_description_kind(model, source, v3, Kind::{Me, Cs, Both})`。CS を含めると `<CoSimulation modelIdentifier="globe_mode_fmu" canHandleVariableCommunicationStepSize="true"/>`（書く側は P5b だけなので、`CoSimulation` の要素には常に付ける）。`model_description` と `model_description_3` は `Kind::Me` で、出力は 1 バイトも変わらない。**CLI `globe fmu export` は P5a だけで（P5b を作る道は CLI に無く、cdylib と `examples/md.rs` で組む）、P5a は ME と CS を元から 1 つの FMU に書くので、`--kind` は足さなかった**。Python・UI の選択肢も足していない。

## 追記（S237、2026-11-08）: MSL を使うモデル

- **測った**: `doc/msl-sim.md` で実行できる例題から、compile の短い順に 20 本（Clocked は除き、パッケージごとに 2 本まで。PID_Controller・ChuaCircuit・Rotational.First・TwoMasses を先頭に）を `globe fmu export -L data/msl --build`（P5a）と P5b で FMI 2.0 に書き出し、FMPy 0.3.32 の validate と ME・CS を `globe simulate` の結果と矩形の管（`compare.rs` と同じ幅。FMPy の ME は CVode で事象の時刻が数 ms ずれるので、管の外が 1% 以下の点までは許す）で比べた（`.github/scripts/fmu_msl_check.py`）。
  - **P5a は 3/20**（`Translational.Examples.Accelerate`・`SignConvention`、`Rotational.Examples.CompareBrakingTorque`。FMI 3.0 でも同じ 3 本）。落ちた 17 本は Rumoca が書き出しを断る（事象・関係式の根・時刻事象 6、初期化 6、Clock 3）か、FMU の `fmi2ExitInitializationMode` が実行時に失敗する（`TwoTanks`）か、カオスのため比べられない（ChuaCircuit）。原文は FMI C profile の `Err`（`rumoca-ir-solve/src/fmi/`）。**Rumoca の側の限界なので GlobeMode では直さず**、ふつうのモデルが当たる初期化の 1 件を [上流の下書き 35](../research/upstream/35-fmi-c-profile-initial-equation-on-state.md) に貯めた。
  - **P5b は 16/20**（ME・CS とも通る）。P5b は 1 ファイルの Modelica しか読まず MSL を読めなかった（`Checker::new(&[])`）。
- **直したもの（GlobeMode 側）**:
  1. `resources/lib/` に `*.mo` を入れると P5b がそれを `MODELICAPATH` として読む（`compile_in`、`model_description_lib`、`examples/md.rs` の 5 番目の引数）。`resources/<完全修飾名>.mo` はそのモデルの文面で、モデルがライブラリの中ならコメントだけでよい。MSL の `.mo` を全部入れる（`Resources/` は除く。zip で約 5 MB、**shortcut**: 使うクラスの閉包に絞らない）。CLI で P5b を作る道はまだ無い（cdylib と `md` と zip で組む）。
  2. 関係式の記憶を初期化の点から決める（`ModelInstance::new`、Rumoca の FMI ME の `exit_initialization_mode` と同じ順）。無いと `abs(w) >= w0` のような noEvent でない関係式が最初の事象まで false の枝で微分を返した（`CompareBrakingTorque` の `SignTorque` は w が指数で減った）。
  3. 初期化の事象（`apply_projected_initial_event_boundary`）を回す。S099 以来の shortcut を外した。StateGraph の `FirstExample` が 1 s ずれて始まっていた。`EnumString` の出力 `level` は最初から `speed` と同じ値になる（試験の期待を 1 から 2 に直した）。
  4. 事象で決まった離散の値（`y_last` など solver の y の状態でない欄）を保つ（`held_y`）。状態から y を作り直すと初期値に戻っていた。
  5. `modelDescription.xml` の内部の名前 `$formal_derivative.1.x` を引用符で囲む（structured の文法に合わず FMPy の validate が断った）。
- **P5b で残る 4 本**: `ChuaCircuit`（カオス。FMPy の ME は 3700 s まで管に収まり、以降は積分器の差が増幅される。P5a も同じ）、`DemonstrateSignalExtrema`（`SignalExtrema` の最小・最大は積分器の刻みの点の値で、刻みの違いで 0.2〜0.4% ずれる）、`RectifierBridge2Pulse.DiodeBridge2Pulse`・`RectifierCenterTap2Pulse.DiodeCenterTap2Pulse`（状態事象の多い回路の中の `Math.Mean`（`when sample`）の出力 `y` が事象の間で保たれない。原因は未調査）。
- **試験**: `globe-mode-cli/tests/fmu_export.rs` に MSL ありのときだけ回る 2 本（`fmpy_simulates_an_msl_example`＝P5a の Accelerate を FMPy の ME・CS で、`unsupported_msl_example_says_why`＝事象のある例題が FMI C profile の原文つきで断られる）。`globe-mode-fmu/tests/fmpy.rs` に `fmpy_runs_an_msl_example_through_resources_lib`（P5b の CompareBrakingTorque、2.0 の ME と 3.0 の CS）と、`globe-mode-fmu` の単体の `relation_memory_starts_from_the_initial_point`。`fmu-export.yml` の step で MSL の 3 本（Accelerate・SignConvention・CompareBrakingTorque）を P5a の FMI 2.0・3.0 と P5b の FMI 2.0 で、Accelerate を P5b の FMI 3.0 でも、FMPy の ME・CS と fmusim で回す（`fmu-export.yml` run 42494360236 が success。fmusim の `mass.s(1)` は P5a が ME 1.0000407・CS 1.0、P5b が ME 1.0000005・CS 1.0）。
- 詳細: [S237 の解説書](../sessions/S237-fmu-export-msl-models/解説書.md)。

## 追記（S238、2026-11-09）: P5b を作る CLI と、使うクラスのファイルだけの同梱

- **`globe fmu export --runtime p5a|p5b`**（既定 p5a）。`p5b` は `<名前>-p5b-fmi2.fmu`（3.0 は `-fmi3`）と展開したフォルダ `<名前>-p5b/` を `--out` に書く。ME と CS を 1 つの FMU に書く（`Kind::Both`）。`--build`・`--target`・`--fmi-headers` は p5a のもので、p5b に付けると終了コード 2。`--kind` は足していない（S231 のまま）。
- **共有ライブラリ（cdylib）の入手は `--runtime-lib <path>` か、無ければ `cargo build --release -p globe-mode-fmu --lib` をその場で呼ぶ**（`--message-format json` から cdylib のパスを読む）。作れない（cargo が無い、globe のソースの作業ツリーの外）ときは理由と `--runtime-lib` の案内を出して終了コード 2。`--build` が `cc`・`zig` を呼ぶのと同じ扱いで、host の OS 用だけ（`binaries/linux64`・`win64`・`darwin64`、3.0 は `<arch>-<os>`。置き場の名前は P5a の `platform` を共有）。
  - 採らなかった案: (a) `globe` に `globe-mode-fmu` を静的に組み込み実行時に取り出す。cdylib は `globe` と同じ crate 群をリンクし直す別の成果物で、組み込むと循環し、`globe` が cdylib の大きさ（約 44 MB、strip 前）だけ増える。(c) release の配布物に同梱。release の組み方（`release.yml`）を変える割に、手元の `globe` ビルドでは使えない。
- **`modelDescription.xml` は FMU に入れる `resources/lib` から作る**（`globe_mode_fmu::model_description_lib`、S237）。出力した FMU が instantiate でき、足りないクラスがあればここで断る（`<モデル>: FMU にできない（…）`）。壊れた作りかけは消す。展開先の守りは `check_out_root`（S212、#23）を P5a と共有。
- **使うファイルだけを `resources/lib` に入れる**: `globe-mode-frontend` の `library_closure`（S037 の分割読み込みの閉包。モデルと利用者のファイルの本文から名前をたどり、外側の `package.mo` と、クラスの見つからないファイルを含む）が選ぶ。`globe deps`（`deps_closure`）は使わなかった: ワークスペースのファイルの単純名の一致で広めに拾い、MSL では 1070 / 2556 ファイル（11 MB）になる。分割読み込みの閉包は 293 / 2556。
- **測定**（`Modelica.Mechanics.Translational.Examples.Accelerate`、FMI 2.0、release）:

  | | S237（MSL の `.mo` を全部） | S238（使うファイルだけ） |
  | --- | ---: | ---: |
  | `resources/lib` のファイル | 2556 | 293（CompareBrakingTorque は 296） |
  | `resources/lib` の大きさ（未圧縮 / zip の中） | 15.3 MB / 3.9 MB | 3.3 MB / 0.53 MB |
  | `globe fmu run`（instantiate + CS を 1 s） | 8.1 s | 1.2 s |
  | `globe fmu export` | （CLI が無かった） | 初回 約 1 分（MSL 全体を読む `-L` の解析）。2 回目から 2 s（Rumoca のキャッシュ） |

  FMU 全体は約 14.9 MB で、大半は strip していない release の cdylib（約 14 MB）。数十〜数百 KB の目標には届かない: 閉包の単位が**ファイル**で、MSL の `Rotational/Components.mo` のように 1 ファイルに多くのクラスを持つので、使わないクラスも入る。クラス単位に刻むのは持ち越し。
- **MSL の 20 本を `--runtime p5b` で FMPy の ME・CS にかけ直した結果は 16/20**（S237 と同じ。落ちる 4 本も同じ: ChuaCircuit、DemonstrateSignalExtrema、DiodeBridge2Pulse、DiodeCenterTap2Pulse）。絞り込みで足りないクラスが出た例題は無い。
- `examples/md.rs` は CLI に置き換えて消した。`fmu-export.yml` の P5b の step と `.github/scripts/fmu_msl_check.py` の `--p5b` は CLI を呼ぶ。
- **shortcut**: P5b のモデルは 1 つの `.mo` か `-L` のライブラリの中のクラス（実行時が `resources/` の `.mo` を 1 つだけ読むため。複数ファイルのパッケージを渡すと終了コード 2）。host の OS 以外の P5b の cross は無い（各 OS でその OS の cdylib を作る。S234）。MSL の `LICENSE` は FMU に同梱していない。Python（`export_fmu` の `runtime`）と UI の選択肢は足していない。
- 詳細: [S238 の解説書](../sessions/S238-fmu-export-p5b-cli/解説書.md)。

## 追記（S239、2026-11-09）: P5b を CLI 以外の面へ

- **本体を facade へ**: `fmu_p5b.rs` の本体を `globe-mode`（feature `fmu-p5b`）の `export_fmu_p5b` に移し、CLI はそれを呼んで終了コードにするだけ（写しは無い）。`FmuOptions` に `runtime: FmuRuntime`（`p5a` 既定 / `p5b`）と `runtime_lib: Option<PathBuf>`（JSON の鍵も同じ。C ABI・NIF・Python が共有）。`Workspace::export_fmu` は `runtime = p5b` なら、モデルを定義するワークスペースのファイル（無ければ開いたライブラリの中のクラス）と開いたライブラリ（`Workspace` が `with_libraries` に渡されたパスを持つ）から作る。誤りは `P5bError::{Usage, Failed}`（CLI は 2 と 1）。feature が無い版（wasm）は理由つきの `Err`。
- **Python**: `export_fmu(..., runtime="p5b", runtime_lib=None)`。cdylib を作れなければ `ExportError`。**LSP**: `globe/exportFmu` に `runtime`（p5a・p5b。ほかは -32602）と `runtimeLib`。**UI**: Desktop だけ「実行時」の select。ブラウザ・Android は cdylib を作れないので出さない。Desktop の sidecar（`globe lsp`）は cargo を呼ぶので、globe のソースの作業ツリーの外では理由つきで失敗する（配布物への cdylib の同梱は持ち越し）。
- **strip**: zip に入れる cdylib の写しを strip する（Linux `strip`、macOS `strip -x`、Windows は無し。`strip` が無ければそのまま）。`runtime_lib` で渡されたファイルは触らない。測定は解説書（FMU 全体 14.3 → 12.9 MB）。
- **doctest を回さない crate が増えた**: `globe-mode-fmu` は cdylib 兼 rlib で、その rlib は hash 無しの名前になる。`globe-mode-fmu` 自身の試験（`tests/fmpy.rs`）と CLI の試験が入れ子で `cargo build -p globe-mode-fmu` を回すと、ワークスペース全体のビルドの rlib が（feature の組が違うので）上書きされる。facade が feature `fmu-p5b` で rlib として依存し、ワークスペース全体で有効になったため、あとの doctest で rustdoc が facade を読むときに E0460 / E0463 になった（`release` や `cargo rustc --crate-type cdylib` に変えても rlib が書き換わった）。そこで facade・`globe-mode-c`・`-lsp`・`-py`・`-wasm` に `doctest = false`（`globe-mode-nif` と同じ）。facade の本文の例（Decay の検査と実行）は `tests/` が通している。cdylib を rlib の依存から外せたら戻す（持ち越し）。
- **LICENSE**: `-L` のルートの `LICENSE`（MSL の BSD-3-Clause）を `resources/lib/LICENSE` に写す。
- 詳細: [S239 の解説書](../sessions/S239-fmu-export-p5b-surfaces/解説書.md)。

## 追記（S240、2026-11-09）: 共有ライブラリを別 crate に（S239 の持ち越しを閉じる）

- `globe-mode-fmu` は rlib だけにし、共有ライブラリは新しい crate `globe-mode-fmu-cdylib`（`crate-type = ["cdylib"]`、`pub use globe_mode_fmu::*;` の 1 行）が作る。`extern "C"` の入口は `Instance` の内部と密なので `globe-mode-fmu` に残し、上流の `#[no_mangle]` として共有ライブラリから出る（`nm -D` で 119 個）。
- 入れ子の `cargo build`（`tests/fmpy.rs`、CLI の試験、facade の `build_runtime`、`fmu-export.yml`）は `-p globe-mode-fmu-cdylib` を指す。共有ライブラリの名前は `libglobe_mode_fmu_cdylib.so` など。FMU の中は `globe_mode_fmu.<拡張子>` のまま。
- facade が依存するのは rlib だけなので、入れ子のビルドに書き換えられず、S239 で置いた `doctest = false`（facade・`globe-mode-c`・`-lsp`・`-py`・`-wasm`）を外した。`globe-mode-nif` は元から false。
## 追記（S243、2026-11-09）: P5b の FMU を小さくする（S238・S239 の持ち越し）

- **cdylib のプロファイル**: workspace の `[profile.release]` は変えず、facade の `build_runtime`（`globe fmu export --runtime p5b` が cargo を呼ぶ道）が `CARGO_PROFILE_RELEASE_OPT_LEVEL=z`・`LTO=fat`・`CODEGEN_UNITS=1`・`PANIC=abort`・`STRIP=true` を環境変数で与える（mise の `wasm:build` と同じ作り。利用者が同じ変数を決めていればそれを尊重する）。`fmu-export.yml` が cdylib を直接作る step も同じ変数を与える。**`panic = "abort"` は今の挙動と矛盾しない**: `extern "C"` の境界を panic が越えると Rust 2024 では abort で、`globe-mode-fmu` は `catch_unwind` を使っていない（#40 の議論）。abort にすると unwind の表（`.eh_frame` など）が要らなくなるだけ。
- **cdylib の大きさ**（Linux x86_64、release、`strip` 後。1 施策ずつ足した）:

  | 施策 | 大きさ | 前の行から |
  | --- | ---: | ---: |
  | 既定の release（opt-level 3、cgu 16、LTO 無し、unwind） | 33.2 MB | |
  | LTO fat + cgu 1（opt-level 3 のまま） | 25.4 MB | -7.8 MB |
  | opt-level s だけ（LTO 無し） | 27.6 MB | （参考） |
  | opt-level s + LTO fat + cgu 1 | 19.1 MB | |
  | + panic = abort | 17.1 MB | -2.0 MB |
  | opt-level z（LTO fat + cgu 1 + abort） | 15.1 MB | -2.0 MB |

  FMU 全体（zip）は 12.9 → 6.3 MB。`Accelerate` の `globe fmu run`（instantiate + CS を 1 s）は 0.29 s → 0.24 s で、小さくして遅くなる向きの影響は出なかった（最大 RSS 59 → 45 MB）。cdylib のビルドは初回 約 6 分（fat LTO。Rumoca を含む）。
- **残りの大きさ**: `nm -S` を crate ごとに足すと（opt-level s、symbol 付き 13.7 MB 分）Rumoca の compile 段（flatten・dae・solve・structural・parse・instantiate ほか）が約 8 MB、`core`・`alloc`・`hashbrown` が約 3 MB、`serde_json` 0.34 MB、`faer`・`nano_gemm` 0.4 MB。実行時が instantiate のたびに `resources` から compile し直すので Rumoca の compile 段は外せず、**cdylib を「数 MB」にするのは Rumoca を使う限り届かない**（FMU の zip 全体で 6.3 MB が今の値）。`cargo-bloat` は入れていない。外せそうな `globe-mode-exec` の積分器（`rumoca-solver-diffsol`・`-rk45`）は feature が `rumoca-exec` 1 つにまとまっており、切り出しても 1 MB 未満なので持ち越し。Rumoca は改変していない。
- **`resources/lib` をクラス単位に**: `library_closure`（ファイルの閉包）のあと、`globe-mode-fmt::prune_classes`（`lex` と `class_spans` だけ。整形はしない）が、使わない入れ子のクラスを落とす。規則は字句だけで、広めに残す: トップレベルのクラスは残す。入れ子のクラス `X` は、親が残り、かつ識別子 `X` が利用者の本文かモデルの名前か残るクラスの本文に出るときだけ残す（`extends` の鎖は、増えなくなるまで繰り返してたどる）。`operator` の付いたクラスの子は全部残す（`'+'` などは名前なしで使われる）。字句に分けられない・文を切り出せないファイルはそのまま残す。落としたクラスだけから参照されていたファイルも要らなくなるので、`globe-mode-frontend::library_slice` が閉包を取り直す（変わらなくなるまで）。`within` と `package` の入れ子はそのまま残る。名前の一致だけを見るので、同じ名前の別のクラスや変数があっても残す（shortcut。スコープまでたどると、足りなくなる危険のほうが大きい）。
- **測定**（`Modelica.Mechanics.Translational.Examples.Accelerate`、FMI 2.0）:

  | | S238（ファイルの閉包） | S243（クラス単位） |
  | --- | ---: | ---: |
  | `resources/lib` のファイル | 294 | 90 |
  | 大きさ（未圧縮 / zip の中） | 2.37 MB / 590 KB | 393 KB / 131 KB |
  | `globe fmu run`（同じ cdylib で、instantiate + CS を 1 s） | 1.07 s | 0.29 s |
  | 最大 RSS | 216 MB | 59 MB |

  20 本のうち `resources/lib` は 76〜122 ファイル、387〜490 KB（`Thermal.FluidHeatFlow` の 2 本だけ `Media` のデータで 1.6 MB）。
- **MSL の 20 本は 16/20**（S238・S237 と同じ。落ちる 4 本も同じ: ChuaCircuit、DemonstrateSignalExtrema、DiodeBridge2Pulse、DiodeCenterTap2Pulse）。切り出しで足りないクラスが出た例題は無い。
- 詳細: [S243 の解説書](../sessions/S243-fmu-p5b-size/解説書.md)。

## 追記（S246、2026-11-09）: P5b の時刻事象と状態事象が重なったときの離散値（S237 の持ち越し）

- **`Math.Mean` の出力が保たれなかった原因**（`DiodeBridge2Pulse`・`DiodeCenterTap2Pulse`）: `Mean` の `when sample(t0 + 1/f, 1/f) then y_last = f*pre(x); reinit(x, 0)` と、整流器の電圧の零交差（ダイオードの切り替え）が同じ時刻（0.01 の倍数）に起きる。FMPy の ME は時刻事象を回したあと、同じ時刻に状態事象でもう 1 度 `fmi2EnterEventMode` / `fmi2NewDiscreteStates` を呼ぶ。`ModelInstance::update_discrete_states` はこの 2 回目にも Rumoca の `apply_projected_event_update` へ `EventUpdateRowFilter::All` を渡していたので、時計の行（`when sample` の中）がもう一度回り、`y_last = f*pre(x)` が `reinit` の後の x（約 -5e-18）で書き直された（99.03 が -5e-16 に落ちて、次の標本まで戻らない）。
- **直し**: `SolveStopSchedule::scheduled_event_coincidence_at(t)` が「処理済み」（`Consumed`）を返すときは `UnownedOnly`（時計に属する行を回さない）にする（`globe-mode-exec/src/rumoca/instance.rs`。`ModelInstance` を使うのは P5b の FMU だけで、exec の `globe simulate` の道は変わらない）。Rumoca の FMI ME の `StateTimeCoincidence::Consumed` と同じ判断。`GlobeModeValidation.Simulation.SampleAtZeroCrossing`（`Mean` と同じ形を MSL なしで）と `tests/fmpy.rs` の `fmpy_matches_exec_on_sample_at_zero_crossing` が再現する（直す前は FMPy の ME で `yLast = -1e-20`、直した後は 0.6355、CS 0.6366、exec 0.6365、解析解 0.6366）。
- **`DemonstrateSignalExtrema` は直さない（FMU の欠陥ではない）**: `u = A(t) sin(phi)`（`phi` は `der(phi) = 2π f(t)` の状態で約 220 rad）を 0.01 s ごとに標本化した極値が、積分器の誤差で動く。t = 0.64 の `y_max` は、解析解 4.648119、exec の BDF（参照）4.633220（-0.32%）、FMPy の ME（CVode、相対許容 1e-6）4.650760（+0.06%）、P5b の FMU を FMPy の ME で相対許容 1e-10 にすると 4.648122、P5b の CS（RK4）4.648119。FMU は積分器を正しく動かせば解析解に 7 桁合い、管（0.2%）の外にいるのは参照の側（と許容 1e-6 の CVode）なので、`fmu_msl_check.py` の許容は広げず、この例題は「参照の積分誤差で比べられない」ものとして残す。S237 の見立て（刻みの違い）の言い換えで、exec と同じ刻みや事象の時刻の出力（`noEvent` の評価点）には原因が無かった。
- **MSL の 20 本は 18/20**（FMI 2.0、FMPy 0.3.32、`--runtime p5b`、ME・CS）。落ちる 2 本は ChuaCircuit（カオス）と DemonstrateSignalExtrema（上）。`fmu-export.yml` の MSL の step に `DiodeBridge2Pulse` を P5b の FMI 2.0 として 1 本足した。
- **まだ見ていない順**: 状態事象が先に見つかり、時刻事象があとから来る形（Rumoca の `Unconsumed`）は、P5b の FMU では FMPy が先に時刻事象を回す順しか見ていない。当たる例題が出たら同じ部品で足す。
- 詳細: [S246 の解説書](../sessions/S246-p5b-discrete-hold/解説書.md)。
