# S152 `ui-dark-mode`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s152-ui-dark-mode` |
| worktree | `.git-worktree/s152-ui-dark-mode` |
| Phase | P4 / P7 |
| セッション文書 | `doc/sessions/S152-ui-dark-mode/` |
| セッション記録 | （親が写す） |

## 依頼

Svelte 版 UI（ブラウザ・Desktop・Android 共用）のダークモード対応。OS に追従し、手動で 明 / 暗 / OS に合わせる。配色は CSS の変数。CodeMirror・uPlot・sigma も追従。試験 1 本。Leptos 版・Dioxus 版、スライドの CSS、色のカスタマイズ UI は対象外。

## 決めたこと

- 変数は 9 本（背景・文字・弱い文字・境界・罫線・ホバー・選択・強調・警告・誤り）。依頼の 5〜8 色より 1 本多いのは、エディタの選択範囲の色を別に要したため（`--sel`）。
- CodeMirror は色を変数で書き、`darkTheme` だけ `Compartment`。タブごとの `EditorState` にも反映する（そうしないと切り替えた先が元の明暗のまま）。
- PNG の書き出しは「いまの背景色」で塗る。依頼は「白地のままでよい」だったが、暗いときは軸の文字が明るく、白地では読めないため。明のときは白のまま。
- sigma は 4 系で `styles` を使う（v3 の `labelColor` 設定は無い）。`DEFAULT_STYLES` を展開して `labelColor`・`backdropColor` だけ差し替えた。
- 試験で軸の文字は「明るい画素を数える」では格子の `#ddd` に当たって判定できず、いまの文字色に近い画素を数える形にした。

## やったこと

- 新規: `src/Svelte/src/theme.css`、`theme.svelte.ts`
- 変更: `App.svelte`（select、`darkSlot`・`modelicaTheme`、タブへの反映、色の変数化）、`LibraryTree.svelte`、`SimSettings.svelte`、`Plot.svelte`、`Graph.svelte`、`lib/editor.ts`、`main.ts`、`tests/test_ui.py`
- 文書: `doc/features.md`（Text ビューの行）、`doc/roadmap.md`（P7 の行と「S152 で済んだこと」）、`doc/sessions/README.md`、`doc/sessions/S152-ui-dark-mode/`

## 検証

- コードの検査: Rust は触っていない（cargo は回していない）。`pnpm check` 0 エラー 0 警告
- `test_ui.py`: 通過（GM-SMOKE は全項目 ok。追加の 1 本を含む）。`test_graph.py`: 通過
- `check_docs.py`: 全デッキ OK（S152 を含む）/ 相対リンク 3063 件中の切れ 0 件。`render_slides.py` は S152 のデッキだけで 1 回目に通った（問題 0 件）
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（メール・ホームのパス・トークン）

## 残したこと

- Leptos 版・Dioxus 版（S153 から）、色のカスタマイズ UI、プロットの系列の色の暗用の調整
- Desktop（Tauri）・Android の実機での見た目は未確認（ブラウザの Chromium だけ）。Android の WebView は `prefers-color-scheme` を OS に合わせるので、同じ CSS で動くはず【未確認】
