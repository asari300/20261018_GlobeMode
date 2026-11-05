# S142 `ui-sim-settings`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s142-ui-sim-settings` |
| worktree | `.git-worktree/s142-ui-sim-settings` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S142-ui-sim-settings/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S140 の点検の順位 2「実行の設定の UI」。`simulate()` は `{model}` だけを渡していた。ソルバ・終了時刻・許容誤差・出力の間隔・パラメータの上書きを UI から決め、experiment annotation の値を出どころつきで初期値に出す。S141 がボタンの行を触るので、`App.svelte` は `simulate` の周りと部品の差し込みだけ。

## 決めたこと

- 「LSP と wasm は変えずに済む見込み」は半分外れた。ソルバの一覧は wasm に `solvers` が既にあったが sidecar に口が無く、experiment の値は検査の結果のどこにも無かった（S10 の `ExperimentVerdict` は判定だけ）。最小の口を 2 つ足した: LSP の `globe/solvers`、`CheckResponse.experiment`（facade の `check` が `check.preflight.experiment` から写す。annotation が無ければ項目ごと出ない）。
- 値の置き場を `CheckResponse` にしたのは、`PreflightReport` や `ExperimentVerdict` に値を足すと salsa のクエリ（Hash / Eq）と S10 の診断に触れるため。`Bits` のままでは JSON が u64 になるので、protocol で f64 に直した `ExperimentValues` を持つ。
- 触った欄だけを送る。experiment の値で埋めた欄を送ると出どころが「指定」になる。
- 数に読めない欄・`名前=値` でない行は実行を止めて理由を出す（黙って捨てると別の設定で走る）。
- 試験が最後の値を読めるよう、実行のあと `final`（先頭 8 本）を 1 行出す。`Plot` には触らない。

## やったこと

- `globe-mode-protocol`: `ExperimentValues`、`CheckResponse.experiment`（`skip_serializing_if`）。`globe-mode`: `check` で写し、`ExperimentValues` を再 export。
- `globe-mode-lsp`: `globe/solvers`、試験 `solvers_and_experiment`。`globe-mode-cli` の snapshot `check-rc.json` に `experiment`（RC の `StopTime = 1`）が 1 項目増えた。
- `src/Svelte`: `SimSettings.svelte`（新）、`Backend.solvers()`（`worker-backend.ts`・`tauri-backend.ts`）、`protocol.ts`、`App.svelte`（部品の差し込み、`simulate()` が `settings` を渡す、`check()` が `experiment` を覚える）。
- `tests/test_ui.py` に 1 本。`doc/features.md`・`doc/roadmap.md`・セッション文書・索引。

## 検証

- `cargo test -p globe-mode-protocol -p globe-mode-lsp`: 2 + 19 通過（`solvers_and_experiment` を含む）。
- `cargo test -p globe-mode-cli`: 通過（`UPDATE_SNAPSHOTS=1` で `check-rc.json` だけ更新）。
- `cargo test -p globe-mode-wasm`: 12 通過・1 無視。`mise run wasm:build`: 通過（共有 target が古く、最初の build は `experiment` の入らない wasm のままだった。protocol・facade・wasm の `lib.rs` を `touch` して作り直した）。
- `pnpm check`: 262 ファイル 0 error 0 warning。
- `test_ui.py`: ok。Decay を終了時刻 2・dopri45 で x(2) = 0.135335（e⁻² = 0.135335）、DecayK は許容誤差の欄が `1e-8`（出どころ experiment）、終了時刻 1・`k = 2` で x(1) = 0.135335。実行のあとの行は「開始 0、終了 1（指定）、許容誤差 1e-8（experiment）、出力の間隔 0.002（既定）、ソルバ dopri45（指定）」。`k 2` の行は実行せず「名前=値」の理由を出した。既存の項目（10 万点 203 ms、止める 30 ms、ChuaCircuit、F12、smoke x1=0.36788）も通った。
- `mise run check`: 645 通過・0 失敗（fmt・clippy・test・deny・`uv sync --locked`）。
- `check_docs.py`: 全デッキ OK / 相対リンク 2904 件中の切れ 0 件。
- `render_slides.py`: 手元では 2 回とも Page.screenshot が 30 s で時間切れ。手元では通らず、CI の docs で確認。
- セッション記録の `--check` と漏洩検査: セッション記録は親が写す。ステージした差分の漏洩検査（メール・ホームのパス・トークン）は 0 件。

## 残したこと

- 開始時刻・固定刻みの幅・壁時計の予算の欄。
- tunable なパラメータの一覧（`TunableParameter`）から選ぶ。今は名前を手で打ち、tunable でなければ GM-E10006。
- 設定のモデルごとの保存、ファイルの開く/保存（S141）、ダークモード。
