# S132 `lsp-definition`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s132-lsp-definition` |
| worktree | `.git-worktree/s132-lsp-definition` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S132-lsp-definition/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

features.md の「補完・定義ジャンプ・参照・リネーム・ホバー」の行で未着手の定義ジャンプと参照を LSP に足す。`textDocument/definition` は部品の型・部品名・`R1.p` のような点つきの名前・extends の先・import を、文書の中とライブラリの中（`ClassIndex`・`LibraryTree`・`Outline` の範囲）へ。`textDocument/references` は同じ文書・同じクラスの中の字句の一致（全ライブラリ横断は持ち越し）。capability を宣言して VS Code 拡張から使えるように。試験は 3 本。リネーム・CodeMirror との接続は持ち越し。新しい依存は足さない。

## 決めたこと

既定のまま進めたこと（ユーザーと新たに決めたことは無い）:

- `ClassIndex` はワークスペースのファイルだけの索引で、ライブラリの索引は検査の内側にあり facade から引けない。そこで `ClassIndex::resolve` と同じ順（スコープの入れ子 → そのクラスの import → 外側）を `definition.rs` に書き、クラスの有無は「開いた文書の `Outline` → `LibraryTree` で決めたファイルの `Outline`」で確かめる。
- ライブラリのファイルは、別の空の `Workspace`（`Server::scratch`）に要るものだけ `set_file` してアウトラインにする。frontend に直接依存しない（facade だけ）。MSL 全部の解析を先にしない。
- 部品名を先に引く（内側のクラスから外へ、extends の先も深さ 8 まで）。点つきの名前は、部品の型を宣言したクラスの中から引いて次の区切りへ進む。
- 範囲は `Outline` の範囲そのまま（クラスは名前から `end` まで、部品は名前だけ）。
- 参照は、カーソルを含む一番内側のクラスの中の字句の一致（前が識別子の文字か `.` なら除く）。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/definition.rs`（新規）: `name_at`・`Resolver`（`locate`・`resolve`・`component`・`definition`）・`references`。
- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `textDocument/definition`・`textDocument/references` の dispatch、`initialize` の `definitionProvider`・`referencesProvider`、`Server::scratch`、`lsp_position`・`file_uri`、試験 3 本。
- `doc/features.md`・`doc/roadmap.md`・`doc/sessions/README.md`、セッション文書。

## 検証

- `cargo test -p globe-mode-lsp`: 16 本通過（新しい 3 本と `name_at` の単体 1 本を含む）。最初の実行は 2 本落ちた（期待した列が違った。`Outline` の範囲はクラスが名前から、部品は名前だけ）ので試験を直した
- コードの検査: `mise run check` の CI の判定 25 本・fmt・clippy は通過。test は共有 target で別の worktree が `globe` の試験のバイナリを消して 1 回落ちたので、`cargo test --workspace --no-fail-fast`（test result ok 64 件、失敗 0）・`cargo deny check`（advisories / bans / licenses / sources ok）・`uv sync --locked` を回し直して通過
- `check_docs.py`: S132 のデッキ OK / 文書 550 件・相対リンク 2761 件中の切れ 0 件。`render_slides.py`: 6 枚 × 3 の問題 0 件（最初の 2 回は負荷で 30 s の時間切れ、Chromium を片付けて 3 回目で通過）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- ブラウザ UI（CodeMirror）からの定義ジャンプ、リネーム、全ライブラリ横断の参照。
- 受け継いだ入れ子のクラス、コメントと文字列の中の名前、for の添字（`definition.rs` の shortcut）。
- MSL で最初の定義ジャンプは `LibraryTree` を作るため全ファイルの見出しを読む（S127 と同じ。2 回目からは使い回す）。
