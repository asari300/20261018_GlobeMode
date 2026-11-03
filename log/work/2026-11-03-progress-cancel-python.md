# S129 `progress-cancel-python`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s129-progress-cancel-python` |
| worktree | `.git-worktree/s129-progress-cancel-python` |
| Phase | P7（ADR-0033 の実装の順の 3） |
| セッション文書 | `doc/sessions/S129-progress-cancel-python/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

親のエージェントから。S123 の次の段のうち Python: `globe-mode-py` に進捗のコールバック（Python の callable、`Progress` を dict）と中断（コールバックが `True` か Ctrl-C）を足す。実行中は GIL を離す。列の道にも付ける。`globe_mode.Workspace.simulate`・`simulate_columns` に `on_progress=None` で通す。試験 3 本。ADR-0033 の結果・機能表・features を更新。wasm・C・NIF・LSP、exec・facade の中身は触らない。

## 決めたこと

- native の `simulate`・`simulate_columns` に keyword 引数 `on_progress` を足す形にした（別名の `simulate_observed` を足さない）。`None` なら前の道そのまま。
- dict のキーは exec の `Progress` のまま（`stage` は `ExecStage` の Debug を小文字に）。ADR の (3) の `progress(stage, t, start, stop)` からは変えたので ADR の結果に書いた。
- GIL は `Python::detach`（pyo3 0.30.1。`allow_threads` は旧名）と `Python::attach`。`check_signals` かコールバックの例外はトークンを立てて覚え、戻ってから投げ直す。
- 最初は `unsafe impl Send` の包みを書いたが、workspace の lint が `unsafe_code` を禁じており、facade の `Workspace` が `Send` だったので外した。
- 中断の試験は、積分が 100 ms より長くかかる `der(x) = sin(100 * time)`・1000 s（esdirk34、止めずに約 0.9 s）にした。Decay は 100 ms より短く、t が終わりの 1 回しか来ない。

## やったこと

- `src/Rust/mod/globe-mode-py/src/lib.rs`: `simulate_observed`（detach・attach・check_signals）、`progress_dict`、`columns_request`・`columns_of` に分けた（列の道を両方で使う）。
- `src/Python/mod/globe_mode/__init__.py`: `on_progress=None` を通す。
- `src/Python/tests/test_api.py`: 試験 3 本。
- `doc/adr/0033-progress-and-cancel.md`（3 段目の結果）、`doc/research/lang-api-matrix-2026-11-03.md`（Python の進捗・中断、穴 3 → 1。FMU export の穴は S118 で埋まっていたのに数が残っていたので合わせた）、`doc/features.md`、`doc/roadmap.md`、セッション文書と索引。

## 検証

- `uv run pytest -q src/Python/tests`: 7 passed, 1 skipped（skip は前から）。止めない Wiggle 0.87 s、`on_progress` 付き 0.90 s。(2) は約 0.1 s、(3) は約 0.2 s で止まる。
- `cargo test -p globe-mode-py`: 1 passed。`cargo clippy -p globe-mode-py --all-targets -- -D warnings`: 警告 0。
- `mise run check`（`CARGO_BUILD_JOBS=2`、load average 10〜33）: `test_ci_plan.py`・fmt・clippy 通る。`cargo test --workspace` は全 47 の試験の組が ok、最後の `globe-mode` の doctest だけが共有 target の別 worktree のビルドと競合して E0460 / rlib が無いで落ちた。`cargo test --workspace --doc --no-fail-fast` を回し直して 17 crate とも ok。`cargo deny check` ok、`uv sync --locked` ok
- `check_docs.py`: デッキ OK / 相対リンク 2,685 件中の切れ 0 件。`render_slides.py`: 6 枚 × 3 表示で問題 0 件（S04 の箱の文字が端に付いていたので広げた）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査（メール・ホームのパス・トークン）: 0 件

## 残したこと

- ADR-0033 の順の 2・4・5（wasm・Svelte、C・NIF、LSP・Desktop）。
- ネイティブの型付きの道の積分の中、lowering・組み立ての中では止まらない（S123 の持ち越しと同じ）。`on_progress` なしでは前のまま GIL を持つ。
