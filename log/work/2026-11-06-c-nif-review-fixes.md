# S202 `c-nif-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s202-c-nif-review-fixes` |
| worktree | `.git-worktree/s202-c-nif-review-fixes` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S202-c-nif-review-fixes/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

C ABI（`globe-mode-c`）と NIF（`globe-mode-nif`）のレビュー（Opus、指摘 1〜6）を、1 件ずつ再現してから直す。安全と停止が最優先。

## 決めたこと

- `set_file` は DirtyIo（レビューは DirtyCpu）。待つだけで CPU を使わず、DirtyCpu はスケジューラ数と同じ本数なので `+S 1` で実行と席を取り合う。
- C ABI の守りは使用中の印（`AtomicBool`）。`ws_arg` が生のポインタから印だけを読み、印を取れたら `&mut Workspace` を貸す（落とすと戻す。panic でも）。`gm_workspace_free` は使用中なら放さず stderr に 1 行（戻り値のない関数なので。ヘッダの署名は変えない）。`&mut GmWorkspace` を作らないので、実行中の `&mut Workspace` と別名にならない。
- 重複は facade・exec の公開の関数に寄せた（`simulate_request`・`Simulated::layout_json`・`ExecStage::as_str`・`Workspace::outline_json`）。既存の関数は変えない。`SimulateRequest::from_settings_json` は型が `globe-mode-ir` にあるので、facade の関数にした。Python・wasm・LSP は今回は触らない。
- Miri は環境に無い（`cargo miri` が stable に無く、nightly は入れない）ので回さなかった。

## やったこと

- `globe-mode-nif/src/lib.rs`: `set_file` を DirtyIo、`format`・`lint` を DirtyCpu、重複を facade へ、スケジューラの表を先頭の docs に。
- `globe-mode-c/src/lib.rs`・`include/globe_mode.h`・`README.md`: 使用中の印、`gm_workspace_free` の `catch_unwind`、スレッドとコールバックの決まり、重複を facade へ、単体の試験 3 本。`tests/header.rs`: `GmProgress` の突き合わせ。
- `globe-mode`・`globe-mode-exec`: 公開の関数 4 つ。
- `src/Zig`・`src/Julia`: コールバックの中の再入の試験。`src/Elixir/globe_mode/test/scheduler_test.exs`（新規）と README の NIF の表。
- `doc/research/lang-api-matrix-2026-11-03.md` の 4 章、`doc/roadmap.md`、`CHANGELOG.md`、`doc/sessions/README.md`。
- レビュー: `log/review/2026-11-06-s202-c-nif-review-fixes-opus.md`。

## 検証

- 再現（旧 `.so`、通常のスケジューラを 1 本に絞った ExUnit）: 実行が `Ws` の Mutex を持つ間の `set_file` で通常のスケジューラが 4506 ms 止まった。1 MB の本文の `lint` で 2209 ms、`format` と合わせて 4020 ms 止まった。直した後は 2 本とも通る（`scheduler_test.exs` 1.2 s）。
- `mise run check`: success（fmt・clippy・`cargo test --workspace`・deny・`uv sync --locked`。`globe-mode-c` の単体 6 本と `tests/header.rs` 2 本を含む）。
- `mise run c:zig`・`c:julia`・`elixir:test`（5 本）・`gleam:test`（3 本）: 通った。Zig・Julia は `reentry:` の行（誤りの JSON）を出す。
- `check_docs.py`: 切れ 0 件。`render_slides.py` S202 のデッキ 1 件 / 問題 0 件。
- feature を push して `ci.yml -f full=true`（run 42174737496）: plan・rust-linux・rust-windows・deny・wasm-node・msl・rust-sundials・ui・python・docs・lang の全ジョブ success。`release.yml` の dry run（run 42174761104、C ABI のライブラリを含む 4 環境と Desktop）: 全ジョブ success。タグ・Release は作っていない。
- Miri: 環境に `cargo miri` が無く回していない。

## 残したこと

- Python・wasm・LSP の重複（対応表の 4 章）。
- Miri（環境に無い）。
