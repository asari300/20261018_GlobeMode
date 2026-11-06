# S201 `worker-plot-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s201-worker-plot-review-fixes` |
| worktree | `.git-worktree/s201-worker-plot-review-fixes` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S201-worker-plot-review-fixes/` |
| セッション記録 | （親が写す） |

## 依頼

main `cc846ba` の wasm の Worker（`web/client.js`・`worker.js`）とプロット（`lib/plot/`）のレビュー（Opus）の指摘 8 件を、失敗する試験で再現してから直す。ユーザーの決定（2026-11-06）: 同じモデルの再実行では、選び直した信号・右の軸・x 軸・絞り込みの文字・「内部の変数も表示」を保つ。「既定に戻す」のボタンを 1 つ。レビューの履歴は [log/review/2026-11-06-s201-worker-plot-review-fixes-opus.md](../review/2026-11-06-s201-worker-plot-review-fixes-opus.md)。

## 決めたこと

- 猶予の 2 秒を試験から短くする口は、既存に無かったので `GlobeWorker` のコンストラクタの第 3 引数に `graceMs` を足した（既定 2000）。
- `cancel()` は実行（`simulate_columns`）だけを見る。実行中でなければ（起動中・hover・lint などが走っている間）何もせず、実行が順番待ちなら印を付けて順番が来たところで cancelled で reject する。`shared: false` のときも `#running` を持つようにして、作り直す道は変えない。以前は実行中でない `cancel()` が何でも殺していたが、UI が呼ぶのは実行の中断だけ。
- 選択を保つ規則: 同じモデル名で利用者が選び直していたら、同じ名前の信号を選ぶ。無くなった信号は外し、全部無くなったら既定（絞り込み・内部名も戻す）。何も選んでいなかった（全部外した）なら空のまま。別のモデルは既定から（絞り込み・内部名も）。タブを切り替えると `columns` が null になって Plot ごと消えるので、タブごとの記憶は作り込まない。選び直していなければ今までどおり。
- 実装は `Plot.svelte` の中だけ。`App.svelte` は触っていない（`{model}` は今までも渡していた）。選択は `$derived` から `$state.raw` に替え、`$effect.pre` が `columns` の変わるたびに前の結果の名前で写し取る。
- 指摘 7 は UI の中で uPlot を投げさせにくいので、試験で canvas の `getContext` を一時的に投げさせて再現した。

## やったこと

- `src/Rust/mod/globe-mode-wasm/web/client.js`: 指摘 1〜4・6。
- `web/worker.js`: 指摘 8（`W()`、20 か所）。
- `web/worker-node.mjs`: 中断の経路の試験（1・2・3・4・5a・6）。
- `src/Svelte/src/lib/plot/Plot.svelte`: 指摘 7 と、選択を保つ処理、「既定に戻す」。
- `src/Svelte/tests/test_ui.py`: `plot_fixes`（選択の保持の 6 場合と、既定に戻すのキーボード、PNG の古い図）。
- 文書: 解説書・スライド・索引・features・roadmap・CHANGELOG。

## 検証

- 再現（直す前）: 指摘 1 は `restarts` が 1、2 は起動中の `cancel()` で `init` が cancelled で reject、3 は旧 `client.js` で実行が止まらず `stop_time` で終わる、4 は `restarts` が 2、6 は待つ実行が決まらない、7 は PNG が出る。プロットの保持は旧 Plot で `plot_fixes` が落ちた。
- `worker-node.mjs`: 通った（`ok`、中断の経路の行が出る）。`mise run wasm:node`: 通った。`mise run wasm:web`（Chromium の module Worker も）: 通った。
- `pnpm check`: 0 エラー・0 警告。`pnpm build`: 成功。`node tests/test_columns.mjs`: 通った。
- `test_ui.py`（全部。MSL あり）: 通った（最後の行 `ok`）。`GM-SMOKE` の行は変えていない（`tree=ok fmt=ok graph=ok cancel=ok settings=ok plot=ok def=ok isolated=1 cancel_path=flag`）。
- wasm は main の先頭 `688a3ca` を release（`mise run wasm:build`）でビルドして `~/.cache/globe-mode-s201/wasm-pkg` に置いて使った。
- `check_docs.py`: 切れ 0 件。
- android.yml 42176048464: success。
- ci.yml（full）1 回目 42176061490: ui ジョブだけ失敗。S196 の `review_fixes` の 1 つ（実行中の F12。`Long` の 1000 s が CI の速い機械では 1.5 s の待ちの間に終わり、「止める」が無い）で、S201 の変更とは無関係。S196 は ui ジョブを回していなかった。`StopTime` を 1000000 にして直した（手元で `review_fixes` と `plot_fixes` が通る）。2 回目 42179480527（full）: 全ジョブ success。

## 残したこと

- タブごとの選択の記憶と、再読み込みをまたぐ保存。
- 1,302 本 × 数万点のピークメモリ（約 2 倍）は【未確認】。
- `test_columns.mjs` には足していない（保持の規則は Plot の中で、UI の試験で確かめた）。
