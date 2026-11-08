# S238 `fmu-export-p5b-cli`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s238-fmu-export-p5b-cli` |
| worktree | `.git-worktree/s238-fmu-export-p5b-cli` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S238-fmu-export-p5b-cli/` |
| セッション記録 | （親が写す） |

## 依頼

S237 の持ち越し 2 つ。P5b の FMU を CLI（`globe fmu export --runtime p5a|p5b`）で作れるようにする。MSL 全体の同梱をやめて、使うクラスのファイルだけにする。

## 決めたこと

- cdylib の入手は (b)「`--runtime-lib <path>`、無ければ `cargo build --release -p globe-mode-fmu` をその場で呼ぶ」。(a) 静的な組み込みは循環して `globe` が約 44 MB 太る、(c) 配布物への同梱は `release.yml` の変更が要る割に手元で使えない。
- 絞り込みは `globe deps`（`deps_closure`、MSL で 1070 / 2556）でなく S037 の分割読み込みの閉包（293 / 2556）。frontend に `library_closure` を公開した。
- `modelDescription.xml` は出力する `resources/lib` から作る（足りなければ export が断る）。
- ME と CS を 1 つに書くので `--kind` は足さない。Python・UI は足さない。
- モデルは 1 つの `.mo` か `-L` のクラス（shortcut）。host の OS 用だけ。

## やったこと

- `globe-mode-cli/src/fmu_p5b.rs`（新）、`main.rs`（`--runtime`・`--runtime-lib`）、`Cargo.toml`（`globe-mode-fmu`・`globe-mode-frontend`）。
- `globe-mode-frontend`: `library_closure`。`globe-mode-fmi`: `platform`・`zip_dir` を pub に。
- `globe-mode-fmu/examples/md.rs` を削除。`.github/workflows/fmu-export.yml`（MSL の step と `fmu-native`）と `.github/scripts/fmu_msl_check.py`（`--p5b CDYLIB`）を CLI に。
- 試験: `globe-mode-cli/tests/fmu_export.rs` に `p5b_runtime_exports_a_runnable_fmu`（Decay、`globe fmu run` で x(1) = e⁻¹、`--build` との組み合わせと無い `--runtime-lib` は終了コード 2）と `p5b_runtime_ships_only_the_used_library_files`（Accelerate、50〜500 ファイル、mass.s(1) = 1）。
- 文書: ADR-0031 の追記、features・roadmap・README・CHANGELOG、セッション文書。

## 検証

- 測定（Accelerate、FMI 2.0、release）: `resources/lib` 2556 → 293 ファイル、zip の中 3.9 → 0.53 MB、`globe fmu run` 8.1 → 1.2 s。目標の数十〜数百 KB には届かない（閉包の単位がファイル）。
- MSL の 20 本（P5b、FMI 2.0、FMPy 0.3.32、release の `globe`、1 本ずつ）: 16 / 20（S237 と同じ。落ちる 4 本も同じ）。
- 手元の試験: `cargo test -p globe-mode-cli --test fmu_export p5b` 2 本 ok（debug で 187 s、負荷あり）。
- コードの検査（`mise run check`）: 通った（fmt・clippy・test・deny・`fmt --check src/Modelica`・`uv sync --locked`）。途中で落ちたのは 2 件で直した: `test_ci_plan.py`（`globe-mode-fmu` に依存元 `globe-mode-cli` ができた）と `cli_flow.rs` の `help_has_examples_and_no_session_numbers`（`--runtime` の説明に S238 と書いた）。
- CI: （後で書く）
- `check_docs.py`: デッキ 全件 OK / リンク 4454 件中の切れ 0 件。`render_slides.py`: S238 のデッキ 問題 0 件
- セッション記録の `--check`: （親が写す）

## 残したこと

- 閉包をクラス単位に刻む、cdylib の strip（FMU の約 14 MB はほぼ cdylib）。
- Python（`runtime=`）・UI の選択肢。複数ファイルのモデル。MSL の LICENSE の同梱。
