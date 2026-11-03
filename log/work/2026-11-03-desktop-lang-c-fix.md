# S134 `desktop-lang-c-fix`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s134-desktop-lang-c-fix` |
| worktree | `.git-worktree/s134-desktop-lang-c-fix` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S134-desktop-lang-c-fix/` |
| セッション記録 | （親が写す） |

## 依頼

S125 で見つけた不具合（Linux の Desktop を `LANG=C` で起動すると CodeMirror の `Intl.Segmenter` が `RangeError: invalid language tag: C` を投げて白画面）を直す。UI 側でガードするか、Rust 側で起動時に locale を補うかを比べて 1 つ選び、ADR-0023 に 1 段落で書く。`desktop.yml` の Linux の UI の測定から S125 の `LANG: en_US.UTF-8` を外して通ることを確かめる。

## 決めたこと

- 原因を WebKit のソース（`Source/WTF/wtf/unix/LanguageUnix.cpp`、確認日 2026-11-03）で確かめた。`C`・`POSIX` は `en-US` になる。不正な `C` になるのは `C.UTF-8` のように `C.` で始まるとき。ランナーの既定はこちら（段の最初の表示で `LANG=C.UTF-8`、`LC_ALL`・`LC_CTYPE` は空。`locale -a` に `C`・`C.utf8`・`en_US.utf8`。run 41641017734）。
- (b) Rust 側を採った（既定のまま進めた判断）。(a) は CodeMirror のほかに uPlot も module の評価で `Intl.NumberFormat(navigator.language)`・`Intl.DateTimeFormat()` を作るので、包むものが増える。(b) は `main.rs` に 20 行。
- 替える値は `en_US.UTF-8`。入っていない環境では `setlocale` が失敗して `C` になり、WebKit は `en-US` にする。`C.UTF-8` に替えても直らない（同じく `C` になる）。

## 経過（CI の切り分け）

- run 41636277997（修正だけ）: Linux の UI の測定が 1 回目で時間切れ（300 s、`GM-SMOKE` の行が無い）。
- run 41641017734（一時の切り分け 1）: WebProcess（`WebKitWebProcess`）の環境は `LANG=en_US.UTF-8` になっていた（アプリの `/proc/<pid>/environ` は起動時の `C.UTF-8` のまま。`set_var` は `/proc` に出ない）。なお時間切れ。
- run 41645574928（一時の切り分け 2。S125 と同じ `bench-diag` で 60 s 後のページを stderr に）: `navigator.language` = `en-US`、`new Intl.NumberFormat()` の locale = `en-US`、本文は描かれていた（`#app` の子 1、文字 175）。残った unhandled rejection は `main-*.js:17` の `smoke()` の `D.setFile(FILE, …)`。**言語は直っていた**。止まっていたのは main の S125 と S127 の merge で `App.svelte` の `FILE` が未定義になったこと（S127 が `file` に改名）で、main では S133 の merge（034a9b1）が `file` に直し済み。main を取り込んで切り分けのコードを外し、回し直した。

## やったこと

- `src/Rust/apps/desktop/src/main.rs`: `fix_c_locale`（Linux だけ。`LC_ALL`・`LC_CTYPE`・`LANG` の効いているものが `C.`・`POSIX.` で始まるとき、その変数を `en_US.UTF-8` に）。`main` の最初で呼ぶ。
- `.github/workflows/desktop.yml`: UI の測定（Linux）から `LANG: en_US.UTF-8` を外し、locale の変数を表示する 1 行を足した。
- `doc/adr/0023-desktop-and-web-process-layout.md`（S134 の節）、`doc/features.md`（Desktop の行）、`doc/roadmap.md`（P4 の行と「S134 で済んだこと」）、セッション文書、索引。

## 検証

- desktop の Rust: `rustfmt --edition 2024 --check` OK。`cargo check` は手元に webkit2gtk が無いので回していない（clippy は desktop.yml）。root の Rust は触っていないので `mise run check` は不要。
- `desktop.yml`（run 41650539889）: Linux の build ジョブ成功。UI の測定は `LANG=C.UTF-8` のまま 3 回とも `GM-SMOKE check=ok simulate=ok x1=0.36787972790802487 msl=ok`（ready_ms 150 / 149 / 152、msl_ms 1,250 / 1,260 / 1,298、rss_kib 149,720 / 150,416 / 149,812、GM-BUNDLE 24,482 KiB）。Windows のジョブはこの報告の時点で実行中（S134 は Linux だけの変更）
- `check_docs.py`: デッキ OK、リンク切れ 0 件
- `render_slides.py`: 1 回目は 30 s の時間切れ（Chromium）、2 回目で デッキ 1 件 / 問題 0 件。図と表のスライドをライト・スマートフォンで目視
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（作業の差分 main..HEAD とこのコミット）

## 残したこと

- 実機（Debian・Ubuntu の `C.UTF-8` の環境）での確認。
- 署名と自動更新は別の WP。
