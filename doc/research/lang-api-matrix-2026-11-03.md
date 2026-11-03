# 言語 API の機能表と参照シナリオ（S110）

- 確認日: 2026-11-03。main の `4e4d909`（S099 のマージ）のソースを読んだ。
- 目的: [roadmap](../roadmap.md) の P7 の完了条件「参照シナリオが 6 言語で通る」の前段として、各言語の API がどの機能を出しているかを並べ、穴を埋める順を決める。コードは変えていない。
- 6 言語は roadmap の P7 の行の C / Zig / Julia、Elixir / Gleam、Python。比べるために Rust の facade、wasm、LSP、CLI も並べる。
- 既に決めた方針（変えない）: 束ねはすべて facade `globe-mode` だけに依存し、入出力は JSON の文字列（[S039](../sessions/S039-python-wheel/解説書.md)、[S075](../sessions/S075-c-abi/解説書.md)、[S082](../sessions/S082-elixir-nif/解説書.md)、[ADR-0014](../adr/0014-session-route-wasm.md)）。Zig・Julia は C ABI を通し、Gleam は Elixir と同じ NIF を `@external` で呼ぶ（[S087](../sessions/S087-gleam-external/解説書.md)）。

## 1. 機能表

「—」は穴（その言語から呼べない）。括弧の中は根拠の関数名。「（S111）」は [S111](../sessions/S111-lang-format-lint/解説書.md)、「（S116）」は [S116](../sessions/S116-lang-columns/解説書.md)、「（S118）」は [S118](../sessions/S118-lang-fmu-export/解説書.md) で埋めたもの（2026-11-03 に追記。下の 2・3 節は S110 のときの判定のまま）。

| 機能 | Rust facade | C ABI | Zig | Julia | Elixir | Gleam | Python | wasm | LSP | CLI |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ワークスペースとファイル | `Workspace::new` / `set_file` / `remove_file` / `load_dir` | `gm_workspace_new` / `gm_set_file` / `gm_load_dir` | C と同じ | `Workspace` / `set_file` / `load_dir`（S111） | `new` / `set_file` / `load_dir` | `new` / `set_file` / `load_dir` | `Workspace` / `set_file` / `load_dir` | `Workspace` / `set_file` / `remove_file` / `files` | `didOpen` / `didChange` / `didClose` | 引数 `FILES` |
| check | `check` | `gm_check` | C と同じ | `check` | `check` | `check` | `check` | `check` | `globe/check` | `globe check` |
| simulate（JSON と CSV） | `simulate` / `simulate_with` | `gm_simulate` | C と同じ | `simulate` | `simulate` | `simulate` | `simulate` | `simulate` | `globe/simulate` | `globe simulate`（CSV・MAT v4） |
| simulate_columns（列） | `Simulated::columns`（S116） | `gm_simulate_columns` / `gm_doubles_free`（S116） | C と同じ（S116） | `simulate_columns`（S116） | `simulate_columns`（binary。S116） | `simulate_columns`（`BitArray`。S116） | `simulate_columns`（f64 の `bytes` → polars） | `simulate_columns`（`SimColumns.column`） | `globe/simulateColumns` | 対象外（ファイルに書く） |
| format | `format`（S111） | `gm_format`（S111） | C と同じ（S111） | `format`（S111） | `format`（S111） | `format`（S111） | `format`（S111） | `format` | `textDocument/formatting` / `globe/format` | `globe fmt` |
| lint | `lint`（S111） | `gm_lint`（S111） | C と同じ（S111） | `lint`（S111） | `lint`（S111） | `lint`（S111） | `lint`（S111） | `lint` | `globe/lint` | `globe lint` |
| graph | `graph` | `gm_graph` | C と同じ | `graph`（S111） | `graph` | `graph` | `graph` | `graph` | `globe/graph` | `globe graph` |
| MSL の全部読み | `with_libraries_full` / `with_library_sources` | — | — | — | — | — | — | `Workspace(json, name, true)` | — | — |
| MSL の分割読み | `with_libraries`（既定）/ `with_library_sources_split` | `gm_workspace_new(libraries_json)` | C と同じ | `Workspace(libraries)` | `new(libraries)` | `new(libraries)` | `Workspace(libraries=…)` | `Workspace(json)`（既定） | `globe lsp -L` | `-L` |
| 設定の override（`parameters` など） | `SimSettings` | `settings_json` | C と同じ | `settings` | `settings`（map） | `settings_json` | `**settings` | `request_json` の `settings` | `request.settings` | `--param` ほか |
| 進捗 | `exec::run_with_progress`（実行系の段だけ。facade の `simulate` には無い） | — | — | — | — | — | — | — | — | — |
| 中断 | `max_wall_seconds`（予算） | 予算 | 予算 | 予算 | 予算 | 予算 | 予算 | 予算と Worker の作り直し | 予算と sidecar の作り直し | `--timeout` |
| FMU export | `export_fmu` / `Workspace::export_fmu`（feature `fmu`。S118） | `gm_export_fmu`（S118） | C と同じ（S118） | `export_fmu`（S118） | `export_fmu`（S118） | `export_fmu`（S118） | `Workspace.export_fmu`（S118） | 対象外（ファイルを書けない） | — | `globe fmu export`（S118 から facade の `export_fmu`） |

### 根拠

| 列 | ファイル |
| --- | --- |
| Rust facade | `src/Rust/mod/globe-mode/src/lib.rs`（`pub fn` と `pub use`。`globe-mode-fmt`・`globe-mode-fmi` には依存していない: `src/Rust/mod/globe-mode/Cargo.toml`）、`src/Rust/mod/globe-mode-ir/src/sim.rs`（`SimSettings` の `parameters`・`max_wall_seconds`）、`src/Rust/mod/globe-mode-exec/src/lib.rs`（`run_with_progress`） |
| C ABI | `src/Rust/mod/globe-mode-c/include/globe_mode.h`、`src/Rust/mod/globe-mode-c/src/lib.rs`（`gm_workspace_new` は `Workspace::with_libraries`） |
| Zig | `src/Zig/build.zig`（ヘッダを translate-c するので `gm_*` はすべて呼べる）、`src/Zig/src/main.zig`（例は check と simulate だけ） |
| Julia | `src/Julia/globe_mode.jl`（`set_file` / `check` / `simulate` だけを包む。`gm_load_dir`・`gm_graph`・`gm_solvers` の包みは無い） |
| Elixir | `src/Rust/mod/globe-mode-nif/src/lib.rs`、`src/Elixir/globe_mode/lib/globe_mode.ex` |
| Gleam | `src/Gleam/globe_mode/src/globe_mode.gleam`、`src/Gleam/globe_mode/src/globe_mode_nif.erl` |
| Python | `src/Rust/mod/globe-mode-py/src/lib.rs`、`src/Python/mod/globe_mode/__init__.py` |
| wasm | `src/Rust/mod/globe-mode-wasm/src/lib.rs`（`js_new` の `eager`）、`src/Rust/mod/globe-mode-wasm/src/fmt.rs`（`format` / `lint`） |
| LSP | `src/Rust/mod/globe-mode-lsp/src/lib.rs`（`handle` の `match`。`run_stdio` は `Workspace::with_libraries`） |
| CLI | `src/Rust/mod/globe-mode-cli/src/main.rs`（`enum Command`）、`src/Rust/mod/globe-mode-cli/src/simulate.rs` |

- 中断はどの言語も `SimSettings.max_wall_seconds`（壁時計の予算。初期化と積分器の 1 歩の内側では止まらない）だけで、呼び手が途中で止める口は無い。wasm と Desktop は UI の側でワーカーを作り直して止める（[ADR-0023](../adr/0023-desktop-and-web-process-layout.md)）。表では「予算」を穴に数えない。
- 「MSL の全部読み」は facade の `with_libraries` が、ディレクトリでないパスが混じるときだけ全部読みに戻る。全部読みは比べるために残した道（S054）なので、束ねに無くても困らない。
- outline・deps・flat・`tunable_parameters`・solvers の一覧は表に入れていない（参照シナリオに要らない）。solvers の一覧は C・Julia 以外の束ねにある（C は `gm_solvers`、Julia は包みが無い）。

### 言語ごとの穴の数

対象の 13 行のうち「—」の数。

| 言語 | 穴 | 中身 |
| --- | --- | --- |
| Rust facade | 2 | FMU export（別 crate にはある）、進捗（実行系にだけある）。format・lint は S111 で埋めた |
| C ABI | 3 | 全部読み・進捗・FMU export（format・lint は S111、列は S116） |
| Zig | 3 | C と同じ |
| Julia | 3 | C と同じ（graph・`load_dir` の包みは S111） |
| Elixir | 3 | 全部読み・進捗・FMU export（format・lint は S111、列は S116） |
| Gleam | 3 | Elixir と同じ |
| Python | 3 | 全部読み・進捗・FMU export（format・lint は S111） |
| wasm | 2 | 進捗・FMU export |
| LSP | 3 | 全部読み・進捗・FMU export |
| CLI | 2 | 全部読み・進捗 |

## 2. 参照シナリオ

S039 の参照シナリオ（open → check → graph → simulate → 表）を土台に、P7 の 6 言語で同じ手順を回せるものにする。MSL は使わない。

1. open: `src/Modelica/GlobeModeValidation/Simulation/Decay.mo` の本文を `set_file` で足す（または `src/Modelica` を `load_dir`）。
2. check: `GlobeModeValidation.Simulation.Decay` の `passed` が真。
3. simulate: 既定の設定で `ok` が真。
4. x(1): `SimulateResponse` の `final`（JSON の名前。Rust では `final_values`）の `x` が e⁻¹ と相対 1e-4 で一致。
5. format: 同じ本文を整形して文字列が返る（文法が壊れていなければ誤りにならない）。

graph は S039 のシナリオにあるが、Julia 以外では既に通るので、ここでは format を足して「編集 → 検査 → 実行 → 整形」の 1 周にした。

### 今すぐ通るか

| 言語 | 判定 | 足りないもの | 確かめ方 |
| --- | --- | --- | --- |
| Rust facade | 通る（format は `globe-mode-fmt` を別に使う）【未確認】 | facade に format が無い | ソースを読んだだけ（cargo を回していない） |
| C ABI | 通らない | format | ソース（`globe_mode.h`）。【未確認】ビルドしていない |
| Zig | 通らない | format | ソース（`main.zig` は 1〜3 まで。x は `final` から取れる）。【未確認】ビルドしていない |
| Julia | 通らない | format | ソース（`globe_mode.jl` は 1〜3 まで）。【未確認】ビルドしていない |
| Elixir | 通らない | format | ソース（`mise run elixir:test` は 1〜3 と最後の時刻 1）。【未確認】ビルドしていない |
| Gleam | 通らない | format | ソース（`mise run gleam:test` は 1〜3 と最後の時刻 1）。【未確認】ビルドしていない |
| Python | 通らない | format | 1〜4 を手元で回した（下） |
| wasm | **通った** | — | Node で回した（下） |
| LSP | **通った** | — | `globe lsp` に stdio で要求を送った（下） |
| CLI | **通った** | — | `globe check` / `simulate` / `fmt --check` を回した（下） |

**P7 の 6 言語で今通るものは 0。** 6 言語とも、足りないのは format だけ（Julia は graph も無いがシナリオには要らない）。1〜4 は 6 言語とも API がある。

**S111 の追記（2026-11-03）**: facade と 6 言語に format / lint を足し、参照シナリオを各言語の試験（`globe-mode-c` の単体試験、`mise run c:zig`・`c:julia`・`elixir:test`・`gleam:test`、`src/Python/tests/test_api.py`）に入れて、6 言語とも手元で通った（[S111](../sessions/S111-lang-format-lint/解説書.md)）。

### 手元で回した結果（2026-11-03、WSL）

既にある build を使った。cargo のビルドは回していない。

| 対象 | 使ったもの | 結果 |
| --- | --- | --- |
| Python | main の `.venv/bin/python -I` と `src/Python/mod/globe_mode/_native.abi3.so`（2026-11-01 の build） | `load_dir` 70 本 → check `passed` → simulate `ok` → `final.x` = 0.367879468116207（e⁻¹ との相対誤差 7.3e-8）、0.37 s。`globe_mode.format` は無い。この `.so` は S085 より前の build で `simulate_columns` が無かった（ソースにはある） |
| wasm | `.git-worktree/target-shared/wasm-pkg/node`（2026-11-02 の build）を Node で | `set_file` → check `passed` → simulate `ok`、`final.x` = 0.367879468116207、`simulate_columns` の x の最後も同じ → `format` 318 文字、`lint` 0 件。815 ms |
| LSP | `.git-worktree/target-shared/release/globe lsp`（2026-11-02 の build） | `didOpen` → `globe/check` `passed` → `globe/simulate` `ok`、`final.x` = 0.367879468116207 → `globe/simulateColumns` の x も同じ → `globe/format` は文字列 |
| CLI | 同じ `globe` | `check --json` で `passed` → `simulate -o -` の最後の行 `1,0.367879468116207` → `fmt --check` は「整形が要る」で終了コード 1（整形は動く。`Decay.mo` が整形の規則にそろっていないだけ） |

- この `globe` の build には `fmu` のサブコマンドが無かった（S094 より前の build）。表の CLI の FMU export はソースによる。

## 3. 埋める順の提案

| 順 | 何を | 効くもの | 理由 |
| --- | --- | --- | --- |
| 1 | facade に `format(text, options_json)` と `lint(text)` を足す（`globe-mode-fmt` に依存。入出力は wasm の `fmt.rs` と同じ形） | 全部 | 束ねは facade だけに依存する方針（S039・S075・S082）を崩さずに済む。wasm・LSP の形をそのまま使える |
| 2 | C ABI に `gm_format` / `gm_lint`、NIF に `format` / `lint`、py に `format` / `lint` を足し、Julia に `format`・`load_dir`・`graph` の包みを足す | 6 言語 | これで参照シナリオの 5 段が 6 言語でそろう。依存の追加は 0 |
| 3 | 参照シナリオを各言語の試験にする（`test_api.py`、`main.zig`、`globe_mode.jl`、`globe_mode_test.exs`、`globe_mode_test.gleam` で x(1) と format を確かめる）。CI のジョブ `lang` で回す | 6 言語 | P7 の完了条件を CI で示す |
| 4 | 列の道を C（`double *` と長さ）と NIF（binary）に（S116 で済） | C・Zig・Julia・Elixir・Gleam | 大きな結果で効く（S085 で Python は 6 倍）。シナリオには要らない |
| 5 | FMU export を facade に（`globe-mode-fmi` の `write_fmu`。S118 で済、wasm は対象外） | 全部 | 呼ぶ人がいれば。S109 が `globe-mode-fmu` を触っている間は待つ |
| 6 | 進捗と途中で止める口 | 全部 | 呼び手から止める形（コールバックか、フラグか）の設計が要る。ADR を書いてから |

1 と 2 は 1 セッションに収まる大きさ（facade に 2 関数、束ねごとに 2〜3 関数）。
