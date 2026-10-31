# S049 `desktop-tauri`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s049-desktop-tauri` |
| worktree | `.git-worktree/s049-desktop-tauri` |
| Phase | P4（計画書の WP-C） |
| セッション文書 | `doc/sessions/S049-desktop-tauri/` |
| セッション記録 | 締めで `log/sessions/` にコピーする（親のセッションが行う） |

## 依頼

`src/Rust/apps/desktop` に Tauri 2 のシェルを作り、GitHub Actions が .deb（ubuntu-24.04）と .msi（windows-2025）を artifact に出す。設計は ADR-0023: UI のプロセスでは計算せず、`globe` を sidecar に同梱し、`globe lsp` の stdio に独自の要求 `globe/check`・`globe/simulate`・`globe/graph` を足して話す。Tauri の Rust 側は facade をリンクしない。

## 決めたこと

- **desktop は別の Cargo workspace**（`[workspace]` と `Cargo.lock`。406 パッケージ）。ルートに入れると webkit2gtk が要り、手元（WSL、webkit2gtk なし）と差分 CI の `cargo build --workspace` が落ちる。ルートの `Cargo.toml`・`deny.toml`・`ci.yml` は触っていない。
- **独自の要求は `globe-mode-lsp` に足した**（facade の `check`・`simulate`・`graph` を呼ぶだけ）。params が読めなければ -32602、モデルが無いなどは -32000。ファイルは標準の `didOpen`（UI の `setFile`）。静かな `globe/setFile` は足していない（要求を 3 つに絞る指示。`didOpen` は毎回診断のために全モデルを検査するので、重ければ後で足す）。
- **sidecar は `tauri-plugin-shell` を使わず `std::process::Command`**: `externalBin` は主のバイナリと同じ場所に `globe` として置かれる。plugin を使うと依存と capability の設定が増え、stdin の書き込みと kill のために結局自前の枠が要る。選ばなかった理由はこれだけ。
- **予算の kill と作り直しは入れた**（ADR に【持ち越し】と書かずに済んだ）。読み取りのスレッドが応答を `mpsc` に流し、`recv_timeout` で待つ。超えたら `Child` を kill（`Drop`）し、次の要求で作り直して開いているファイルを再生する。`globe/simulate` は 180 s、`initialize` は 300 s、検査は無し。
- **`TauriBackend` は `window.__TAURI__.core.invoke`**（`withGlobalTauri`）。`@tauri-apps/api` は足していない。App.svelte は触っていない。
- **UI のビルドは `pnpm exec vite build`**。`pnpm build` は先に `prepare-wasm`（`copy-wasm.mjs`）が wasm の出力を探し、無ければ `exit(1)` で落ちる。Desktop の UI に wasm は要らないので、`copy-wasm.mjs` は直さずに `vite build` だけを回した。ローカルでも wasm 無しで通った（`svelte-check` 0 誤り）。
- **tauri-cli は `cargo install tauri-cli --version "^2" --locked`**。`tauri-action`（第三者の action）も `@tauri-apps/cli`（npm の依存）も増やさない。代わりに毎回ビルドで約 3〜7 分かかる（rust-cache の `cache-bin` で 2 回目以降は短くなる【未確認】）。
- **desktop 用の `deny.toml`**（`src/Rust/apps/desktop/deny.toml`）: ルートの方針に MPL-2.0 の 5 crate の例外と RUSTSEC-2024-0370 の ignore だけを足し、Rumoca の bans を外して `globe-mode` の ban を足した。ルートの `deny.toml` を `--config` で使うと MPL-2.0 で落ちるため（手元で確認）。
- **ワークフローの暫定の `push` トリガー**: main に無いワークフローは `gh workflow run --ref` で起動できない（HTTP 404、workflow not found on the default branch）ので、feature ブランチの push で回した。**main に入れる前に消した**（desktop.yml は `workflow_dispatch` だけ）。main に入ったら `gh workflow run desktop.yml` を 1 度回して確かめること。
- 選ばなかった案: UI のビルドを各 OS で回す（Windows で mise の shim が不安定。ubuntu の 1 ジョブで `dist` を artifact にして配る方が速く安全）。AppImage / macOS（要件の外）。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: 独自の要求 3 つとエラー応答 `error()`、単体試験 `custom_requests_check_simulate_and_graph`。`src/Rust/mod/globe-mode-cli/tests/lsp.rs`: stdio の子プロセスを叩く統合試験 `lsp_serves_custom_requests`（**所有の外**。既存の `globe lsp` の試験と同じ補助関数を使うため同じファイルに足した）。
- `src/Rust/apps/desktop/`: `Cargo.toml`・`Cargo.lock`・`build.rs`・`tauri.conf.json`・`capabilities/default.json`・`deny.toml`・`.gitignore`、`src/main.rs`（command `rpc`・`notify`、イベント、smoke）、`src/worker.rs`（枠・予算・作り直し、試験 3 本）、`icons/`（自前の最小の地球の絵を Python で生成。RGBA の PNG 4 枚と ICO）。
- `src/Svelte/src/lib/tauri-backend.ts`、`.github/workflows/desktop.yml`、`doc/licenses.md`（Tauri の行）、ADR-0023 の「結果」の追記、`doc/roadmap.md`、`doc/sessions/README.md`、セッション文書。

## 検証

- 手元（WSL）: `cargo test -p globe-mode-lsp` 7 本 ok（新しい 1 本を含む）。`cargo clippy -p globe-mode-lsp --all-targets -- -D warnings` ok。`cargo fmt`（lsp・cli）と `rustfmt --edition 2024`（desktop の 2 ファイル）ok。`worker.rs` は scratch の crate に `#[path]` で取り込み、モックの sidecar（Python）で 3 本 ok。**desktop の crate と `globe-mode-cli` の統合試験は手元でビルドしていない**（指示）。
- `cargo deny --manifest-path src/Rust/apps/desktop/Cargo.toml --config src/Rust/apps/desktop/deny.toml check`: advisories・bans・licenses・sources ok（手元と Actions の両方）。ルートの `deny.toml` では MPL-2.0 の 5 crate と RUSTSEC-2024-0370 で落ちる。
- `pnpm install --frozen-lockfile && pnpm check && pnpm exec vite build`（wasm 無し）ok。
- **GitHub Actions `desktop.yml`**（feature ブランチの push で起動）:

  | run | 結果 | メモ |
  | --- | --- | --- |
  | 41195285155 | 失敗（Linux の smoke だけ） | .deb は作れた。`apt-get install` に渡した .deb のパスの探し方が悪かった（`ls` のグロブ。相対パスのまま渡した）。Windows の途中で次の push に置き換えて取り消し |
  | **41196686607** | **成功（ui・Linux・Windows）** | Linux 12 分 39 秒、Windows 17 分 46 秒（rust-cache が空の初回。`globe` の release ビルドが大半） |

- run 41196686607 の artifact:

  | artifact | 中身 | zip の大きさ | 本体の大きさ |
  | --- | --- | --- | --- |
  | `desktop-linux` | `GlobeMode_0.0.1_amd64.deb` | 19,962,466 B | 20,018,578 B（19.09 MiB） |
  | `desktop-windows` | `GlobeMode_0.0.1_x64_en-US.msi` | 17,913,642 B | 18,161,664 B（17.32 MiB） |
  | `ui-dist`（中間） | `src/Svelte/dist` | 245,021 B | — |

  .deb の中身は `/usr/bin/globe-mode-desktop` と `/usr/bin/globe`（55,247,752 B）。
- Actions 上の試験: `worker::tests` の 3 本 ok（Linux。本物の `globe` で `globe/check`・`simulate`・graph の誤り・通知・kill 後の作り直しと再生）、Windows は 2 本（予算の試験は `cfg(unix)`）ok。
- **smoke（Linux、xvfb、.deb を `apt-get install` して `/usr/bin/globe-mode-desktop` を起動）**: `smoke: ok passed=true failed_stage=null`。最大の常駐メモリ（`/usr/bin/time -v`）**122,688 kB**（アプリ側の最大のプロセス）、sidecar の VmHWM **21,424 kB**（MSL なし・小さいモデル 1 つ）、壁時計 33 秒（WebKit の起動と終了を含む）。**MSL を読んだ後の Desktop の RSS は測っていない【未確認】**（ADR-0023 の `globe check ChuaCircuit` の 1.48 GiB がその目安）。
- `check_docs.py`: 締めで回す。セッション記録の `--check`・漏洩検査: 締め（親のセッション）で行う。

## 残したこと

- App.svelte の `inTauri()` による `TauriBackend` の切り替え（指示で別のセッション）。
- MSL の置き場（同梱か初回に取得か）、MSL を読んだ後の RSS、Windows の smoke（`.msi` を入れて起動する道は作っていない）、署名と自動更新。
- 静かな `globe/setFile`、`didOpen` が毎回全モデルを検査する点。
- `ci.yml`・`ci_plan.py` に desktop の道は入れていない（所有の外）。desktop は別 workspace で `desktop.yml`（手動）だけが回す。差分 CI は `src/Rust/apps/` の変更で何も走らせない【未確認。`.github/` を触ったので全体の検査になる】。
- `copy-wasm.mjs` が wasm 無しで落ちるのはブラウザ版の仕様なので直していない。
- main に入れた後: `gh workflow run desktop.yml` を 1 度回す（暫定の `push` トリガーを消したため、最終のファイルでの run は無い。差は `on:` の 2 行だけ）。
