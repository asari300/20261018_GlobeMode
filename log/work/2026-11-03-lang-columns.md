# S116 `lang-columns`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s116-lang-columns` |
| worktree | `.git-worktree/s116-lang-columns` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S116-lang-columns/` |
| セッション記録 | （親が写す） |

## 依頼

S110 の「埋める順」の 4。列の道（`simulate_columns`）を C ABI と Elixir NIF に足し、Zig・Julia・Gleam から使えるようにする。Python（S085）・wasm（`SimColumns`）と同じ形。facade に列の道が無ければ先に足す。各言語の試験に Decay の列の道で x(1) が e⁻¹、点の数が JSON の道と同じ、を 1 本ずつ。機能表と features を更新。FMU export の facade、進捗・中断は持ち越し。`globe-mode-exec`・`globe-mode-fmu`・`src/Python`・upstream の文書・`.github/workflows/` は触らない。

## 決めたこと

- facade には `Simulated::columns(&self) -> (ColumnsLayout, Vec<&[f64]>)` を足した（借用で返し、束ねがそれぞれの形へ 1 回だけ写す）。`Workspace::simulate_columns` は足さず、束ねが `no_csv = true` で `simulate_with` を呼ぶ（Python・wasm と同じ）。
- Python（`globe-mode-py`）と wasm（`SimColumns`）の列の組み立てを `Simulated::columns` に寄せた。wasm の `SimColumns` は `Simulated`（`compiled` は放す）を持つ。外から見える形は変えていない。
- C は不透明な結果の型を作らず、`gm_simulate_columns` が JSON を返し、列を 1 本の連続した `double *`（`Box<[f64]>`）と個数で出す。`gm_doubles_free(data, len)` で放す。誤りは既存と同じく `{"error"}`、そのとき `*data = NULL`・`*len = 0`。
- レイアウトの JSON は C と NIF で同じ `{"response", "names", "points"}`（`response` に `csv` は無い）。
- NIF は `{:ok, {layout_json, [binary]}}`（DirtyCpu、f64 の little-endian）。
- 既定のまま進めたこと: 外部の依存の追加 0、CI の定義は変えない。

## やったこと

- `src/Rust/mod/globe-mode/src/lib.rs`: `Simulated::columns`。
- `src/Rust/mod/globe-mode-py/src/lib.rs`、`src/Rust/mod/globe-mode-wasm/src/lib.rs`: 列の組み立てを facade に寄せた。
- `src/Rust/mod/globe-mode-c/`（`src/lib.rs`・`include/globe_mode.h`・`tests/header.rs`）: `gm_simulate_columns`・`gm_doubles_free`、設定の JSON の読みを `request` に分けた、単体試験。ヘッダの突き合わせに `size_t`・`double *` などを足した。
- `src/Zig/src/main.zig`、`src/Julia/globe_mode.jl`・`test.jl`: 包みと試験。
- `src/Rust/mod/globe-mode-nif/src/lib.rs`、`src/Elixir/globe_mode/`、`src/Gleam/globe_mode/`: `simulate_columns` と試験。
- `doc/research/lang-api-matrix-2026-11-03.md`（列の行と穴の数）、`doc/features.md`、`doc/roadmap.md`。

## 検証

- clippy（`-p globe-mode -p globe-mode-c -p globe-mode-nif -p globe-mode-py -p globe-mode-wasm --all-targets -D warnings`）: 0 件。
- `cargo test -p globe-mode-c -p globe-mode-py`: 通った（C 2 本 + ヘッダ 1 本、py 1 本）。`cargo test -p globe-mode-wasm`: 9 passed, 1 ignored。
- `mise run c:zig`: 501 点、x = 0.367879468116207（JSON の道と同じ）。
- `mise run c:julia`: 501 点、x = 0.367879468116207。JSON の道 4.99 ms、列の道 4.16 ms（1 回）。
- `mise run elixir:test`: 1 test, 0 failures。501 点、x = 0.367879468116207。JSON の道 1,803 µs、列の道 1,602 µs（1 回）。
- `mise run gleam:test`: 1 passed。
- `python3 .github/scripts/test_ci_plan.py`: 25 本 OK（依存を変えていないので試験は足していない）。
- `mise run wasm:node`: 通った（Decay の bdf・dopri45・auto、x(1) = 0.367879468116207。data/msl が無いので MSL は飛ばした）。`mise run wasm:compare`: 通った（148.6 s）。
- `mise run check`: 1 回目は `cargo test` の doctest（`globe-mode`）が E0460（共有 target で別の作業が `globe-mode-fmt` を作り直した）で落ちた。touch して `cargo test --workspace --no-fail-fast` を回し直し、63 の試験の束で 620 passed・0 failed・11 ignored。fmt・clippy（workspace）・`test_ci_plan.py` は 1 回目で通った。`cargo deny check`: advisories・bans・licenses・sources ok。`uv sync --locked`: 通った。
- `check_docs.py`: デッキ OK、相対リンクの切れ 0 件。`render_slides.py`: 問題 0 件（Chromium の最初の screenshot が 2 回時間切れになり、3 回目で通った）。`gleam format`・`zig fmt --check`: 通った。
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件。

## 残したこと

- FMU export を facade に、進捗と途中で止める口（S110 の順の 5・6）。
- 時間の差は Decay（2 列 501 点）で 1 回測っただけ。大きな結果での比べは S085 の Python（6 倍）だけ。
