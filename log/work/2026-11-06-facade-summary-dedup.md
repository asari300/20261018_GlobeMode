# S184 `facade-summary-dedup`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s184-facade-summary-dedup` |
| worktree | `.git-worktree/s184-facade-summary-dedup` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S184-facade-summary-dedup/` |
| セッション記録 | （親が写す） |

## 依頼

S182・S183 が CLI の中だけで直した・置き場が無くて足さなかった 2 点（`--force` の診断の重複、検査結果の 1 行の要約）を、全部の面が同じものを見る facade・protocol に寄せる。UI の表示は変えない。

## 決めたこと

- 重複: facade の `simulate` の応答自体に重複があった（`Unbalanced` の強行で `GM-E5002` が 2 つ。除く処理を外して facade の試験で確認）。wasm・LSP・UI も同じ応答を見るので同じ重複を受け取っていた。Python の `simulate` に `force` は無く、強行の重複は出ない。`Over`（`GM-E8001`）は強行でコンパイルが通るので重複しない。`simulate_observed` の最後で除き、CLI の除去は消した。
- 要約: protocol に `CheckResponse::summary_line()` と `refused_summary()`。`CheckResponse.summary`（常に）、`SimulateResponse.summary`（実行しなかったときだけ）。CLI の `check` の最後の行は `ws.check(model).summary`（salsa の結果を使う）、`simulate` の最後の行は CLI の理由（設定つき）で `refused_summary` を呼ぶ。文面は変えていない。
- UI（Svelte）は変えない。

## 検証

- `cargo test -p globe-mode --test summary`（3 本）、`-p globe-mode-protocol`（キーの試験に `summary`）、`-p globe-mode-cli`（`cli_flow.rs` 7 本は無変更で通る。`--json` の snapshot 3 本は `summary` の 1 キーだけ）、`-p globe-mode-lsp`（RPC 1 本に `summary` を足した）、`-p globe-mode-wasm`（強行の重複 1 本）。
- Python: `uv sync --reinstall-package globe-mode` のあと `pytest src/Python/tests` で `test_read.py` に 1 本足した（`check(...)["summary"]`）。
- `mise run wasm:node`: ok。
- 応答を厳密に読む所: 無かった（`CheckResponse`・`SimulateResponse` は `Serialize` だけ。protocol.ts・C ABI・NIF・Zig・Julia・Elixir・Gleam・xtask は既知のキーを引くだけ）。
- `mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test`（1 本ずつ）: 通った。`mise run check`（load 5 前後で開始）: 通った（fmt・clippy・test・deny・`uv sync --locked`）。
- `check_docs.py`: 切れ 0 件（文書 749 件・相対リンク 3767 件）。`render_slides.py`（S184 のデッキだけ）: 問題 0 件。
- `wasm:compare` は実行系を変えていないので回していない。

## 残したこと

- UI（Svelte）の `unsupported` の要約と失敗の行を `summary` に置き換える。
- `globe simulate --json`（CLI 専用の形）に `summary` を足すかは未定。Python の `simulate` に `force` を通すかも別の判断。
