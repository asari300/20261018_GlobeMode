# S208 `facade-dedup`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s208-facade-dedup` |
| worktree | `.git-worktree/s208-facade-dedup` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S208-facade-dedup/` |
| セッション記録 | （親が写す） |

## 依頼

振る舞いを変えずに、Python・wasm・LSP・C・NIF の同じ処理を facade の関数に寄せる。issue #16 の残り（`memory.mjs`、xtask の `None` 分岐）を見直す。減った行数を面ごとに数える。

## 決めたこと

- 対象は 3 つ以上の面で同じもの。`layout_json`（C・NIF の 2 面）、進捗の写し（面ごとに形が違う）、`tunable_parameters` の診断のつなぎ（wasm・LSP の 2 面）、`load_dir` の文面（Python・NIF）、`check`/`deps`/`flat`/`solvers`（1 行の呼び出し）は残した。
- 足した facade の関数: `to_json`、`FmuOptions::from_json`、`Workspace::open`。既存の `simulate_request`・`ExecStage::as_str`・`Workspace::outline_json`・`Simulated::columns` を Python・wasm・LSP から使うようにした。
- Python の `outline` は文面を `outline_json` から取り、`.diagnostics` だけ `outline()` から足す（`SourceError` の形を保つ）。
- xtask の `None` 分岐は、S203 の「広い範囲」を数え直したら 10 か所ほどだったので直した（`Options.solver: &str`、`known_deviations: &Path`、CLI の `solver: String`）。ワーカーの `Option<String>` は残した。

## やったこと

洗い出しの表は解説書の 2 章。ほかに `memory.mjs`（`common.mjs` を使う）と xtask の `msl-sim`（`solver: &str`、`known_deviations: &Path`）。clippy の `format_in_format_args` が出たので、報告の文面は書式の中に直接入れた（出力は同じ）。

## 検証

- 減った行数（`git diff --numstat -- src`、追加 86 / 削除 167、差し引き -81）: facade +24、Python -44、LSP -18、C -9、wasm（lib.rs）-8、NIF -7、`memory.mjs` -10、xtask -9（`main.rs` 0、`sim.rs` -9）。面の合計は -105。
- 試験の期待値を変えた所: 0 件。
- `mise run check`: 1 回目の clippy が wasm の `&stage`（needless_borrow）と xtask の `format!` in `writeln!` で落ち、直した。その後 `cargo test --workspace` は共有 target の取り違え（別の worktree のビルドが `target-shared` の同じ名前の成果物を上書き）で 3 か所が落ちた: `globe-mode-fmu --test fmpy`（6 本）、`globe-mode-cli --test review_fixes`（`stdin_needs_an_explicit_dash`）、`globe-mode-c`・`globe-mode-py` の doctest（古い `globe_mode` の rlib で `to_json` が見つからない）。いずれも変えた crate と `touch` して単独で回し直すと通った（fmpy 11 本、cli 全部、c・py・fmu・facade の `cargo test` が EXIT 0）。`cargo deny check`: advisories・bans・licenses・sources ok。`globe fmt --check src/Modelica`: 71 ファイル整形済み。`uv sync --locked`: ok。
- `mise run wasm:node`: ok。`mise run wasm:compare`: 387 / 400 件が一致、既知の差 13 件、一致しない 0 件（前と同じ）。`memory.mjs` を `--limit 2` で回し、動く（ピーク 125.8 MiB）。
- `mise run c:zig`・`c:julia`・`elixir:test`（5 tests）・`gleam:test`（3 passed）: ok。
- Python: `uv sync --reinstall-package globe-mode` のあと `pytest` 42 passed・1 skipped（`anywidget` が無い）。`mypy` は `uvx` で `anywidget`・`traitlets` が見つからない 3 件（optional の extra。この変更と無関係で、Python の側は変えていない）。
- `check_docs.py`: スライド全部 OK、リンク 838 文書 / 相対リンク 4101 件 / 切れ 0 件。`render_slides.py`（S208 のデッキ）: 7 枚、問題 0 件。
- CI の run: 下に追記。
- issue #16: 5 つの項目がすべて済んだ（`fixed.rs`・`tableau()`・`db()`・`.mjs` の写し・`read_mat4` は S203、`memory.mjs` と xtask の `None` 分岐はこのセッション）ので `Fixes #16`。

## 残したこと

- `layout_json` を使う面が 3 つになったら Python の `defaults` の足し方も寄せる。
- Astro の fmt の CI。
