# S209 `fmt-stdin-dash`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s209-fmt-stdin-dash` |
| worktree | `.git-worktree/s209-fmt-stdin-dash` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S209-fmt-stdin-dash/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

`globe fmt` / `globe lint` は `--json` の有無に関係なく、パスか `-` を必須にする（ユーザーの決定 2026-11-07「`-` を必須にすべて揃える」）。呼び出し元を洗って直し、各アプリの試験を回す。

## 決めたこと

- `paths_or_stdin` の `[] if json` の腕を消し、`json` の引数も外した。使い方の文面は「標準入力。--json でも - が要る」を足しただけ。
- Rust のウェブアプリ 4 種は `globe-mode-fmt` を直接呼ぶので直さない（S199 の表の「10 種」は CLI の子プロセス 6 種 + crate 4 種）。
- marimo の `test_format_inserts_origin` は S062 から落ちていた（例題が origin を持つので `not in` が成り立たない）。origin の無い入力で確かめ直した。

## やったこと

CLI（`main.rs`、`tests/review_fixes.rs`・`fmt_dirs.rs`）、6 アプリの呼び出し（`--json -`）とその README・docstring、`doc/web-apps.md`・`format-style.md`・`architecture.md`・README・roadmap・CHANGELOG・索引、スライドと解説書。

## 検証

- `cargo test -p globe-mode-cli`: 全通過。`stdin_needs_an_explicit_dash` は直す前に `fmt --json` が 0 で落ちた。
- 新 CLI で直す前の FastAPI: 3 本落ち（HTTP 500）。直して 6 本通る。ほかのアプリの試験は解説書の表。
- `python3 .github/scripts/test_ci_plan.py`: 25 本 OK。
- `mise run check`、`check_docs.py`、CI の run: 下の「仕上げ」。

## 残したこと

ウェブアプリの CI（S203 から既出）。
