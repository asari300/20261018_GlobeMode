# S043 `split-all-msl`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s043-split-all-msl` |
| worktree | `.git-worktree/s043-split-all-msl` |
| Phase | P3（S037 の続き） |
| セッション文書 | `doc/sessions/S043-split-all-msl/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「続けて作業せよ。サブエージェントは Sonnet/high。レスポンスは日本語」。S037 で残した 2 つ: 分割読み込みを MSL の全例題で確かめること、GitHub Actions の `msl-sim.yml` の結果を `doc/msl-sim.md` に取り込むこと。

## 決めたこと

- 全例題の確認は手元（RAM 3 GB）でなく GitHub Actions の `msl-sim.yml` のジョブ `split-all` で回す。`msl-sim` と並行（`needs` なし）にし、落ちても msl-sim を止めない。
- 一覧は `cargo xtask msl-funnel --list`（索引だけ）から作る。funnel の JSON を待たない。
- 不一致は最初の 1 本で止めず、全部回してから一覧にする。panic も `catch_unwind` で結果として比べる。
- `GM_SPLIT_FRESH=1` でモデルごとに分割読み込みの検査器を作り直す（サブエージェントの提案を採った）。読んだファイルは外さないので、続けて回すと先のモデルが読んだファイルが閉包の足りなさを隠すため。
- `doc/msl-sim.md` の更新は、S037 で起動した run 41170414005 の artifact が出てから行う（この時点で msl:sim の段を実行中）。

## やったこと

- `src/Rust/mod/globe-mode-frontend/tests/split.rs`: `GM_SPLIT_MODELS`・`GM_SPLIT_FRESH`、不一致の一覧、panic の扱い。レビューで doc コメントの一覧の作り方（jq → `msl-funnel --list`）をワークフローに合わせた。
- `.github/workflows/msl-sim.yml`: ジョブ `split-all`（fetch-msl → `--list` → release の試験を `tee`、`shell: bash` で pipefail。一覧と log を artifact `split-all` に）。
- 文書: roadmap の S043 の節、セッション文書、索引。

## 検証

- 手元（release、サブエージェント）: 4 本の一覧で全部一致。存在しないモデルを混ぜた 3 本で、両方 `GM-E1001` の一致。`GM_SPLIT_FRESH=1` の 4 本で一致（19.5 s。作り直しで 1 本あたり約 1 s 増える）。`msl-funnel --list` は 566 本。
- `cargo fmt --all --check`: OK。`cargo clippy -p globe-mode-frontend --all-targets -- -D warnings`: 警告 0。
- `mise run check` は回していない（変えたのは ignored の統合試験とワークフローだけ。CI が全体を検査する）。
- 566 本の `split-all` は、このコミットの後に GitHub Actions で回す。
