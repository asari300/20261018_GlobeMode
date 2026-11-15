# S346 `fmt-leading-comment`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s346-fmt-leading-comment` |
| worktree | `.git-worktree/s346-fmt-leading-comment` |
| Phase | 整形（`globe-mode-fmt`） |
| セッション文書 | `doc/sessions/S346-fmt-leading-comment/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S343 の残り。コメントだけの行にあるコメント（次の引数の前の行の `// …`）は次の引数の頭に付き、その引数は `annotation::position_in` が名前を引けず「その他」（後ろ）に回って並べ替えられない。先頭にコメント行を持つ annotation の引数も、名前で並べ替えられ、コメントが引数と一緒に動くようにする。再現を確かめ、最小の修正と回帰試験 1 本（冪等も）。`globe fmt --check src/Modelica` と、MSL への `globe fmt --check` で「トークンが変わった」0 件。

## 決めたこと

- 再現: `Documentation` / `Icon` / `experiment` の各引数の前の行にコメントを置いた `annotation(…)` は、直す前は元の順のまま（3 つとも「その他」）。
- 直し: `annotation.rs` に `body`（引数の中身から頭のコメントを除いた節）を足し、`position_in` と `arg_name` がそこから名前を引く。並べ替えは `Arg` ごと動かすので、名前が引ければコメントは一緒に動く。`layout.rs` は変えない。
- `arg_name`（図形プリミティブ）も同じ原因（先頭の字句を見る）なので同じ `body` を使う。直す前は先頭にコメントがあると「名前のない引数」とみなして、図形全体の並べ替えと `origin` の補完をやめていた。依頼の文言は annotation の引数だが、1 行ずつの違いで同じ入力の形なので一緒に直した。
- lint の `annotation-order` は `position_in` を使うので、直す前は通知しなかった入力（`// a` 付きの `Documentation` の後ろに `experiment`）も通知する（コードの上ではそう。直す前の `globe lint` は回していない。直した後は `globe lint` が `annotation-order` を出すのを確かめた）。
- 見送り: コメントの次のトークンが 1 段深く字下げされる（`layout::push_token` の規則。括弧の引数すべての出力が変わる）、`Documentation` の `info = "…"` の HTML 整形（`mark_html` は `[名前, =, 文字列]` の 3 字句で見るので頭にコメントがあると外れる。並べ替えが外れていたころと同じで退行ではない）。

## やったこと

- `src/Rust/mod/globe-mode-fmt/src/annotation.rs`: `body` を足し、`position_in`・`arg_name` が使う。
- `src/Rust/mod/globe-mode-fmt/tests/format.rs`: `leading_line_comments_travel_with_annotation_args`。
- `doc/format-style.md`・`CHANGELOG.md`。
- `doc/sessions/S346-fmt-leading-comment/`、`doc/sessions/README.md`、この記録。

## 検証

- `cargo test -p globe-mode-fmt`: lib 34 本、tests/format.rs 24 本（新しい試験を含む）が OK。`cargo clippy -p globe-mode-fmt --all-targets -- -D warnings` 0 件、`cargo fmt --check -p globe-mode-fmt` OK。ビルドは `-p globe-mode-cli` まで（debug。release・msl 系は回していない）。
- 手元の再現（`globe fmt -`）: 2 回目の整形で変わらない（冪等）。修飾子の引数（`Real x(// c start = 1, …)`）は並べ替え対象でないので順のまま。
- `globe fmt --check src/Modelica`: 0 ファイルが整形が要る、75 ファイルが整形済み。
- MSL 4.1.0 の `data/msl`（2,556 ファイル）に `globe fmt --check`（debug、約 8 s）: 「整形でトークンが変わった」0 件（MSL は元から整形前なので全部「整形が要る」）。
- `mise run check` 全体（deny・uv）、`docs:check`、`msl:*` は回していない（並行の WP で RAM が厳しい。fmt の crate に絞った）。

## 残したこと

- コメントの次のトークンの字下げ、`Documentation` の `info` の HTML 整形（頭にコメントがあるとき）。
- 引数と同じ行のブロックコメントは対象外。
