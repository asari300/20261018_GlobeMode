# S118 `lang-fmu-export`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s118-lang-fmu-export` |
| worktree | `.git-worktree/s118-lang-fmu-export` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S118-lang-fmu-export/` |
| セッション記録 | （親が写す） |

## 依頼

S110 の「埋める順」の 5。FMU export を facade に置き（FMI の版 2|3、`--build`・`--target` 相当）、CLI はそれを呼ぶ形に（挙動を変えない）。C ABI `gm_export_fmu`（JSON の options、結果は `{"path"}` か `{"error"}`）、NIF `export_fmu`、Python `Workspace.export_fmu`、Zig・Julia・Elixir・Gleam のラッパ。wasm は対象外。各言語の試験に Decay の FMU（2.0、build なし）で `modelDescription.xml` があることを 1 本ずつ。機能表と features を更新。進捗と中断は持ち越し。`globe-mode-fmu`・`globe-mode-fmi` の中身・`globe-mode-exec`・`src/Python/marimo`・`.github/workflows/` は触らない。

## 決めたこと

- facade の feature は既定にせず `fmu`（`sundials` と同じ任意の feature）。`globe-mode-fmi/export` と `serde` を引く。CLI・C・NIF・py が有効にし、wasm は引かない（zip を wasm に入れない、ファイルを書けない）。
- `FmuOptions { fmi_version (既定 2), build, targets, fmi_headers }` を serde で読む（省略した鍵は既定、知らない鍵は誤り）。`build = true` で `fmi_headers` が無ければ誤り。
- facade には自由関数 `export_fmu(&CompiledModel, …)` と `Workspace::export_fmu(model, …)` の 2 つ。CLI はコンパイルの診断を本文つきで出すので、コンパイルは今のまま自分で行い、書き出しだけ自由関数を呼ぶ（挙動を変えない）。`globe-mode-fmi` は CLI の dev-dependency に下げた（試験が `parse_model_description` を使う）。
- `Workspace::export_fmu` のコンパイルの誤りは、診断の文を 「; 」でつないだ文字列。
- NIF は `export_fmu/4`（ws, model, out_dir, options_json。DirtyIo）。Elixir の `GlobeMode.export_fmu(ws, model, out_dir, options \\ %{})` が `/3` になる。
- `test_ci_plan.py` に fmi の変更が C・NIF・py と lang・python に届くことを足した（facade が fmi に依存したので届く先が広がる。fmi の変更で 2 → 8 crate）。`Cargo.lock` が変わるので push は全体の検査になる。

## やったこと

- `src/Rust/mod/globe-mode/`（`Cargo.toml`・`src/lib.rs`）: feature `fmu`、`FmuOptions`・`export_fmu`・`Workspace::export_fmu`。
- `src/Rust/mod/globe-mode-cli/`（`Cargo.toml`・`src/main.rs`）: facade の `export_fmu` を呼ぶ。
- `src/Rust/mod/globe-mode-c/`（`Cargo.toml`・`src/lib.rs`・`include/globe_mode.h`）: `gm_export_fmu` と単体試験。
- `src/Rust/mod/globe-mode-nif/`、`src/Elixir/globe_mode/`、`src/Gleam/globe_mode/`: `export_fmu` と試験。
- `src/Rust/mod/globe-mode-py/`、`src/Python/mod/globe_mode/__init__.py`、`src/Python/tests/test_api.py`: `Workspace.export_fmu` と試験。
- `src/Zig/src/main.zig`、`src/Julia/globe_mode.jl`・`test.jl`: 包みと試験。
- `.github/scripts/test_ci_plan.py`、`doc/research/lang-api-matrix-2026-11-03.md`（FMU export の行）、`doc/features.md`、`doc/roadmap.md`。

## 検証

- `cargo test -p globe-mode-c -p globe-mode --features globe-mode/fmu`: 通った（C 2 本 + ヘッダ 1 本、facade 3 + 3 + 2 本、doctest）。
- `cargo test -p globe-mode-cli --test fmu_export`（facade 経由に替えた後）: 5 passed。
- clippy（`-p globe-mode -p globe-mode-c -p globe-mode-cli -p globe-mode-nif -p globe-mode-py --all-targets -D warnings` と `-p globe-mode` だけ＝feature `fmu` 無し）: 0 件。
- `mise run c:zig`: 通った（`zig-out/gm-zig-fmu/GlobeModeValidation_Simulation_Decay.fmu`、`modelDescription.xml` 2,214 バイト）。
- `mise run c:julia`: 通った（`mktempdir()` に `.fmu`）。
- `mise run elixir:test`: 1 test, 0 failures。
- `mise run gleam:test`: 1 passed。`gleam format` で試験を整形し直した。`zig fmt --check`: 通った。
- `uv sync --reinstall-package globe-mode` → `uv run pytest -q src/Python/tests`: 4 passed, 1 skipped（skip は既存）。
- `python3 .github/scripts/test_ci_plan.py`: 25 本 OK（lang の試験に fmi の変更を足した）。fmi の変更で検査する crate は main の 2 から 8 に（lang・python・wasm も回る）。
- `mise run check`（load average 4.4 のとき）: fmt・clippy（workspace）通過、`cargo test` は 63 の束で 622 passed・0 failed・11 ignored、`cargo deny check` は advisories・bans・licenses・sources ok、`uv sync --locked` 通過。
- `check_docs.py`: デッキ OK、文書 510 件・相対リンク 2,549 件・切れ 0 件。`render_slides.py`: 問題 0 件（1 回目は Chromium の screenshot が 30 s で時間切れ。古い Chromium を止めて 2 回目で通った）。
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件。

## 残したこと

- 進捗と途中で止める口（S110 の順の 6。ADR を先に）。
- LSP からの FMU export。
- `build = true`（`cc`・zig）は CLI の `tests/fmu_export.rs` だけが確かめる（各言語の試験は build なし）。
