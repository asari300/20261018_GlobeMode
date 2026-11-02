# S078 `marimo-anywidget`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s078-marimo-anywidget` |
| worktree | `.git-worktree/s078-marimo-anywidget` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S078-marimo-anywidget/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S039 の持ち越しの anywidget。`globe_mode.widget.SimulationPlot(result)` が `Workspace.simulate` の結果を anywidget の `_esm` で描く（依存なしの `<canvas>`、信号のチェックボックス、整形は Python の 1 関数）。Decay を描く marimo ノートブック `sim_app.py`。traitlets を確かめるテスト 1 本。Rust は変えない。main へのマージ・push は親が行う。

## 決めたこと

- 描画は依存なしの `<canvas>`（約 60 行の JS）。uPlot は使わない（CDN はオフラインで開けない。npm から同梱するとビルドの段と 50 KB 前後の同梱物が増える。Decay の確認には軸の端の値で足りる）。
- `anywidget` は extra `widget` ではなく通常の依存に足した。ci.yml の `python`（`uv sync --locked`）と release.yml の wheel の pytest（`uv run --with wheel/*.whl`）がどちらも extra を入れず、extra にすると両方を変える必要があるため。代償は wheel を入れる人に推移的に 18 本（ipython・ipywidgets・traitlets など）が増えること。軽くしたくなったら extra に移し、上の 2 か所に `--extra widget` / `[widget]` を足す。
- anywidget のライセンスは MIT（依頼文の BSD-3-Clause ではなかった。METADATA の `License-Expression: MIT`）。推移的依存は MIT・BSD-3・ISC・Apache-2.0 で、すべて許可。ADR は要らない（設計の選択肢を減らさない、条件付きでない）。
- 整形は `plot_data(response)`（既存の `table()` で polars に読み、列を list に）。traitlets は `time: List(Float)`・`series: Dict`・`visible: List(Unicode)`。`visible` はチェックボックスと同期し、Python からも読み書きできる。
- marimo のノートブックは `mo.ui.anywidget(...)` で包む（`fmt_app.py` と同じく 1 セル 1 役割、`app_title`、`if __name__ == "__main__": app.run()`）。モデルは `Path(__file__).parents[2] / "Modelica"` を `load_dir` で読む（`test_api.py` と同じ）。

## やったこと

- `pyproject.toml`・`uv.lock`: `anywidget>=0.11.0`
- `src/Python/mod/globe_mode/widget.py`: `plot_data`・`SimulationPlot`
- `src/Python/tests/test_widget.py`: Decay の traitlets（501 点、x(1) が e⁻¹、`visible` の既定）と、実行できなかった結果の ValueError
- `src/Python/marimo/sim_app.py` と README の 1 行
- `doc/licenses.md`・`doc/roadmap.md`（S078 の節、S039 の持ち越しに取り消し線）・`doc/sessions/README.md`・セッションのスライドと解説書

## 検証

- コードの検査: Rust は変えていない。`uv sync --locked` は通った。`uv run pytest -q src/Python/tests`: 4 passed
- `uv run marimo check src/Python/marimo/sim_app.py`: 0。`uv run python src/Python/marimo/sim_app.py`: 終了コード 0
- `marimo export html` の HTML を headless Chromium で開いた: canvas 1、コンソールのエラー 0、折れ線を目で確認（手元だけ）
- `mise run ci:plan`（コミット後）: `python` と `docs`・`slides` が走る（Rust・deny は走らない）
- `check_docs.py`: デッキ OK（S078 を含む全件）/ 文書 337 件・相対リンク 1630 件中の切れ 0 件
- `render_slides.py`: 1 回試して `Page.screenshot` が 30 s で時間切れ（手元の負荷）。CI の `docs` ジョブに任せる
- セッション記録の `--check`: 親が写すので未実施
- ステージした差分の漏洩検査（メール・ホームのパス・トークン）: 0 件

## 残したこと

- Arrow での受け渡し（S039 の持ち越しのまま）。点数の多い結果では JSON の list が重い
- 目盛り・ズーム・ホバー。要るなら uPlot を同梱する
- marimo の WASM（Pyodide）で wheel を使う道
- ブラウザでの widget の検査は自動化していない（pytest は traitlets だけ）
