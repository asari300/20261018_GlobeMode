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

{VERIFY}

## 残したこと

- 閉包のクラス単位化、配布物への cdylib の同梱、複数ファイルのモデル、host 以外の OS の P5b、FMI 3.0 の SE。
