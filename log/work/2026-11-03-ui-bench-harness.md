# S125 `ui-bench-harness`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s125-ui-bench-harness` |
| worktree | `.git-worktree/s125-ui-bench-harness` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S125-ui-bench-harness/` |
| セッション記録 | （親が写す） |

## 依頼

P7 の完了条件「`doc/bench/ui.md` に 3 UI × 3 OS の比較を書く」の前段として、負荷の測定ハーネスを作り、今ある Svelte 版（ブラウザと Tauri のデスクトップ）の数字を Linux と Windows で出す。Leptos 版・Dioxus 版はまだ無いので「未」にし、同じハーネスで測れる形にする。測るもの: 起動から操作可能まで、Decay の check・simulate・プロット、MSL を読んで ChuaCircuit を check、そのときの RSS、バンドルの大きさ。測り方の ADR（Proposed、しきい値なし）。新しい依存なし。`globe-mode-*` の mod の Rust・`ci.yml`・`android.yml`・`release.yml` は触らない（親のサブエージェントとして。main へのマージ・push は親）。

## 決めたこと

既定のまま進めた（ユーザーとの相談は無い）。詳細は [ADR-0034](../../doc/adr/0034-ui-bench-method.md)。

- 測る道は新しく作らず、S100 の `GM-SMOKE` の行の末尾（`failure=` の前）に `ready_ms`・`decay_ms`・`msl_ms` を足した。待ち手（`test_ui.py`・`android.yml`）は先頭で照合するので壊れない。
- Desktop は UI に同じ手順を回させる。`GLOBE_DESKTOP_BENCH=<ファイル>` のとき、UI が起動時に command `bench` を呼び、行をファイルに書いてアプリを終える。Windows の GUI のアプリは stdout を持たないので、ファイルにした。ブラウザと揃えるため、このときは取得済みの MSL を起動時に渡さない（MSL を読むのは UI の手順の中）。
- RSS はプロセスごとの最大の 1 つ（S056 の `time -v` と同じ意味）。合計にしない。
- 表は `bench_ui.py report` が `target/ui-bench.json` から作る（cargo のビルドを避けて Python）。3 回の中央値。
- macOS は matrix に無いので「未」。
- Linux の Desktop で UI が描かれない原因を 3 回の切り分けで特定した（下）。製品の直しはせず、測定の手順に `LANG=en_US.UTF-8` を付けた。

## やったこと

- `src/Svelte/src/App.svelte`: smoke に時刻 3 つ。Desktop のとき `tauri.bench()` が true なら smoke を回す。
- `src/Svelte/src/lib/tauri-backend.ts`: `bench(line?)`。
- `src/Rust/apps/desktop/src/main.rs`: command `bench`（async）、`GLOBE_DESKTOP_BENCH` のときは起動時の MSL の探索をしない。
- `.github/workflows/desktop.yml`: Linux（xvfb、`time -v`）と Windows（pwsh、`PeakWorkingSet64` を 0.2 s ごと）で 3 回ずつ測り、artifact `ui-bench-<os>`。Windows は先に Rust の smoke で MSL を取得する（Windows で smoke が回るのはこれが初めて）。
- `src/Svelte/tests/bench_ui.py`（新規）: `browser`・`add`・`report`。自己検査（行の解析と表）付き。`test_ui.py` は `serve()` を切り出し、行の時刻の assert を 1 つ足した。
- `doc/adr/0034-ui-bench-method.md`（新規）、`doc/adr/README.md`、`doc/architecture.md` の 6 章に 1 行、`doc/bench/ui.md`（新規、生成物）、`doc/roadmap.md`。

## Linux の Desktop で UI が描かれなかった件

| run | 変えたこと | 分かったこと |
| --- | --- | --- |
| 41599680670 | 最初 | Windows は 3 回 ok。Linux は 600 s で時間切れ（行が無い） |
| 41604013928 | command を async に | 変わらず |
| 41607420553 | `bench` の呼び出しを stderr に | UI が `bench` を 1 度も呼んでいない |
| 41610599773 | 90 s 後にページの様子を出す | `__TAURI__` はあり、`readyState` は complete、本文は空 |
| 41614535845 | module を import し直す | import は ok、CSP の違反は無い |
| 41617814119 | 初めからの `error` を集める | `RangeError: invalid language tag: C`（`main-*.js` の 10 行目） |

- 原因: ランナーの既定（LANG=C）で WebKitGTK の既定の言語が `C` になり、CodeMirror（`@codemirror/commands`）の `Intl.Segmenter` が RangeError を投げて、module の評価が止まる（ブラウザ・Windows では起きない）。
- 製品への影響: Linux で `LANG=C`（や `POSIX`）のまま Desktop を起動すると、画面が空になる【CI の xvfb でだけ確かめた。実機は未確認】。S056 以降の Linux の smoke は Rust 側だけで UI を通らないので、気づかなかった。直すのは持ち越し。

## 検証

数（中央値、3 回。[doc/bench/ui.md](../../doc/bench/ui.md)）:

| 測るもの | Svelte ブラウザ Linux（手元） | Svelte Tauri Linux | Svelte Tauri Windows |
| --- | ---: | ---: | ---: |
| 起動から操作可能まで（ms） | 1,232 | 114 | 178 |
| Decay の検査・実行・プロット（ms） | 258 | 54 | 43 |
| MSL を読み ChuaCircuit を検査（ms） | 2,800 | 1,045 | 1,927 |
| 最大の RSS（MiB） | 345.5 | 151.6 | 128.7 |
| バンドル（MiB） | 15.6 | 22.7 | 20.6 |

- Desktop: `desktop.yml` run 41622934035（ui・Linux・Windows とも success。Linux は 3 回とも ok、Windows も 3 回とも ok）。Windows の最初の run 41599680670 も 3 回 ok（起動 179 ms、MSL 1,798 ms、RSS 128.8 MiB）。
- ブラウザ（手元）: 開始時の load 9.05。1 回目 ready 1,232・decay 258・msl 2,800 ms、2 回目 1,287・225・3,286、3 回目 272・**100,525**・2,441（3 回目の Decay だけ跳ねた。並行の重い仕事で load が上がった。中央値には効かない）。最初の試行（load 1.1 で開始）は 2 回目で Chromium が落ちた（`Target page, context or browser has been closed`。並行で load が 20 を超えた）。
- `bench_ui.py` の自己検査（行の解析と表）: 通った。
- `pnpm check`（svelte-check）: 0 errors。
- desktop の Rust: 手元に webkit2gtk が無いので `rustfmt --check` だけ。fmt・clippy（-D warnings）・試験は run 41622934035 の Linux で通った。ルートの workspace の Rust は触っていない。
- `check_docs.py`: デッキ OK / リンク切れ 0 件。markdownlint（新しい 4 文書）: MD013（行の長さ）以外 0 件。
- セッション記録の `--check`: 親が写す。
- `test_ui.py`（dist を作り直して、load 6.8）: ok。`GM-SMOKE … ready_ms=322 decay_ms=261 msl_ms=3820`。
- `uv sync --locked`: 通った。`mise run check` は回していない（ルートの workspace の Rust を触っていない）。
- **`render_slides.py`: 通っていない**。S125 のデッキも、前に通った S121 のデッキも 1 枚目の `Page.screenshot` が 30 s で時間切れになる（load 4〜7、rustc 0 本のときも同じ）。手元の Chromium の環境の問題と見て、デッキの直しはしていない。マージの前に回し直すこと。

## 残したこと

- Leptos 版・Dioxus 版（`src/Rust/apps/experimental/`）。同じ `GM-SMOKE … ready_ms=…` の行を出せば `bench_ui.py add` で表に載る。
- Linux の Desktop の `LANG=C` で画面が空になる件の製品側の直し（実機の確認も）。
- ブラウザの Windows 列（Actions の Windows で wasm の dist を配る）、macOS の列（matrix に足すかはユーザー）、Android の列。CPU・フレーム時間・PSS。
- `ready_ms` の 0 はページの読み込みの始まり。プロセスの起動から webview を作るまでは含まない（ADR-0034 の結果）。
