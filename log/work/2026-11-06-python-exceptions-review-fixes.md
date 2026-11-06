# S200 `python-exceptions-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s200-python-exceptions-review-fixes` |
| worktree | `.git-worktree/s200-python-exceptions-review-fixes` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S200-python-exceptions-review-fixes/` |
| セッション記録 | （親が写す） |

## 依頼

ユーザーの決定（2026-11-06「例外を入れる」）で `GlobeModeError` の階層を入れ、Python の束縛のレビュー（Opus、main cc846ba）の指摘 1〜10 を、1 件ずつ失敗する試験で再現してから直す。範囲は `globe-mode-py`・`src/Python/mod/globe_mode`・`src/Python/tests`・`src/Python/README.md`。

## 決めたこと

- 階層は `GlobeModeError` ⊃ `LibraryError`（+`RuntimeError`）・`SettingsError`・`ModelNotFoundError`・`SourceError`・`ExportError`・`ResultError`（+`ValueError`）。`ResultError` は `table` のための 1 つ（基底で受けられるように足した）。呼び方の誤りは素の `TypeError`、コールバックの例外と panic はそのまま。理由は解説書 2.1。
- クラスは Python（`_errors.py`）、Rust の束ねが名前で引いて出す。Python 側で組み込みの型を付け替える案は、利用者のコールバックの例外まで付け替えるので採らなかった。
- 「未知のモデルを例外に」は変えない（指示どおり）。
- GIL: `simulate`・`simulate_columns` を `simulate_observed` の 1 本に。`check`・`graph`・`deps`・`flat`・`export_fmu` を `py.detach`。`outline`・`load_dir`・`set_file` は包まない（軽いか、読むだけ）。`detach` が同じスレッドで動くことを PyO3 0.30.1 のソースで確認（解説書 2.3）。
- 列の道は `PyBytes::new_with` で直接詰める。測定でピークが 33〜36% 減り時間は同じなので採った。
- `Workspace("")` は `GM-E1003` の `LibraryError` で落ちる（誤りは出る）ので直さない。
- 指摘 8 の (a)(b)(c) は今のコードで通った（直す所は無く、試験だけ足した）。

## やったこと

- `src/Rust/mod/globe-mode-py/src/lib.rs`: 例外を `error()` で Python のクラスから作る、`simulate_observed` を `on_progress` 任意の 1 本に、`simulate_json`・`simulate_columns`・`columns_of`・`graph_json` を削除、`py.detach`、列の bytes を直接詰める。
- `src/Python/mod/globe_mode/_errors.py`（新規）、`__init__.py`（`__all__`、数の検査、`on_progress` キーワード専用、`targets`、bytes、`None`、`table`）、`_native.pyi`。
- 試験: `test_errors.py`（階層）、`test_review.py`（指摘 1〜8）。
- 文書: `src/Python/README.md`、CHANGELOG、`doc/features.md`・`doc/roadmap.md`、解説書・スライド、レビュー履歴。

## 検証

- `pytest src/Python/tests`: 42 passed・1 skipped（前は 22 + 1。新しく `test_errors.py` 5 本、`test_review.py` 15 本。再現の段階では `test_review.py` が 15 本中 11 本落ち、4 本（8a・8b・8c・2 の Ctrl-C のうち当時の弱い版）は元から通った）。
- `mypy`（anywidget を入れた環境）: 0 件。
- `mise run check`: fmt・clippy・deny・`fmt --check src/Modelica`（71 ファイル整形済み）・`uv sync --locked` は通った。`cargo test --workspace`（`--no-fail-fast`）は 1 本落ちた: `globe-mode-cli` の `fmu_export::fmpy_simulates_the_built_fmu`（`No such file or directory`）。単独で回し直すと 6 本とも通った（共有 target での並行実行の干渉。自分の差分と無関係）。1 回目の `fmt_dirs::piped_stdin_still_formats_stdin` の失敗も、古いビルドの `globe` で、CLI を touch して回し直したら通った。clippy は最初に `chunks_exact_mut`（`as_chunks_mut` に直した）。
- marimo: `marimo check`（sim_app・fmt_app）0 件、`marimo export html` は sim_app（anywidget を入れて）・fmt_app とも成功。
- `check_docs.py`: リンク 794 文書・切れ 0 件、スライド OK。`render_slides.py`: デッキ 1 件 / 問題 0 件。
- 列の道の測定（release）: 10⁶ 点でピーク 145 から 121 MiB、4×10⁶ 点で 350 から 259 MiB、時間は同じ。
- run（feature を push して）: `release.yml` の dry run 42169948299 は success（4 つの環境の「wheel を入れて pytest」を含む全ジョブ）。`ci.yml -f full=true` 42169960483 は success（11 ジョブ）。

## 残したこと

- 「未知のモデルを例外に」はユーザーの判断待ちのまま。
- numpy を実際に入れた環境の確認は無い（`numbers.Real` に登録した偽物で確認）。polars の `memoryview` のコピーは減らしていない。
- 既定のソルバの Ctrl-C は積分の中に届かない（ADR-0033）。
