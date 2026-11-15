# S350 `frontend-select-default`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s350-frontend-select-default`（base は `feature/s349-native-check-desktop`。S349 が main に未着だったため） |
| worktree | `.git-worktree/s350-frontend-select-default` |
| Phase | Track B（自前コア）の製品へのつなぎ |
| セッション文書 | `doc/sessions/S350-frontend-select-default/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

検査のフロントエンドの select に「設定に従う」（既定）を足し、選んでいるあいだは `frontend` を渡さない（LSP が設定 → 既定の順で決める）。ブラウザ・Android は設定ファイルが無いので Rumoca と同じ扱いで、表示は既存の形に合わせる。Svelte に業務ロジックを置かない。`test_ui.py` の `check_frontend_select` と `test_tauri.mjs` を直し、「設定に従う」で `frontend` が乗らないことを確かめる。ADR-0039 の付記 S349 の注を直す。

## 決めたこと

- **3 択、既定は「設定に従う」**（App.svelte の値は `config`）。ブラウザ・Android も同じ 3 択（依頼の「表示は既存の形に合わせる」を、実行環境ごとに選択肢を変えない、と読んだ）。ブラウザの「設定に従う」は `frontend` なし = Rumoca。
- **`CheckResponse` に `frontend`（自前の段で検査したときだけ `"native"`）を足した。** 依頼の範囲は UI だけだったが、「設定に従う」で Desktop の設定が native のとき、UI はどちらで検査したか知らない。知らないと要約が出ず、`GM-U0006` の原文の見出しが「Rumoca の原文」になる（誤り）。UI に設定の読み方を置く（Svelte に業務ロジック）より、Rust が応答で言うほうがよいと判断した。欄は 2 か所の構築と `native_response` だけで、Rumoca の応答と既存の JSON は変わらない。S349 は「protocol の型は変えない」と決めていたが、それは要求の型の話で、応答の欄を 1 つ足すのは別。
- **UI の条件は「選んだ値が native、または応答の `frontend` が native」。** 応答の `frontend` だけにしないのは、この WP で wasm を作り直さず（RAM と依頼の制約）、共有 target にある S347 の wasm（応答に `frontend` が無い）でブラウザの試験を回したため。古い wasm・sidecar でも、自前を選んだときの要約は今までどおり出る。
- **S349 の解説書・スライドの「次回」は書き換えない**（その時点の記録）。ADR-0039 の付記 S349 の注だけ、S350 で直した旨に変えた。

## やったこと

- `src/Rust/mod/globe-mode-protocol/src/lib.rs`（`CheckResponse.frontend`）、`src/Rust/mod/globe-mode/src/lib.rs`（`native_response` が付ける、`check_rumoca` は `None`）。
- `src/Svelte/src/App.svelte`（`checkFrontend` を `CheckFrontend | "config"` に、「設定に従う」を足す、`config` なら `undefined` を渡す、要約の条件）、`lib/protocol.ts`（`frontend?: "native"`）、`lib/backend.ts`（コメントだけ）。
- 試験: LSP `config_frontend_native_drives_diagnostics`（設定が native で `globe/check` が `frontend` を省くと native、`rumoca` を渡すと Rumoca）・`check_selects_the_frontend`（native の応答に `frontend`）、facade `tests/check_with.rs`、`tests/test_tauri.mjs`、`tests/test_ui.py` の `check_frontend_select`。
- 文書: ADR-0039（付記 S349 の注と S347 の UI の行を直し、付記 S350）、CHANGELOG、README（英・日）、features、roadmap、`doc/sessions/S350-frontend-select-default/`、`doc/sessions/README.md`、この記録。

## 検証

- `cargo test -p globe-mode-lsp --features native --lib -- config_frontend_native check_selects`: 2 本 OK。`cargo test -p globe-mode --features native --test check_with`: 2 本 OK。
- `cargo fmt --check`（protocol・globe-mode・lsp）OK、`cargo clippy -p globe-mode-protocol -p globe-mode -p globe-mode-lsp --all-targets --features globe-mode-lsp/native -- -D warnings` 0 件。
- `node src/Svelte/tests/test_tauri.mjs`: ok。`pnpm check`（svelte-check）: 287 ファイル、error 0・warning 0。
- UI: `pnpm build` のあと、`check_frontend_select` だけを Playwright で（共有 target の S347 の wasm〔`wasm-pkg/web`、19,598,045 バイト〕）: OK、console の error 0。Desktop の偽の Tauri で、既定「設定に従う」・`rumoca`・`native`・「設定に従う」・「設定に従う」（応答が `frontend: "native"`）の順に、`globe/check` の `frontend` は `None・rumoca・native・None・None`、要約は「自前」の回と、最後の（応答が `frontend: "native"` の）回だけに出て、それ以外では出なかった。`test_ui.py` の全体と `test_graph.py` は回していない（select 以外の UI は変えていない。RAM が厳しいため）。
- 回していないもの: `mise run check` 全体（deny・uv。依存を足していない）、release・wasm のビルド、`wasm:node`・`wasm:compare`（応答の欄を 1 つ足しただけで、wasm の挙動を変えていない）、`msl:*`。実機の Desktop（Tauri）の起動は【未確認】。
- セッション記録は無いので `--check` は省く。

## 残したこと

- 実機の Desktop での目視。
- release の wasm の作り直し（CI が作る wasm では応答に `frontend` が載る。この WP のローカルの試験は古い wasm）。
