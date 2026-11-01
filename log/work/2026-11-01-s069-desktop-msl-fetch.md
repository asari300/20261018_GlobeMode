# S069 `desktop-msl-fetch`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s069-desktop-msl-fetch` |
| worktree | `.git-worktree/s069-desktop-msl-fetch` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S069-desktop-msl-fetch/` |
| セッション記録 | [2026-10-31-0b797c72-...](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md)（親セッション。S048〜S052 と同じ） |

## 依頼

S056 の持ち越し「MSL の置き場（同梱か取得か）」。配布版の Desktop が初回に MSL 4.1.0 を取得して使えるようにする。サブエージェント（親が main へのマージと CI の最終確認を行う）。

## 決めたこと

- 同梱せず、初回に app data dir の `msl/4.1.0/` へ取得する（[ADR-0030](../../doc/adr/0030-desktop-msl-placement.md)。ADR 番号は 0029 を並行の S068 が使うので 0030）。比べた案は同梱・初回取得・利用者が指定。`--library`・`GLOBE_LIBRARY` は今のまま優先する。
- ダウンロードは外部コマンド `curl`（xtask と同じ）。desktop の `Cargo.lock` の reqwest は wasm 用で TLS を持たず、HTTP クライアントの crate を足すと TLS の依存が増えるため。.deb に `Depends: curl`。
- 依存は `zip` 9（xtask と同じ版・features）と `sha2`（lock に既にあった）だけ。`cargo deny`（desktop）は ok。
- xtask のコードは共有しない（別 workspace、xtask は重い bin）。定数の一致を `desktop.yml` が確かめる。
- `doc/sessions/README.md` の行は、依頼の「先頭に」ではなく、既存の並び（新しい順に末尾へ足す）に合わせて表の末尾に足した。
- UI の変更は最小: Desktop でだけボタンの文言を変え、`library()` が `fetch_msl` を呼ぶ。起動時に sidecar がライブラリを読んでいれば最初から「読んだ」状態にする（`hasLibrary()`）。

## やったこと

- `src/Rust/apps/desktop/src/msl.rs`（新）: `ready`・`fetch`・`install`（sha256 照合 → 展開 → 目印を最後に）、試験 2 本。
- `src/Rust/apps/desktop/src/main.rs`: `Libraries` を共有の可変に、`MslDir`、command `fetch_msl`、起動時に置き場を見る、smoke の `GLOBE_DESKTOP_SMOKE_FETCH`。`worker.rs`: `set_libraries`。
- `src/Rust/apps/desktop/Cargo.toml`・`Cargo.lock`（`zip` 9.2.1 のほか `typed-path`・`zopfli`・`nix`・`cfg_aliases`）、`tauri.conf.json`（deb の depends）。
- `src/Svelte/src/lib/tauri-backend.ts`・`src/Svelte/src/App.svelte`。
- `.github/workflows/desktop.yml`: fmt と clippy（`-D warnings`）、xtask との定数の一致、取得の smoke と 2 度目の起動の smoke、artifact に `smoke-fetch.txt`・`smoke-reuse.txt`・`time-fetch.txt`。
- 文書: ADR-0030、ADR README、ADR-0023 の追記、architecture、roadmap、licenses、セッション文書、索引。

## 検証

- 手元: `cargo fmt`（desktop）ok。手元に webkit2gtk が無く desktop はビルドできない。tauri に依存しない `msl.rs`・`worker.rs` を別 crate に取り込んで `cargo test` 5 本ok（msl 2、worker 3）・`cargo clippy --all-targets -D warnings` ok。`main.rs` は CI のビルドで確かめた。
- `cargo deny --manifest-path src/Rust/apps/desktop/Cargo.toml --config src/Rust/apps/desktop/deny.toml check`: advisories・bans・licenses・sources ok。
- Svelte: `pnpm check` 260 ファイル 0 errors 0 warnings。`pnpm build` ok、`uv run python src/Svelte/tests/test_ui.py`（Playwright）は通った（MSL を読む 1.3 s、ChuaCircuit の検査 4.9 s・実行 0.8 s）。ブラウザ（`inTauri()` が false）の道だけで、実機の Tauri では UI のボタンは動かしていない。
- `desktop.yml`（run 41308587091、feature ブランチへの dispatch）: ui・Linux 成功。Windows 成功（fmt・clippy・定数の一致は Linux のみ）。成果物（artifact の zip の大きさ）は desktop-linux 20,924,954 B・desktop-windows 18,758,029 B で、S056 の .deb 20,023,364 B・.msi 18,165,760 B より約 0.9 MB・0.6 MB 大きい（`zip`・`sha2` と msl.rs。S056 は bundle ファイルそのままの値で厳密な比較ではない）。先に回した run 41308486608 は次の dispatch で取り消した（定数の一致の検査を足す前の版）。
- smoke（Linux、`ChuaCircuit`）:

  | 起動 | 取得 | simulate の後（アプリ / sidecar の VmHWM） | 壁時計 |
  | --- | --- | --- | --- |
  | 1 `GLOBE_LIBRARY=data/msl` | 無し | 139,860 / 171,444 kB | 33.5 s |
  | 2 取得（渡さない） | 1.0 s | 144,316 / 171,576 kB | 32.3 s |
  | 3 2 度目（渡さない） | 無し（置き場の目印を使う） | 141,072 / 171,516 kB | - |

  sidecar の VmHWM は S056 の 1.52 GiB から約 168 MiB に（S054 の閉包の読み込みによる。今回の変更ではない）。
- `check_docs.py`: デッキ OK（S069 を含む）/ リンク 1480 件中の切れ 0 件。`render_slides.py` は親が回す。

## 残したこと

- 署名、自動更新、取得の進捗の割合と取り消し、MSL 4.0.0 への切り替えの UI、`fetch_msl` の同時呼び出しの排他（UI がボタンを無効にするだけ）。
- Windows の取得（`curl.exe`、`%APPDATA%`）は smoke が Linux だけなので未確認。実際のネットワーク断も試していない（単体試験は sha256 の不一致と展開先の外を指す名前）。
- 実機の Tauri で UI のボタンを押す試験は無い（Playwright はブラウザの WorkerBackend のみ）。
- 親がやること: main へのマージ（S068 が ADR-0029 と索引の末尾に足すので、`doc/adr/README.md` と `doc/sessions/README.md`、roadmap の衝突を直す）、セッション記録の写し、`render_slides.py`、`mise run check`。
