# S201 `worker-plot-review-fixes` のレビュー（Opus/medium）

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s201-worker-plot-review-fixes` |
| 対象 | main `cc846ba` の `src/Rust/mod/globe-mode-wasm/web/{client.js,worker.js,worker-node.mjs}`・`src/Svelte/src/lib/worker-backend.ts`・`lib/plot/`（読むだけのレビュー） |
| レビュアー | Opus（medium、読んで論証しただけ。再現はこのセッションでした） |
| 依頼の文面 | wasm の Worker とプロットの指摘 8 件を、失敗する試験で再現してから直す |
| 作業記録 | `log/work/2026-11-06-s201-worker-plot-review-fixes.md` |

## 指摘（原文のまま）

```text
1. [正しさ・確認済み・Must] web/client.js:165-172 cancel() が呼ぶたびに this.#grace = setTimeout(kill) で上書き。応答で止まるのは最後のタイマーだけ → 止めるを 2 回（Ctrl+. のキーリピート）押すと、フラグで止まった 2 秒後に残ったタイマーが今の worker を殺す（restarts+1、MSL の再生、その間の次の実行が cancelled で reject）。直し方: タイマーを置く前に clearTimeout(this.#grace)。
2. [正しさ・仕組みは確認済み] client.js:165-169,84-87 起動（init）中に止めると #die が「起動で落ちた」扱いになり #ready が永久に reject（ページを読み直すまで使えない）。直し方: cancel() の頭を `if (!w || !this.#alive || !this.#pending.size) return;`。
3. [正しさ・仕組みは確認済み] client.js:169 #running は simulate_columns を送ったときだけ。実行がキューで待つ間に止めると、前の hover・lint が殺され worker が作り直され、simulate は止まらず走る。直し方: #cancelQueued の印を持ち、#raw が simulate_columns を送る前に見て cancelled で reject。simulate 以外は殺さない。
4. [正しさ・可能性] client.js:34-41,69 message のリスナーがどの worker から来たかを見ない（error・exit は w !== this.#w で守っている）。捨てた worker の遅れた panic の応答で新しい worker が殺されうる。直し方: `(m) => w === this.#w && this.#reply(m)`（2 か所）。
5. [試験・確認済み] web/worker-node.mjs:58-90 2 秒の猶予が切れて terminate() に落ちる道と、2 回止める道の試験が無い。直し方: 進捗を出さない呼び出しで cancel() → cancelled で reject、restarts === 1。`cancel(); cancel();` の後 2.5 秒待って restarts === 0。
6. [正しさ（小）・確認済み] client.js:173 公開の terminate() が #pending を reject しない。直し方: pending を reject して空にし #ready = Promise.reject(...)。
7. [正しさ（小）・確認済み] plot/Plot.svelte:84,112 new uPlot が投げると plot が destroy 済みのまま（PNG に古い図、destroy 二重）。直し方: 後片付けを `() => { plot?.destroy(); plot = undefined; }`。
8. [簡素化] web/worker.js:33-78 `(ws ??= new gm.Workspace())` が 22 回。`const W = () => (ws ??= new gm.Workspace());`。
問題なし: id、中断のフラグの戻し、転送、NaN/Inf、CSV のエスケープ、ファイル名、defaults。未確認: 1,302 本 × 数万点のピークメモリ（約 2 倍）。
```

## 採否

| # | 再現 | 採否 | 理由・試験 |
| --- | --- | --- | --- |
| 1 | した（`restarts` が 1 になった） | 採った | `cancel()` が猶予のタイマーを置く前に `clearTimeout`。`worker-node.mjs`「1」: 実行中に `cancel(); cancel();` のあと猶予（試験は `graceMs: 300`）の倍待っても `restarts === 0` |
| 2 | した（起動中の `cancel()` で `init` が cancelled で reject され、`#ready` が reject のまま） | 採った | 実行中でなければ `cancel()` は何もしない（頭の `!this.#alive` の代わりに、実行の id が無いことで見る）。試験「2」: 起動中に `cancel()` しても `version()` が通り `restarts === 0` |
| 3 | した（前の呼び出しが送られる前の `cancel()` は何もせず、実行は止まらず終わった） | 採った | `#queued`（順番待ちか実行中の `simulate_columns` の数）と `#cancelQueued`。順番が来たところで cancelled で reject。`simulate_columns` 以外は殺さない。試験「3」: 前の `simulate` が殺されず、待っていた実行だけが cancelled |
| 4 | した（捨てた worker の poisoned な応答で `restarts` が 2 になった） | 採った | `message` のリスナー 2 か所を `w === this.#w && …` に。試験「4」: 記録した最初の worker のリスナーに遅れた応答を流しても `restarts === 1` |
| 5 | 試験が無いのを確認 | 採った（試験を足した） | 試験「5a」: Worker にフラグを渡さない（`postMessage` で `cancel` を外す）応えない実行は、猶予のあと作り直して cancelled で reject、`restarts === 1`。「1」が `cancel(); cancel();` の道 |
| 6 | した（待っている実行が 5 秒たっても決まらない） | 採った | `terminate()` が待っている呼び出しを reject し、以後の呼び出しも reject（`#ready` を reject に）。試験「6」 |
| 7 | した（`new uPlot` が投げたあと PNG が出た。canvas の `getContext` を投げさせて再現） | 採った | 後片付けを `plot?.destroy(), plot = undefined` に。`test_ui.py` の `plot_fixes` の 7 |
| 8 | （簡素化） | 採った | `worker.js` の `(ws ??= new gm.Workspace())` 20 か所を `W()` に。挙動は同じ |

## 結論

- 採用 8 件・保留 0 件・不採用 0 件。
- 指摘 1 の「2 回目の `cancel()` で Worker が殺される」は、猶予 2 s を実時間で待たないよう `GlobeWorker` のコンストラクタの第 3 引数に `graceMs` を足して再現した（既定は 2000 ms のまま）。
- 指摘 3 は、レビューの「前の hover・lint が殺される」ではなく、実際は `cancel()` が呼ばれた時点で前の呼び出しがまだ `pending` に無く（`#call` は直列のキューを非同期に進める）、何も起きないのが常に先に出る失敗だった。どちらも直った（実行だけを順番待ちの中で止める）。
