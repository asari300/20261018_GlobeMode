# S094 `fmi-export-rumoca`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s094-fmi-export-rumoca` |
| worktree | `.git-worktree/s094-fmi-export-rumoca` |
| Phase | P5（1 回目、P5a） |
| セッション文書 | `doc/sessions/S094-fmi-export-rumoca/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

P5 の FMI Export のうち P5a（Rumoca のテンプレート）を進める。ユーザーの判断（2026-11-02）: P5a を先行し、それを参考に P5b（自前のランタイム、S095 の `globe-mode-fmu`）を並行する。まず Rumoca の rev に FMI 向けのテンプレート・codegen が何まであるかを調べ、あればそれで Decay の `modelDescription.xml` と C を出し、無ければ Solve IR から自前の minijinja テンプレートで出す。`globe-mode-fmi` に `export_me_sources`、CLI に `globe fmu export`（`--build` で `cc`、zip）、FMPy で x(1) ≈ e⁻¹ を試験で確かめる。ADR-0031。

## 決めたこと

- **判定: Rumoca にある。自前のテンプレートは書かない**。根拠（rev `276e82bd`、`~/.cargo/git/checkouts/rumoca-*/276e82b/`）:
  - `crates/rumoca-phase-codegen/src/templates/fmi2/`・`fmi3/`（`target.toml`・`modelDescription.xml.jinja`・`model.c.jinja` ほか）。`fmi2` は「FMI 2.0.5 source-code ME+CS FMU」、README は公式の XSD・FMPy・FMI-VDM・C の ABI の試験で確かめたと書く。`fmi-ls-wasm` もある。
  - 入口は umbrella crate `rumoca` の `src/target_manifest.rs`（`validate_target_requirements` → `resolve_manifest_renderer` → `write_manifest_file`）。使う部品は `rumoca_compile::codegen::{SolveTemplateRenderer, targets::{TargetBundle, TargetTemplateSource, validate_dae_target_capabilities, validate_solve_target_capabilities}}` と `rumoca_phase_solve::fmi::lower_to_fmi_component`（`finish_fmi_component` を中で呼ぶ）、`FmiComponent::into_codegen_view().try_c()`。すべて公開 API。
  - テンプレートが見る artifact は `artifact.identities.fmu`（guid）だけ。umbrella の `ArtifactSession` は使わず、`serde_json` の値を渡した。
  - `fmi2` の `[capabilities]` は事象・外部関数・表・乱数・クロック・初期化を持たない。対象外のモデルは `Err`。
- **置き場（既定のまま進めたこと）**: 依頼は `globe-mode-fmi` に `export_me_sources` だったが、CLAUDE.md §3 と ADR-0002 が「Rumoca は frontend と exec からだけ使う」なので、Rumoca を呼ぶ部分は `globe-mode-frontend/src/fmi_export.rs`（`render_rumoca_fmi`）に置き、`globe-mode-fmi` の feature `export` がそれを呼んで FMU の形にする。frontend の依存に `rumoca-phase-solve`（workspace にあり、Cargo.lock の外部パッケージは増えない）。
- **版**: FMI 2.0（fmuCheck が 2.0 まで。Rumoca の `fmi2` は ME と CS を 1 つの FMU に入れる。依頼の「ME」はこれで満たす）。
- **`--build`**: プラットフォーム名は FMI 2.0 の `linux64`（依頼の `x86_64-linux` は FMI 3.0 の名前）。ヘッダは同梱せず `--fmi-headers <DIR>`（FMPy 0.3.32 の `fmpy/c-code` に `fmi2Functions.h` ほか）。
- **zip**: xtask と同じ `zip` 9（`deflate`）を `globe-mode-fmi` の optional な依存に。Cargo.lock の外部パッケージは 0 増。minijinja は直接使わない（Rumoca の中で使われる）。
- **guid**: モデル名のハッシュから UUID の形を作る（同じツールチェーンなら同じ FMU）。
- **入力の種**: `lower_to_fmi_component(dae, &HashMap::new())`。Rumoca の CLI は `rumoca-sim` の `host_driven_input_seeds`（crate 内の非公開）で入力の start を種にする。入力を持つモデルを出すときに直す（コードに `shortcut:`）。
- ADR-0031（Accepted）。ADR-0026 の Import の A/B はそのまま。

## やったこと

- `src/Rust/mod/globe-mode-frontend/src/fmi_export.rs`（新規）、`lib.rs`、`Cargo.toml`（`rumoca-phase-solve`）。
- `src/Rust/mod/globe-mode-fmi/src/export.rs`（新規。`MeSources`・`export_me_sources`・`write_fmu`）、`lib.rs`、`Cargo.toml`（feature `export`）。
- `src/Rust/mod/globe-mode-cli/src/main.rs`（`globe fmu export`）、`Cargo.toml`、`tests/fmu_export.rs`（新規、2 本）。
- `Cargo.toml`（workspace の依存に `globe-mode-fmi`）、`Cargo.lock`。
- `.github/scripts/test_ci_plan.py`: frontend の依存元に `globe-mode-fmi` が入ったので期待値の 4 つに足した。
- 文書: ADR-0031 と索引、roadmap（P5 の行と S094 の節）、architecture（frontend・fmi の行、§3.4 の P5a）、features（FMI 2.0 Export の行）、licenses（zip、Rumoca の生成物）。

## 検証

- Decay の FMU: 16,612 バイト。`modelDescription.xml`（fmiVersion 2.0、ScalarVariable 8、ME と CS）、`sources/` の C 7 ファイル 444 行（`model.c` 343 行）、`binaries/linux64/Rumoca_89c9c7fb_9a4c_6d51_81b7_3efcfd749be6.so`。`globe fmu export --build` は 1.6 s（debug）。
- FMPy 0.3.32（`uv run --no-project --with fmpy`）: `validate` は「No problems found.」。`simulate --stop-time 1`: ME（既定の許容誤差）x(1) = 0.3678329758（差 4.6e-5）、ME（`--relative-tolerance 1e-8`）0.3678794151（差 2.6e-9）、CS 0.3678794412（e⁻¹ = 0.3678794412、差 3e-16）。
- `cargo test -p globe-mode-cli --test fmu_export`: 2 passed（2.1 s）。
- fmusim: v0.15.0 の `fmusim-x86_64-linux.zip`（sha256 `d92bd905…e29a0`）を取得したが、外から取ったバイナリの実行が手元の許可で止められたので消した。**通っていない（持ち越し）**。fmuCheck も持ち越し。
- コードの検査（`mise run check`）: EXIT=0（2 回目）。テスト 606 passed・0 failed・11 ignored（60 バイナリ）、clippy・fmt 通過、`cargo deny` は advisories・bans・licenses・sources とも ok。1 回目は `.github/scripts/test_ci_plan.py` の期待値 4 件（frontend の依存元に fmi が入った）で落ち、直した後の 2 回目の前の回は共有 target の古いビルドで `globe-mode-fmt` の tests/format.rs が NotFound（core・fmi・fmt を touch して解消）
- `check_docs.py`: デッキ OK、文書 397 件・相対リンク 1,909 件中の切れ 0 件。`render_slides.py` は 7 枚 × 3 表示で問題 0 件（スマートフォンでコードがはみ出したので短くした）
- セッション記録の `--check`・漏洩検査: 親が行う。

## 残したこと

- fmusim・fmuCheck での検査。FMI 3.0（`"fmi3"` を渡すだけの見込み、試していない）。Windows・macOS 向けのビルド（`zig cc`）。
- 入力を持つモデル、事象のあるモデル（Rumoca の `fmi2` が断る。P5b の対象）。
- 生成物が Rumoca のテンプレート（Apache-2.0）の文面を含むことの扱い【未確認】。
- `mise run msl:compare` は回していない（Rumoca の CLI の取得と実行が要る。frontend の変更は新しいモジュールを足しただけで、検査の道は変えていない）。`wasm:node`・`wasm:compare` も手元では回さず CI のジョブ `wasm`・`wasm-node` に任せる（frontend の変更で CI の計画に入る）。
- `write_fmu` は `--out` の下の `<名前>/` が既にあれば消して書き直す。
- S095 と同時に `Cargo.toml` の workspace の依存・`test_ci_plan.py`・ADR の索引・roadmap を触るので、マージで衝突しうる。
