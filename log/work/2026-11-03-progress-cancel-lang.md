# S131 `progress-cancel-lang`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s131-progress-cancel-lang` |
| worktree | `.git-worktree/s131-progress-cancel-lang` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S131-progress-cancel-lang/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

ADR-0033 の順の 4: C ABI・Zig・Julia と NIF・Elixir・Gleam に進捗と中断を足す。S129（Python）を型にする。C は `gm_simulate_observed(ws, model, settings_json, callback, user_data)`（`GmProgress`、非 0 で中断）と列の版、NIF は進捗を pid へのメッセージ、中断は `cancel(token)`。各言語の試験 1 本（`der(x) = sin(100 * time)` を 1000 s、最初の進捗で止めて `cancelled`）。wasm・Svelte・LSP・Desktop、exec・facade の中身、Python、`.github/workflows/` は触らない。新しい外部の依存は足さない。

## 決めたこと

- C の口は依頼どおり。`time` が無いときは NaN と `has_time = 0`。`stage` は Python と同じ小文字の文字列で、コールバックの間だけ有効。`gm_simulate_columns` は `gm_simulate_columns_observed` の `callback = NULL` の包みにし、`callback` が NULL なら `SimulateHandle::default()`（前と同じ道）。
- `tests/header.rs` は `typedef` の行と `#[cfg(test)]` 以降（試験のコールバックの `extern "C" fn`）を飛ばす。
- Julia は閉包の `@cfunction` を使わず、固定の trampoline に関数を `Any` で渡す。
- NIF はトークンを先に作る形（`cancel_token()` と `simulate_observed(ws, model, settings_json, pid, token)`）。ADR の (3) の表の `simulate_async` が `{ref, token}` を返す形は、DirtyCpu の中で値を返せないので採らなかった。進捗は `NifMap` の map で、呼び出し元の env の `send`。`cancel` は `:ok`。
- Gleam は `gleam_erlang` を足さず、不透明な `CancelToken`・`Pid` と `erlang:self/0`。試験の spawn と受信は `test/globe_mode_test_ffi.erl`。
- NIF の列の道には進捗を付けなかった（依頼に無い）。

## やったこと

- `src/Rust/mod/globe-mode-c/src/lib.rs`・`include/globe_mode.h`・`tests/header.rs`: `GmProgress`・`GmProgressCallback`・`gm_simulate_observed`・`gm_simulate_columns_observed`、単体試験 1 本。
- `src/Zig/src/main.zig`、`src/Julia/globe_mode.jl`・`test.jl`: 包みと試験。
- `src/Rust/mod/globe-mode-nif/src/lib.rs`: `Token`・`cancel_token`・`cancel`・`simulate_observed`。
- `src/Elixir/globe_mode/lib/globe_mode.ex`・`test/globe_mode_test.exs`、`src/Gleam/globe_mode/src/*`・`test/*`: 包みと試験。
- ADR-0033 の「4 段目の結果」、`doc/research/lang-api-matrix-2026-11-03.md` の進捗・中断の行と穴の数、`doc/features.md`、`doc/roadmap.md`、セッション文書、索引。

## 検証

- `cargo test -p globe-mode-c`: 3 passed（新しい `a_nonzero_callback_cancels_with_the_partial_trajectory` を含む）、`header_matches_extern_c_signatures` 1 passed。
- `cargo clippy -p globe-mode-c -p globe-mode-nif --all-targets -- -D warnings`: 警告 0。
- `mise run c:zig`: `cancel: calls=7, termination=cancelled`。
- `mise run c:julia`: `cancel: progress=7, stages=["lowering", "preflight", "build", "initialization", "integration"]`。
- `mise run elixir:test`: 2 tests, 0 failures。
- `mise run gleam:test`: 2 passed。
- `.github/scripts/test_ci_plan.py`: 25 tests OK。
- `mise run check`（`CARGO_BUILD_JOBS=2`）: fmt・clippy 通る。`cargo test --workspace` は 2 回とも facade の doctest だけが共有 target の古いビルドで落ちた（E0463・E0460。facade は変えていない）。facade・db・core・fmi・fmt を `touch` して `cargo test -p globe-mode --doc` は 1 passed、続く `cargo test --workspace --doc`・`cargo deny check`（advisories・bans・licenses・sources ok）・`uv sync --locked` も通った。
- `check_docs.py`: S131 のデッキ OK / 文書 544 件・相対リンク 2724 件中の切れ 0 件。
- `render_slides.py`: **通っていない**。S131 のデッキも、前に通った S129 のデッキも `Page.screenshot` が 30 s で時間切れ（Chromium を片付け、load average 4〜5 で 4 回）。手元の環境の問題とみて、CI の docs で確かめる。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件。

## 残したこと

- ADR-0033 の順の 2（wasm・Svelte）と 5（LSP・Desktop）。
- NIF の `simulate_columns` に進捗は無い。
- ネイティブの型付きの道（bdf・dopri45・auto）の積分の中、lowering・組み立ての中では止まらない（S123 の持ち越しと同じ）。
