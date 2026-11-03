# S127 `ui-library-tree`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s127-ui-library-tree` |
| worktree | `.git-worktree/s127-ui-library-tree` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S127-ui-library-tree/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

`doc/features.md` の「ライブラリツリー」（Must）の UI を作る（S048 の持ち越し）。MSL を読んだあと、パッケージの木をたどって `Modelica.Electrical.Analog.Examples.ChuaCircuit` を選び、そのソースをエディタに開いて check → simulate → プロットまで行けるようにする。ユーザーの方針は「先に全体の流れとして使えるアプリを完成させる」で、検索・アイコン・ダークモード・Diagram ビューは持ち越し。`globe-mode-exec`・`globe-mode`（facade）・`core`・`ir`・`fmi`、`desktop.yml`・`doc/bench`・ADR-0034、`doc/msl-*.md` は触らない。新しい依存は足さない。

## 決めたこと

- 木のデータは `globe-mode-fmt` の新しい `tree` モジュール（`LibraryTree`）に置いた。wasm と LSP の両方が直に依存し、依存 0 で wasm32 でも動く。facade と ir は触れない条件なので `ClassIndex` に口を足さなかった。分割読み込みの `ClassIndex` は読んだファイルの分しか持たないので、木には向かない。
- 見出し（`within`、最初のクラスの種類・名前・説明文字列）だけを小さな字句読みで読む。fmt の `lexer` はファイル全体を字句にするので使わなかった。
- 口は遅延の形: wasm `library_tree(prefix)`・`library_source(path)`、LSP `globe/libraryTree`・`globe/librarySource`。単位はファイル（1 ファイルのパッケージは葉）。
- 開くときは、エディタの文書の名前をライブラリのファイルの名前にする。同じ名前の `setFile` がライブラリのファイルを差し替えるので、重複の定義にならず、編集が検査・実行に効く（試験で確かめた）。
- 並びは名前の順。ブラウザの MSL.json は `.mo` だけで `package.order` が無い。
- wasm は読み込みに渡したファイルの組を持つ（約 15 MiB の 2 重持ち。shortcut のコメントを付けた）。LSP はディスクから読む。

## やったこと

- `src/Rust/mod/globe-mode-fmt/src/tree.rs`（新規）・`lib.rs`: `LibraryTree`・`TreeEntry` と単体の試験。
- `src/Rust/mod/globe-mode-wasm/src/lib.rs`: `Workspace::with_tree`・`tree_children`・`tree_source`、JS 向け `library_tree`・`library_source`。`tests/decay.rs` に `library_tree_opens_a_model_file_as_the_document`。
- `src/Rust/mod/globe-mode-wasm/web/client.js`・`worker.js`: `libraryTree`・`librarySource`。
- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `globe/libraryTree`・`globe/librarySource` と試験 `library_tree_and_source`。
- `src/Svelte/src/lib/backend.ts`（interface と `TreeEntry`）・`worker-backend.ts`・`tauri-backend.ts`、新しい部品 `LibraryTree.svelte`、`App.svelte`（文書の名前を `$state` に、木の差し込み、`openLibrary`）。S125 の smoke の行には触れていない。
- `src/Svelte/tests/test_ui.py`: MSL を読んだあと、木で Modelica → Electrical → Analog → Examples → ChuaCircuit を開いて検査（error 0）。
- 文書: features.md・roadmap.md（P4 の行、S048 の持ち越し、S127 で済んだこと）・architecture.md（`globe-mode-fmt` の行）・索引・スライド・解説書。

## 検証

- `cargo test -p globe-mode-fmt -p globe-mode-wasm -p globe-mode-lsp`: 37 件成功・1 件 ignored（fmt 7、wasm の単体 9・`decay` 10・その他、lsp 11）
- `mise run wasm:build`: 成功（release 46.8 s、wasm 15,430,281 バイト）
- `pnpm check`: 0 errors・0 warnings。`pnpm build`: MSL.json 2,556 ファイル、15.2 MiB
- `test_ui.py`: 成功（全体 80 s）。木から ChuaCircuit を開くまで 0.2 s、検査まで 1.0 s、診断 0 行・error 0 行。既存の行（MSL を読む 0.9 s、ChuaCircuit の検査 1.8 s・実行 0.8 s、graph 7 ノード・9 辺、GM-SMOKE msl=ok）も通る
- `mise run check`: `cargo fmt --check`・`cargo clippy --workspace --all-targets -D warnings` は成功。`cargo test --workspace` は lib と結合の 46 件の test result がすべて ok、doctest は共有 target の取り合い（別の worktree が同時に `globe-mode-fmt`・`analysis` をビルドし E0463・E0460）で落ち、`-p globe-mode --doc` だけ単独で回し直して 1 件成功。残りの doctest は通っていない（この変更は doctest を足していない）。`cargo deny check` は advisories・bans・licenses・sources が ok、`uv sync --locked` も成功
- `check_docs.py`・`render_slides.py`: `check_docs.py` はデッキ OK、文書 532 件・相対リンク 2,668 件で切れ 0 件。`render_slides.py`（S127 のデッキ、3 つの表示 × 6 枚）は問題 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 検索ボックス、アイコン、`package.order` の順、ダークモード、Diagram ビュー。
- 1 ファイルのパッケージの中の入れ子のクラス（今は葉で、ファイル全体を開く）。
- wasm のライブラリの本文の 2 重持ち（facade が分割読み込みの本文を返せるようになったら外す）。
- Desktop（Tauri の sidecar）での木は LSP の試験だけで、実アプリでは確かめていない。
- `wasm:node`・`wasm:compare` は実行の道を変えていないので回していない。
