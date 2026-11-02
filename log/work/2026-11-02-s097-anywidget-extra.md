# S097 `anywidget-extra`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s097-anywidget-extra` |
| worktree | `.git-worktree/s097-anywidget-extra` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S097-anywidget-extra/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S078 で `anywidget` を `globe_mode` の通常の依存にした（ci.yml の `python` と release.yml の wheel の pytest が extra を入れないため）。代償で wheel を入れる人に ipython など 18 本が付く。ユーザーの判断（2026-11-02）で extra に戻す。Rust のコードは変えない。

## 決めたこと

- ユーザーと決めた: anywidget を extra `widget`（`anywidget>=0.11.0`）にする。
- 既定のまま進めた:
  - ci.yml の `python` は `--extra widget`（`--all-extras` と同じ大きさで、入れるものが読める）。
  - release.yml は `--with "globe-mode[widget] @ $(ls wheel/*.whl)"`。`--with anywidget` を足すより wheel の METADATA の extra まで確かめられる。uv が相対パスの direct reference と extra を受けることは、手元のダミーの wheel（hatchling、extra `x = ["six"]`）で確かめた（extra ありで `six` が入り、なしで入らない）。
  - `test_widget.py` は `pytest.importorskip("anywidget")`。`mise run check` の `uv sync --locked`（extra なし）でも通る。
  - `widget.py` は import を遅延させず、`ImportError` に `pip install "globe-mode[widget]"` を添える。
  - `.github/scripts/ci_plan.py` は変えない（`pyproject.toml`・`uv.lock` で `python` が起きる）。

## やったこと

- `pyproject.toml`・`uv.lock`: anywidget を extra `widget` に。
- `.github/workflows/ci.yml`（`python` だけ）・`.github/workflows/release.yml`（wheel の pytest の 1 行）。
- `src/Python/mod/globe_mode/widget.py`・`src/Python/tests/test_widget.py`。
- 文書: `doc/licenses.md`、`doc/roadmap.md`（S078 の節に 1 行）、`src/Python/marimo/README.md`、`CHANGELOG.md`、`doc/sessions/S097-anywidget-extra/`、`doc/sessions/README.md`。

## 検証

- extra なしの依存（`uv export --frozen --no-dev --no-emit-project`、ロック上の全 OS）: 49 → 30（外れたのは anywidget と推移的な 18 本）。
- 手元の pytest（`--extra widget`）: 5 passed
- 手元の pytest（extra なし）: 3 passed, 1 skipped（`test_widget.py`）。`import globe_mode.widget` は入れ方を添えた ImportError
- ci.yml（`-f full=true`）: run 41421793770 成功（`python` は 5 passed。docs の slides も成功）
- release.yml（dry-run）: run 41421802034 成功（3 つの wheel の pytest がどれも 5 passed）
- `check_docs.py`: デッキ OK、文書 396 件・相対リンク 1897 件・切れ 0 件
- `render_slides.py`: 手元は時間切れ（他の worktree の cargo と同時。Page.screenshot 30 s）。CI の slides に任せた
- コードの検査（fmt / clippy / test / deny）: Rust を変えていないので回していない。

## 残したこと

- main へのマージ・push・セッション記録の写しは親が行う。
- PyPI に出すときは README に `globe-mode[widget]` の入れ方を書く。
