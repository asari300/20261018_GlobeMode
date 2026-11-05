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

## 追記（S117、2026-11-03）: `modelDescription.xml` の型と書く側の共有

- **P5a は Rumoca のテンプレートが描く XML をそのまま FMU に入れる（置き換えない）**。`description`・`unit`・`SourceFiles`・`UnitDefinitions` などテンプレートだけが持つものを落とさないため。型 `ModelDescription` と読み（`parse_model_description`）・書き（`write_model_description_2`、FMI 2.0 のみ）は `globe-mode-fmi` の既定の feature が持ち、P5b（`globe-mode-fmu`）と共有する。Decay の 2.0 の XML は読み→書き→読みで同じ `ModelDescription` になる（`globe-mode-cli` の `tests/fmu_export.rs`）。3.0 の書く側は持ち越し。

## 追記（S124、2026-11-03）: FMI 3.0 の書く側

- **`write_model_description_3` を `globe-mode-fmi` に足した**。2.0 と同じ型・同じ範囲（Real・Integer・Boolean、3.0 の要素名は `Float64`・`Int32`・`Boolean`）で、変数は型の要素に属性をまとめ、`ModelStructure` は `Output`・`ContinuousStateDerivative`・`InitialUnknown` の `valueReference` で書く。3.0 には `numberOfEventIndicators` が無いので、読む側は `<EventIndicator>` の数を `number_of_event_indicators` に入れ、書く側は 1 つ以上なら `Err`（valueReference の欄を足すのは P5b が 3.0 を出すとき）。Reference-FMUs の Dahlquist 3.0 と P5a の Decay 3.0 で読み→書き→読みが同じ `ModelDescription` になる。P5a は引き続き Rumoca のテンプレートの XML を置き換えず、`globe-mode-fmu` は 2.0 のまま（[S124 の解説書](../sessions/S124-fmi3-writer/解説書.md)）。

## 追記（S137、2026-11-03）: `EventIndicator` の valueReference

- `globe-mode-fmi` の `ModelStructure` に `event_indicators`（3.0 の `EventIndicator` の valueReference）を足した。読む側は 3.0 で `<EventIndicator>` の valueReference を入れ（2.0 は空のまま）、`write_model_description_3` はそれを書く（S124 の「イベント指標が 1 つ以上なら `Err`」を外し、数が `number_of_event_indicators` と違うときだけ `Err`）。2.0 の読み書きと P5a（Rumoca のテンプレートの XML をそのまま使う）は変わらない。P5b（`globe-mode-fmu`）が 3.0 の ME を出すときに使う（[ADR-0032](0032-p5b-own-runtime.md) の追記）。
