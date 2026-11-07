# GlobeMode 整形・lint（Streamlit）

GlobeMode の整形器と lint を、Streamlit の 1 ページの画面で使う。計算は `globe` の CLI（`globe fmt --json -` / `globe lint --json -`）を子プロセスで呼んで行い、この画面は表示と入出力だけを受け持つ（[doc/web-apps.md](../../../doc/web-apps.md)）。

English summary: a one-page Streamlit UI for the GlobeMode formatter and linter. It calls the `globe` CLI (`GLOBE_BIN`, default `globe`) as a subprocess; no formatting logic lives here.

## 起動

このディレクトリは独自の `pyproject.toml` と `uv.lock` を持つ（ルートの依存には入れない）。

```bash
cd src/Python/streamlit
uv sync
GLOBE_BIN=/path/to/globe uv run streamlit run fmt_app.py   # GLOBE_BIN を省くと PATH の globe
```

手元のバイナリは `cargo build -p globe-mode-cli` で作る `target/debug/globe`。

## 検査

```bash
cd src/Python/streamlit
GLOBE_BIN=/path/to/globe uv run pytest
```

`test_fmt_app.py` は `streamlit.testing.v1.AppTest` で画面を動かし、例題 `src/Rust/mod/globe-mode-fmt/tests/data/objects_before.mo` を整形して `origin = {0, 0}` が入ること、lint の表が出ること、文法の壊れた入力でエラーが出ることを確かめる。`GLOBE_BIN` が無い、または見つからないと失敗する。

## 依存

| 名前 | 版 | ライセンス | 出典 | 確認日 |
| --- | --- | --- | --- | --- |
| streamlit（実行時） | 1.66.1 | Apache-2.0 | https://github.com/streamlit/streamlit/blob/develop/LICENSE （ライセンス文を確認。版は `uv.lock` と導入済みパッケージの metadata で確認） | 2026-10-31 |
| pytest（開発用、配布物に入らない） | 9.2.0 | MIT | https://github.com/pytest-dev/pytest （導入済みパッケージの metadata で確認） | 2026-10-31 |

`uv.lock` には 41 件の package（プロジェクト自身を含む）が記録されている。Streamlit の推移的依存のライセンスは個別には確認していない【未確認】。

## 注意

- 配布物の `doc/licenses.md` への写しは、ルートの依存と同じくあとでまとめて行う。
