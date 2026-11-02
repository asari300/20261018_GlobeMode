# S077 `ci-lang-wheel`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s077-ci-lang-wheel` |
| worktree | `.git-worktree/s077-ci-lang-wheel` |
| Phase | P7 / P8 |
| セッション文書 | `doc/sessions/S077-ci-lang-wheel/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S075 の持ち越し（Zig・Julia の例を CI に入れる）と S074 の持ち越し（wheel の版をタグにそろえる、Linux の wheel を manylinux にする）。CI の定義が中心で、Rust のコードは変えない。main へのマージと push は親が行う。

## 決めたこと

- ci.yml にジョブ `lang`（ubuntu-24.04）。`mise run c:zig` と `mise run c:julia`。mise-action の `install_args: rust zig julia`、rust-cache の `key: lang`（release の成果物なので debug と分ける）、`MISE_TASK_RUN_AUTO_INSTALL: "false"`。
- `ci_plan.py` の出力 `lang` = 全体、または `globe-mode-c` が影響を受けた（依存の crate の変更を含む。`python` と同じ考え方）、または `src/Zig/*`・`src/Julia/*` を変えた。`mise.toml` は既に `FULL_TRIGGERS`。
- wheel の版: タグの push のときだけ `sed` で `pyproject.toml` の `version` をタグ（`v` を除く）に。dry-run は 0.1.0 のまま。
- manylinux: Linux のときだけ `MATURIN_PEP517_ARGS="--compatibility manylinux_2_39"`。`pyproject.toml` に書かないのは手元の editable のビルドに効かせないため。auditwheel・新しい action は足していない。
- 段は先に `manylinux_2_34` を試したが、maturin が `GLIBC_2.39`（libc）と `GLIBC_2.35`（libm）の記号で落とした（run 41376053596）。ランナーの glibc に合わせて `manylinux_2_39` にした。古い glibc 向けは見送り（docker か maturin の `--zig` が要る）。

## やったこと

- `.github/scripts/ci_plan.py`: 出力 `lang` と `LANG_FILES`。
- `.github/scripts/test_ci_plan.py`: `test_lang_runs_for_c_abi_zig_julia`、`test_nothing_relevant` に `lang`。
- `.github/workflows/ci.yml`: plan の outputs に `lang`、ジョブ `lang`。
- `.github/workflows/release.yml`: 「wheel を作る」に版の差し替えと `MATURIN_PEP517_ARGS`、冒頭の説明。
- 文書: `doc/sessions/S077-ci-lang-wheel/`、索引、roadmap（S039 と S075 の持ち越しに取り消し線）、CHANGELOG、CLAUDE.md（`lang` の 1 行）。

## 検証

- `python3 .github/scripts/test_ci_plan.py`: 24 件 OK。
- `mise run ci:plan`: `.github/` の変更で full、`lang: true`。
- ci.yml（`-f full=true`）run 41376046452: 全ジョブ success。`lang` は約 10 分 40 秒（キャッシュ無しの release ビルド込み）、zig 0.17.0・julia 1.13.2、どちらも `simulate: ... last time=1`。
- release.yml（dry-run）run 41376053596: `manylinux_2_34` で Linux の 2 つが失敗（上記）。windows は success。
- release.yml（dry-run）run 41378655279（`manylinux_2_39`）: 5 ジョブとも success、約 11 分。`globe_mode-0.1.0-cp312-abi3-manylinux_2_39_x86_64.whl`・`manylinux_2_39_aarch64`・`win_amd64`、pytest は 3 つとも 2 passed。タグの版の差し替えは手元で sed だけ確かめた（タグの push は試していない）。
- Rust のコードは変えていないので `mise run check` は回していない（CI の full で rust-linux・rust-windows・deny が success）。
- `check_docs.py`: スライド OK、文書 337 件 / 相対リンク 1628 件 / 切れ 0 件。
- `render_slides.py`: 手元で 1 回試して Page.screenshot が 30 s で時間切れ（別のセッションの cargo の負荷）。CI のジョブ docs に任せる（run 41376046452 の docs は success だが、S077 のスライドを足す前のコミット）。

## 残したこと

- 古い glibc（2.17〜2.36）向けの wheel。
- PyPI への公開そのもの（trusted publishing の設定はユーザー）、macOS。
- `lang` は `globe-mode-c` の依存（core など）の変更でも回るので、Rust を触る push の多くで 10 分ほど増える（キャッシュが効けば短くなる見込み【未確認】）。
