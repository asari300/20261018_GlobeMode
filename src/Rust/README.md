# src/Rust

GlobeMode の Rust のコア。

S002 の時点で、次の 4 つの crate がある。

| crate | 内容 |
| --- | --- |
| `mod/globe-mode-core` | 診断（`Diagnostic`）、検査段階（`Stage` S0〜S10）、ソースの範囲 |
| `mod/globe-mode-ir` | 中立 IR（`Outline`、`Connect`、`BalanceCount`） |
| `mod/globe-mode-frontend` | `trait Frontend` と Rumoca アダプタ（feature `rumoca`） |
| `mod/globe-mode-cli` | CLI `globe`（`outline` / `check`） |

```bash
cargo run -p globe-mode-cli -- check src/Modelica/GlobeModeExamples/package.mo -m GlobeModeExamples.RC
mise run check
```

予定の構成:

- `mod/<crate>/`: workspace の library crate（`globe-mode-*`）。一覧は [doc/architecture.md](../../doc/architecture.md#22-rust-cratesrcrustmod)
- `apps/desktop/`: Tauri 2 のシェル（Linux、Windows、Android）
- `apps/experimental/{leptos,dioxus}/`: 負荷を比べる試験版（P7）
- `xtask/`: build、bump-rumoca、licenses、wasm --threads
