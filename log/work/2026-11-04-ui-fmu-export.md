# S144 `ui-fmu-export`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s144-ui-fmu-export` |
| worktree | `.git-worktree/s144-ui-fmu-export` |
| Phase | P4/P5 |
| セッション文書 | `doc/sessions/S144-ui-fmu-export/` |
| セッション記録 | （親が写す） |

## 依頼

S140 の点検の順位 4「FMU export を UI から」。CLI と各言語 API にだけあった。Desktop は LSP の `globe/exportFmu`、ブラウザ・Android は wasm32 で作れるかを先に確かめ、結論を ADR-0031 に 1 段落。

## 決めたこと

- ブラウザで出来る。`cargo check --target wasm32-unknown-unknown -p globe-mode-fmi --features export` が通った（2 分 45 秒、初回）。ファイルに触るのは `write_fmu` と `cc` だけで、`export_me_sources` は文字列を返す。
- 依頼は「`globe-mode-fmi` の中身は触らない」だったが、zip をメモリ上で組む関数が無く、Rumoca のライセンス文（`RUMOCA_LICENSE`）が `export.rs` の非公開の定数なので、写すと表示がずれる。`export.rs` に `fmu_bytes` を 1 つ足した（S137 は `lib.rs` と `tests/parse.rs` を触っていて、`export.rs` とは重ならない）。
- 層は facade の `Workspace::export_fmu_bytes(model, fmi_version)`（`export_fmu` と DAE への変換の部分を `compile_for_fmu` で共有）→ wasm の `Workspace.export_fmu`（`Uint8Array`）→ `worker.js` の op `export_fmu` → `client.js` の `exportFmu` → `WorkerBackend.exportFmu`。
- lsp と wasm の Cargo.toml が facade の feature `fmu` を有効にする。新しい外部の依存は無い（`zip` は既にある）。`ci_plan` は fmi の変更で lsp と wasm も回す（`test_ci_plan.py` の期待を足した）。
- Desktop は `dialog.save` で選んだパスのフォルダを `outDir` にする（依頼どおり）。facade は `<モデル名の . を _ にしたもの>.fmu` と展開した同名のフォルダを書くので、選んだファイル名は使わない（shortcut として解説書に書いた）。
- UI は FMI 2.0 に固定（ボタン 1 つの依頼）。`--build`・FMU import は持ち越し。

## やったこと

- `globe-mode-fmi`: `export::fmu_bytes`。`globe-mode`: `export_fmu_bytes`・`compile_for_fmu`。`globe-mode-lsp`: `globe/exportFmu`（表と試験 `export_fmu_writes_the_source_fmu`）。`globe-mode-wasm`: `export_fmu`、`worker.js`・`client.js`、試験 `decay_exports_a_source_fmu_as_bytes`。
- `src/Svelte`: `Backend.exportFmu`、`worker-backend.ts`・`tauri-backend.ts`、`browser-file.ts`（`browserSave` がバイト列も受ける）、`App.svelte` のボタン 1 つ（「保存」の並びの後ろ）、`test_ui.py` に 1 本。
- `doc/features.md`・`doc/roadmap.md`・ADR-0031 の追記・セッション文書・索引。

## 検証

- `cargo test -p globe-mode-lsp -p globe-mode-wasm`: lsp 20 通過、wasm の `decay` 13 通過・1 無視。
- `mise run ui:check`（wasm:build、`pnpm check`: 263 ファイル 0 error 0 warning、`pnpm build`、`test_ui.py`、`test_graph.py`）: ok。`test_ui.py` に「FMU: Mine.fmu がダウンロードされ、modelDescription.xml と sources/ がある」。
- `mise run wasm:node`: ok（Finished in 29.73s）。
- `python3 .github/scripts/test_ci_plan.py`: 25 通過。
- release の wasm（`globe_mode_wasm_bg.wasm`）は 18,575,997 バイト。FMU 前の値は同じ条件で測っていない（10 月 31 日のビルドは 14,657,824 バイトだが、その後の変更も入る）ので、増え分は未測定。
- desktop.yml: push した feature ブランチで `gh workflow run desktop.yml --ref feature/s144-ui-fmu-export`、run 41699371219 成功（コミット a65c4bd。以降の変更は `cargo fmt` だけ）。
- `mise run check`: 647 通過・0 失敗（ci_plan の試験 25、fmt・clippy・test・deny・`uv sync --locked`）。最初の 1 回は `cargo fmt --check` が lsp の試験と wasm の試験の改行で落ちたので `cargo fmt --all` で直して回し直した。
- `check_docs.py`: スライド OK、リンクの切れは作業記録の追加後に 0 件。`render_slides.py`: 手元では全デッキも S144 だけも 2 回とも Page.screenshot が 30 s で時間切れ。手元では通らず、CI の docs で確認。
- セッション記録の `--check` と漏洩検査: セッション記録は親が写す。ステージした差分の漏洩検査（メール・ホームのパス・トークン）は 0 件。

## 残したこと

- `--build`（cc / zig）の UI、FMI 3.0 の選択、FMU import。
- Desktop で選んだファイル名の反映（params を `path` にして一時フォルダで組む）。
- wasm のサイズの増え分の切り分け、Android の WebView でのダウンロードの実機確認。
