# marimo: 整形・lint のアプリ

GlobeMode の整形器と lint（`globe fmt --json` / `globe lint --json`）を marimo のアプリで使う。計算は Rust 側の `globe` に任せ、このアプリは子プロセスで呼ぶだけ（[doc/web-apps.md](../../../doc/web-apps.md)）。

English summary: a marimo notebook app (`fmt_app.py`) that formats Modelica source and lists lint messages by calling the `globe` CLI (`GLOBE_BIN`, default `globe` on PATH). Japanese UI.

## 中身

| ファイル | 役割 |
| --- | --- |
| `fmt_app.py` | marimo のノートブック（画面）。入力・`.mo` の選択・インデント・origin・整形と lint のボタン・結果 |
| `globe_cli.py` | `globe` を呼ぶ関数（`format_text`・`lint_text`）。画面に依存しない |
| `sim_app.py` | `GlobeModeValidation.Simulation.Decay` を Python パッケージ `globe_mode` で実行し、`globe_mode.widget.SimulationPlot`（anywidget）で折れ線にする（S078。`uv run marimo run src/Python/marimo/sim_app.py`、`globe` は要らない） |
| `test_fmt_app.py` | 例題 `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo` で CLI を 1 往復させる検査 |

## 起動

`globe` のバイナリを用意する（手元では `cargo build -p globe-mode-cli` の `target/debug/globe`）。

```bash
GLOBE_BIN=/path/to/globe uv run marimo run src/Python/marimo/fmt_app.py
```

`GLOBE_BIN` を省くと PATH の `globe` を使う。編集しながら使うときは `marimo edit` に変える。

## 検査

```bash
GLOBE_BIN=/path/to/globe uv run pytest src/Python/marimo/test_fmt_app.py
uv run marimo check src/Python/marimo/fmt_app.py
uv run python src/Python/marimo/fmt_app.py   # 全セルを実行して終わる（エラーが出ないこと）
```

## 依存

`pyproject.toml` の `marimo>=0.25.1` を使う（このアプリ専用の追加は無い）。

| 名前 | 版 | ライセンス | 出典 URL | 確認日 |
| --- | --- | --- | --- | --- |
| marimo | 0.25.1 | Apache-2.0（インストール済みの METADATA の License-Expression で確認） | https://github.com/marimo-team/marimo | 2026-10-31 |
