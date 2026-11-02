# S071 `ui-w1001-tolerant-wasm`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s071-ui-w1001-tolerant-wasm` |
| worktree | `.git-worktree/s071-ui-w1001-tolerant-wasm` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S071-ui-w1001-tolerant-wasm/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S066 の持ち越し 2 つ（親のエージェントから）。`GM-W1001` が UI の診断の一覧に出るかを確かめ、出ていなければ出す。wasm の分割読み込み `with_library_sources` / `with_library_sources_split` を、ディスクの道と同じ寛容さ（読めるファイルだけ、読み飛ばしは `GM-W1001`）にする。ponytail（最小の完全な変更）で。main へのマージと push は親が行う。

## 決めたこと

- 既定のまま進めた: `read_tolerant` の中身を関数に切り出し（`syntax_reason`・`skipped_warning`・`to_skipped`）、in-memory の `parse_sources_lenient` と共有した。新しい型は作らない。`parse_files_parallel_lenient` はパスを読む関数なので in-memory には使えない。
- 分割読みは読むときに読み飛ばす（作るときに全部の構文を確かめない）。分割の目的を保つため。代わりに `skipped_library_files()` には出ない（`shortcut:` のコメント）。
- UI は元から `file`・`range` の無い診断の行を出していた。読み飛ばしたファイル名が `notes` にしか無いので、表に `notes` を出した。
- `Workspace::with_libraries`（ネイティブの CLI・Desktop の既定の道。S054 から分割読み）も寛容になった。今までは構文の誤りのファイルを使うモデルの検査が `GM-E1003` で落ちていた。

## やったこと

- `src/Rust/mod/globe-mode-frontend/src/rumoca.rs`: 上の共有。`with_library_sources` は読めないファイルを読み飛ばして警告。`library_session` は `(Session, 読み飛ばし)` を返し失敗しない。`SplitState::warning` を足し、`check_synced` で付ける。`ensure_library` は失敗しなくなったので `Result` を外した。
- `src/Rust/mod/globe-mode-frontend/tests/split.rs`: 構文の誤りのファイル `Bad.mo` を使うモデルが `GM-E1003` ではなく、失敗と `GM-W1001`（notes に `Bad.mo: …`）になることに直した。
- `src/Rust/mod/globe-mode/tests/library_closure.rs`: `tests/data/tolerant/a` を in-memory（分割・全部）とディレクトリの分割で読む試験 `broken_library_file_is_skipped_in_memory` を足した。
- `src/Svelte/src/App.svelte`: 診断の表に `notes`。`src/Svelte/tests/test_ui.py`: `MSL.json` を fixture に差し替えて `GM-W1001` の行を確かめる。

## 検証

- `cargo test -p globe-mode --test library_closure`（3 本、新しい `broken_library_file_is_skipped_in_memory` を含む）、`-p globe-mode-frontend --test split`（2 本）・`--test tolerant_roots`（4 本）が通る。
- `mise run check`（fmt・clippy `-D warnings`・test・deny・`uv sync --locked`）: EXIT 0。test の集計 597 通過・0 失敗・11 ignored、deny は advisories・bans・licenses・sources とも ok。
- `mise run wasm:node`: EXIT 0（MSL 2556 ファイル、in-memory のライブラリ 169.2 ms、ChuaCircuit bdf 2330 ms・113 MiB）。
- `mise run wasm:compare`: 367 / 380 件が一致、既知の差 13 件、一致しない 0 件。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。
- `mise run ui:check`（svelte-check 0 errors・0 warnings、`test_ui.py`、`test_graph.py`）: EXIT 0。新しい確認は `GM-W1001 S1 warning ライブラリ MSL の 1 ファイルを読めず、読み飛ばした` の行と notes の `A/Broken.mo: unexpected `x``。
- `check_docs.py`: スライド OK、リンク 1550 件中の切れ 0 件。`render_slides.py` は手元で `Page.screenshot` が 30 s で時間切れ（いつもの手元の事情）。CI に任せる。
- 最初の `wasm:node`・`ui:check` は、main のチェックアウトの `data/msl/msl`（09:43 に別のセッションが張った、行き先の無い symlink）で ENOENT になった。その symlink を消して回し直した。

## 残したこと

- 分割読みで、まだ読んでいない構文の誤りのファイルは知らせない。
- Desktop の UI では確かめていない（同じ表なので同じに出るはず、【未確認】）。
