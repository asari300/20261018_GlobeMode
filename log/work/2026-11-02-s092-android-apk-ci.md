# S092 `android-apk-ci`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s092-android-apk-ci` |
| worktree | `.git-worktree/s092-android-apk-ci` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S092-android-apk-ci/` |
| セッション記録 | （親が写す） |

## 依頼

親のエージェントから。P6 の「APK」。Tauri の `tauri android init` で desktop の workspace に `gen/android` を足し、Android では sidecar を使わず `WorkerBackend`（wasm）を使う。`android.yml`（手動と、desktop・Svelte の変更時）で署名なしの debug APK を作り artifact `globe-android` に残す。時間があればエミュレータ（API 34）で起動まで。`desktop.yml`・`ci_plan.py`・ci.yml・`mise.toml` は触らない。main へのマージ・push はしない。

## 決めたこと

- 計算は Web と同じ Worker の道（親の指示）。mobile の入口 `src/lib.rs` は command を出さず UI を表示するだけ。Desktop の `main.rs` は変えない。
- backend の選択は `App.svelte` の 1 行（User-Agent に Android があれば Worker）。
- `tauri.android.conf.json` で `externalBin` を空にした。
- `gen/android` は手元に SDK が無いので CI で `init --ci` し、artifact から取り込んでコミットした（既定のまま進めた）。
- `gh workflow run` は既定のブランチに定義が無いと 404 になるので、試行の間だけ push の `branches` に feature を足し、最後に戻した。
- ABI は aarch64 と x86_64 だけ（既定のまま進めた）。

## やったこと

- `src/Rust/apps/desktop/`: `Cargo.toml` の `[lib]`、`src/lib.rs`、`tauri.android.conf.json`、`.gitignore`、`gen/android/`（40 ファイル）
- `src/Svelte/src/App.svelte`: Android なら `WorkerBackend`
- `.github/workflows/android.yml`: ui・apk・emulator
- 文書: ADR-0023 の「S092」、architecture.md の Android の行、roadmap.md、CLAUDE.md、licenses.md、セッションの索引・スライド・解説書

## 検証

- run 41408245562: failure（setup-android の既定の `tools`）。run 41409143035: success（ui 48 s・apk 9 分 44 秒・emulator 2 分 47 秒）
- APK 251,627,503 B。エミュレータの `Displayed … +2s455ms`、`FATAL EXCEPTION` 0
- コミットした `gen/android` での run: run 41411137967 で success（ui 59 s・apk 4 分 8 秒（cache あり）・emulator 2 分 32 秒）。APK は同じ 251,627,503 B、`Displayed` まで 3.2 s
- コードの検査: desktop の workspace は手元に webkit2gtk が無いのでビルドしていない（Android の lib は CI でビルドが通った。Desktop の fmt・clippy は desktop.yml で回る）。`mise run check` はルートの workspace を変えていないので回していない
- `check_docs.py`: デッキ OK、文書 377 件・相対リンク 1820 件・切れ 0 件。`render_slides.py` は Page.screenshot が 30 s で時間切れ（2 回）。CI に任せる

## 残したこと

- エミュレータで検査と実行を UI から回す自動化（P6 の完了条件）
- Android での MSL、Rust の直接 link、release の署名つき APK
- desktop.yml を回していない（`[lib]` を足したので Desktop のビルドにも lib が入る。中身は空）
