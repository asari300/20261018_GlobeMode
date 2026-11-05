# S186 `ui-rename-follow-open-position`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s186-ui-rename-follow-open-position` |
| worktree | `.git-worktree/s186-ui-rename-follow-open-position` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S186-ui-rename-follow-open-position/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S185 の監査（[doc/research/app-flow-2026-11-06.md](../../doc/research/app-flow-2026-11-06.md)）の R2・R3・R7・R8・R9（WP の案 B1・B3）を Svelte 版 UI で直す。記述どおりに再現してから直す。

## 決めたこと

- **R2 は Rust を変えない**。依頼は `rename` の応答に新旧の完全修飾名を足す案だったが、`globe/rename`・wasm `rename` の応答は編集の配列そのもので、追加のフィールドを足すと形が変わる（既存の呼び手が壊れる）。代わりにリネームの前後で `Backend.models(file)`（S170。Rust の outline から作る実行できるクラスの完全修飾名の列）を引き、いまのモデルの番号の名前が変わったら追う。リネームは宣言の順序を変えないので同じ番号が同じクラス。名前解決の規則は Svelte に無い。
- 欄の書き換えと Ctrl+Z は CodeMirror の `invertedEffects` に載せる（リネームの編集と同じ transaction に `modelRenamed(from, to)`）。欄が from のときだけ書き換える（利用者が直した名前は触らない）。
- **R3 は Rust に `range` を足した**。`librarySource` の応答（wasm `library_source`・LSP `globe/librarySource`）に追加の `range`。`Resolver::library_class_range` が、読んだファイルのアウトライン（`scratch`）で既存の `walk` を使う。`locate` はワークスペースの文書を先に見るので使わなかった。UI は開いた後に宣言の行頭へカーソル（フォーカスは動かさない）。タブの `title` には既にパスが出ていた（`title={f}`）ので足さなかった。
- R7 は `closeCompletion(view)` をショートカットの処理に。R8 は `:focus-visible` のボタン（キーボードで押した）のときだけ、読み終えてフォーカスが body なら検索欄へ。R9 は `focus({preventScroll})` と `scrollIntoView({block: "start"})`。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/definition.rs`: `Resolver::library_class_range`（private）、`library_source`（pub。`{file, text, range?}`）。`lib.rs`: `globe/librarySource` が呼ぶ。LSP の表と試験に range。
- `src/Rust/mod/globe-mode-wasm/src/lib.rs`: `library_source` が同じ関数を呼ぶ。`node/smoke.mjs` に BusUsage の行の確認。
- `src/Svelte/src/App.svelte`・`lib/editor.ts`（`modelRenamed`・`modelFollow`）・`backend.ts`・`worker-backend.ts`・`tauri-backend.ts`（型に `range?`）。
- `src/Svelte/tests/test_ui.py`: 5 点。
- CHANGELOG・features・roadmap・索引・セッション文書。

## 検証

- 再現（S185 の記述どおり、直す前の main の挙動はコードで確認）: R2 は欄が更新されない（`rename()` は `view.dispatch` だけ）、R3 は `openLibrary` が本文を開くだけ、R7 は `shortcut()` が候補を閉じない、R8 は `loadMsl` がフォーカスを扱わない、R9 は `focus()` だけ。
- wasm: main `2b88542` に今回の差分を載せた作業ツリーを release でビルド（`globe-mode-wasm`、wasm-bindgen 0.2.129）。出力先は `~/.cache/globe-mode-s186/wasm-pkg`（終わったら削除）。
- `cargo test -p globe-mode-lsp library_tree_and_source`: 通った。
- `node smoke.mjs --msl data/msl`（`mise run wasm:node` と同じ中身。`--pkg` で上の出力を指定）: ok（BusUsage の range を含む）。
- `src/Svelte/tests/test_ui.py`（`pnpm check` 0 errors、`pnpm build`、main の `.venv` の `python -I`）: ok。GM-SMOKE の行は変えていない（最後の smoke の項目も全部 ok）。
- `mise run check`: 通った（fmt・clippy・test 695 passed 0 failed・deny・`uv sync --locked`）。1 回目は並行の S187 のビルドと共有 target が重なり `globe-mode-db` の `switching` の 2 本が落ちたが、単独で 11 passed、2 回目の check は全部通った（今回の差分は db を触らない）。
- `mise run wasm:compare` は回していない（シミュレーションの経路は変えていない。librarySource の追加のフィールドだけ）。
- `check_docs.py`: デッキ OK / リンク切れ 0 件（文書 756 件・相対リンク 3806 件）。`render_slides.py`（S186 のデッキだけ）: 問題 0 件、画像を目で確認。
- セッション記録の `--check`・漏洩検査は、記録を写すのが親なので親が行う。ステージした差分の漏洩検査（メール・ホームのパス・トークン）は 0 件。

## 残したこと

- R6（候補の順位）・R5（`-L` の案内）・R1（README）・R4（FMU）は別のセッション。
- 入れ子のクラスを開いたタブの名前は `package.mo` のまま（ツールチップにパス）。
- wasm の `library_source` の range は `wasm:node` の smoke で確かめた（ネイティブの単体試験は JsValue のため書けない）。
