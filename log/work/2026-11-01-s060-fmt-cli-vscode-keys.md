# S060 `fmt-cli-vscode-keys`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s060-fmt-cli-vscode-keys` |
| worktree | `.git-worktree/s060-fmt-cli-vscode-keys` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S060-fmt-cli-vscode-keys/` |
| セッション記録 | `log/sessions/2026-10-31-ad17d3be-95c2-47b5-8ad8-6421e92ef889.md`（S053 と同じセッション） |

## 依頼

- 「`doc/web-apps.md` に起動コマンド一覧を追加」。直前にユーザーは Rust の 4 アプリを確かめて合格とし、Phoenix は `mix deps.get` を飛ばして止まっていた。
- 「VSCode は Ctrl+Shift+I でエージェントウィンドウが開いてしまう」「拡張はどこにある？」「Shift + alt + F でフォーマット」「設定で保存時にフォーマット実行(default=false)」。スクリーンショットでは `.mo` の言語が Lua だった。
- 「CLI はディレクトリ指定で走査（astral-ruff のような操作性）」。

## 決めたこと

- ユーザーが決めたこと: Shift+Alt+F で整形、保存時の整形は設定で既定オフ、CLI は ruff のように。
- 調べて分かったこと: VS Code の窓はプロファイル「20250606_4900」で開いていた。`code --install-extension` は既定のプロファイルに入れるため、その窓に拡張が無かった（サーバ側のプロファイルの `extensions.json` に `globe-mode` が無い）。Lua は言語の自動推定。VS Code Server の CLI の `--help` にプロファイルの指定は無いので、README は画面の「VSIX からのインストール」に変えた。
- 既定のまま進めたこと:
  - パスを省いたとき、端末ならカレントディレクトリ、パイプ・`--json`・`-` なら標準入力。ウェブアプリ 10 種はどれも `--json` とパイプで呼ぶので、そのまま動く。
  - 走査は crate `ignore`（ruff と同じ。MIT OR Unlicense）。`require_git(false)` で git の外でも `.gitignore` を守る。明示したファイルは除外しない。
  - 壊れたファイルは続けて、最後に要約と終了コード 2（前は 1 本で止まって終了コード 1）。
  - 出力の文言は CLI のほかに合わせて日本語。
  - キーは拡張の `keybindings` で Shift+Alt+F を `.mo` の `editor.action.formatDocument` に割り当てる（Linux でも同じキーになる）。
  - 保存時の整形は拡張の設定 `globeMode.formatOnSave`。`onWillSaveTextDocument` で `vscode.executeFormatDocumentProvider` を待つ。設定は保存のたびに読む。
  - 拡張の版は 0.0.2 に上げた（入れ直したときに更新として扱われる）。
- 入れなかったもの: `--diff`、`--exclude`、設定ファイル。

## やったこと

- `globe-mode-cli`: `src/main.rs`（`paths_or_stdin`、`collect_mo`、`fmt` と `lint` の要約と継続）、`Cargo.toml`（`ignore`）、`tests/fmt_dirs.rs`（3 本）。`Cargo.lock` に `ignore`・`globset`。
- `src/VSCode/globe-mode`: `package.json`（0.0.2、`keybindings`、`globeMode.formatOnSave`）、`extension.js`（保存時の整形）、`test/save.test.mjs`、`README.md`（キー、保存時の整形、プロファイルの窓への入れ方、入ったかの確かめ方）。
- 文書: `doc/web-apps.md`（5 章 起動コマンドの一覧）、`doc/format-style.md`（使い方と VS Code のキー）、`doc/licenses.md`（ignore・globset）、`doc/roadmap.md`（S060）、セッション文書と索引。
- loggerhead（ユーザーの作業ツリー）: 追跡先が無く `git pull` が効かなかったので、手元の main から早送りした（`git merge --ff-only main`）。根で `pnpm install` が作った空の `package.json` と `pnpm-lock.yaml` は消した。

## 検証

- `cargo test -p globe-mode-cli --test fmt_dirs`: 3/3。1 回目は lint の期待の桁を 1 と書いて落ちた（実際は 7。`model A` のクラス名の位置）。
- CI（run 41259596381）は rust-windows だけ落ちた。`fmt_dirs` の期待値が `./a.mo` と `/` を決め打ちしていて、Windows では `.\a.mo` になるため（振る舞いは正しい）。期待値を `Path::new(".").join(..)` で作るように直した。
- 拡張の `pnpm test`: 2/2（`globe lsp` との往復、保存時の整形）。
- `cargo clippy -p globe-mode-cli --all-targets -- -D warnings`: 通過。`cargo deny check licenses`: 通過。
- `src/Modelica` で `globe fmt --check`: 「69 ファイルは整形が要る、0 ファイルは整形済み」。
- `mise run check`（fmt / clippy / test / deny / uv）: 通過。
- `render_slides.py`: S060 のデッキ 1 件 / 問題 0 件（1 回目と 2 回目はスクリーンショットで時間切れ。残っていた `headless_shell` を止めたら通った）。
- `check_docs.py`: S060 のデッキ OK、文書 262 件 / リンク 1313 件中の切れ 0 件。
- セッション記録の `--check`: 取りこぼし 0 件。
- ステージした差分の漏洩検査: 0 件。

## 残したこと

- VS Code の窓での確認はしていない（ユーザーが行う）。
- `--diff`・`--exclude`・設定ファイル。
- `src/Modelica` の 69 本の書き換え。
