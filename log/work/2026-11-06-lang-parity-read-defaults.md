# S191 `lang-parity-read-defaults`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s191-lang-parity-read-defaults` |
| worktree | `.git-worktree/s191-lang-parity-read-defaults` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S191-lang-parity-read-defaults/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

Python にだけ入った口を C ABI・NIF・Zig・Julia・Elixir・Gleam・wasm にも揃える。(1) 読み取り `outline`・`deps`・`flat`（S183）、(2) 列の応答の `defaults`（S190）を C ABI と NIF にも、(3) `summary`（S184）が各言語で出ることを試験で確かめる、(4) README と対応表の更新。

## 決めたこと

- 呼び先は facade の `Workspace::outline` / `deps` / `flat` だけ。戻り値と誤りの文面は Python（`ValueError` の文）と同じにし、C ABI は `{"error": ...}`、NIF は `{:error, 理由}`、wasm は例外（文字列）で返す。`flat` が通らないときは `null`、無いモデルの `deps` は `[]`。
- C ABI は既存の `guard`・`gm_string_free` をそのまま使った。新しい解放の関数も `unsafe` の型も足していない（`extern "C"` の署名が 3 つ増えただけ）。ヘッダは手書きに 3 行足し、`tests/header.rs` の突き合わせが通る。
- `defaults` は C ABI・NIF の layout JSON に `names`・`points` と同じ階層で足した（wasm の `layout()` は `ColumnsLayout` をそのまま直列化するので既に出ていた）。Zig・Julia・Elixir・Gleam は JSON を読む側なので関数の変更は無く、試験で確かめた。
- `summary` は直す必要が無かった。`check` の JSON には常に出て、`simulate` は失敗のときだけ出る（S184 の仕様）。
- wasm の UI（Svelte）は触らない。口だけ置いた（`Workspace`・`GlobeWorker.outline` / `deps` / `flat`）。

## やったこと

- `globe-mode-c`: `gm_outline`・`gm_deps`・`gm_flat`、`gm_simulate_columns*` の JSON に `defaults`、ヘッダ、単体試験。
- `globe-mode-nif`: `outline`・`deps`・`flat`、`simulate_columns` の layout に `defaults`。
- Elixir（`GlobeMode.outline/2` ほか 3 つ、`decode` 経由で `{:ok, map | list | nil}`）、Gleam（`@external` 3 つと `globe_mode_nif.erl` の export）、Julia（`ccall` 3 関数）、Zig（`main.zig` に読み取り・summary・defaults の確認）。
- wasm: `Workspace.outline` / `deps` / `flat`、`worker.js` の op、`client.js` の `GlobeWorker`。
- 試験: `globe-mode-c` 単体、Zig `main.zig`、Julia `test.jl`、Elixir 1 本、Gleam 1 本と既存に summary・defaults、wasm の `tests/decay.rs` 1 本、`node/smoke.mjs`（`mise run wasm:node`）と `web/worker-node.mjs` に 1 項目。
- README（C・Zig・Julia・Elixir・Gleam）、対応表、`CHANGELOG.md`、`doc/features.md`、`doc/roadmap.md`、索引。

## 検証

- `cargo test -p globe-mode-c`: 3 passed、`header_matches_extern_c_signatures` 1 passed。
- `cargo test -p globe-mode-wasm --test decay`: 21 passed。
- `mise run c:zig`: 通った（`check: passed=true, summary=…: OK（S0〜S10 を通過）`、`columns` の `defaults` が `["x"]`、`read: outline=280 bytes, deps=["Wiggle.mo"], flat={"variables":1,…}`）。
- `mise run c:julia`: 通った。
- `mise run elixir:test`: 3 tests, 0 failures。`mise run gleam:test`: 3 passed。
- `mise run wasm:node`: 通った（`outline・deps・flat・summary: 読めた`）。`node web/worker-node.mjs`: 通った（同じ行）。
- `mise run check`: 通った（fmt・clippy -D warnings・test 698 passed 0 failed・deny・`globe fmt --check src/Modelica`・`uv sync --locked`）。load は 1 未満から始めた。
- `check_docs.py`: デッキ OK / リンク 3860 件中 切れ 0 件。`render_slides.py`（S191 のデッキだけ）: 問題 0 件。
- 全体検査（`gh workflow run ci.yml --ref feature/s191-lang-parity-read-defaults -f full=true`、run 42140012073）: success（rust-linux・rust-windows・rust-sundials・deny・msl・docs・wasm-node・ui・lang・python）。
- `release.yml` の dry run（run 42140025145、feature ブランチ、タグなし）: success（4 環境の build・desktop・third-party）。Release とタグは作っていない。

## 残したこと

- UI が wasm の読み取りを使うか（今は口だけ）。
- Julia の JSON は正規表現のまま（標準ライブラリが無い）。
- Hex・JLL などへの公開はユーザーの判断待ち。
