# S111 `lang-format-lint`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s111-lang-format-lint` |
| worktree | `.git-worktree/s111-lang-format-lint` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S111-lang-format-lint/` |
| セッション記録 | （親が写す） |

## 依頼

S110 の「埋める順」の 1 と 2。参照シナリオ（Decay を開く → check → simulate → x(1) が e⁻¹ → format）を 6 言語で通す。facade に `format` / `lint`、C ABI・Zig・Julia（`graph`・`load_dir` も）・Elixir NIF・Gleam・Python に写し、各言語の既存の試験に参照シナリオを 1 本ずつ足す。機能表と features を更新。simulate_columns の C/NIF、FMU export、進捗・中断は持ち越し。

## 決めたこと

- facade の形は wasm の `fmt.rs` と同じ（`format(text, indent, insert_origin) -> Result<String, String>`、`lint(text) -> String` の JSON）。S110 の提案の `options_json` にはしなかった（wasm・LSP と同じ 2 つで足りる）。
- facade は `globe-mode-fmt` と `serde_json`（どちらも既に workspace にある。`serde_json` は dev-dependencies から移した）に依存する。外部の依存の追加は 0。
- C の `gm_format` は `{"text": …}` か `{"error": …}`（既存の `gm_*` と同じく NULL を返さない）。
- wasm の `fmt.rs` は facade に寄せなかった（wasm を変えると `wasm:node`・`wasm:compare` が要る）。

## やったこと

- `src/Rust/mod/globe-mode/`（`Cargo.toml`・`src/lib.rs`）: `format`・`lint`。
- `src/Rust/mod/globe-mode-c/`（`src/lib.rs`・`include/globe_mode.h`・`tests/header.rs`）: `gm_format`・`gm_lint` と単体試験に x(1)・format・lint。
- `src/Zig/src/main.zig`: x(1) と `Decay.mo` の format。
- `src/Julia/globe_mode.jl`・`test.jl`: `load_dir`・`graph`・`format`・`lint` と試験。
- `src/Rust/mod/globe-mode-nif/src/lib.rs`、`src/Elixir/globe_mode/`、`src/Gleam/globe_mode/`: `format`・`lint` と試験。
- `src/Rust/mod/globe-mode-py/src/lib.rs`、`src/Python/mod/globe_mode/__init__.py`、`src/Python/tests/test_api.py`: `format`・`lint` と試験。
- `.github/scripts/test_ci_plan.py`: facade・fmt の変更で C・NIF・py と `lang`・`python` が回ることの試験。
- `doc/research/lang-api-matrix-2026-11-03.md`（表と穴の数）、`doc/features.md`、`doc/roadmap.md`、セッション文書と索引。

## 検証

- `cargo test -p globe-mode -p globe-mode-c -p globe-mode-py`: 通った。`cargo clippy -p globe-mode -p globe-mode-c -p globe-mode-nif -p globe-mode-py --all-targets -- -D warnings`: 警告 0。
- 参照シナリオ: C 通った / Zig 通った（x = 0.367879468116207、整形 382 バイト）/ Julia 通った（x = 0.367879468116207）/ Elixir 1 test 0 failures / Gleam 1 passed / Python 通った（3 passed, 1 skipped）。
- `test_ci_plan.py`: 25 件 OK。
- `mise run check`: fmt・clippy（`-D warnings`）・`cargo deny check`（advisories・bans・licenses・sources ok）・`uv sync --locked` は通った。`cargo test --workspace` は共有 target の別の worktree（S109・S112）のビルドが混じって 1〜55 件落ちた（`events_are_refused` は S109 の exec、`NotFound` は `s112-fmu-outputs` の `CARGO_MANIFEST_DIR`）。`src/Rust` の `.rs` を touch して回し直し、617 passed / 0 failed
- `check_docs.py`: デッキ OK、リンク 2200 件中の切れ 0 件。`render_slides.py` は S111 のデッキ 6 枚 × 3 表示で問題 0 件（目でも確認）
- セッション記録の `--check`: 親が写す（このセッションでは写さない）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 列の道を C・NIF に、FMU export の facade、進捗と中断（S110 の順の 4〜6）。
- wasm の `fmt.rs` を facade に寄せること（重複 1 か所）。
