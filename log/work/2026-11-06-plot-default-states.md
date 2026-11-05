# S190 `plot-default-states`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s190-plot-default-states` |
| worktree | `.git-worktree/s190-plot-default-states` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S190-plot-default-states/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

実行の後のプロットが既定で描く信号を、結果の列の先頭の 3 本から「状態変数（`der()` が付く変数）を先に、足りなければ出力、最大 3 本」に変える（S185 の P4、ユーザーが案 B を選んだ）。どれが状態・出力かは Rust 側が決める。

## 決めたこと

- **種類の情報は既にあった**: 結果の変数の `role`（`VariableRole`）。ただし Rumoca の状態選択で `der()` の付く変数が `$state_coordinates[i]` に替わると、元の変数は Algebraic になる（`PID_Controller` で State は `$state_coordinates[1..6]` だけ）。役割だけの規則は、そのモデルで内部名を除くと状態が 0 本になり出力に落ちた。そこで DAE の状態（状態選択の前）の名前 `CompiledModel::state_names` を frontend に足し、規則に渡す。
- **形は「名前の一覧 `defaults`」**（`kinds` を返して UI が選ぶ案はやめた）。規則を UI と Python で二重に持たないため。`ColumnsLayout.defaults`（`#[serde(default)]`）。既存のフィールドと `SimulateResponse` は変えない（CLI の `--json`・snapshot・FMU に触らない）。
- 規則は `Trajectory::default_plot(dae_states)` の 1 関数。`Simulated::default_plot` に実行の最後に入れる（wasm は `compiled` を放すので、放す前に決める）。
- Python は native の戻り値の組を変えず、結果の JSON に `defaults` を添える。
- C ABI・NIF は layout を鍵ごとに組み立てていて自動では出ない。1 行ずつで足せるが、Julia・Zig・Gleam・Elixir の側も要るので持ち越し。
- **依頼の食い違い**: 「再実行で利用者の選択を保つ今の挙動」は元から無い（新しい結果で選択・軸・x は既定に戻る。S128 から）。保つ挙動は足さず、試験は絞り込み・内部名の切り替えで選択が保たれることにした。

## やったこと

- `globe-mode-ir/src/sim.rs`（`default_plot` と単体試験）、`globe-mode-frontend/src/rumoca.rs`（`state_names`）、`globe-mode-protocol`（`ColumnsLayout.defaults`）、`globe-mode/src/lib.rs`（`Simulated::default_plot`）、`globe-mode-lsp`、`globe-mode-py`、`globe-mode-cli`（`Simulated` の分解に `..`）。
- UI: `plot/csv.ts`（`Columns.defaults?`）、`worker-backend.ts`・`tauri-backend.ts`、`Plot.svelte`。
- 試験: LSP・wasm（`decay.rs`）・`smoke.mjs`・`test_columns.py`・`test_ui.py`（Spring）。
- 文書: features・roadmap・CHANGELOG・lang-api-matrix・セッションの索引・スライド・解説書。

## 検証

- 8 例の前後は解説書の表（LSP に `globe/simulateColumns` を送って比べた）。
- `mise run check`: 通った（fmt・clippy -D warnings・workspace の test 67 結果 ok・deny・`uv sync --locked`）。
- 単体: `default_plot`（規則の 5 つの場合 + DAE の状態で補う場合）、LSP の `globe/simulateColumns`（`defaults == ["x"]`）、wasm の `decay.rs`、`mise run wasm:node`（`defaults == ["x"]`、`ok`）、`pytest test_columns.py`（2 本）。
- `mise run ui:check`: `test_ui.py` と `test_graph.py` が通った。Spring の既定は x・v の 2 本、`e` を足した選択は絞り込みと内部名の切り替えで保たれた。S169 の既存の試験は先頭 3 本を前提にしていなかったので直していない。`GM-SMOKE` の `plot=ok` のまま（行に項目は足していない）。
- `mise run wasm:compare` は回していない（実行系の `globe-mode-exec`・`globe-mode-sim` は触っていない。数値は変わらない）。
- `check_docs.py`: デッキ 1 件 OK / リンク 3817 件中の切れ 0 件。`render_slides.py` は S190 のデッキだけ、問題 0 件。

## 残したこと

- 一覧の並びを状態を上に（持ち越し）。C ABI・NIF・他言語の `defaults`。
- `DoublePendulum` の 3 本目は `damper.phi_rel`（列の順）。振り子 2 の `revolute2.phi` にしたければ、状態の並びの規則が要る。
