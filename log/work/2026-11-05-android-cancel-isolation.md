# S159 `android-cancel-isolation`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s159-android-cancel-isolation` |
| worktree | `.git-worktree/s159-android-cancel-isolation` |
| Phase | P4 / P7 |
| セッション文書 | `doc/sessions/S159-android-cancel-isolation/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S157 の持ち越し。Android（Tauri の WebView、`WorkerBackend`）で、wasm の中断が SharedArrayBuffer のフラグの道で働くか【未確認】を確かめ、働かなければ最小の変更で働かせる。どうしても付けられないなら、事実を ADR-0033 と features.md に書く。

## 決めたこと

- smoke の行に `isolated=` と `cancel_path=` を足す（`def=ok`／`recent=ok` の後、`ready_ms` の前。`bench_ui.py` と既存の grep は変えずに済む）。
- Tauri 2.12.5 の `app.security.headers` を Android だけの `tauri.android.conf.json` に付けて試す（Desktop の `tauri.conf.json` は共有せず、WebView2・WebKitGTK に影響させない）。
- 結果が効かなかったのでヘッダは外し、Android は作り直しのままと記録する。`android.yml` の合格条件は `isolated=0 cancel_path=restart`。

## やったこと

- `src/Svelte/src/App.svelte`: smoke の `cancelPath` と `isolated`。`src/Svelte/tests/test_ui.py`: `isolated=1 cancel_path=flag` の assert。
- `.github/workflows/android.yml`（`isolated=0 cancel_path=restart`）・`desktop.yml`（`isolated=0 cancel_path=sidecar`）の条件。
- ADR-0033 の 7 段目、`doc/features.md`・`doc/roadmap.md`・索引、S157 の解説書の【未確認】に S159 への注記、スライドと解説書。

## 検証

- `pnpm check` 0 エラー、`test_ui.py`: ok（`GM-SMOKE … def=ok isolated=1 cancel_path=flag ready_ms=295 …`）。
- `android.yml`（ヘッダあり、run 41988941532）: ui・apk は成功、emulator は debug・release とも新しい条件で失敗。行は `… def=ok isolated=0 cancel_path=restart ready_ms=2890 decay_ms=1085 msl_ms=9119`（debug）。他の項目は全部 ok。
- 診断入り（run 41990456155）: `GM-ISO coop=same-origin coep=require-corp sab=undefined`。ヘッダは届くが SharedArrayBuffer が無い。
- ヘッダを外し条件を `isolated=0 cancel_path=restart` に直した版（`android.yml` run 41992190810）: success（ui・apk・emulator の debug と release）。行は `… def=ok isolated=0 cancel_path=restart ready_ms=3489 decay_ms=2543 msl_ms=17781`（debug）。回した android.yml は計 3 回（上限 3 回）。
- `desktop.yml`（run 41988951740、Android のヘッダを付けた最初のコミット。Desktop の設定は同じ）: success（Linux の .deb と Windows の .msi）。ui-bench の行（Linux）は `… cfg=ok … def=ok recent=ok isolated=0 cancel_path=sidecar ready_ms=143 …` を 3 回とも。
- Rust は変えていない（`mise run check` は回していない）。
- `check_docs.py`: デッキ OK、リンク 3220 件中の切れ 0 件。`render_slides.py`（S159 のデッキだけ）: 問題 0 件（1 回目で通った）。
- セッション記録の `--check`: （親が写す。このセッションでは写さない）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- Android で isolated にする別の道（Kotlin 側の WebView の設定など）は公式の文書で確かめられず【未確認】。
- Desktop の既定 `auto` の積分の中の中断。
