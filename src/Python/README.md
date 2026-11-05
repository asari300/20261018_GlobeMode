# src/Python

- `mod/globe_mode/`: Python パッケージ `globe_mode`（Rust の facade を PyO3 で包み、maturin で wheel にする）。型は `py.typed` と `_native.pyi`。`uv run globe-mode` は版とソルバの数を出す
- `marimo/`: marimo のノートブックと anywidget（wasm を読む）
- `streamlit/`: 結果ダッシュボードとパラメータスイープ

使い方（MSL は `cargo xtask fetch-msl` で `data/msl` に取る。引数・戻り値・誤りは `help(globe_mode)`、`help(Workspace.simulate)`）。

```python
from globe_mode import Workspace, table

ws = Workspace("data/msl")  # ライブラリが 1 つなら文字列か Path でよい。複数ならリスト
model = "Modelica.Electrical.Analog.Examples.ChuaCircuit"
print(ws.check(model)["passed"])  # モデルの誤りは例外でなく passed: False と diagnostics
res = ws.simulate_columns(model, stop_time=2)  # 設定のキーが違えば TypeError（stoptime など）
print(res["ok"], table(res).shape)
```

- 誤りの型: 未知の設定のキーは `TypeError`、設定の値の型の誤りは `ValueError`（どの設定かを言う）、ライブラリが読めなければ `RuntimeError`（本文は最初の error の `コード: 文`、元の診断は `.diagnostics`）。
- `on_progress` は既定のソルバ（`bdf`・`auto`・`dopri45`）では積分の中に届かない（段の変わり目だけ。積分を止めたいなら `max_wall_seconds`）。`esdirk34` などなら積分の時刻が付き、`True` を返して止められる。

経路の説明は [doc/architecture.md](../../doc/architecture.md#5-言語-api)。
