# S150 `ui-tunable-params`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s150-ui-tunable-params` |
| worktree | `.git-worktree/s150-ui-tunable-params` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S150-ui-tunable-params/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

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

- 途中で前のプロセスが落ちたので、未コミットの変更を途中経過としてコミットしてから main（S147〜S149）を取り込んだ。衝突は `doc/features.md`・`doc/roadmap.md`・`doc/sessions/README.md`・`worker.js` で、どれも両方を残した（`worker.js` は `hover` と `tunable_parameters` の両方）。
- `cargo test -p globe-mode-lsp -p globe-mode-wasm -p globe-mode`: 通過（lsp 23、wasm 14 + 1 無視、facade 3・3・3 と doctest）。新しい試験は `decay_lists_tunable_parameters`・`tunable_parameters_are_listed`。
- `mise run wasm:build`・`pnpm check`（0 エラー）・`pnpm build`: 通過。
- `test_ui.py`（main の `.venv` で）: 通過。Decay は「上書きできるパラメータは無い」、DecayK は一覧に `k`（説明「減衰の速さ」、placeholder 1）、一覧の `k` に 2 で x(1) = 0.135335（e⁻² = 0.135335）、ChuaCircuit は検査 1.8 s・一覧 14 本・絞り込み「Ro.」で 4 本。前任の試験の絞り込み語 `R0` は一覧に無く 0 本だったので `Ro.` に直した。
- `mise run check`（`CARGO_BUILD_JOBS=2`）: 通過。cargo test 652 本・失敗 0、fmt・clippy・deny（advisories / bans / licenses / sources ok）・`uv sync --locked`。最初の fmt で新しい試験と wasm の関数の整形が落ちたので `cargo fmt` で直した。
- `check_docs.py`: 切れ 0 件。`render_slides.py` は S150 のデッキだけで 1 回目に通過（デスクトップ明暗・モバイルの 6 枚、問題 0 件）。
- 持ち越し・未確認: `wasm:node`・`wasm:compare` は回していない（実行系は触らず、wasm は一覧を返す関数を足しただけ。CI の `wasm-node` に任せる）。
