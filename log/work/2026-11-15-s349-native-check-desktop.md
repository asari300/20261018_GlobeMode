# S349 `native-check-desktop`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s349-native-check-desktop`（base は `feature/s347-native-check-wasm-ui`。S347 が main に未着だったため） |
| worktree | `.git-worktree/s349-native-check-desktop` |
| Phase | Track B（自前コア）の製品へのつなぎ |
| セッション文書 | `doc/sessions/S349-native-check-desktop/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

Desktop（Tauri の sidecar の LSP）からも、検査に native を選べるようにする（S347 の持ち越し）。LSP の `globe/check` が任意の `params.frontend`（`"rumoca"`・`"native"`。無ければ `globe-mode.toml` の `[check] frontend` か既定）を読んで facade の `check_with` に渡し、知らない名前は誤りで返す。`TauriBackend` が渡して `checkFrontends = true`。試験は LSP で native を選ぶ 1 本と、`test_tauri.mjs` など既存の Desktop の試験の形に合わせた 1 本。VS Code の拡張は触らない。ADR-0039 付記・roadmap・features を直す。

## 決めたこと

- **名前の解釈は facade の `CheckFrontend::from_str`**（CLI の `--frontend`・wasm・Python と同じ）。LSP に名前の表を持たない。
- **知らない名前・文字列でない値は -32602**（`params.model` が文字列でないときと同じ code）。黙って Rumoca にすると、S347 が避けた「選んだのに効かない」になる。
- **`frontend` が無いときは設定、無ければ Rumoca**。`publishDiagnostics` と同じ決め方（`[check] frontend`）。設定が無ければ今までと同じなので、`frontend` を渡さない呼び出しは変わらない。診断側の `lint.frontend.parse().ok()` を共通の関数にまとめることは、動いているコードに触るのでやらなかった。
- **protocol の型は変えない**: `globe-mode-protocol` に検査の要求の型は無く（応答の `CheckResponse` だけ）、TS の `CheckFrontend` は S347 のまま。依頼の「protocol の型・TS の型も合わせる」は、足す型が無いことを確かめて済みとした。
- **App.svelte は変えない**: select は最初から `rumoca` を渡すので、Desktop の「検査」ボタンは `[check] frontend = "native"` があっても select を変えない限り Rumoca で検査する（波線の診断は設定どおり）。これは ADR に書いた。
- **VS Code の拡張**: 拡張は `globe/check` を使わない（`src/VSCode` を grep して 0 件）ので、触らない。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `Server::check_frontend`（params → `CheckFrontend`）、`globe/check` が `check_with` を呼ぶ、先頭の表の `globe/check` の行、試験 `check_selects_the_frontend`。
- `src/Svelte/src/lib/tauri-backend.ts`（`check(model, frontend?)`・`checkFrontends = true`）、`lib/backend.ts`（コメントだけ）。
- 試験: `src/Svelte/tests/test_tauri.mjs`（`params.frontend` に乗る・省けば乗らない）、`tests/test_ui.py` の `check_frontend_select`（Desktop の偽の Tauri に select があり、`globe/check` に `rumoca`・`native` の順で届く）。
- 文書: README（英・日の「Desktop はまだ選べない」を直した）、ADR-0039 の付記 S349（と S347 の「Desktop は未」の追記）、roadmap の Track B の 2 か所、features の診断の行、CHANGELOG（S349 の行と既知の限界）、`doc/sessions/S349-native-check-desktop/`、`doc/sessions/README.md`、この記録。

## 検証

- `cargo test -p globe-mode-lsp --lib check_`: `check_selects_the_frontend`・`custom_requests_check_simulate_and_graph` OK。
- `cargo test -p globe-mode-lsp --features native --lib`: 72 本 OK・1 本 ignored（既定の feature でも `check_selects_the_frontend` OK）。`cargo clippy -p globe-mode-lsp --all-targets -- -D warnings`（既定・`--features native` の両方）0 件、`cargo fmt --check -p globe-mode-lsp` OK。
- `node src/Svelte/tests/test_tauri.mjs`: ok。`pnpm check`（svelte-check）: 287 ファイル、error 0・warning 0。
- UI: `pnpm build` のあと `test_ui.py` を最後まで（MSL つき。worktree の `data/msl` は main の `data/msl` への symlink で、`.gitignore` 済み）、`test_graph.py` ともに終了コード 0（`check_frontend_select` は Desktop の select と `globe/check` の `frontend` が `rumoca`・`native` の順で届くところを含めて OK、console の error 0）。1 回目は MSL の symlink が無く、最後の smoke（`MSL.json` 404）で落ちた。symlink を張って再実行した。
- 文書: `check_docs.py` は全デッキ OK、リンクは文書 1,243 件・相対リンク 5,229 件中の切れ 0 件。`render_slides.py`（S349 のデッキだけ）: デッキ 1 件・問題 0 件（1 回目は負荷でスクリーンショットが時間切れ、負荷が下がってから通した）。
- 回していないもの: `mise run check` 全体（deny・uv）は依存を足していないので省き、`cargo fmt`・clippy・変えた crate の試験だけ。`wasm:build`（release の wasm）は、wasm の挙動を変えていないので回さず、共有 target に残っていた S347 の build（`wasm-pkg/web`、19,598,045 バイト）を UI の試験に使った。`wasm:node`・`wasm:compare`・`msl:*` は回していない（実行系・wasm・frontend を変えていない）。実機の Desktop（Tauri）の起動は【未確認】。
- セッション記録は無いので `--check` は省く。

## 残したこと

- 実機の Desktop での目視。
- Desktop の「検査」ボタンは、select が `rumoca` のあいだ設定の `[check] frontend` を使わない（`globe/check` の応答だけの話。波線は設定どおり）。揃えるなら select の既定を「設定に従う」にする別の WP。
- `simulate`・BLT・段・グラフは Rumoca のまま。
