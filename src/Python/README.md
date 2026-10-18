# src/Python

- `mod/globe_mode/`: Python パッケージ `globe_mode`。S002 で `src/20261018_globemode` から改名した。今は uv_build でビルドし、P6 で PyO3 と maturin に置き換える。`uv run globe-mode` で動く
- `marimo/`: marimo のノートブックと anywidget（wasm を読む）
- `streamlit/`: 結果ダッシュボードとパラメータスイープ

経路の説明は [doc/architecture.md](../../doc/architecture.md#5-言語-api)。
