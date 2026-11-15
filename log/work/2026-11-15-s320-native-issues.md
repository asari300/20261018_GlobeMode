# S320 `native-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s320-native-issues` |
| worktree | `.git-worktree/s320-native-issues` |
| Phase | Track B・P8 |
| セッション文書 | `doc/sessions/S320-native-issues/` |
| セッション記録 | 省く（`log/local/` にこの WP 専用の記録が無い） |

## 依頼

3 回目のレビューが立てた issue のうち native と LSP の 5 件（#111・#113・#114・#115・#116）を 1 件ずつ監査して直す。issue は閉じない。

## 決めたこと

- #111: 上限は 64 段（MSL の入れ子は 30 段未満。debug の 2 MiB スレッドでも溢れない）。native は `GM-U0006` で断る（Rumoca の `GM-E3001` とは言葉が違う）。`inst_class` を入口（数える）と本体に分けた（戻りの経路が多いので、数えの戻し忘れを避ける）。
- #115: issue の案の `canonicalize` は、木のクラスごとに syscall になるので使わず、字句の正規化 `same_path_key`（絶対・`..` を畳む・`/`・ドライブ小文字）にした。symlink は解かない（`shortcut:` コメントに書いた）。
- #115 は `drop_lib_for` だけでなく、定義ジャンプの `read` と「開いている文書か」の判定も同じ文字列比較だったので同じ関数にした（試験を書いて初めて分かった）。
- #116: issue の案どおり `HashMap<_, Resolution>` と値渡し。RSS の再現は試験にしない。

## やったこと

- `globe-mode-flat`: `flatten.rs`（`inst_depth`・`inst_class_body`・`res_of` の呼び出し側）、`lib.rs`（`extra` の所有・`res_of` が値・`read_dirs` が lossy）。試験 `self_containing_model_is_refused_not_overflowed`。
- `globe-mode-frontend`: `native.rs`（閉包から作業中のファイルと同じ名前を除く）。試験 `tests/native_issues.rs` 2 本。
- `globe-mode-lsp`: `same_path_key` と 3 か所の置き換え、試験 2 本（unix の相対 `--library`、windows の区切り）。
- 文書: 解説書・スライド・索引・ADR-0039 の付記（S320）。

## 検証

- 修正を戻すと #113・#114 の試験は落ちる（確認済み）。#111 と #115 の試験は修正前に落ちることを実行では確かめていない（#111 は abort、#115 は定義の行が古いまま。#115 の試験は書く途中で、`..` の畳みと開いている文書の判定が足りないと分かった）。
- `cargo test`（flat 19 本、frontend `native_issues` 2 本、lsp 70 本）、`cargo clippy -D warnings`（flat・frontend・lsp、native feature、all-targets）: 通った。`cargo fmt --all`: 差分なし。
- `mise run msl:diff-frontend`（release、254 s、VmHWM 1.1 GiB）: 回帰 0 本、改善 0 本。MSL の例題は flat 279/566、名前一致 273/273、方程式数一致 254/273、DAE 279/279、釣り合いの判定一致 191/201（前回と同じ）。`doc/frontend-diff.md` は時刻だけの差なので戻した。
- `check_docs.py`・リンク: 下の通り。

## 残したこと

- LSP の `definition.rs` に残る `t.path == path` の比較（相対の `--library` で、開いている文書どうしの比較なら絶対パスで来るので影響は小さい）。
- #115 の Windows は手元で未確認（単体試験は CI の rust-windows で走る）。
- #116 の RSS の計測は試験にしていない。
