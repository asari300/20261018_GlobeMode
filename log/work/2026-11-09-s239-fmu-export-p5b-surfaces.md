# S239 `fmu-export-p5b-surfaces`

| | |
| --- | --- |
| 日付 | 2026-11-09 |
| ブランチ | `feature/s239-fmu-export-p5b-surfaces` |
| worktree | `.git-worktree/s239-fmu-export-p5b-surfaces` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S239-fmu-export-p5b-surfaces/` |
| セッション記録 | （親が写す） |

## 依頼

S238 の持ち越し。P5b の Export を CLI 以外の面（facade・Python・LSP・Desktop の UI）へ。cdylib の strip。MSL の LICENSE の同梱。

## 決めたこと

- 本体は facade `export_fmu_p5b`（feature `fmu-p5b`）へ移し、CLI は呼ぶだけ。`FmuOptions` に `runtime`・`runtime_lib`（JSON の鍵も）。
- `Workspace::export_fmu` の p5b は、`deps_closure` が 0 個ならライブラリのクラス、1 個ならその本文、2 個以上なら断る（CLI と同じ shortcut）。ライブラリのパスは `Workspace` が持つ。
- UI は Desktop だけに「実行時」select。ブラウザ・Android は出さない。
- strip は zip に入れる写しだけ。`runtime_lib` で渡したものは触らない。
- LICENSE は `-L` のルートにあれば `resources/lib/LICENSE` に写す。

## やったこと

- `globe-mode/src/fmu_p5b.rs`（新）、`lib.rs`、`Cargo.toml`、`globe-mode-db`（`text`）。
- `globe-mode-cli`（`fmu_p5b.rs` を薄くした）、`globe-mode-lsp`（`runtime`・`runtimeLib`）、`globe-mode-py`（feature）、`src/Python`（`runtime=`・`runtime_lib=`）。
- `src/Svelte`（`backend.ts`・`tauri-backend.ts`・`App.svelte`・`test_ui.py`）。
- 試験: facade `tests/fmu_p5b.rs` 2 本、LSP `export_fmu_passes_the_runtime`、Python `test_export_fmu_p5b`、`test_ui.py` の `fmu_runtime_select`。
- 文書: ADR-0031 の追記、features・roadmap・licenses・README・Python の README・CHANGELOG、セッション文書。

## 検証

- strip（Linux x86_64、release、Decay の P5b の FMU）: 共有ライブラリ 44.5 → 33.1 MB（zip の中 14.3 → 12.8 MB）、FMU 全体 14.3 → 12.9 MB。Accelerate（`-L data/msl`）は 13.5 MB（S238 は 14.9 MB）。strip した FMU は `globe fmu run` で x(1) = 0.36788（e⁻¹）。`resources/lib/LICENSE` が入る。
- 試験: facade `tests/fmu_p5b.rs` 2 本（`runtime_lib` に試験自身の実行ファイルを渡し、strip で小さくなること・渡したファイルが残ること・`build` との併用と無い `runtime_lib` は `Err`）、LSP `export_fmu_passes_the_runtime`、Python `test_export_fmu_p5b`（48 passed）、`test_ui.py` の `fmu_runtime_select`（`mise run ui:check` が通る）。
- `mise run check`: 通った。途中で直したもの: `test_ci_plan.py`（`globe-mode-fmu` の依存元が facade とその依存元になった）、`cargo fmt`、**doctest が E0460 / E0463 で落ちた**（facade が cdylib 兼 rlib の `globe-mode-fmu` に依存したため。`globe-mode-fmu` の試験の入れ子の `cargo build` が hash 無しの rlib を上書きする。CLI の入れ子を release や `cargo rustc` に変えても、`globe-mode-fmu` 自身の `tests/fmpy.rs` が同じことをする）。facade・c・lsp・py・wasm に `doctest = false`（nif と同じ）。
- `mise run ui:check`（wasm のビルド、svelte-check、`test_ui.py`）: 通った。`wasm:node`・`wasm:compare` は回していない（exec・sim・wasm の中身は変えていない。wasm には `fmu-p5b` を付けていない）。
- CI（feature ブランチ）: `fmu-export.yml` run 42521513013 が success、`ci.yml -f full=true` run 42521523173 が success。
- `check_docs.py`: 切れ 0 件。`render_slides.py`: S239 のデッキ 問題 0 件。
- セッション記録の `--check`: （親が写す）

## 残したこと

- 閉包のクラス単位化、配布物への cdylib の同梱、複数ファイルのモデル、host 以外の OS の P5b、FMI 3.0 の SE。
