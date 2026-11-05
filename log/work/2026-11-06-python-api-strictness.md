# S181 `python-api-strictness`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s181-python-api-strictness` |
| worktree | `.git-worktree/s181-python-api-strictness` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S181-python-api-strictness/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S178 の監査の Python の穴（P2・P7・P10・P11、WP 案 C1）のうち、黙って誤った結果になるものと読めない誤りを直す。未知の設定キー・文字列のライブラリ・型の誤りの文面・`RuntimeError` の JSON 本文・docstring と型・古い文面。例外の階層と「未知のモデルを例外に」は判断待ちなので、既存の例外の型と診断を返す設計は変えない。

## 決めたこと

- 文字列・`PathLike` の単体は 1 つのライブラリとして読む（`TypeError` にしない）。README と試験の `[...]` の書き方は壊さず、最初の 10 分の誤りを増やさないため。
- 未知の設定のキーは `TypeError`、型の誤りは前と同じ `ValueError`（設定名を言う）。`SimSettings` に `deny_unknown_fields` は付けない（wasm・C ABI・NIF・CLI も読む型）。キーと型の一覧は Rust の見本 JSON（`settings_sample`）から取り、項目を足すとコンパイルが落ちる。
- `export_fmu` の options は明示のキーワード引数にした（未知は Python の `TypeError`）。
- 型は `py.typed` と `_native.pyi`、inline の注釈。結果の dict の TypedDict は後回し（`Progress`・`Settings` だけ）。maturin は python-source の全ファイルを wheel に入れるので `pyproject.toml` は変えない。
- 例外の階層・未知のモデルの扱い・値の型の誤りを `TypeError` にするかは、材料を解説書 2.5 に書いて判断待ち。

## やったこと

- `src/Python/mod/globe_mode/__init__.py`（検査・docstring・型・`main`）、`_native.pyi`・`py.typed`（新規）、`src/Python/README.md`、`src/Python/tests/test_strict.py`（6 本、README の例を含む）。
- `globe-mode-py/src/lib.rs` に `settings_sample`、facade `globe-mode` は `SimSettings` の再公開 1 語。
- 文書: 解説書・スライド、索引、CHANGELOG（未知のキーが誤りになる点）、roadmap、features、lang-api-matrix。

## 検証

- 再現（修正前）: `Workspace(str)` が 1 文字ずつ（`ライブラリ / を読み込めない`）、`stoptime=3` が `ok: True`、型の誤りがどの設定か不明、`RuntimeError` が JSON、を worktree の `.venv` で確かめた。
- `pytest src/Python/tests`: 14 passed、1 skipped（widget の extra が無い分）。`mypy`（既定）0 件。README の例を `data/msl` の ChuaCircuit で実行: `True`、`(3, 45)`。
- `on_progress`: Decay で bdf は段 5 回で `time` が全部 `None`、esdirk34 は積分の t が付く。lowering で `True` を返すと build の後で止まり `ok: False`。
- コードの検査（`mise run check`）: fmt・clippy（-D warnings）・test（64 個の結果が全部 ok）・deny（advisories・bans・licenses・sources ok）・`uv sync --locked` が通った
- `check_docs.py`: デッキ OK / リンク 3685 件中の切れ 0 件。`render_slides.py`: S181 のデッキ 6 枚 × 3 構成（desktop-light・dark・mobile）、問題 0 件
- dry run（`release.yml`、`--ref feature/s181-python-api-strictness`）: 1 回目 42057547958 は Windows の wheel の pytest が 1 本落ちた（README の例の試験が Windows のパスの `\` をコードの文字列に混ぜた。`repr` に直した。ほか 15 本は通った）。2 回目 42059942761 は全ジョブ success（3 OS の「wheel を入れて pytest」が通った）。Windows の wheel に `py.typed`・`_native.pyi` が入っていることを確かめた。Release 0 件・タグ 0 件のまま

## 残したこと

- C2（`outline`・`flat`・`deps`・`check` の要約）、結果の TypedDict、例外の階層（判断待ち）。
- `render_slides.py` は cargo が止まってから回す。
