# src/Elixir

Rustler の NIF（`globe-mode-nif`、Erlang モジュール名 `globe_mode_nif`）を呼ぶ薄いラッパ。ロジックは Rust 側に置く。

| 場所 | 内容 |
| --- | --- |
| `globe_mode/` | mix プロジェクト。`GlobeMode.new(libraries)` / `set_file` / `load_dir` / `check` / `graph` / `outline` / `deps` / `flat`（読み取り。S191）/ `simulate` / `solvers`（S082）。`simulate_columns` の layout の `defaults` は既定で描く信号の名前（S190）。結果は `{:ok, map}` か `{:error, 理由}`。Hex の依存は無い（JSON は Elixir 1.18 の標準 `JSON`） |
| `phoenix_fmt/` | `globe fmt` の CLI を呼ぶウェブアプリ |

```bash
mise run elixir:test   # cargo build --release -p globe-mode-nif → mix test（Decay を check → simulate）
```

最初の例（`src/Elixir/globe_mode` で `mix deps.get` のあと `GLOBE_MODE_NIF=<target/release/libglobe_mode_nif> iex -S mix`）:

```elixir
{:ok, ws} = GlobeMode.new(["../../Modelica/GlobeModeValidation"])
model = "GlobeModeValidation.Simulation.Decay"
{:ok, %{"passed" => true}} = GlobeMode.check(ws, model)
{:ok, res} = GlobeMode.simulate(ws, model)   # 設定は第 3 引数の map（%{stop_time: 2} など）
res["final"]["x"]                            # 0.367879468116207（e⁻¹）
```

- NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス。例 `target/release/libglobe_mode_nif`）。
- 配布（rustler_precompiled と Hex）は未（P7）。
