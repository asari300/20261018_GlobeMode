# S150 `ui-tunable-params`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s150-ui-tunable-params` |
| worktree | `.git-worktree/s150-ui-tunable-params` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S150-ui-tunable-params/` |
| セッション記録 | （親が写す） |

## 依頼

S142 の持ち越し。上書きできるパラメータ（`TunableParameter`）の一覧から選べるようにし、今の値・単位・説明を出す。`Backend.tunableParameters(model)` を足し、`SimSettings.svelte` が検査のあとに一覧を取って、値を打った行だけ `名前=値` と同じ形で `settings` に送る。多いときは絞り込みの欄を 1 つ。`App.svelte` は触らない依頼だった。

## 決めたこと

- facade の `Workspace::tunable_parameters`（CLI の `--list-parameters` 用）が既にあったので、Rust の新しい計算は無し。wasm と LSP は `ParametersResponse` の JSON をそのまま返す。lowering の失敗（`parameters: null`）は `Backend` で空の配列にし、DAE まで変換できない失敗は reject（wasm は例外、LSP は -32000）。
- 依頼の `value` でなく、Rust の型のまま `default`（モデルの値）。型を 2 つ持たない。
- 取るのは「検査」のあと。`SimSettings` には検査したモデル名を知る口が無く、`App.svelte` にしか合図が無いので、**`App.svelte` を 3 か所だけ触った**（`checkedModel` の宣言、`check()` の中の代入、`<SimSettings checked=…>`）。`smoke()` や `run` の周りには触れていない。依頼の「触らない」から外れた点（S147 とは行が離れており、衝突しない見込み）。
- 合図は `{ model }` の新しいオブジェクトにして、同じモデルを検査し直しても一覧を取り直す。
- 一覧は取り直しても、打った値は名前が残っていれば残す。textarea は残し、同じ名前なら textarea が優先。
- 絞り込みは 13 本以上のときだけ（名前と説明の部分一致）。min / max は表示も検査もしない（持ち越し）。

## やったこと

- `globe-mode-wasm`: `Workspace.tunable_parameters`、web の `worker.js`・`client.js`、試験 `decay_lists_tunable_parameters`。
- `globe-mode-lsp`: `globe/tunableParameters`、試験 `tunable_parameters_are_listed`。
- `src/Svelte`: `protocol.ts`（`TunableParameter`）、`backend.ts`・`worker-backend.ts`・`tauri-backend.ts`、`SimSettings.svelte`、`App.svelte`（上の 3 か所）、`tests/test_ui.py`。
- `doc/features.md`・`doc/roadmap.md`・セッション文書・索引。

## 検証

- `cargo test -p globe-mode-wasm -p globe-mode-lsp -p globe-mode`: 通過（wasm 14 + 1 無視、lsp 21、facade 3・3・3 と doctest）。新しい試験は `decay_lists_tunable_parameters`・`tunable_parameters_are_listed`。
