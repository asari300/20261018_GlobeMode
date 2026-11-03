# S104 `marimo-wasm-msl-worker`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s104-marimo-wasm-msl-worker` |
| worktree | `.git-worktree/s104-marimo-wasm-msl-worker` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S104-marimo-wasm-msl-worker/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md)、[2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S102 の持ち越し: (1) MSL.json を widget に渡して ChuaCircuit を check → simulate し、メモリを `wasm_check.py` で測る（MSL はブラウザ版と同じく `_esm` で `fetch`、Pyodide を通さない）。(2) Web Worker で回し、simulate 中にメインのスレッドが応答することを 1 つ確かめる。(3) `--offline` の書き出しに合う最小の CSP を `wasm_check.py` のサーバに付け、違反 0 件で動かす。research に追記。Rust はビルドしない。`src/Svelte`・desktop・workflow は触らない。新しい依存なし。前任のエージェントがプロセスごと落ちた（load 100 超）ので、未コミットの `wasm_app.py`・`wasm_check.py` から再開した。

## 決めたこと

- Worker は新しく書かず、`globe-mode-wasm/web` の `client.js`（`GlobeWorker`）と `worker.js` を書き出し先の `wasm/` に写して `_esm` から `import()` した（ブラウザ版の `WorkerBackend` と同じもの）。前任の方針のまま。
- 固まらないことは、simulate の間に 20 ms の `setInterval` を回し、回数と間隔の最大で確かめる（回数 > 0・最大 < 1 s）。前任の方針のまま。
- CSP は前任の案（`script-src … blob:`、`worker-src 'self' blob:`）では動かなかった。anywidget が `_esm` を `data:` の URL で読み、marimo が `data:` の woff2 を読むため。`script-src` に `data:`、`font-src 'self' data:` を足し、そのうえで 1 項目ずつ外して 9 通り試し、`script-src` の `blob:` と `worker-src` を外した（要らなかった）。
- S102 で「`_esm` は blob の URL で読まれる」と書いたのは誤り（data: の URL）。`wasm_app.py` のコメントを直し、research に明記した（S102 の記録は書き換えない）。

## やったこと

- `src/Python/marimo/wasm_app.py`: `WasmSim` を `GlobeWorker` で回す形に（traitlet `pkg` → `wasm`、`msl` を追加）。MSL の ChuaCircuit のセルを追加。結果に `msl_ms`・`sim_ms`・`ticks`・`max_gap_ms`。
- `src/Python/marimo/wasm_check.py`: `client.js`・`worker.js`・`MSL.json` を写す、`--msl`・`--csp`、`--offline` のとき CSP の header と違反の数、ChuaCircuit と timer の assert。
- `src/Python/marimo/README.md`: 2 行を更新。
- `doc/research/marimo-wasm-2026-11-02.md`: 「MSL・Web Worker・CSP（2026-11-03、S104）」を追記。
- `doc/roadmap.md`・`doc/sessions/README.md`・`doc/sessions/S104-marimo-wasm-msl-worker/`。

## 検証

- `wasm_check.py --offline`: 2 回通過（最終の CSP で 1 回、`blob:`・`worker-src` を含む案で 1 回）。Decay x(1) = 0.367879468116207（誤差 2.7e-8）。ChuaCircuit: MSL の読み込み 0.7 s、check passed、simulate 0.5〜0.6 s、C1.v(T) = -0.9928894683235796。timer 26〜29 回・最大の間隔 20 ms。
- メモリ: JS heap used 36 MiB / total 40 MiB、wasm の線形メモリ Decay 2 MiB・ChuaCircuit 112 MiB、ブラウザの RSS の合計 1152〜1154 MiB（2 GiB 未満）。開いてから結果まで 14.9〜17.0 s。外への要求 0 件、CSP の違反 0 件。
- CSP の 1 項目ずつ外す試し（手元の使い捨ての Playwright のスクリプト、load average 10 未満で 1 本ずつ）: `script-src` の `'unsafe-inline'`・`data:`・`'wasm-unsafe-eval'` は外すと動かない（違反 4・2・1 件）、`style-src` の `'unsafe-inline'`・`img-src data:`・`font-src data:` は外すと動くが違反 2・6・1 件、`script-src` の `blob:`・`worker-src`・`worker-src` の `blob:` は外しても違反 0 件。
- 前任の CSP（`data:` 無し）では Chua も Decay も動かず違反 3 件。load 100 超の時は Chromium が 9〜35 s で落ちた（外から止められたと見られる。load が下がってから通った）。
- `uv run marimo check src/Python/marimo/wasm_app.py`: 警告 0。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。
- `check_docs.py`: デッキ 109 件 OK / 文書 433 件・相対リンク 2114 件中の切れ 0 件
- `render_slides.py`: S104 のデッキ 1 件・3 表示 × 7 枚、問題 0 件。初めは S01 の screenshot が load に関係なく 30 s で時間切れになった（S102 のデッキは通る）。1 枚ずつ試して S01 の `.lead` の「→」が原因と分かり、文言を変えて通した（`.lead` の中の「→」だけで固まる理由は【未確認】）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレスの形に当たった 1 件は `@app.cell` で誤検出）

## 残したこと

- emscripten の wheel（調査の道 (B)）。
- `wasm_check.py` は CI に入れていない（書き出しの時にネットが要る）。
- ChuaCircuit の simulate は 0.6 s と短い。長いモデルでの timer の間隔は測っていない。
