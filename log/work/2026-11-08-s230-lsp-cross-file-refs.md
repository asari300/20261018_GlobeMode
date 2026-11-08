# S230 `lsp-cross-file-refs`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s230-lsp-cross-file-refs` |
| worktree | `.git-worktree/s230-lsp-cross-file-refs` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S230-lsp-cross-file-refs/` |
| セッション記録 | （親が写す） |

## 依頼

roadmap の「エディタの言語サービス」の先頭: 参照・リネームが開いている文書の中にしか届かない。根の下の開いていない `.mo` とライブラリ（`-L`）に広げ、複数ファイルの一括編集にする。継承した部品と import の別名は対象外。

## 決めたこと

- 走査は frontend の `collect_mo_files` を facade が再公開（1 行）。LSP の `collect_mo` は薄い包みにして、隠しフォルダを読まない。
- `Resolver::outline(path)` が開いていないファイルを `scratch` に読む。`class_of` と `definition` の 2 か所だけ差し替えた。
- 参照は `textDocument/references` だけ。リネームは `textDocument/rename` の `changes` に、自分・開いているほかの文書・根の下の開いていない `.mo`。ライブラリは数えて `window/showMessage`。
- UI の `globe/*` は変えない。応答が配列で複数ファイルを表せず、ディスクの版は UI の未保存の本文と食い違うため。このため「`globe/rename` の応答に注意を付ける」は `textDocument/rename` の通知にした。
- S198 の守り（開いていない `.mo` に同じ名前の識別子があれば断る）は、親の指示で弱い形で戻した: 解決できた参照は `changes` に入れ、識別子があるのに `class_hits` が 0 件だった根の下のファイルが 1 つでもあれば、そのファイルを挙げて `textDocument/rename` を断る。ファイル名に対応する最上位のクラスを断る守りも残す。
- 限界（shortcut）: 開いていない中間のパッケージの `import` を通る参照は引けないので断る。同名の部品だけのファイルも断る（索引が無く区別できない）。

## やったこと

- `globe-mode-lsp/src/definition.rs`: `Resolver::outline`。
- `globe-mode-lsp/src/lib.rs`: `disk_class_hits`・`text_edits`、references と rename の handler、`class_rename_blocked` からディスクの走査を除去、先頭の表。
- `globe-mode/src/lib.rs`: `collect_mo_files` の再公開。
- 試験: `globe-mode-lsp/tests/cross_file.rs`（2 ファイルのワークスペース、隠しフォルダ、同名の部品、CRLF、文字列の中の非 ASCII の後の桁、ライブラリ）、`class_rename_refuses_what_the_disk_would_contradict` を更新。VS Code の e2e に rename の一括（`fixtures/Defs.mo`・`Pairs.mo`）。
- 文書: features、roadmap、VS Code の README、CHANGELOG、セッション文書。

## 検証

- MSL 4.1.0 の `RealInput`（release の `globe lsp -L data/msl`）: 初回 2.0 s（213 本を読む）、2 回目以降 0.29 s。465 箇所、213 ファイル。目安の 1 s を初回は超える（絞り込みで減らせない）。
- `cargo test -p globe-mode-lsp`: 通った（lib 47、cross_file 1、folding 1）。
- `mise run check`: 伏せ字の試験・ci_plan の試験・fmt・clippy（-D warnings）は通った。`cargo test --workspace` は globe-mode-fmu の fmpy 3 本が落ちた（別の worktree S231 が共有 target の `tmp/` と試験の実行ファイルを同時に使っていた。FMPy の「Bad CRC-32」「ファイルが無い」と、他の worktree の「CS は断る」。LSP と無関係）。fmu を除いた `cargo test --workspace --no-fail-fast` は、cross_file の 1 回が落ちて（原因を取れず）、単独で 42 回回して全部通った。`cargo deny check` ok、`globe fmt --check src/Modelica` rc=0、`uv sync --locked` ok。
- `check_docs.py`: デッキ 全件 OK / リンク切れ 0 件。`render_slides.py` は回していない（親が回す）。
- ステージした差分の漏洩検査（ホームのパス・メール・トークン）: 0 件。
- `vscode.yml` run 42456754949: success（e2e の rename の一括を含む）。
- `ci.yml` run 42456752698（1 回目）: rust-windows が failure（cross_file の試験の URI が `file://C:\...` になっていた。試験の側を `file:///C:/...` に直した）。docs は取り消し。run 42458115328（直した後）: 全ジョブ success。

## 残したこと

- 継承した部品と import の別名のリネーム。
- 根の下の `.mo` の索引（開いていない中間のパッケージの import）。
- UI の `globe/references`・`globe/rename`。
