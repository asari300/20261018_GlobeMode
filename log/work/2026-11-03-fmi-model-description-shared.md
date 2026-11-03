# S117 `fmi-model-description-shared`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s117-fmi-model-description-shared` |
| worktree | `.git-worktree/s117-fmi-model-description-shared` |
| Phase | P5（P5a・P5b の共通） |
| セッション文書 | `doc/sessions/S117-fmi-model-description-shared/` |
| セッション記録 | （親が写す） |

## 依頼

S099〜S114 の持ち越し「`modelDescription.xml` の生成を P5a（Rumoca のテンプレート）と P5b（`globe-mode-fmu` の文字列の連結）で共有する」を片付ける。`globe-mode-fmi` に書く側を 1 つ足し、`globe-mode-fmu` はその型を組み立てて呼ぶだけにする。P5a は置き換えず、Decay の 2.0 で読み→書き→読みの round-trip を試験 1 本で確かめる。ADR-0031・0032 に 1 段落ずつ追記。FMI 3.0 の書く側・CS・`dependencies`・Enumeration/String は持ち越し。

## 決めたこと

- 書く側は `write_model_description_2(&ModelDescription) -> Result<String, Error>`。2.0 以外、2.0 に無い causality（structuralParameter）、Real・Integer・Boolean 以外の型は `Err`（Enumeration は declaredType が要るため。`shortcut:` で印）。
- `start`・`DefaultExperiment` の値は属性の文字列のまま持つ。型によらず書き戻しが同じになり、`ModelDescription` の `Eq` も保てる。
- `ModelStructure` は 2.0 の `index` と 3.0 の `valueReference` をそのまま持つ（版で数え方が違う）。Dahlquist の 2.0 と 3.0 の変数表の一致の試験は `derivative` を外して比べるよう直した。
- 空の `Derivatives`・`InitialUnknowns` は書かない（2.0 の XSD で `Unknown` が 1 つ以上要る）。状態のあるモデルの P5b の XML は前と同じ。
- P5a は Rumoca の XML のまま（`description`・`unit`・`SourceFiles` を落とさないため）。
- round-trip は依頼の Decay（`globe-mode-cli` の `tests/fmu_export.rs`）に加え、`globe-mode-fmi` の `tests/parse.rs` でも Reference-FMUs の Dahlquist 2.0 で 1 本（cli を通さず速い）。

## やったこと

- `src/Rust/mod/globe-mode-fmi/src/lib.rs`: 型の欄（`Initial`・`DefaultExperiment`・`ModelStructure` と変数・ルートの欄）、読む側で埋める、書く側 `write_model_description_2`。
- `src/Rust/mod/globe-mode-fmi/tests/parse.rs`: Dahlquist の比較を直し、round-trip の試験を足した。
- `src/Rust/mod/globe-mode-fmu/src/lib.rs`・`Cargo.toml`: `model_description` を型の組み立てに。`globe-mode-fmi`（既定の feature）に依存。
- `src/Rust/mod/globe-mode-cli/tests/fmu_export.rs`: Decay の round-trip。
- `.github/scripts/test_ci_plan.py`: fmi の変更で fmu・cli が回ること。
- ADR-0031・0032 の追記、`doc/roadmap.md`、セッション文書、索引。

## 検証

- 対象の試験（`cargo test -p globe-mode-fmi -p globe-mode-fmu -p globe-mode-cli --test fmu_export --test parse --test fmpy`）: 20 件すべて通過。`fmu_export` 5（Decay の round-trip を含む）、`parse` 5（Dahlquist の round-trip を含む）、`fmpy` 10（FMPy 0.3.32 の validate と simulate、390.5 s）。
- `mise run check`: `test_ci_plan.py` 25 件 OK、fmt OK、clippy（`-D warnings`）OK。`cargo test --workspace` は 330 通過・6 失敗で、6 件はすべて `globe-mode-fmu` の `tests/fmpy.rs`。試験の中の `cargo build --lib` が共有 target の別 worktree の古い `globe-mode-exec`（`set_input` が無い、E0599）で落ちたもの。exec・fmi・fmu・frontend・core を `touch` して `cargo test -p globe-mode-fmu` を回し直し 10/10 通過（273.8 s）。`cargo deny check` は advisories・bans・licenses・sources とも ok、`uv sync --locked` OK。
- `check_docs.py`: デッキ OK / 文書 507 件・相対リンク 2540 件中の切れ 0 件。
- `render_slides.py`（S117 のデッキ）: 1 回目は Chromium の screenshot が 30 s で時間切れ（load average 28）、回し直して問題 0 件。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件。
- 外部の依存の追加 0（`Cargo.lock` は workspace 内の `globe-mode-fmu` → `globe-mode-fmi` の 1 行だけ）。

## 残したこと

- FMI 3.0 の書く側、CS、`Outputs` の `dependencies`、Enumeration・String。
