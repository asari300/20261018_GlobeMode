# S229 `fmu-import-facade`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s229-fmu-import-facade` |
| worktree | `.git-worktree/s229-fmu-import-facade` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S229-fmu-import-facade/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0026 (k) の次。FMI Import を CLI だけから、facade・Python・Desktop の UI へ。ブラウザ・Android は対象外（UI で「Desktop だけ」と案内）。最小で完全に。

## 決めたこと

- ワーカーの実行ファイルは呼び手が渡す。CLI・LSP は `current_exe()`、Python は `GLOBE_BIN` か PATH の `globe`（無ければ `FmuError`）。
- facade の feature は `fmu-import`。wasm が引く `globe-mode-lsp` が共有ライブラリの読み込みを背負わないよう、LSP にも同名の feature を作り CLI だけが付ける。wasm32 の `cargo check` で確かめた。
- 中断は `WorkerSpec::cancel`（`Arc<AtomicBool>`、親が 50 ms おきに確認）。`CancelToken::flag()` を exec に 1 つ足した。進捗に頼ると DoStep が戻らない FMU を止められない。
- `Kind` の JSON を小文字（`"cs"` / `"me"`）にした。外から入る request の形を `--kind` と揃えるため。ワーカーの親子は同じバイナリなので互換の問題は無い。
- `globe-mode-lsp` の単体の試験は params の誤りだけ。ワーカーが試験のバイナリになり、`fmu __worker` を引数にしたテストの絞り込みで自分を再び起動する（フォーク爆弾になる）ため。成功と fmuRun の -32000 は `globe-mode-cli/tests/lsp.rs`。
- `import_fmu_info` の誤りにはファイルのパスを前に付ける（以前は「No such file」だけだった）。`run_fmu` の誤りには付けない（ワーカーの文面そのまま）。
- UI: `fmuInfo` の警告つきで `confirm`、承知なら `fmuRun`。実験設定の入力欄と ME の積分器の選択は出さない（依頼の範囲外）。
- Python の `on_progress` と Ctrl-C は進捗が出たときだけ見る（`shortcut` のコメント）。進捗の出ない FMU は `timeout`。

## やったこと

- `globe-mode`: `fmu_import.rs`（`import_fmu_info`・`run_fmu`・`FmuRunRequest`・`FmuRun`・`FmuInfo`）、feature `fmu-import`。
- `globe-mode-fmi-import/src/worker.rs`: `WorkerSpec::cancel` と `Kind` の小文字。`globe-mode-exec`: `CancelToken::flag`。
- `globe-mode-cli/src/fmu.rs`: `info`・`run` が facade を呼ぶ（出力は同じ）。
- `globe-mode-lsp`: `globe/fmuInfo`・`globe/fmuRun`（`$/progress`・`$/cancelRequest`）。
- `globe-mode-py` と `globe_mode`: `fmu_info`・`run_fmu`・`FmuError`・`_native.pyi`。
- Desktop: `RPC_METHODS` 20 から 22。UI: `results.ts`（`isFmuFile`）・`browser-file.ts`・`backend.ts`・`tauri-backend.ts`・`App.svelte`。
- 試験: `globe-mode-cli/tests/fmu_worker.rs`・`lsp.rs`、`globe-mode-lsp` の単体、`src/Python/tests/test_fmu.py`、`test_ui.py`・`test_tauri.mjs`。
- 文書: ADR-0026 (p)、features・roadmap・README・Python の README・CHANGELOG、`fmu-import.yml` に `--test lsp`。

## 検証

- `mise run check`（fmt・clippy -D warnings・workspace の test〔80 suite ok、失敗 0〕・deny・`uv sync --locked`）: 通った。
- `cargo check -p globe-mode-wasm --target wasm32-unknown-unknown`: 通った（`fmu-import` は wasm に入らない）。
- pytest（`GLOBE_BIN` を試験の debug の `globe` に）: 49 passed, 1 skipped（既存）。`test_fmu.py` は 4 本。
- `svelte-check` 0 エラー、`test_tauri.mjs` ok、`render_slides.py` 問題 0 件、`check_docs.py` 切れ 0 件。
- `test_ui.py`: 自分の `open_fmu_file` は単独で通った（`open_results_file` と一緒に）。全体を回すと **S188 の Tab で補完を確定する試験（`lines()[2].startswith("mod")`、editor の Tab・補完）で 2 回とも落ちた**。この箇所は自分の変更の前にあり、`editor.ts` も App のキー操作も触っていない。手元の共有 target の古い wasm（ほかの worktree のビルド）を使ったための食い違いとみられるが、原因は確かめていない【未確認】。CI の `ui` ジョブでは全体が通った。
- `fmu-import.yml`（feature ブランチ、3 OS）: run 42450598977 が success（ubuntu-24.04・windows-2025・macos-latest。`fmu_import`・`fmu_worker`・`lsp` を含む）。
- `ci.yml -f full=true`（feature ブランチ）: run 42450606920 が success（rust-linux・rust-windows・python・ui〔`test_ui.py` 全体を含む。手元で落ちた S188 の試験もここでは通った〕・wasm-node・lang ほか全ジョブ）。

## 残したこと

- C ABI・NIF からの呼び出し、マスター、SE、sandbox。
- UI の実験設定の入力欄と ME の積分器の選択。
- Desktop の「止める」の e2e（実機）。LSP の `$/cancelRequest` は既存の仕組みに `globe/fmuRun` を足しただけで、旗からの kill は `fmu_worker.rs` で確かめた。
- Python の Ctrl-C は進捗の区切りのみ。
