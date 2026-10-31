# S039 `python-wheel`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s039-python-wheel` |
| worktree | `.git-worktree/s039-python-wheel` |
| Phase | P7（言語 API の Python。[計画書](../../doc/plan/2026-10-30-release-estimate-and-parallel-plan.md)の WP-G） |
| セッション文書 | `doc/sessions/S039-python-wheel/` |
| セッション記録 | `log/sessions/2026-10-30-0e301cfb-c9ff-440e-b48e-89148a3cfe93.md`（S033・S035・S038 と同じセッション） |

## 依頼

「進める。サブエージェントは Sonnet/high」。S038（WP-A）の次として、計画書の WP-G（`globe-mode-py` と maturin で `src/Python/mod/globe_mode` に wheel。参照シナリオ open → check → graph → simulate → 結果）を実施する。実装は Sonnet（effort high）のサブエージェント 1 人、検査と文書と締めは Fable。

## 決めたこと

- **入出力は JSON の文字列**（wasm と同じ）。`dict` への変換と結果の表（polars）は Python 側の `__init__.py`。pythonize・numpy は足さない（計画書の「numpy で」は、既に依存にある polars に読み替えた）。
- **build-backend を uv_build から maturin に**。`uv sync` が wheel を作る。maturin は mise に入れない（隔離環境で取る）。
- **CI の `python` のジョブに rust と rust-cache を足し、pytest を回す**。`ci_plan.py` は `globe-mode-py` が影響を受けたとき（facade の変更など）も `python` を真にし、`src/Modelica/*` の変更でも `globe-mode-py` を検査対象にする（試験が `src/Modelica` を読むため）。
- `#[pyclass(unsendable)]`（salsa のデータベースは Sync でない）。
- 例外の振り分け: ライブラリが読めない・`load_dir` の失敗は `RuntimeError`、settings の JSON の誤り・索引に無いモデルは `ValueError`、モデルの誤りは `ok` と `diagnostics`。
- サブエージェントの判断で受け入れたもの: `.gitignore` に `globe_mode/_native*.so`（`uv sync` が 54 MB の .so を置く）、`mise.toml` の uv の行のコメント（uv_build の記述が古くなった）。
- Fable が直したもの: `Workspace.__init__` の既定値 `[]` を `()` に（可変の既定値）。

## やったこと

- `src/Rust/mod/globe-mode-py/`（新規）: `_native.Workspace{new(libraries), set_file, load_dir, check, graph, simulate}` と `solvers()`。単体試験 1 本（JSON の往復、settings の誤り、索引に無いモデル）。
- `src/Python/mod/globe_mode/__init__.py`: `Workspace`（`dict` を返す）、`table()`（polars）、`solvers()`、`main()`（ソルバの数を表示）。`src/Python/tests/test_api.py`（2 本）。
- `pyproject.toml`（maturin、pytest、`testpaths`）、`uv.lock`、`Cargo.toml`（`globe-mode-py`、`pyo3 = "0.30"`）、`Cargo.lock`（+pyo3 の 5 crate）、`.gitignore`、`mise.toml`（コメントだけ）。
- `.github/workflows/ci.yml`（python のジョブ）、`.github/scripts/ci_plan.py`・`test_ci_plan.py`（22 本）。
- 文書: `doc/licenses.md`（pyo3・maturin・pytest の 3 行）、`doc/architecture.md` §2.2（`globe-mode-py` の行）、`doc/roadmap.md`（S039 の節）、計画書の WP-G に「済」、`doc/sessions/README.md`、`doc/sessions/S039-python-wheel/`（スライド 6 枚と解説書）。

## 検証

- `mise run check`: fmt / clippy（-D warnings）/ test 542 本通過・0 失敗・10 ignore / deny（advisories・bans・licenses・sources ok）/ uv sync --locked ok
- `cargo test -p globe-mode-py`: 1 本。`uv run pytest -q`: 2 本通過。`uv run globe-mode`: 「ソルバ 26 種」。
- wheel: `globe_mode-0.1.0-cp312-abi3-linux_x86_64.whl` 17,281,282 バイト（17.3 MB。中の `_native.abi3.so` は 54 MB）。
- `cargo deny check`: advisories / bans / licenses / sources ok。新しい crate は pyo3 0.30.1 と pyo3-ffi・pyo3-macros・pyo3-macros-backend・pyo3-build-config（MIT OR Apache-2.0）。
- `test_ci_plan.py`: 22 本 OK。
- `check_docs.py`: デッキ 44 件 OK / リンク 1035 件中の切れ 0 件（main の S040 を取り込んだ後）
- `render_slides.py`: 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件
- CI: {{CI}}

## 残したこと

- `graph` はライブラリとして読んだモデルを索引から引けない（試験は `load_dir` で回す）。facade 側の仕様を決める。
- Windows・macOS・aarch64 の wheel と配布（WP-I）。CI の `rust-windows` は `globe-mode-py` を含めて `cargo test --workspace` を回す（Python の有無で落ちる可能性があり、この push の全体 CI で確かめる）。
- 結果の Arrow での受け渡し、`export_fmu`、marimo の anywidget。
