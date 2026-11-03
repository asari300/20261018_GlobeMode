# S141 `ui-file-open-save`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s141-ui-file-open-save` |
| worktree | `.git-worktree/s141-ui-file-open-save` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S141-ui-file-open-save/` |
| セッション記録 | （親が写す） |

## 依頼

S140 の点検（`doc/research/app-flow-2026-11-04.md`）の順位 1。自分のモデルを持ち込み、持ち帰れるようにする。「開く」「保存」を、ブラウザ・Desktop・Android の 3 列で。最小の変更で。複数の文書、最近使ったファイル、FMU export の UI、実行の設定の UI（S142）、ダークモードはやらない。

## 決めたこと

既定のまま進めたこと（ユーザーと新たに決めたことは無い）:

- 口は `Backend.openFile()`・`saveFile(name, text, saveAs?)`。ブラウザ（と sidecar の無い Android）は `<input type=file>` と Blob のダウンロード、Desktop は dialog / fs プラグイン。
- File System Access API は使わない（最小は input でよいとの指示。Firefox・Safari に無く、分岐が増える）。ブラウザの「保存」は毎回ダウンロード。
- Desktop は command を足さず、`window.__TAURI__.dialog` / `fs` を使う（S135 の流儀）。`generate_handler!` は触っていないので S136 の `cancel_run`・S135 の `updater_configured` の並びはそのまま。プラグインは `Builder::plugin` で 2 行。
- fs のスコープは静的に置かない（dialog で選んだパスが許可に入る）。
- 自動保存は本文とファイル名だけ（モデル名は保存しない）。最初の版はモデル名も保存して、desktop.yml の UI の測定（起動を 3 回繰り返す）で 2 回目から落ちた（smoke が `model` を ChuaCircuit にして、次の起動がそれを拾い、MSL を読む前に検査して check=ng。localStorage は起動をまたぐ）。本文から入れ直す形に直した。ライブラリのファイルを開いている間は止める（戻すと MSL のファイルが次回の起動で開いてしまう）。
- 開くとき、名前が違えば前のファイルを `setFile(前の名前, "")` で空にする（Workspace に残る二重定義の予防）。
- 開いた本文の先頭の宣言の名前をモデル名に入れる（既定の `Decay` のままだと検査が通らない）。

## やったこと

- `src/Svelte/src/lib/backend.ts`・`worker-backend.ts`・`browser-file.ts`（新）・`tauri-backend.ts`: 口と 2 つの実装。
- `src/Svelte/src/App.svelte`: 「開く」「保存」（Desktop は「名前を付けて保存」）、`localStorage` の保存と復元（`own` の旗でライブラリのファイルを除く）、保存先の 1 行。simulate の周りは触っていない。
- `src/Svelte/tests/test_ui.py`: 1 本（`Mine.mo` を開く、モデル名、検査、ダウンロードの内容、再読み込みで復元）。
- `src/Rust/apps/desktop`: `Cargo.toml` に `tauri-plugin-dialog` 2.8.4・`tauri-plugin-fs` 2.7.1（Desktop だけ）、`main.rs` の `plugin` 2 行、`capabilities/desktop.json` の 4 つの権限、`Cargo.lock`（3 パッケージ増: rfd 0.17.2 ほか）。`deny.toml` は変更なし。
- `doc/licenses.md`・`doc/features.md`・`doc/roadmap.md`（P4 の行と「S141 で済んだこと」）・ADR-0023 に 1 段落。

## 検証

- `pnpm check`: 0 エラー 0 警告。
- `test_ui.py`（wasm を `mise run wasm:build` で作り直して、`pnpm build` 後）: 通過（`開く・保存: Mine.mo を開いて検査が通り、ダウンロードの内容が一致し、再読み込みで戻った`、MSL も ok、console の error と外への要求は 0）。
- `cargo deny`（desktop の manifest と deny.toml）: advisories・bans・licenses・sources とも ok。
- Desktop のビルド: 手元では回さない（webkit2gtk が無い）。`desktop.yml`: 下の欄。
- コードの検査（root の `mise run check`）: root の Rust は触っていないので回していない。
- `check_docs.py`・`render_slides.py`・リンク・`--check`・漏洩検査: 下の欄。

（仕上げで埋める）

## 残したこと

- File System Access API（同じファイルへの上書き）、複数の文書（タブ）、最近使ったファイル。
- Android の WebView でのダウンロードとファイル選択は実機で確かめていない（`accept=".mo"` の絞り込みと、Blob のダウンロードが WebView で動くか【未確認】）。
- Desktop の dialog の操作そのものは自動試験が無い（smoke はプラグインを入れて起動するところまで）。
- FMU export の UI、実行の設定の UI（S142）、ダークモード。
