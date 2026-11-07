# S214 `lsp-bindings-issues` のレビュー（ponytail-audit、親が立てた issue）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s214-lsp-bindings-issues` |
| 対象 | GitHub issue #25・#30・#33・#35・#37・#39（2026-11-07 の `/ponytail-audit` 5.3.2。読んだだけで実行していない） |
| レビュアー | Claude Code の ponytail（`/ponytail-audit`） |
| 依頼の文面 | LSP と言語バインディングの 6 件を、いまの main で 1 件ずつ再現してから直す |
| 作業記録 | `log/work/2026-11-07-s214-lsp-bindings-issues.md` |

## 指摘（原文のまま）

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 33 | 16/20 級（enhancement） | `globe-mode-lsp/src/lib.rs:234-242`・`642-694`・`788` | `didChange` のたびに lint・整形・最上位モデル全部の `ws.check()` を 1 要求ずつ回し、積まれた編集をまとめない。検査 1 秒のモデルで 10 文字打つと 10 回の全検査が積まれ、補完とホバーが約 10 秒遅れる。`globe/simulate` の既定 `bdf`/`auto` はネイティブの道で、`CancelToken` を段の間でしか見ない。LSP は既定の `max_wall_seconds` を置かない。直し方: `for got in rx` の中で `try_recv` で読み切り、同じ URI の新しい `didChange` が待っていれば古いのを捨てる。LSP の実行には要求に無ければ既定の `max_wall_seconds` を与える。 | 採用（前半と予算。積分中の `$/cancelRequest` は持ち越し） | 再現した（50 回の `didChange` で `publishDiagnostics` が 51 回）。積まれた編集をまとめて 2 回に、`globe/simulate`・`globe/simulateColumns` に 180 s の既定の予算。ネイティブの積分が中断を段の間でしか見ない件は実行系の変更になるので触らない。 |
| 25 | 4/20（bug） | `globe-mode-lsp/src/lib.rs:632-637`、引き金は `globe-mode-frontend/src/rumoca.rs:906` | `update` は `ws.set_file` → `diagnostics()` → `self.texts.insert` の順。`compile_model_strict` に `catch_unwind` が無く、Rumoca が panic すると `texts` が前の版のままになり、次の整形が古い本文から編集を作って最新の編集を上書きする。直し方: `self.texts.insert` を `diagnostics` の上に動かす（1 行）。別に `compile_model_strict` を `catch_unwind` で包む。 | 一部採用 | **再現できない**: Rumoca が panic する入力が、いまの main では見つからない（注入の口も無い）。順序は読んで確かめた（`diagnostics` は `self.texts` を読まず、`update` の外の失敗でも本文が最新になる方が安全）ので `texts.insert` を前に動かした。通知の道の `catch_unwind` は S198 のリクエスト単位のものが効く。`compile_model_strict` の周りは `globe-mode-frontend` の変更になるので持ち越し。 |
| 37 | 16/20（enhancement） | `globe-mode-frontend/src/rumoca.rs:835`（`Checker::reset`）、`1322-1338` | 長く生きる Workspace（LSP・Python・C・NIF）が `Checker::reset` を呼ばず、メモリが増える一方。直し方: xtask と同じく、前回の reset から検査した異なるモデルの数が閾値を越えたら `checker.reset()` を呼ぶ。 | 採用（LSP 側は何もしない） | LSP は facade の `Workspace` の上に乗っている（`Server.ws`・`scratch` とも）。`Checker` を持たないので、S212 が facade に入れる規則がそのまま効く。 |
| 39 | 18/20（bug） | `src/VSCode/globe-mode/extension.js:46`、`globe-mode-lsp/src/lib.rs:519` | VS Code は独自の `globe/format` で常に `indent: options.tabSize` を送り、サーバはその値を設定ファイルより優先する。`[fmt] indent = 4` がありタブ幅が 2 なら、保存時の整形は 2 空白で書き、CI の `globe fmt --check src/Modelica` が落ちる。`textDocument/formatting` は設定が勝つ。直し方: `globe/format` でも同じ規則を使う。 | 採用 | 再現した（設定 4 に `indent: 2` を送ると 2 空白）。`Server::indent` を 1 つにして両方が使う。設定ファイルが無いときだけ要求の値（ブラウザ版の UI）。 |
| 35 | 14/20（bug） | `globe-mode-nif/src/lib.rs:59-63`（`with`）、`91-121` | Rustler は `f` の中の panic を捕まえるが Mutex は poisoned のまま、以後の呼び出しは全部失敗する（C ABI は使えるまま）。`check`・`graph`・`outline`・`deps`・`flat` は `DirtyCpu` で Mutex を待って塞がる。直し方: `lock().unwrap_or_else(PoisonError::into_inner)`。Mutex を待つ NIF は `DirtyIo` に。 | 採用 | 再現した（panic の後の `set_file` が `{:error, "ワークスペースが panic で壊れた"}`）。`with` を `into_inner` にし、Mutex を取る NIF を DirtyIo に。試験用の NIF `panic_for_test` を足した。 |
| 30 | 9/20（bug） | `src/Julia/globe_mode.jl:75-85` | 進捗コールバックの中で `f` が throw するか Ctrl-C を押すと、Julia が Rust のフレームを越えて巻き戻す。ヘッダが禁じている未定義動作で、「使用中」の印が消えず、以後その workspace への `gm_*` はすべて「使用中」。直し方: `try … catch` で包み、ccall が返ってから投げ直す。`GC.@preserve ws` も付ける。 | 採用 | 再現した（`error("boom")` のあと `check` が「ワークスペースは使用中」）。例外を `Observer` に退避して中断（`true`）を返し、`ccall` の後に投げ直す。`GC.@preserve ws o`。 |

## 結論

- 採用 5 件（#33・#39・#35・#30、#37 は LSP 側は何もしない）、一部採用 1 件（#25）、不採用 0 件。
- #25 の再現は Rumoca の panic の入力が無くてできなかった。#33 の積分中の中断は持ち越し。
- 依頼の文面の #39（`tabSize` が無視される）は issue の本文と逆で、issue の本文（要求が設定に勝つ）が実際の症状だった。
