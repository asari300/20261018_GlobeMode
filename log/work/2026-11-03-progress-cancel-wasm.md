# S133 `progress-cancel-wasm`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s133-progress-cancel-wasm` |
| worktree | `.git-worktree/s133-progress-cancel-wasm` |
| Phase | P7（[ADR-0033](../../doc/adr/0033-progress-and-cancel.md) の実装の順の 2） |
| セッション文書 | `doc/sessions/S133-progress-cancel-wasm/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0033 の順の 2: wasm と Svelte の UI。wasm に進捗のコールバック、Worker は進捗を `postMessage` で親へ、`GlobeWorker` に `onProgress` と `cancel()`（`terminate()` と作り直し）。Svelte の `App.svelte` に進捗の表示と「止める」ボタン。`performance.now()` の費用を Decay と ChuaCircuit で測る。LSP・Desktop・C/NIF、exec と facade の中身は触らない。新しい依存は足さない。

## 決めたこと

- **口は新しい `simulate_observed` でなく、既存の `simulate_columns(request_json, on_progress?)` に省略できる引数を足した。** ADR-0033 の (3) の表の wasm の形（`simulate_columns(request, onProgress)`）どおりで、UI が使うのは列の道だけなので口を 1 つ増やさずに済む。省略すると前と同じ `run_columns`（facade の `simulate_with` と同じ道）を通るので、`run`・`wasm:compare` の道は変わらない。
- JS の関数は js-sys を足さずに `#[wasm_bindgen] extern` で `Function.prototype.call` を宣言して呼ぶ（`ProgressFn`）。関数が投げた例外は無視する（Rust の状態を壊さない）。
- 進捗の形は Python（S129）と同じ: `stage` は小文字、`time` は積分の t で無ければ `undefined`、`start_time`・`stop_time`。
- 止めた呼び出しは `cancelled: true` の Error で reject する。UI はそれを失敗でなく「止めた」（`data-run="cancelled"`）として出す。途中の列は返らない（ADR のとおり。Worker を捨てるため）。
- `cancel()` は実行中の呼び出しがあるときだけ効く（暇なときに押しても Worker を作り直さない）。作り直しのあとはライブラリとファイルを既存の再生（`#start`）で送り直す。
- Tauri の `TauriBackend` は `cancel` を持たないので、「止める」ボタンは出さない（進捗の引数も無視する）。

## やったこと

- `src/Rust/mod/globe-mode-wasm/src/lib.rs`: `run_columns_observed(request, SimulateHandle)`（`run_columns` はその包み）、JS 向けの `simulate_columns(request_json, on_progress?)`、`extern` の `ProgressFn`。
- `src/Rust/mod/globe-mode-wasm/web/worker.js`・`client.js`: 進捗の `postMessage` と `onProgress`、`GlobeWorker.cancel()`。
- `src/Svelte/src/lib/backend.ts`・`worker-backend.ts`・`App.svelte`: `Progress` 型、`simulateColumns(req, onProgress?)`・`cancel?()`、進捗の 1 行（`role="status"`、クラス `run`。`p.note` と分けた。試験の `p.note` の locator とぶつかったため）と「止める」。
- 試験: `tests/decay.rs`、`node/smoke.mjs`（進捗と `performance.now()` の費用）、`web/worker-node.mjs`（Worker 越しの進捗と `cancel()`。panic の作り直しの回数は 1 から 2 に）、`src/Svelte/tests/test_ui.py`（Long を「止める」）。
- 文書: ADR-0033 の「2 段目の結果」、機能表の wasm の欄（穴 2 から 1）、features.md、roadmap、索引、スライドと解説書。

## 検証

- `cargo test -p globe-mode-wasm --test decay observed`: 1 passed。
- `mise run wasm:node`: ok（Decay の進捗 6 回、lowering → preflight → build → initialization → integration、最後の t = 1、列は同じ）。
- `performance.now()` の費用: Decay（esdirk34）55 回 / 実行、ChuaCircuit（bdf）12,074 回 / 実行。JS から 1 回 60〜160 ns。ChuaCircuit の実行の中央値 363.1 ms に対して約 0.7〜1.9 ms（0.5% 未満）。定数の `now` との差は Decay +2.1 ms、ChuaCircuit −40.7 ms で、揺れ（load average 10〜20）に埋もれて見分けられない。
- `node web/worker-node.mjs`: ok（進捗 6 回、Long を t = 25.0 で止めて作り直し、続けて Decay、panic の作り直し restarts = 2）。
- `mise run wasm:compare`: 400 件中 387 一致、既知の差 13、一致しない 0（S123 と同じ）。
- `pnpm check`: 0 errors。`test_ui.py`: ok（「実行中: integration、t = 54.86 / 1000（5%）」から 41 ms で「止めた」、ボタンが戻り、続く検査も通る。MSL・木・smoke も通る）。
- コードの検査（fmt / clippy / test / deny）: `mise run check` exit 0（共有 target の古いビルドで 1 回 E0599、touch して回し直して通った）
- `check_docs.py`: デッキ全部 OK / リンク 2763 件中の切れ 0 件
- `render_slides.py`: S133 のデッキ 1 件 / 問題 0 件（2 回は load の高い中で screenshot が時間切れ。Chromium を片付けて load 4.5 で通った）
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- ADR-0033 の順の 4（C・NIF。S131）と 5（LSP・Desktop）。`TauriBackend` は `cancel` を持たないので Desktop では「止める」が出ない。
- wasm で止めたときの途中の列は返らない。MSL を読んだあとに止めると、作り直しで MSL を読み直す（数秒。次の呼び出しはそれを待つ）。
- 依頼の「`Workspace.simulate_observed(model, settings_json, on_progress)`」は、ADR-0033 の (3) の表の形（`simulate_columns(request, onProgress)`）に合わせ、既存の `simulate_columns` に省略できる引数を足す形にした。
