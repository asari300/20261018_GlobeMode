# S236 `ui-results-fmu-followups`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s236-ui-results-fmu-followups` |
| worktree | `.git-worktree/s236-ui-results-fmu-followups` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S236-ui-results-fmu-followups/` |
| セッション記録 | （親が写す） |

## 依頼

S222・S229 が持ち越した UI の仕上げ 4 点: 結果のタブの再読み込みでの復元、MAT のパラメータの表、最近使ったファイルへの結果と `.fmu`、FMU の実行の設定フォーム。

## 決めたこと

- Desktop の Rust（`read_recent`・`write_recent`）は触らない。結果と FMU は sidecar が path を読むので、白リストを広げる必要が無い（任意ファイルを JS に渡す道を増やさない）。
- 結果のタブはパスだけを保存。FMU の結果は path を持たないので戻さない。
- `OpenedResults` は `parameters` を持っていなかった（`Trajectory` だけ）ので、タプルに足して返した。
- ME の積分器の選択は出さない。

## やったこと

- Rust: `globe-mode`（`OpenedResults::parameters`）、`globe-mode-lsp`（`globe/readResults` に `parameters`）、`globe-mode-wasm`（`ResultColumns::parameters`、`worker.js`）。
- Svelte: `lib/FmuForm.svelte`（新）、`App.svelte`（保存・復元・最近・パラメータの表・フォーム）、`backend.ts`・`tauri-backend.ts`・`worker-backend.ts`・`results.ts`。
- 試験: `test_ui.py`（`open_fmu_file` を書き換え、`results_followups` を足した）、`test_tauri.mjs`、`smoke.mjs`・`worker-node.mjs`、LSP の `read_results_over_rpc`。
- 文書: features・roadmap・ADR-0026・Svelte README・CHANGELOG。

## 検証

- `mise run check`（fmt・clippy・test・deny・uv sync）: 通った（deny は advisories・bans・licenses・sources とも ok）。
- `pnpm check`: 270 ファイル、エラー 0・警告 0。`node tests/test_tauri.mjs`: ok。
- `test_ui.py` 全体: 通った（`open_fmu_file` と `results_followups` を含む）。
- `globe-mode-lsp` の `read_results_over_rpc`（MAT の `parameters` が空でない、CSV は `[]`）、wasm の `smoke.mjs`（`parameters()`）と `worker-node.mjs`: 通った。
- `check_docs.py`: デッキ OK、リンク切れ 0 件。`render_slides.py`: 1 デッキ、問題 0 件。
- Desktop の Rust は触っていないので `desktop.yml` は回さない（`ci.yml` の全体検査だけ）。
- CI の run: （親への報告に書く）

## 残したこと

ME の積分器の選択、FMU の出力の選択と `set` の欄。ブラウザで結果のファイルを覚える。
