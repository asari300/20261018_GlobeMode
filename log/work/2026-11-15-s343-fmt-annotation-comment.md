# S343 `fmt-annotation-comment`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s343-fmt-annotation-comment` |
| worktree | `.git-worktree/s343-fmt-annotation-comment` |
| Phase | 整形（`globe-mode-fmt`） |
| セッション文書 | `doc/sessions/S343-fmt-annotation-comment/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

issue #63「annotation の引数を並べ替えると、後ろの行コメントのせいで `,` が次の行の頭に出る」を直す。再現を自分で確かめ、最小の修正と回帰試験を 1 つ。`globe fmt --check src/Modelica` が通ること、整形が冪等なこと。コミットに `Fixes #63`。

## 決めたこと

- 再現: issue の見出しの形（`,` が行頭）は、コメントが `,` の前にあるとき（`a // c` の次の行に `, b`）に出た。S226 の記述（`Icon(…), // a` の後ろ）は、コメントが次の引数の頭に付くので、並べ替えが外れる（`Documentation, // a` / `Icon, // b` / `experiment // c` が元の順のまま、`// a` が別の行で字下げが崩れる）。どちらも原因は同じで、`layout::seq` が `,` で字句を切るので、`,` の後ろのコメントは次の引数のものになり、`,` の前のコメントは引数の末尾に残って `,` がその後ろに描かれる。
- 直し: `layout::Arg` に `trail: Vec<Token>` を足し、`seq` で (1) 区切りの直後の同じ行（`newlines_before == 0`）の行コメント、(2) 引数の末尾の行コメントを入れる。描画は 引数 → `,` → コメント、次の引数は改行、`flat_into` は `trail` があれば断る、`flatten` は `,` の後ろに `trail`。`annotation.rs` は `Arg` の作り直し 3 か所に `trail: Vec::new()` を足しただけで、並べ替えは `Arg` ごと動かすので変えない。
- annotation だけに限らない（原因が `layout` の木の作り方）。`src/Modelica` に `, //` は 0 件で、MSL 4.1.0 でも 1 ファイル。
- コメントだけの行にあるコメント（次の引数の前の行）と引数と同じ行のブロックコメントは変えない（今までどおり次の引数の頭に付く）。

## やったこと

- `src/Rust/mod/globe-mode-fmt/src/layout.rs`: `Arg.trail`、`take_trail`、`seq` の 2 か所、`flatten`・`flat_into`・`expand`。
- `src/Rust/mod/globe-mode-fmt/src/annotation.rs`: `Arg` の 3 つのリテラルに `trail: Vec::new()`。
- `src/Rust/mod/globe-mode-fmt/tests/format.rs`: `trailing_line_comments_travel_with_annotation_args`（`,` の後ろのコメントと前のコメントの 2 通りが同じ出力になり、2 回目で変わらない）。
- `doc/format-style.md`: 「触らないもの」の行に、引数の行コメントの扱い。
- `doc/sessions/S343-fmt-annotation-comment/`、`doc/sessions/README.md`、この記録。

## 検証

- `cargo test -p globe-mode-fmt`: lib 34 本、tests/format.rs 23 本（新しい試験を含む）が OK。`cargo clippy -p globe-mode-fmt --all-targets -- -D warnings` 0 件、`cargo fmt --check -p globe-mode-fmt` OK。ビルドは `-p globe-mode-cli` まで（debug。release・msl 系は回していない）。
- 手元の再現（`globe fmt -`）: 3 つの入力（`,` の後ろのコメント、修飾子と関数の引数のコメント）とも、直した後に 2 回目の整形で変わらない。
- `globe fmt --check src/Modelica`: 0 ファイルが整形が要る、75 ファイルが整形済み。
- MSL 4.1.0 の `data/msl`（2,556 ファイル）に `globe fmt --check`: 「整形でトークンが変わった」0 件（MSL は元から整形前なので全部「整形が要る」）。
- `mise run check` 全体（deny・uv）、`docs:check`、`msl:*` は回していない（並行の WP で RAM が厳しい。fmt の crate に絞った）。

## 残したこと

- コメントだけの行にあるコメントが付いた引数は、`position_in` が先頭のコメントを見るので「その他」に回る（直すなら先頭のコメントを飛ばす 1 行）。
- 引数と同じ行のブロックコメントは対象外。
