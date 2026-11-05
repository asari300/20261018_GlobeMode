# S157 `cancel-partial-result`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s157-cancel-partial-result` |
| worktree | `.git-worktree/s157-cancel-partial-result` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S157-cancel-partial-result/` |
| セッション記録 | （親が写す） |

## 依頼

実行を中断したとき、そこまでの結果（列）を UI のプロットに残す。ブラウザ（wasm）と Desktop の両方で、止めるとそこまでの時系列がプロットに出て、止めた旨が出る。最小の変更で。再開、進捗の表示の作り替え、他言語への展開はしない。

## 決めたこと

- 現状を調べた結果、ネイティブ（exec・facade・LSP・Desktop の session の道のソルバ）は途中の列を既に返していた。wasm は `terminate()` で捨て、`App.svelte` は応答の `GM-I11004` を「止めた」として見せていなかった。exec・facade は変えない。
- wasm の合図は SharedArrayBuffer(4) のフラグ。cross-origin isolated でない配信、2 s 応えない実行は前のとおり `terminate()`（ユーザーが決めたのではなく、ADR-0033 が却下した理由「ヘッダを付けられるか未確認」を、効く配信でだけ効かせて避けた）。再実行で作る案、blob URL の revoke を合図にする案は却下（解説書 2 章）。
- 進捗の関数の戻り値は厳密に `true` のときだけ止める。`push` の戻り値のような真の値で意外に止まらないため。
- COOP / COEP は `vite.config.ts`（dev・preview）、`web-bundle.sh`（`_headers`・`nginx.csp.conf`）、`test_ui.py` の `serve()` に付けた。Tauri の `tauri.conf.json`（Android の WebView）には付けない【Android で効くかは未確認】。
- Desktop の既定 `auto` の積分の中の中断は、Rumoca に口が無いので持ち越し。

## やったこと

- `src/Rust/mod/globe-mode-wasm/src/lib.rs`: `simulate_columns` の進捗の関数が `true` を返したら `CancelToken` を立てる。
- `web/worker.js`・`web/client.js`: SharedArrayBuffer のフラグ、`cancel()` の猶予 2 s、`shared` のオプション。
- `src/Svelte/src/App.svelte`: `GM-I11004` の応答で「止めた（… そこまでの n 点を表示）」。
- `src/Svelte/vite.config.ts`・`scripts/web-bundle.sh`・`tests/test_ui.py`: ヘッダ、止めたあとプロットに線が残る確認。
- `node/smoke.mjs`・`web/worker-node.mjs`: 中断の項目、`shared: false` の作り直し。
- ADR-0033 に 6 段目、`doc/features.md`・`doc/roadmap.md`・索引、スライドと解説書。

## 検証

- `mise run wasm:node`: ok（中断: t = 34 で止まり 18 点と GM-I11004）。
- `worker-node.mjs`: ok（フラグで restarts 0、t = 86 までの 44 点。`shared: false` で reject、restarts 1）。
- `mise run ui:check`（svelte-check と `test_ui.py`・`test_graph.py`）: ok。止めたあとプロットに 64 点が残った（215 ms で「止めた」）。
- `mise run wasm:compare`: 400 件中 387 件が一致、既知の差 13、一致しない 0（S123 と同じ）。
- `mise run check`（fmt・clippy・test・deny・`uv sync --locked`）: 通った。
- Desktop（`desktop.yml`）: 回していない（Desktop の Rust は変えていない。共有の `App.svelte` の表示の分岐だけで、smoke の `cancel` の判定 `runState === "cancelled" || GM-I11004` は変わらない）。親が main の CI で確かめる。
- `check_docs.py`: 全デッキ OK、リンク 3168 件中の切れ 0 件。`render_slides.py`（S157 のデッキだけ）: 問題 0 件。
- セッション記録の `--check`: （親が写す。このセッションでは写さない）
- ステージした差分の漏洩検査: （下に追記）

## 残したこと

- Android の WebView に COOP / COEP を付けられるか【未確認】。付いていなければ作り直しのまま。
- Desktop の既定 `auto` の積分の中の中断（型付きの道に口が無い）。
- py・c・nif への展開、再開（resume）。
- Firefox・Safari での SharedArrayBuffer（isolated のとき）の動きは Chromium でしか確かめていない。
