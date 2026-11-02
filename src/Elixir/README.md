# src/Elixir

Rustler の NIF（`globe-mode-nif`、Erlang モジュール名 `globe_mode_nif`）を呼ぶ薄いラッパ。ロジックは Rust 側に置く。

| 場所 | 内容 |
| --- | --- |
| `globe_mode/` | mix プロジェクト。`GlobeMode.new(libraries)` / `set_file` / `load_dir` / `check` / `graph` / `simulate` / `solvers`（S082）。結果は `{:ok, map}` か `{:error, 理由}`。Hex の依存は無い（JSON は Elixir 1.18 の標準 `JSON`） |
| `phoenix_fmt/` | `globe fmt` の CLI を呼ぶウェブアプリ |

```bash
mise run elixir:test   # cargo build --release -p globe-mode-nif → mix test（Decay を check → simulate）
```

- NIF の場所は環境変数 `GLOBE_MODE_NIF`（拡張子 `.so` を除いたパス。例 `target/release/libglobe_mode_nif`）。
- 配布（rustler_precompiled と Hex）は未（P7）。
