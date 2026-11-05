# S189 `fmt-validation-fmu-name`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s189-fmt-validation-fmu-name` |
| worktree | `.git-worktree/s189-fmt-validation-fmu-name` |
| Phase | P4・P5 |
| セッション文書 | `doc/sessions/S189-fmt-validation-fmu-name/` |
| セッション記録 | （親が写す） |

## 依頼

ユーザーの決定（2026-11-06）2 点。(1) 「`src/Modelica` は `globe fmt` で整形する」。(2) 「FMU のファイル名に FMI の version をサフィックスで付ける」。S185 の監査の R10・R4 と S187 の解説書 3.2（119 ファイルの内訳）が背景。S186・S187・S188 が main に入ってから始めた。

## 決めたこと

- 整形は既定の規則（インデント 2・80 桁）で `src/Modelica` の 71 ファイルを `globe fmt`。整形器の規則は変えない。誤りになったファイルは 0 なので整形器の修正も無し。
- 意味が変わらない確認は、Python の `Workspace("data/msl")` に `src/Modelica` を読んで全クラス 85 本の `check`（診断の `(コード, 重大度)`・収支・止まった段）と、`Simulation`・`Examples` の `simulate`（`final`・`termination`）を JSON にして整形の前後を diff（0 行）。
- CI は `ci.yml` rust-linux の「サンプルの検査」と `mise run check` に `globe fmt --check src/Modelica` を 1 行ずつ（`ci_plan.py` は `src/Modelica/*` の変更で CLI の検査を起こす既存の対応に入っている。`test_ci_plan.py` は check で通った）。
- FMU の名前は `<名前>-fmi2.fmu` / `-fmi3.fmu`。名前を作るのは `globe-mode-fmi` の `MeSources::file_name` 1 か所で、`write_fmu` が使う。CLI・facade・Python・LSP・C・NIF・各言語は `write_fmu` の戻り値をそのまま使うので揃う。UI は wasm がバイト列だけ返すため TS（`worker-backend.ts`、`tauri-backend.ts`）に同じ形を書いた。
- 展開したフォルダ `<名前>/` は版を付けない（依頼は外側の `.fmu` の名前だけ。FMU を組む置き場）。利用者が出力先を明示する口（`--out`・Python の引数）は**ディレクトリ**で、ファイル名を指定する口が元から無いので「明示した名前をそのまま使う」に当たる場面は無い。
- R4 の後半は `export_me_sources` で Rumoca の原文の前に「このモデルは FMU に書き出せない（理由: …）」を付けるだけ（MSL の `First`・`PID_Controller` と検証用の `BouncingBall` で確認）。
- CLAUDE.md は触らない。足すとよい文は報告に書いた。

## やったこと

- 整形: `src/Modelica/**/*.mo` 71 本。更新した試験: CLI の snapshot 3 枚、frontend（`examples`・`loops`）、db（`switching`）、LSP（`lib.rs` の 3 か所）、wasm（`tests/decay.rs`）。
- CI: `.github/workflows/ci.yml`、`mise.toml`、`README.md`（`globe fmt --check src/Modelica`）、`src/Modelica/README.md` に 1 行。
- FMU: `globe-mode-fmi/src/export.rs`（`file_name`・日本語の前置き）、doc コメント（facade・CLI・LSP・Python・C）、試験（CLI `fmu_export.rs` に 2.0・3.0 の同居と失敗の文面、LSP、Python、C、Gleam、`test_ui.py` に 3.0 の名前）、`fmu-export.yml`、UI の 2 行（`worker-backend.ts`・`tauri-backend.ts`。`App.svelte` は触らない）。
- 文書: セッション文書、索引、roadmap・features・CHANGELOG（挙動が変わる点を明記）。

## 検証

- `globe fmt --check src/Modelica`: 71 ファイル整形済み、終了コード 0。2 回目の `fmt` は差分 0。
- 意味: 85 モデルの `check` と実行の JSON が整形の前後で一致（diff 0）。
- `mise run check`: 整形のコミット時と FMU のコミット時の 2 回、通った（fmt・clippy・test・deny・`fmt --check src/Modelica`・`uv sync --locked`）。
- `mise run wasm:node`: ok。`mise run wasm:compare`: 387 / 400 一致、既知の差 13、一致しない 0。`mise run msl:compare`: 20 / 20。
- `mise run ui:check`（`pnpm check`、`pnpm build`、`test_ui.py`、`test_graph.py`）: 通った（FMU のダウンロード名 `Mine-fmi2.fmu`・`Mine-fmi3.fmu` を含む）。
- `mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test`: 通った。Python の `uv run pytest -q`: 21 passed, 1 skipped（ネイティブを `uv sync --reinstall-package globe-mode` で作り直してから。古いネイティブだと `test_export_fmu` が旧い名前で落ちる）。
- push した feature ブランチでの CI: RUNS
- `check_docs.py`・`render_slides.py`・セッション記録の `--check`・漏洩検査: CHECKS

## 残したこと

- 展開したフォルダ `<名前>/` に版を付けるか。
- Desktop の保存ダイアログで選んだファイル名は使っていない（フォルダだけ）。
- 共有 target の古いビルドで `ColumnsLayout` の `defaults` の E0063 が出た（別の worktree のビルド。`touch` で直った）。
