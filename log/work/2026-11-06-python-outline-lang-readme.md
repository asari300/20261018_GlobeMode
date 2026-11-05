# S183 `python-outline-lang-readme`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s183-python-outline-lang-readme` |
| worktree | `.git-worktree/s183-python-outline-lang-readme` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S183-python-outline-lang-readme/` |
| セッション記録 | （親が写す） |

## 依頼

S178 の監査の穴 10 の残り（WP の C2 と D）。Python に CLI と同じ読み取り（`outline`・`flat`・`deps`）、`check` の 1 行の要約（facade・protocol に既にあれば通す）、S180 の診断が Python の `check` に出る試験、C・Zig・Julia・Elixir・Gleam の README に書いてあるとおりに動く最初の例、`__marimo__/` を `.gitignore`、marimo にモデル名の入力欄。

## 決めたこと

- 口は facade の既存の `Workspace::outline` / `deps` / `flat` を `globe-mode-py` から呼ぶだけ（facade は変えていない。計算は Python に書かない）。戻り値は `outline` が dict、`deps` が `list[str]`、`flat` が dict か `None`。`outline` のファイルが無い・構文の誤りは `ValueError`（`format` と同じ）。
- `check` の 1 行の要約は足さない。CLI の最後の行は `main.rs` の中だけにあり、facade・protocol に文字列が無い（`unsupported_summary` は GM-U の要約で別物）。持ち越し。
- C ABI・NIF・wasm には広げない。S110 の対応表に「Python だけ」の行を足した。
- C の README は新しく作った（`globe-mode-c` に説明の置き場が無かった）。
- 親の指示は「S181 が main に入ってから worktree」だったが、約 40 分待っても入らなかったので、先に main から worktree を切って S181 に触らない D の部分（README・marimo・.gitignore）を進めた。Python の層に入るときは、S181 の branch の Python 関係のファイル（`src/Python/`・`globe-mode-py/src/lib.rs`・`globe-mode/src/lib.rs`・対応表）を `git checkout feature/s181-python-api-strictness -- <files>` と `git apply` で取り込んで、その上に足した（merge はしていない。内容が同じなので、親が S181 を main に入れたあとの main の取り込みは衝突しない見込み。`doc/sessions/README.md`・`doc/features.md`・`CHANGELOG.md`・`doc/roadmap.md` は S180 を含む main のもので、S181 の行は入っていないので、そこは親が衝突を解く）。

## やったこと

- `src/Rust/mod/globe-mode-py/src/lib.rs`: `Workspace.outline`・`deps`・`flat`。
- `src/Python/mod/globe_mode/__init__.py`・`_native.pyi`・`src/Python/README.md`: docstring・型・README の 1 行。
- `src/Python/tests/test_read.py`: outline・deps・flat・GM-E1002 の 5 本（ファイルは `encoding="utf-8"`、パスに `\` を混ぜない）。
- C・Zig・Julia・Elixir・Gleam の README に最初の例。Julia の README を実物に合わせた。
- `.gitignore` に `__marimo__/`、`src/Python/marimo/sim_app.py` にライブラリとモデル名の入力欄（`mo.ui.text` 2 つ。実行できなければ `mo.stop` で理由）、marimo の README。
- `doc/research/lang-api-matrix-2026-11-03.md` に outline・deps・flat の行。CHANGELOG・features・roadmap・索引。

## 検証

- README の例を取り出して実行: C（`passed: true`、x(1) = 0.3678794681）、Zig（x(1)=0.367879468116207）、Julia（`true`・0.367879468116207）、Elixir（0.367879468116207）、Gleam（x(1) = 0.367879468116207）。
- `mise run c:zig`・`c:julia`・`elixir:test`（2 tests, 0 failures）・`gleam:test`（2 passed）: 通った。
- `uv run pytest src/Python/tests`: 19 passed, 1 skipped（`test_read.py` の 5 本を含む）。
- `marimo check`・`python sim_app.py`・`marimo export html`: エラー 0。
- `mise run check`: 通った（fmt・clippy・test 678 passed 0 failed・deny・`uv sync --locked`）。load は 0.5 から始めた
- `check_docs.py`: デッキ OK / リンク切れ 0 件（文書 739 件・相対リンク 3736 件）。`render_slides.py`（S183 のデッキだけ）: 問題 0 件
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件
- release.yml の dry run（`gh workflow run release.yml --ref feature/s183-python-outline-lang-readme`、run 42074315197）: 全 10 ジョブ success。「wheel を入れて pytest」は linux x86_64・linux aarch64・macOS arm64・windows x86_64 の 4 つとも 21 passed。Release 0 件・タグ 0 件のまま

## 残したこと

- C ABI・NIF・wasm に `outline`・`deps`・`flat` を広げる。
- `check` の人が読む 1 行の要約を facade か protocol に置く（今は CLI の中だけ）。
- 結果の dict の `TypedDict` 化、例外の階層（ユーザーの判断）。
- `sim_app.py` の入力欄の見え方をブラウザで開いては見ていない（`marimo check` と書き出しまで）。
