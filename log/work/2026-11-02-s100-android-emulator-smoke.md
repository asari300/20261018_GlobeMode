# S100 `android-emulator-smoke`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s100-android-emulator-smoke` |
| worktree | `.git-worktree/s100-android-emulator-smoke` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S100-android-emulator-smoke/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S092 の持ち越しの 1 つ目。`android.yml` のジョブ `emulator` で、APK の起動だけでなく、検査と実行が UI から通ったことを自動で確かめる（P6 の Android の完了条件）。UI の変更は最小で、Desktop とブラウザの既定の挙動は変えない。MSL の同梱・署名つき APK・Rust の直接 link は持ち越し。exec・fmi・fmu・ci.yml・release.yml は触らない（S098・S099 が触る）。

## 決めたこと

- 既定のまま進めた（親の指示の最小の道）:
  - 入口はビルド時の `VITE_GM_SMOKE=1`（Tauri の Android はクエリを渡せない）と `?smoke=1`（ブラウザの試験用）。どちらも無ければ何もしない。
  - 中身は既存の `check()`・`simulate()` を順に呼ぶだけ。判定は error 0 と x(1) が e⁻¹ と 1e-3 以内。
  - logcat はタグに依らず `GM-SMOKE` で grep する（WebView の console のタグは `chromium` か Tauri の `Console` か、版で違うので決め打ちしない）。
  - CI の debug APK（artifact `globe-android`）は起動すると smoke を回す。debug の試験用なので許した。

## やったこと

- `src/Svelte/src/App.svelte`: `smoke()` と、`onMount` の 1 行の呼び出し。
- `src/Svelte/tests/test_ui.py`: `?smoke=1` のページで `GM-SMOKE check=ok simulate=ok x1=0.36` の行を待つ。
- `.github/workflows/android.yml`: `ui` に `VITE_GM_SMOKE: "1"`、`emulator` の `sleep 45` を「`GM-SMOKE` が出るまで 5 s ごと最大 60 回」に替え、行を Step Summary に出し、`check=ok simulate=ok` が無ければ失敗。
- 文書: 解説書・スライド、ADR-0023 の「S100」の節、roadmap の P6 の行と「S100 で済んだこと」、索引の行。

## 検証

- コードの検査: Rust は触っていないので `mise run check` は回していない。`pnpm check`（svelte-check）0 error・0 warning。
- `test_ui.py`（wasm は共有 target の `wasm-pkg/web`、`uv sync` が S098・S099 の maturin と lock を取り合ったので main の `.venv` の Python で回した）: ok、`GM-SMOKE check=ok simulate=ok x1=0.3678797279080248`、15.6 s。
- `android.yml`: run 41444392188 の 2 回目の試行で全ジョブ success（ui 4 分 05 秒・apk 12 分 40 秒・emulator 2 分 27 秒）。logcat は `Displayed … +3s446ms` の 5.7 s 後に `I Tauri/Console: File: http://tauri.localhost/assets/main-CYuiFFjw.js - Line 17 - Msg: GM-SMOKE check=ok simulate=ok x1=0.3678797279080248`（ブラウザと同じ値）。1 回目の試行は apk が Gradle の plugin portal から Kotlin 2.3.21 の POM を引けずに落ちた（`Could not parse POM`、コードと無関係）ので `gh run rerun --failed` で回し直した。所要は push から成功まで約 27 分（rerun の待ちを含む）
- `check_docs.py`: デッキ NG 0 件 / 相対リンク 2048 件中の切れ 0 件。render_slides（S100 のデッキ、3 表示）問題 0 件（数字を埋める前の版。埋めた後の 2 回は S098・S099 の cargo の負荷で Chromium の screenshot が 30 s で時間切れ。変えたのは KPI の短い文字列だけ）
- ステージした差分の漏洩検査: 0 件（main からの差分全体）

## 残したこと

- Android での MSL の置き場、Rust を mobile の lib に直接 link する道、release の署名つき APK（`VITE_GM_SMOKE` を付けない）。
