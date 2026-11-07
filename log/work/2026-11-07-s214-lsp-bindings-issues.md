# S214 `lsp-bindings-issues`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s214-lsp-bindings-issues` |
| worktree | `.git-worktree/s214-lsp-bindings-issues` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S214-lsp-bindings-issues/` |
| セッション記録 | （親が写す） |

## 依頼

2026-11-07 の `/ponytail-audit` で立った issue のうち LSP と言語バインディングの 6 件（#25・#30・#33・#35・#37 の LSP 側・#39）を、いまの main で 1 件ずつ再現してから直す。S208 が main に入るのを待ってから worktree を切る。

## 決めたこと

- 再現は「直す前の挙動を残したまま試験だけ足した版」で確かめた（HEAD のソース + 新しい試験）。LSP は `cargo test`、NIF は debug の `.so` を `~/.cache/globe-mode-s214/` に写して `mix test`、Julia は release の `libglobe_mode_c.so` を写して `julia test.jl`。
- #33 は issue の案どおり、キューを `try_iter` で読み切って、次が同じ URI の `didChange` なら捨てる。隣り合っているときだけ捨てる（間の hover は途中の版を見る）。時間の予算は 180 s（`msl-sim` と同じ）。積分中に `$/cancelRequest` が効かない件は実行系なので触らない。
- #25 は Rumoca が panic する入力が無く再現できない。`texts.insert` を前に動かすだけ（`diagnostics` は `self.texts` を読まない）。`compile_model_strict` の `catch_unwind` は `globe-mode-frontend`（S212・S213 の領域）なので持ち越し。
- #37 の LSP 側は何もしない（`Server.ws`・`scratch` とも facade の `Workspace`）。`Fixes #37` は付けない（S212 の担当で、Python・C・NIF の側も残る）。
- #39 は依頼の文面と issue の本文が逆だった。issue の本文（VS Code が毎回送る tabSize が設定に勝つ）が実際の症状で、設定ファイルがあればそれが勝つようにした。
- #35 の試験は panic を起こす口が要るので、試験用の NIF `panic_for_test` を足した（Mutex を持ったまま panic する。Gleam の `.erl` にも宣言が要る）。

## やったこと

- `globe-mode-lsp/src/lib.rs`: `serve` のキューのまとめ、`DEFAULT_MAX_WALL_SECONDS`・`with_default_budget`、`Server::indent`、`update` の順序。試験 3 本（`queued_did_changes_are_coalesced`・`simulate_gets_a_default_wall_budget`・`config_file_at_the_root_drives_lint_and_formatting` の期待を直し、設定なしの場合を足した）。
- `globe-mode-nif/src/lib.rs`: `with` が `PoisonError::into_inner`（`Result` を返さなくなったので呼び出し側の `?` を整理）、Mutex を取る 8 つの NIF を DirtyIo、`panic_for_test`、表の更新。Elixir と Gleam の宣言。Elixir の試験 2 本。
- `src/Julia/globe_mode.jl`・`test.jl`: `Observer`、例外の退避と投げ直し、`GC.@preserve`。試験 1 本。
- 文書: Elixir・Julia・VS Code の README、architecture.md の NIF の行、roadmap・CHANGELOG・索引、スライドと解説書、レビュー履歴。

## 検証

直す前（HEAD + 新しい試験）:

- #33: 50 回の `didChange` で `publishDiagnostics` が 51 回（試験の失敗の文は「51 回検査した」）。直した後は 2 回（`didOpen` と最後の版。3 回回して 2 回ずつ）。
- #39: 設定 `indent = 4` に `indent: 2` を送ると `"\n  Real x;"`（試験の失敗は 2 空白の出力）。
- #35: `panic_for_test` のあと `set_file` が `{:error, "ワークスペースが panic で壊れた"}`。dirty CPU を 1 本にした試験は debug の NIF では結果が出る前に時間切れで確かめきれなかった（release の直した版では通る）。
- #30: コールバックが `error("boom")` を投げた後の `check` が `{"error":"ワークスペースは使用中…"}`。

直した後:

- `cargo test -p globe-mode-lsp --lib`: 45 本通過・1 本 ignored。
- `mise run elixir:test`: 7 本通過（約 70 秒、release のビルドを含む）。`gleam:test`: 3 本通過。`c:zig`: 通過（110 s）。`c:julia`: 通過（`callback exception: ok`）。
- `mise run check`: 通過（fmt・clippy・test・deny・`uv sync --locked`。clippy の `unit_arg` と fmt で 1 回ずつ落ちて直した）。
- `check_docs.py`: スライド OK / 文書 852 件・リンク 4143 件中の切れ 0 件。`render_slides.py`: デッキ 1 件・問題 0 件。
- CI（feature ブランチ、2026-11-07）: `ci.yml` 全体検査（`full=true`）run 42331734738 success、`vscode.yml` run 42331737707 success。

## 残したこと

- #25 の `compile_model_strict` の `catch_unwind`（フロントエンド）。
- #33 の積分中の `$/cancelRequest`（ネイティブの積分が中断を段の間でしか見ない）。
- #37 の Python・C・NIF の側と LSP を含む `Checker::reset` の規則は S212。
- LSP のレビュー（`reviews/lsp.md`）と C・NIF のレビュー（`c-nif.md`）の残りは触っていない（C・NIF の `set_file`・`format`・`lint` のスケジューラは S202 で済み）。
