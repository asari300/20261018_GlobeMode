# src/Rust

GlobeMode の Rust のコア。

次の表は S002 の時点のもの。いまの crate は [doc/architecture.md](../../doc/architecture.md#22-rust-cratesrcrustmod) の 2.2 章。

| crate | 内容 |
| --- | --- |
| `mod/globe-mode-core` | 診断（`Diagnostic`）、検査段階（`Stage` S0〜S10）、ソースの範囲 |
| `mod/globe-mode-ir` | 中立 IR（`Outline`、`Connect`、`BalanceCount`） |
| `mod/globe-mode-frontend` | `trait Frontend` と Rumoca アダプタ（feature `rumoca`） |
| `mod/globe-mode-cli` | CLI `globe`（`outline` / `check`、`--library`） |
| `xtask` | `cargo xtask fetch-msl` / `fetch-rumoca` / `msl-compare` / `bump-rumoca`（S003） |

```bash
cargo run -p globe-mode-cli -- check src/Modelica/GlobeModeExamples/package.mo -m GlobeModeExamples.RC
mise run check
```

構成は次のとおり。

- `mod/<crate>/`: workspace の library crate（`globe-mode-*`）。一覧は [doc/architecture.md](../../doc/architecture.md#22-rust-cratesrcrustmod)
- `apps/desktop/`: Tauri 2 のシェル（Linux、Windows、macOS、Android）
- `apps/experimental/{leptos,dioxus}/`: 負荷を比べる試験版（P7）
- `apps/web-{actix,axum,loco,rocket}/`: 整形・lint のウェブアプリ（[doc/web-apps.md](../../doc/web-apps.md)）
- `xtask/`: build、bump-rumoca、licenses、wasm --threads
