# src/Rust

GlobeMode の Rust のコア（P1 から作る）。

- `mod/<crate>/`: workspace の library crate（`globe-mode-*`）。一覧は [doc/architecture.md](../../doc/architecture.md#22-rust-cratesrcrustmod)
- `apps/desktop/`: Tauri 2 のシェル（Linux、Windows、Android）
- `apps/experimental/{leptos,dioxus}/`: 負荷を比べる試験版（P7）
- `xtask/`: build、bump-rumoca、licenses、wasm --threads
