# S258 `python-new-apis`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s258-python-new-apis` |
| worktree | `.git-worktree/s258-python-new-apis` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S258-python-new-apis/` |
| セッション記録 | なし（並行のサブエージェントの担当で、このセッションだけの記録が `log/local/` に無い） |

## 依頼

S249 の `run_cosim`・S253 の `Ssp`・S254 の `linearize`・S256 の `blt` を Python から使えるようにする。既存の `run_fmu` などの流儀に合わせ、新しい依存は足さない。

## 決めたこと

- `cosim(spec, *, stop_time, step, on_progress)`: dict・`.json`・`.ssp`。戻りは `run_fmu` と同じ。`run_fmu` の GIL を離す・進捗・Ctrl-C の部分を `run_observed` に取り出して共有。
- `Workspace.linearize`: 戻りは CLI の `--json` と同じキー。numpy は任意（あれば ndarray、無ければ list）。誤りは `GlobeModeError`。
- `blt` は S256 が main に入っていなかったので入れない（解説書の次回）。

## やったこと

- `src/Rust/mod/globe-mode-py/src/lib.rs`: `cosim`、`Workspace.linearize`、`run_observed`。
- `src/Python/mod/globe_mode/{__init__.py,_native.pyi}`: `cosim`・`linearize`、docstring、例外の表。
- `src/Python/tests/test_fmu.py`（cosim 1 本）・`test_api.py`（linearize 1 本）。
- `doc/features.md`・`doc/roadmap.md`・`README.md`・セッション文書・索引。

## 検証

- `uv run pytest -q src/Python/tests/test_fmu.py src/Python/tests/test_api.py`: 13 本 pass（cosim と linearize は新規。Reference-FMUs が無ければ cosim は skip）。numpy の枝は `uv run --no-sync --with numpy` で shape（2,2)(2,0)(0,2)(0,0) を手で確認（lock には numpy が無く、試験は list の枝だけ）。
- コードの検査・`check_docs.py`・リンク: 締めの欄に記す。

## 残したこと

`blt`（S256 待ち）。型を細かくした `TypedDict`。
