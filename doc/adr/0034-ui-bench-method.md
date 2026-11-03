# ADR-0034 UI の比較は GM-SMOKE の行に時刻を足して測り、3 回の中央値を doc/bench/ui.md の表にする

- 状態: Proposed（2026-11-03、S125。Svelte 版だけを測った。Leptos 版・Dioxus 版を作るときに同じ手順で測る）
- 関係: [ADR-0023](0023-desktop-and-web-process-layout.md)（ブラウザは Worker の wasm、Desktop は sidecar）、[ADR-0030](0030-desktop-msl-placement.md)（Desktop の MSL の置き場）。どれも覆さない

## 背景

- [roadmap](../roadmap.md) の P7 の完了条件に「`doc/bench/ui.md` に 3 UI × 3 OS の比較を書く」がある。[architecture](../architecture.md) の 6 章は測定項目を「起動時間、RSS / PSS、CPU、フレーム時間、バンドルサイズ」と書いている。
- 既に、UI の端から端までを 1 回で通す道がある: `?smoke=1`（と Android のビルド時の `VITE_GM_SMOKE=1`）で起動すると、UI が Decay を検査・実行し、MSL を読んで ChuaCircuit を検査し、console に `GM-SMOKE check=ok simulate=ok x1=… msl=ok` の 1 行を出す（S100・S103）。`test_ui.py` と `android.yml` がこの行を待つ。
- Desktop の smoke（S056・S069）は Rust 側から sidecar に頼むもので、UI（webview）を通らない。最大の RSS は `/usr/bin/time -v` の Maximum resident（プロセスの木の中の最大の 1 つ）で取っている。
- Leptos 版・Dioxus 版（`src/Rust/apps/experimental/`）はまだ無い。

## 決定

1. **何を測るか（最初の 5 つ）**

   | 項目 | 定義 |
   | --- | --- |
   | `ready_ms` | ページ（webview）の読み込みの始まり（`performance.now()` の 0）から、計算の相手（Worker の wasm か sidecar）との最初の往復（`setFile`）が戻るまで |
   | `decay_ms` | その後、Decay の検査・実行が済み、プロットの DOM に反映される（Svelte の `tick()`）まで |
   | `msl_ms` | MSL を読み始めてから ChuaCircuit の検査が戻るまで。ブラウザは `MSL.json` の取得と `library`、Desktop は取得済みの置き場を `fetch_msl` で sidecar に渡し、sidecar が読み直す |
   | `rss_kib` | 1 回の間の最大の常駐メモリ。プロセスごとの最大のうち最も大きい 1 つ（合計ではない）。Linux はブラウザが Chromium の子孫の `VmHWM`、Desktop が `time -v`。Windows の Desktop は 0.2 s ごとに見た `PeakWorkingSet64` |
   | `bundle_kib` | 配るものの大きさ。ブラウザは `dist/`（wasm を含み、`msl/` を除く）、Desktop は .deb / .msi |

   CPU・フレーム時間・PSS は持ち越す（UI が 1 つしか無い今は比べる相手が無く、測り方の決め手も無い）。

2. **どう測るか**: UI は `GM-SMOKE` の行の末尾（`failure=` の前）に `ready_ms=… decay_ms=… msl_ms=…` を足す（既存の待ち手は先頭で照合しているので壊れない）。
   - ブラウザ: `src/Svelte/tests/bench_ui.py browser` が `dist/` を配り、Chromium を回ごとに起動し直して `?smoke=1` を開く（cache を持ち越さない）。
   - Desktop: `GLOBE_DESKTOP_BENCH=<ファイル>` で起動すると、UI が同じ手順を回し、command `bench` で行をそのファイルに書いてアプリを終える。ブラウザと揃えるため、このときは取得済みの MSL を起動時に渡さない。`desktop.yml` が Linux（xvfb）と Windows で回し、`rss_kib=` と `GM-BUNDLE kib=` を足した `bench.txt` を artifact `ui-bench-<os>` に残す。Windows は先に Rust の smoke（`GLOBE_DESKTOP_SMOKE_FETCH`）で MSL を取得しておき、測る回にダウンロードを含めない。
   - Leptos 版・Dioxus 版も同じ行を出せば、同じ取り込み（`bench_ui.py add <UI>/<OS> bench.txt --source …`）で表に載る。
3. **何回測るか**: 3 回。表は中央値（バンドルは 1 つ）。1 回目は冷えた起動（ブラウザは新しいプロファイル）。
4. **表**: `bench_ui.py report` が `target/ui-bench.json` から `doc/bench/ui.md` を作る。行 = 測るもの、列 = 4 UI × 3 OS（Svelte ブラウザ・Svelte Tauri・Leptos・Dioxus × Linux・Windows・macOS）。測っていない欄は「未」、出典（手元か run id）を下に書く。
5. **しきい値は置かない**。CI は「行が 3 回 ok で出る」ことだけを確かめ、数の大小では落とさない（ランナーと手元で数が大きく揺れ、比べる相手もまだ無いため）。

## 結果

- 良い: UI の手順が `test_ui.py`・`android.yml` と同じ道なので、測るためだけのコードは時刻の 3 つと Desktop の command 1 つで済む。新しい依存は無い。
- 良い: Leptos 版・Dioxus 版は「`GM-SMOKE … ready_ms=…` の 1 行を出す」ことだけを満たせば比べられる。
- 悪い: `ready_ms` はページの読み込みの始まりからで、プロセスの起動から webview を作るまでは含まない（Desktop はその分を `time -v` の Elapsed で見るしかない）。ブラウザと Desktop で 0 の意味が少し違う。
- 悪い: 手元の数は WSL の負荷（load average）で揺れる。手元の回は load が 10 未満のときに回し、そのときの load を作業記録に書く。
- 悪い: macOS は `desktop.yml` の matrix に無いので「未」。足すか（.dmg のビルドと測定のジョブ。ランナーの時間が増える）はユーザーが決める。
