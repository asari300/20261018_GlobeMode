# src/Elixir

Rustler の NIF（`globe-mode-nif`、Erlang モジュール名 `globe_mode_nif`）を呼ぶ薄いラッパ。ロジックは Rust 側に置く。

| 場所 | 内容 |
| --- | --- |
| `globe_mode/` | mix プロジェクト。`GlobeMode.new(libraries)` / `set_file` / `load_dir` / `check` / `graph` / `outline` / `deps` / `flat`（読み取り。S191）/ `simulate` / `solvers`（S082）。`simulate_columns` の layout の `defaults` は既定で描く信号の名前（S190）。結果は `{:ok, map}` か `{:error, 理由}`。Hex の依存は無い（JSON は Elixir 1.18 の標準 `JSON`） |
| `phoenix_fmt/` | `globe fmt` の CLI を呼ぶウェブアプリ |

```bash
mise run elixir:test   # cargo build --release -p globe-mode-nif → mix test（Decay を check → simulate）
```

最初の例（`src/Elixir/globe_mode` で `mix deps.get` のあと `GLOBE_MODE_NIF=<target/release/libglobe_mode_nif> iex -S mix`）は次のとおり。

```elixir
{:ok, ws} = GlobeMode.new(["../../Modelica/GlobeModeValidation"])
model = "GlobeModeValidation.Simulation.Decay"
{:ok, %{"passed" => true}} = GlobeMode.check(ws, model)
{:ok, res} = GlobeMode.simulate(ws, model)   # 設定は第 3 引数の map（%{stop_time: 2} など）
res["final"]["x"]                            # 0.367879468116207（e⁻¹）
```

- NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス。例 `target/release/libglobe_mode_nif`）。
- 配布（rustler_precompiled と Hex）は未（P7）。

## NIF のスケジューラ（S202）

`Ws` の Mutex を取る NIF と、本文の長さやモデルの大きさに比例する NIF は dirty スケジューラで回す（通常のスケジューラで Mutex を待つと、別のプロセスの長い実行が終わるまで BEAM の通常のスケジューラが 1 本止まる）。`set_file` は S202 までは通常のスケジューラだった。表は `globe-mode-nif/src/lib.rs` の先頭。

| NIF | スケジューラ |
| --- | --- |
| `new` `load_dir` `set_file` `export_fmu` `check` `graph` `outline` `deps` `flat` `simulate` `simulate_observed` `simulate_columns` | DirtyIo（Mutex を待つものは S214 からすべて。DirtyCpu だと、実行が Mutex を持つ間に待つ呼び出しがコアの数だけ溜まって、ほかの dirty CPU の NIF まで止める） |
| `format` `lint` | DirtyCpu（Mutex を取らない。本文の長さに比例する） |
| `cancel_token` `cancel` `solvers` | 通常（Mutex を取らない定数時間） |

試験 `test/scheduler_test.exs` は通常のスケジューラを 1 本に絞り、実行が Mutex を持つ間に別のプロセスが `set_file` を呼んでも、ただのタイマーのループが止まらないことを確かめる。S214 からは dirty CPU も 1 本に絞り、`check` が Mutex を待っていても `format` が通ることを確かめる。

`panic` で Mutex が poison されても、workspace は使い続けられる（S214。C ABI と同じ。試験用の NIF `panic_for_test(ws)` が Mutex を持ったまま panic する）。
