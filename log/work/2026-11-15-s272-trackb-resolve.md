# S272 `trackb-resolve`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s272-trackb-resolve` |
| worktree | `.git-worktree/s272-trackb-resolve` |
| Phase | Track B（B2 の最初の一切れ） |
| セッション文書 | `doc/sessions/S272-trackb-resolve/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

型付き AST をクラス・宣言・`extends`・`import` まで足し、クラスの木と Modelica 3.6 の 5.3 の探索で、型名と成分参照の先頭をクラスか部品に解決する（IDE の機能に使える範囲）。Rumoca 側（`class_index`、LSP）にはつながない。代わりに MSL 全体で解決率を数え、可能なら LSP の定義ジャンプと突き合わせる。

## 決めたこと

- 新しい crate `globe-mode-resolve`（依存は syntax と rowan の `TextRange` だけ）。ADR は新設せず ADR-0039 に付記（番号の衝突を避けるため）。
- クラスの木は各ファイルの `within` で組む（ディレクトリ名に頼らない。MSL は `Modelica 4.1.0/` のような版つきのディレクトリ）。`package.order` は読まない。
- ファイルごとに木を捨てる。クラス情報と `Ref`（型名・`extends`・`import`・式の成分参照）だけ持つ。
- `annotation` の中と `redeclare` の引数の中は読まない（opaque）。`for` の添字は節点の範囲で局所にする。
- `class extends X` は外側のクラスの継承元の同名のクラスを継ぐ（初回の未解決 2229 件のほとんどがこれだった）。先頭 `.` の組み込み（`.sqrt`）も組み込みとして扱う。
- 失敗が `redeclare` つきの `extends` か `replaceable` なクラスに関わるなら `Reason::Redeclare`（型が変わりうるので未対応）。

## やったこと

- `globe-mode-syntax/src/ast.rs`: `StoredDefinition`・`ClassDef`・`ShortBody`・`ExtendsClause`・`ImportClause`（`ImportKind`）・`ComponentClause`・`Declaration`・`Name`。
- `globe-mode-resolve`（新規）: `lib.rs`（型）、`build.rs`（取り出しと `within` での組み立て）、`lookup.rs`（探索）、`tests/resolve.rs`。
- `globe-mode-lsp`: dev-dependency に resolve を足し、`tests/resolve_compare.rs`（`#[ignore]`）で LSP の `textDocument/definition` と MSL の型名で突き合わせる。
- doc: ADR-0039 の付記、roadmap（Track B の 2 か所）、architecture の crate 表、features、ADR 索引、セッション文書と索引。

## 検証

- `cargo test -p globe-mode-resolve`: 3 pass・1 ignored。`cargo clippy -p globe-mode-syntax -p globe-mode-resolve --all-targets -- -D warnings` 警告 0、fmt ok。
- MSL 4.1.0（debug）: 2556 本・クラス 6751・参照 161869 件。未解決 10 件（すべて `Redeclare`。`Medium.saturationPressure` など）。
- LSP との突き合わせ: MSL の型名を、ファイルを 40 本おきに選んで 1 ファイル最大 3 箇所、計 110 箇所（`Modelica.Units.SI.Force` など、修飾つきの名前を含む）で、LSP の `textDocument/definition` の結果（ファイルとクラス名の位置）と自前の解決が **110 箇所すべて一致**（LSP が null・不一致は 0）。debug で約 5 分（大半は LSP 側の読み込み）。
- 依存の変更なし（`Cargo.lock` は新しい workspace crate の行だけ）。

## 残したこと

- `redeclare` / `replaceable` と修飾で変わる型、部品の型を通した `a.b.c` の続き、`protected` へのアクセス検査。
- LSP・`Frontend` へのつなぎ（`class_index` の置き換え）、文の型付き AST、診断の番号（`GM-E`）。
