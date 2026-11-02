# S074 `wheel-release`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s074-wheel-release` |
| worktree | `.git-worktree/s074-wheel-release` |
| Phase | P6 / P8 |
| セッション文書 | `doc/sessions/S074-wheel-release/` |
| セッション記録 | （親が写す） |

## 依頼

`release.yml` に wheel のジョブを足す。`globe` と同じ 3 ターゲットで `uv build --wheel`、ネイティブで動くものは `uv run pytest`、成果物を Release の assets と SHA256SUMS に足し、dry-run でも artifact に残す。wheel のライセンス表は `globe` 用の表を流用し、流用できる根拠を書く。

## 決めたこと

- 新しいジョブを作らず、`build` の matrix に step を足した（Rust・cache を共有）。aarch64 は既存の行と同じネイティブの `ubuntu-24.04-arm` ランナーで作った（cross・zig は使っていない）。ネイティブなので pytest は 3 ターゲットすべてで回した（依頼は 2 つ）。
- **ライセンス表は流用しなかった。** 依頼は「同じ依存なので流用」だったが、`cargo tree --locked -p <crate> -e normal,build --target <3 ターゲット>` で比べると、`globe-mode-py` の依存（322）は `globe-mode-cli`（326）の部分集合ではなく、**pyo3 0.30.1 の 5 crate**（pyo3・pyo3-build-config・pyo3-ffi・pyo3-macros・pyo3-macros-backend）が cli に無い。これらは MIT OR Apache-2.0 なので許可の範囲だが、MIT の著作権表示を載せる必要がある。そこで新しい設定は作らず、`third-party` ジョブで同じ `about.toml`・`about.hbs`・`--fail` を `globe-mode-py` の manifest にも回し（1 行）、`THIRD-PARTY-wheel.md`（336,053 バイト）を wheel の `globe_mode/THIRD-PARTY.md` に入れた。
- uv は Linux では mise-action の `install_args` に足し、Windows では `python -m pip install uv`（mise の shim を避ける既存の流儀。setup-uv は足していない）。
- pytest は `uv run --isolated --no-project --with wheel/*.whl --with pytest` で、作業ツリーではなく作った wheel を読む。
- platform タグは maturin の既定（`linux_x86_64`・`linux_aarch64`・`win_amd64`）のまま。manylinux にはしていない。

## やったこと

- `.github/workflows/release.yml`: `third-party` で wheel 用の表、`build` に「wheel を作る」「wheel を入れて pytest」と artifact `globe-wheel-<target>`、`release` の `SHA256SUMS` に `globe_mode-*.whl`。
- 文書: `doc/sessions/S074-wheel-release/`、索引、roadmap（S039・S052 の持ち越し）、CHANGELOG。

## 検証

- dry-run: run 41365579800（`gh workflow run release.yml --ref feature/s074-wheel-release`、入力なし）、約 20 分、5 ジョブとも success。
- wheel: `linux_x86_64` 17,878,716 バイト、`linux_aarch64` 16,706,158、`win_amd64` 16,033,654（artifact の zip は 17,830,930・16,656,268・16,013,482）。
- wheel を入れた pytest: 3 ターゲットとも `2 passed`。
- `SHA256SUMS`: 6 行（globe 3、wheel 3）。
- `python3 .github/scripts/test_ci_plan.py`: 23 件 OK。`mise run ci:plan`: `full: true`（理由「依存・ツール・CI の定義を変更: .github/workflows/release.yml」、7 ファイル）。main への push は全体の検査になる。
- Rust は変えていないので `mise run check` は回していない。
- `check_docs.py`: スライド OK、文書 319 件・相対リンク 1,552 件・切れ 0 件。`render_slides.py` は回していない（手元の `uv sync` が他の worktree の cargo と同時に重く、.venv が無いまま止めた。CI の docs ジョブに任せる）。

## 残したこと

- PyPI への公開（manylinux、version をタグと揃える。いまは pyproject の 0.1.0 のままで、タグの version と一致しない）。
- macOS の wheel。
