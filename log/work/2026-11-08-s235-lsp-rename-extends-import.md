# S235 `lsp-rename-extends-import`

| | |
| --- | --- |
| 日付 | 2026-11-08 |
| ブランチ | `feature/s235-lsp-rename-extends-import` |
| worktree | `.git-worktree/s235-lsp-rename-extends-import` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S235-lsp-rename-extends-import/` |
| セッション記録 | （親が写す） |

## 依頼

S230 の持ち越し: 継承した部品と `import` の別名を、LSP の参照とリネームに届かせる。`redeclare` の先は対象外。

## 決めたこと

- 部品は `Resolver::definition` が返す宣言の場所で同定する（`component_hits`。`class_hits` の部品版）。カーソルが継承先でも定義元に寄る。
- ディスクの探索は、`A` を継承するクラスの名前の組を不動点まで広げ、`x` と組の名前を含むファイルだけ調べる。鎖の途中のファイルは `scratch` にアウトラインを読む。
- `Resolver::locate` は、木にない名前を `scratch` の読み済みファイルからも探す。開いていない `.mo` の `import`・`extends` が引けなかった原因（自分自身を `locate` できない）をこれで直した。
- `import` の別名は `alias_at`（宣言したクラスの中の字面）。`class_of` は別名のトークンをクラスとして扱わない。
- 定義元がライブラリの部品はリネームを断る（参照は返す）。
- S230 の「解決できなかったので断る」守りは仕組みを残す。S230 の試験の `D = Elsewhere.Pkg` は実在せず `D.Base` は別のクラスなので、期待は断るのまま（コメントを直した）。解決できる場合は新しい 2 本の試験で `changes` に入ることを確かめた。

## 検証

- MSL 4.1.0 `SISO` の `u`（release の `globe lsp -L data/msl`）: 初回 4.3 s、2 回目 0.75 s、321 箇所・92 ファイル。
- `cargo test -p globe-mode-lsp`: 通った（lib 48、cross_file 5、folding 1）。`mise run check`: fmt・clippy（-D warnings）・`cargo test --workspace`・deny・`globe fmt --check src/Modelica`・`uv sync --locked` すべて通った。
- `check_docs.py`: 切れ 0 件。`render_slides.py`（自分のデッキ）問題 0 件。
- `ci.yml` run 42476548450 success（feature ブランチ）、`vscode.yml` run 42476551094 success（3 OS）。

## 残したこと

- `redeclare`・`replaceable` の先。継承先の同名の宣言との衝突。短いクラス定義の鎖。定義元が開いていない根の下の `.mo` でカーソルが継承先のとき。UI の `globe/*`。
