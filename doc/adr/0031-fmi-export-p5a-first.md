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
