# S128 `ui-plot-series`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s128-ui-plot-series` |
| worktree | `.git-worktree/s128-ui-plot-series` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S128-ui-plot-series/` |
| セッション記録 | （親が写す） |

## 依頼

features.md のプロットの行の「未」: 系列の選択、2 軸、パラメトリック（x に時刻以外の変数）。列が多いとき（数百）に重くならないこと。cargo は回さず UI だけ。App.svelte は props を渡す最小限に。

## 決めたこと

- 選んだ系列だけを uPlot に渡し、選択が変わるたびに作り直す（既定のまま進めた）。
- 2 軸は系列ごとの「右」のチェックで scale `y2`。軸は使う scale の分だけ出す。
- パラメトリックは uPlot の mode 1 のまま、x の範囲を列の最小・最大で渡し、拡大を止める（`shortcut:` の注記。mode 2 は凡例と scale の作りが変わるので採らなかった）。
- App.svelte は変えていない（`columns` の props だけで足りた）。

## やったこと

- `src/Svelte/src/lib/plot/Plot.svelte`: 選択・右の軸・x の列。`data-series`・`data-right`・`data-x` を出す。系列のチェックボックスに `name="signal"`。
- `src/Svelte/tests/test_ui.py`: 信号のチェックボックスの選び方を `name=signal` に。Decay で選択・右の軸・x、ChuaCircuit で `C1.v` 対 `C2.v`。
- 文書: features.md・roadmap.md・索引・セッション文書。

## 検証

- コードの検査（fmt / clippy / test / deny）: Rust を触らないので回していない
- `pnpm check`: 260 ファイル、error 0・warning 0
- `test_ui.py`: ok（Decay: data-series=1、data-right=1、data-x=x。ChuaCircuit: 列 44 本、C1.v 対 C2.v で系列 4 本。10 万点の最初の描画 20.8 ms）
- `check_docs.py`: S128 のデッキ OK / 文書 535 件・相対リンク 2,681 件中の切れ 0 件。`render_slides.py`: 6 枚、問題 0 件
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- パラメトリックのときの拡大（uPlot の mode 2）。
- 凡例の装飾、MAT の読み込み、画像の保存、ダークモード。
- パラメトリックの線の見た目は目視していない（試験は DOM と error 0 だけ）。
