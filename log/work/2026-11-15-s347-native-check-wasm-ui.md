# S347 `native-check-wasm-ui`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s347-native-check-wasm-ui` |
| worktree | `.git-worktree/s347-native-check-wasm-ui` |
| Phase | Track B（自前コア）の製品へのつなぎ |
| セッション文書 | `doc/sessions/S347-native-check-wasm-ui/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

wasm とブラウザ・Tauri の UI からも、検査に native（自前の Track B の段）を選べるようにする（既定は Rumoca のまま）。wasm の `check` に frontend の指定を足し、`native` を入れたときの wasm の大きさの増分を測って書く（大きすぎるなら既定に入れるかを決めずに止めて報告）。UI の検査の設定に選択を 1 つ（業務ロジックは置かない）。試験は wasm の Node で native の check が通る 1 本と、UI の既存の形に合わせた 1 本。`wasm:node`・`wasm:compare` を通す。LSP と VS Code は触らない（S345 と並行）。

## 決めたこと

- **大きさ**: 同じ release の設定（`mise run wasm:build` と同じ fat LTO・codegen-units 1・opt-level s・strip）で、`-p globe-mode-wasm` を `native` なし・ありで続けてビルドした（共有 target。スクリプトはスクラッチパッド）。

  | | cargo の出力 | `wasm-bindgen --target web` 後 | `gzip -9` |
  | --- | ---: | ---: | ---: |
  | なし | 20,885,103 | 19,242,637 | 5,891,577 |
  | あり | 21,261,979 | 19,597,599 | 5,998,898 |
  | 増分 | +376,876 | +354,962（+1.84%） | +107,321（+1.82%） |

  小さいので**既定の feature に入れた**（`default = ["native"]`）。CI・android・web・release・ui-bench が `cargo build -p globe-mode-wasm`（や `mise run wasm:build`）のままで入る。診断の注の文字列を足したあとの配る `.wasm` は 19,598,045 バイト（+355,408）。
- **口**: `Workspace.check_with(model, frontend)` を足し、`check(model)` は変えない（facade の `check` / `check_with` の組と同じ。既存の呼び出しが 1 つも変わらない。`frontend` が `rumoca`・`native` 以外なら例外）。Worker の `check({model, frontend})`、`GlobeWorker.check(model, frontend?)`。
- **UI**: 検査ボタンの横の select「Rumoca / 自前（試験）」。`Backend.check(model, frontend?)`、選べる実行環境だけの `Backend.checkFrontends`（`WorkerBackend` は true）。native で検査したら Rust の要約（`CheckResponse.summary`。S0〜S6 までしか見ていない旨を含む）を 1 行出し、`GM-U0006` の原文の見出しを「自前の段の原文」にする。選択は保存しない（ページを開くたびに Rumoca）。
- **Desktop は出さない**: sidecar の LSP `globe/check` は `frontend` を取らない（S313）。取らない要求に選択を出すと黙って Rumoca になる。LSP は S345 と並行なので触らず、`TauriBackend` は `checkFrontends` を持たない。LSP が `params.frontend` を読むようになれば、渡して `checkFrontends = true` にするだけ。
- **ブラウザの MSL**: wasm で MSL を読んだワークスペース（`with_library_sources`）は、native から MSL のクラスが見えない（ライブラリのファイルが DB の外の `Checker` にある。S313 の制限）。Node で `ChuaCircuit` を native で検査すると `GM-E1002`、MSL のクラスを使う自分のモデルは `GM-E1001` で止まり、モデルの誤りに見える。UI から選べるようにしたので、facade の `Native` の道で、ファイルの組で読んだライブラリがある（`library_files > 0` かつ `library_paths` が空）ときの `GM-E1001`・`GM-E1002` に注「native は … ライブラリのクラスを見られない。Rumoca で確かめる」を足した。ライブラリを native に見せる（facade が本文の写しを持つ）のは MSL の写しが増えるのでやらない。

## やったこと

- `src/Rust/mod/globe-mode-wasm/Cargo.toml`（feature `native`、既定）、`src/lib.rs`（`check_with`）、`web/worker.js`・`web/client.js`、`tests/decay.rs`（`check_with_selects_the_frontend`）、`node/smoke.mjs`（Decay の native・rumoca は `check` と同じ・知らない名前は例外、MSL があれば `ChuaCircuit` の注）、`web/worker-node.mjs`（Worker 越し）。
- `src/Rust/mod/globe-mode/src/lib.rs`（`Native` の道の注）、`tests/check_with.rs`（`native_explains_a_library_it_cannot_see`）。
- `src/Svelte/src/App.svelte`・`lib/backend.ts`・`lib/worker-backend.ts`・`lib/protocol.ts`（`summary`）、`tests/test_ui.py`（`check_frontend_select`）。
- 文書: ADR-0039 の付記 S347（と S313 の付記の持ち越し・「wasm は立てない」の追記）、roadmap の Track B の 2 か所、features の診断の行、architecture の `globe-mode-wasm` の行、README（英・日）、CHANGELOG、`doc/sessions/S347-native-check-wasm-ui/`、`doc/sessions/README.md`、この記録。

## 検証

- `cargo test -p globe-mode-wasm`: 29 本 OK（新しい `check_with_selects_the_frontend` を含む）。`cargo clippy -p globe-mode -p globe-mode-wasm --features globe-mode/native --all-targets -- -D warnings` 0 件、`cargo fmt --check` OK。
- `cargo test -p globe-mode --features native --test check_with`: 2 本 OK（`native_explains_a_library_it_cannot_see` を含む）。
- `mise run wasm:node`（data/msl を worktree に symlink して MSL も）: 全部 OK（`check_with(native): …Decay: OK（native: S0〜S6 を通過。S7 以降は検査していない）`、MSL の `ChuaCircuit` は Rumoca が通り native は注つきで止まる、MSL の分割読み込みが全部読みと同じ 5 本一致）。`web/worker-node.mjs` OK。
- `mise exec -- pnpm check`（svelte-check）: 287 ファイル、error 0・warning 0。`test_ui.py` の `check_frontend_select` だけを先に回して OK（console の error 0）。
- `cargo test -p globe-mode --features native`: lib・tests・doctest が全部 OK（doctest は 1 回、共有 target の入れ替わりで落ちたが、そのあと同じコマンドで 1 本 OK）。
- `mise run wasm:compare`: **480 件中 466 件が一致、既知の差 13 件、一致しない 1 件**（`Pendulum` / `dopri853`、相対差 2.0e-9、閾値 1e-9）。S322 の記録と同じ数字で、この WP は検査の口と UI だけなので実行には触れていない。
- UI（`mise run ui:check` の中身）: `pnpm check` 0 error、`pnpm build`、`test_ui.py` は最後まで OK（新しい `check_frontend_select` を含む。console の error 0）、`test_graph.py` OK。
- `check_docs.py`: スライドは S347 を含め全部 OK、リンクは文書 1,234 件・相対リンク 5,211 件中の切れ 0 件。`render_slides.py`（S347 のデッキだけ）: デッキ 1 件・問題 0 件（負荷が高い間は 1 回、スクリーンショットが時間切れ。下がってから通した）。
- 回していないもの: `mise run check` 全体（deny・uv）は、依存を足していないので省いた（`cargo fmt --check`・clippy・変えた crate の試験だけ）。`msl:*`・`wasm:web` の Chromium 側（`browser.py`）・`mise run docs:check` 全体は回していない（並行の WP で RAM が厳しい。S347 のデッキだけ検査した）。
- セッション記録は無いので `--check` は省く。

## 残したこと

- **Desktop の UI**: sidecar の LSP `globe/check` が `params.frontend` を読むようにして、`TauriBackend` が `check(model, frontend)` で渡し `checkFrontends = true` にする（S345 の後。LSP の変更が要る）。
- **ブラウザの MSL を native に見せる**: facade が `with_library_sources` のファイルの写しを持つ必要がある（MSL の本文が 2 重になる）。分割読み込み（S037）の塊を native に渡す道も要る。今は注を付けるだけ。
- native の検査は S0〜S6 まで。実行（`simulate`）・BLT・段・グラフは Rumoca のまま。選択は保存しない。
