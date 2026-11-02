# S084 `desktop-msl-switch`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s084-desktop-msl-switch` |
| worktree | `.git-worktree/s084-desktop-msl-switch` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S084-desktop-msl-switch/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S069 の持ち越し 2 つ。(1) Desktop の MSL の取得の進捗（受信バイトと全体）と取り消し（取得中にもう一度押すと `AtomicBool` で止めて途中のファイルを消す）。(2) MSL 4.0.0 への切り替え（`fetch_msl` に `version`、置き場 `msl/<version>/`、xtask の 4.0.0 の定数を `msl.rs` に写して `desktop.yml` で 2 版分の一致を確かめる、UI は取得ボタンの横に `<select>`）。ponytail（最小の完全な変更）。main へのマージ・push は親。

## 決めたこと

- 進捗は curl を `spawn` して zip の大きさを 0.25 秒ごとに見る。全体は先に `curl --head --location` の最後の Content-Length。HTTP クライアントの crate を足さない（S069 の方針のまま）。
- 取り消しは別の command `cancel_msl`（`MslDir` に持たせた `Arc<AtomicBool>` を立てる）。`fetch_msl` の始めに旗を下ろす。ダウンロード中だけ止める（展開は数秒なので止めない）。
- 4.0.0 の定数は xtask と同じ名前（`MSL_400_URL`・`MSL_400_SHA256`）にした。`desktop.yml` の sed の組を 2 つ足すだけで検査できる。
- `install_msl` は「その版を既に読んでいる」なら何もしない。「外から渡したライブラリがある（`msl/` の下に無いパスがある）」なら Err（S069 は何もせず返していたが、版の選択が黙って効かず「読んだ」と出るのを避ける。`TauriBackend.library()` はライブラリが空のときだけ呼ぶので S069 の道は変わらない）。版を替えるときは sidecar を新しい 1 本の `--library` で作り直す。
- UI は版の `<select>` を Desktop だけに出す。読んだ版は UI が覚える（起動時に取得済みを読んだときは不明なので、同じ版でも押せる。Rust 側が何もせず返す。外から渡したライブラリのときはエラーが出る）。
- `TauriBackend` に `fetchMsl(version)`・`cancelMsl()`・`onMslProgress(cb)` を足した。イベントは `withGlobalTauri` の `__TAURI__.event.listen`（`@tauri-apps/api` は入れない。`core:default` の権限で足りる）。

## やったこと

- `src/Rust/apps/desktop/src/msl.rs`: `VERSIONS`、`source(version)`、`dest(root, version)`、`ready(dest, version)`、`fetch(dest, version, progress, cancel)`、`content_length`。試験 `versions_have_sources_and_own_dirs` を足した。
- `src/Rust/apps/desktop/src/main.rs`: `MslDir` を置き場の根と取り消しの旗に。`fetch_msl { version }`、`cancel_msl`、イベント `msl-progress`。起動時は 4.1.0 → 4.0.0 の順に取得済みを探す。smoke は 4.1.0。
- `src/Svelte/src/lib/tauri-backend.ts`・`src/Svelte/src/App.svelte`: 版の選択・取り消しボタン・進捗の表示。ブラウザ版の道（「MSL を読む」）は変えていない。
- `.github/workflows/desktop.yml`: 定数の一致の検査を 4 組に。
- 文書: 解説書・スライド・索引・roadmap（S069 の持ち越しに取り消し線、S084 の節）・ADR-0030 の追記。

## 検証

- 手元に webkit2gtk が無いので、desktop の workspace の `cargo check`/`clippy`/`test` は手元で回せない（`desktop.yml` の冒頭の注記と同じ）。代わりに `msl.rs` だけを一時の crate（scratchpad、`#[path]`、desktop の `Cargo.lock`）で `cargo test`（3 本 通過）と `cargo clippy -D warnings`（通過。`double_ended_iterator_last` を 1 件直した）。
- 同じ一時の crate で、本物の 4.0.0 の URL に対し進捗（全体 29,117,960 バイト、0.25 秒ごとに増える）と 8 回目の通知での取り消し（Err「MSL の取得を取り消した」、`.download` が消える）を確かめた。
- `cargo fmt --check`（desktop）: 通過。定数の一致の検査（desktop.yml と同じ sed）を手元で: 4 組とも一致。
- `desktop.yml`（run 41394655717、2 つ目のコミット 66217a4。1 つ目の run 41393238274 は再 dispatch で取り消し）: success。Linux で定数の一致（4 組）・fmt・clippy `-D warnings`・`cargo test --release`（6 本 通過）・.deb の smoke 3 回（`GLOBE_LIBRARY`、取得 1.1 s で `msl/4.1.0`、2 度目の起動で再利用）。Windows の .msi も作れた。4.0.0 の取得は CI に入れていない。
- `mise run ui:check`: svelte-check 0 errors / 0 warnings（260 ファイル）、`test_ui.py` 通過（MSL.json が無いので MSL の試験は飛ばした）。`test_graph.py` は 1 度目に wasm の読み込み待ち（120 s）で落ちた（他の worktree の cargo で負荷が高かった）。単独で回し直して通過。この変更は Graph と wasm に触れていない。
- `check_docs.py`: デッキ OK / 文書 352 件・相対リンク 1703 件中の切れ 0 件。`render_slides.py`: デッキ 1 件・問題 0 件（desktop-light・dark・mobile の 6 枚ずつ）。
- Desktop を実際に起動して版の選択・進捗・取り消しを押す確認はしていない（手元に webkit2gtk が無い。CI の smoke は 4.1.0 の取得だけ）。

## 残したこと

- 展開中の取り消し、複数ウィンドウでの同時取得の排他（ADR-0030 の (e)）。
- 使わなくなった版の置き場の削除。4.0.0 の取得の CI（時間がかかるので smoke に入れていない）。Windows での取得の確認。
