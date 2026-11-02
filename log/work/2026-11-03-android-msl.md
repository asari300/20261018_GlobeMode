# S103 `android-msl`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s103-android-msl` |
| worktree | `.git-worktree/s103-android-msl` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S103-android-msl/` |
| セッション記録 | （親が写す） |

## 依頼

S092 の持ち越しの 2 つ目。Android で MSL を読めるようにする（今は MSL.json を同梱しないので「MSL を読む」が失敗する）。(a) mobile でも `fetch_msl` と (b) ビルド時に MSL.json を同梱、を実装前に比べて小さい方を選び、ADR-0023 に追記する。`android.yml` の smoke を広げて MSL の例題 1 つを検査し `msl=ok` を足す。exec・fmu・fmi・graph・src/Python・ci.yml・release.yml・desktop.yml は触らない。新しい依存は足さない。

## 決めたこと

- (b) を採った。(a) は Rust の command・取得・展開に加え、取得物を WebView の `WorkerBackend` に渡す新しい道が要る。(b) は `android.yml` の `ui` に `cargo xtask fetch-msl` の 1 段で、既存の `pack-msl.mjs` と「MSL を読む」の道がそのまま使える。
- 形はブラウザと同じ全部読みの `MSL.json`（分割読み込みは UI がまだ使っていない）。版は 4.1.0 だけ。
- smoke の例題は ChuaCircuit（ブラウザの試験と同じ）。検査だけ（実行はしない）。

## やったこと

- `src/Svelte/src/App.svelte`: `smoke()` が Decay の後に `loadMsl()` と ChuaCircuit の `check()` を呼び、`msl=ok|ng` を同じ行に足す。
- `src/Svelte/tests/test_ui.py`: smoke の待ちを 600 s に、`MSL.json` があれば `msl=ok` を確かめる。
- `.github/workflows/android.yml`: `ui` に `cargo xtask fetch-msl` と `test -s dist/msl/MSL.json`、`emulator` の待ちを最大 10 分に、`msl=ok` が無ければ失敗。
- 文書: 解説書・スライド、ADR-0023 の「S103」の節、roadmap の P6 の行と「S103 で済んだこと」、索引の行。

## 検証

- コードの検査: Rust は触っていないので `mise run check` は回していない。`pnpm check`（svelte-check）0 error・0 warning。
- `test_ui.py`（wasm は共有 target の `wasm-pkg/web`、`data/msl` は main の取得物への symlink）: ok、`GM-SMOKE check=ok simulate=ok x1=0.3678797279080248 msl=ok`、MSL を読む 0.5 s、ChuaCircuit の検査 1.8 s（error 0）。
- `android.yml`: run 41521456071 が 1 回目で全ジョブ success（ui 6 分 45 秒・apk 4 分 02 秒・emulator 2 分 16 秒）。logcat は `Displayed … +2s48ms` の 13.6 s 後に `I Tauri/Console: File: http://tauri.localhost/assets/main-DvUbHPCg.js - Line 17 - Msg: GM-SMOKE check=ok simulate=ok x1=0.3678797279080248 msl=ok`。
- APK（debug、universal）: 251,629,679（run 41453487541）→ 257,983,183 bytes（+6,353,504、+6.1 MiB）。`MSL.json` は 15.2 MiB・2556 ファイル（gzip -9 で 2,643,652 bytes）。

## 残したこと

- release の署名つき APK、Rust を mobile の lib に直接 link する道、分割読み込みを UI に使うこと（ブラウザと一緒に）。
