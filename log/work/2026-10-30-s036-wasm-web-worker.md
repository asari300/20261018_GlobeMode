# S036 `wasm-web-worker`

| | |
| --- | --- |
| 日付 | 2026-10-30 |
| ブランチ | `feature/s036-wasm-web-worker` |
| worktree | `.git-worktree/s036-wasm-web-worker` |
| Phase | P3（候補 3「ブラウザ」、P6 の前提。[plan](../../doc/plan/2026-10-30-release-estimate-and-parallel-plan.md) の WP-F） |
| セッション文書 | `doc/sessions/S036-wasm-web-worker/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「3：ブラウザから進めて。サブエージェントは Sonnet/high、レスポンスは日本語」（roadmap の次のセッションの候補 3）。

## 決めたこと

- 範囲は候補 3 のうち「`--target web` と Web Worker」と「panic の後の作り直し」。MSL の分割読み込み（メモリ）は db に query を足す大きさなので次に回した（既定のまま進めた判断）。
- **Rust を変えない**: `globe-mode-wasm/src/lib.rs` は WP-0（別のセッションの S035 protocol-facade）が型を差し替える。ぶつけないため、作り直しは JS 側だけで行う。
- **panic の後は Worker ごと作り直す**: wasm32 は panic=abort で、`RuntimeError` の後の線形メモリは壊れている。`--target web` の糊は初期化を一度しかしないので、同じ Worker で読み直す道は無い。Worker を捨て、`GlobeWorker` が記憶したライブラリとファイルを再生する。
- **Worker 1 本をブラウザと Node で共用**: CI（`wasm-node`）が Node だけで Worker の道を試せる。Node は `file:` を fetch できないので .wasm をバイト列で渡す。
- **ブラウザの試験は HTTP サーバを立てない**: Playwright の `page.route` で手元のファイルを配る（ポートの取り合いが無い）。CI には入れない（Chromium が要る）。
- S 番号: main の索引の最後は S034 だが、別のセッションが S035（protocol-facade）の worktree を持っているので S036 にした。

## やったこと

- `mise.toml`: `wasm:build` が同じ .wasm から `--target web` の糊も作る（`target/wasm-pkg/web`）。新しいタスク `wasm:web`（Node の Worker の試験 → ブラウザの試験）。
- `src/Rust/mod/globe-mode-wasm/web/`: `worker.js`（56 行。`{id, op}` を受けて返す）、`client.js`（`GlobeWorker`。呼び出しを直列にし、panic・異常終了で作り直して再生、`restarts`）、`worker-node.mjs`（Node の試験）、`index.html` と `browser.py`（Chromium の試験）。
- `.github/workflows/ci.yml`: `wasm-node` に Node の Worker の試験を足した。`ci_plan.py` は変えていない（`web/` は crate の中なので `wasm_node` に当たる）。
- レビューで 1 点直した: `library` が新しい Workspace を作る前に古いものを `free` していた（読めない JSON で解放済みの参照が残る）。順を入れ替えた。
- 文書: roadmap（候補 3 と S036 の節）、features.md（ブラウザの行）、architecture.md（wasm の行とタスクの行）、セッション文書、索引。

## 検証

- `mise run wasm:web`（540 s。うち wasm-bindgen 2 回が大半、3 GB 機で swap）: 通過。
  - Node（`worker_threads`）: Decay（bdf）の x(1) = 0.367879468116207（誤差 2.7e-8）。panic は文言つきで reject、`restarts` = 1、作り直し後も同じ値（388.5 ms）。線形メモリ 2 MiB。
  - Chromium（module Worker）: 同じ値、`restarts` = 1。線形メモリ 3 MiB。
- `mise run wasm:node`（サブエージェントが実行）: 通過（Decay の bdf・dopri45・auto、MSL の `ChuaCircuit`、panic の文言）。node と web の `_bg.wasm` はどちらも 14,593,419 bytes。
- `.github/scripts` の unittest: 20 件通過（サブエージェントが実行）。
- `mise run check`・`wasm:compare`: 回していない（Rust と .wasm は変えていない）。
- `check_docs.py`: デッキ OK / 文書 174 件・相対リンク 964 件中の切れ 0 件。`render_slides.py`: デッキ 41 件 / 問題 0 件。
- セッション記録の `--check`・漏洩検査: 下のコミットの前に確かめた（取りこぼし 0 件、漏洩 0 件）。

## 残したこと

- MSL を読んだ後の線形メモリを減らす分割読み込み（S012 で 262 MiB、`ChuaCircuit` の後 513 MiB）。測り直していない。
- 作り直しはライブラリ全体の読み直しを含む（MSL で約 13 s）。
- ブラウザの試験は CI に入れていない。Firefox・Safari は試していない。
