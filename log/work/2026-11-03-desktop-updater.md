# S135 `desktop-updater`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s135-desktop-updater` |
| worktree | `.git-worktree/s135-desktop-updater` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S135-desktop-updater/` |
| セッション記録 | （親が写す） |

## 依頼

features.md の Desktop の行の「署名と自動更新は未」。Tauri 2 の updater プラグインで、Release に置いた .deb / .msi から自動更新できるようにし、署名は secret があるときだけ行う（S106 の Android と同じ流儀）。鍵は作らない・コミットしない。`release.yml` に desktop の bundle と `latest.json` を足す（`desktop.yml` は触らない）。UI に「更新を確認」の最小の入口（`App.svelte` は 1 行）。ブラウザ版では出さない。

## 決めたこと

- Linux は .deb のまま更新する: 手引き（v2.tauri.app/plugin/updater/）は AppImage しか書いていないが、プラグインは 2.10 で全形式に対応し、`latest.json` は `linux-x86_64-deb`・`windows-x86_64-msi` のキーを先に引く（CHANGELOG と `updater.rs`、確認日 2026-11-03）。
- 公開鍵は `tauri.conf.json` に書く（今は空）: 空のビルドは `main.rs` がプラグインを入れず、command `updater_configured` が偽で UI は「未設定」。repository variable にして `--config` で入れる案は、動く部品が増えるので採らなかった。
- JS は npm の `@tauri-apps/plugin-updater` を入れず `window.__TAURI__.updater`（`withGlobalTauri`）: 既存の `tauri-backend.ts` が npm を増やさない方針のため（依頼の「npm 側は `@tauri-apps/plugin-updater`」からは外した）。
- `release.yml` は既に `workflow_dispatch`（dry-run、Release を作らない）を持っていたので input は足さなかった。タグの段は変えていない。
- 署名の分岐は `env` に写した `TAURI_SIGNING_PRIVATE_KEY` で、`--config` の `createUpdaterArtifacts` を true / false にする。鍵が無いのに true だと tauri-cli が失敗するため。タグの push では `--config` で `version` もタグに合わせる。
- desktop の `deny.toml` に CDLA-Permissive-2.0 を足した（`reqwest` 経由の `webpki-root-certs`。ルートの licenses.md で S053 から許可済み）。
- main の `App.svelte` の `FILE`（S125 と S127 の merge で残った、定義の無い名前）を `file` に直した。`pnpm check` が 1 error で落ち、smoke の `setFile` が実行時に ReferenceError になるため。`App.svelte` の変更はこの 1 語と「更新を確認」の 1 行。

## やったこと

- `src/Rust/apps/desktop/Cargo.toml`・`Cargo.lock`: `tauri-plugin-updater` 2.14.1（Desktop だけ）。48 パッケージ（新しい版を含む）が増えた。
- `src/Rust/apps/desktop/tauri.conf.json`: `plugins.updater`（`endpoints`・`pubkey` 空・Windows は passive）。
- `src/Rust/apps/desktop/capabilities/desktop.json`: `updater:default`（Linux・Windows・macOS だけ）。
- `src/Rust/apps/desktop/src/main.rs`: command `updater_configured`、`pubkey` があるときだけプラグインを入れる。
- `src/Rust/apps/desktop/deny.toml`: CDLA-Permissive-2.0。
- `src/Svelte/src/lib/tauri-backend.ts`: `checkUpdate()`。`src/Svelte/src/App.svelte`: ボタン 1 行と `FILE` の直し。
- `.github/workflows/release.yml`: ジョブ `desktop-ui`・`desktop`、`release` の SHA256SUMS に `GlobeMode_*`、`latest.json` の段。
- 文書: features.md、roadmap.md（P4 の行と S135）、ADR-0023（S135）、licenses.md、セッション文書と索引。

## 検証

- desktop の `cargo deny check`: advisories・bans・licenses・sources とも ok（警告 `license-not-encountered` 2 件は前からのもの）
- desktop の `cargo fmt --check`: ok。`cargo check`・clippy・Tauri のビルドは手元に webkit2gtk（pkg-config も）が無いので回していない（CI の `release.yml`・`desktop.yml` に任せる）
- `pnpm check`: 261 ファイル、0 errors・0 warnings（直す前は `FILE` の 1 error）
- `latest.json` の段: `.sig` のある .deb と無い .msi で `linux-x86_64-deb` だけが出る。`.sig` が無ければ「作らない」と出て終わる（手元、release.yml から抜き出して）
- `release.yml` の dry run: **回していない**。エージェントの `gh workflow run release.yml --ref feature/s135-desktop-updater` が権限の確認で止まった。feature は push 済み
- `check_docs.py`: デッキ 137 件 OK / 文書 558 件・相対リンク 2804 件中の切れ 0 件
- `render_slides.py`（S135 のデッキ）: 1 回目は 7 枚 × 4 の表示で問題 0 件。2 回目は Chromium の時間切れ（この環境の既知の問題）
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）

## 残したこと

- `release.yml` の dry run（`gh workflow run release.yml --ref feature/s135-desktop-updater`）。run id と artifact `globe-desktop-linux`・`globe-desktop-windows` の中身をここに足す
- 鍵の生成と secret の登録、`pubkey` の記入（ユーザー。手順は解説書の 4 と ADR-0023）
- 署名つきの Release からの実機での更新（Linux は pkexec、Windows は passive の msi）、Desktop の THIRD-PARTY.md、macOS
