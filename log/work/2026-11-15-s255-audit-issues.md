# S255 `audit-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s255-audit-issues` |
| worktree | `.git-worktree/s255-audit-issues` |
| Phase | 保守 |
| セッション文書 | `doc/sessions/S255-audit-issues/` |
| セッション記録 | なし（並行のサブエージェントの担当で、このセッションだけの記録が `log/local/` に無い） |

## 依頼

2026-11-14 の ponytail-audit が立てた GitHub issue #90〜#97 を 1 件ずつ監査し、妥当なものを直す。妥当でないものは issue にコメントする（閉じない）。

## 決めたこと

- 8 件とも今の main で再現でき、見送りは 0 件。issue へのコメントは付けていない（マージで `Fixes #n` が閉じる）。
- #90: 提案の規則だと既存の記録 4 件と散文に当たったので、改行と `::` を除き、残った仮の値 4 件を既存の記録で伏せた。
- #93: 読む側で空欄を許さず、書く側を `NaN` にした。
- #95: 提案に無かった `generate_context!` の入れ物（空の `dist` と sidecar 名のファイル）を CI のジョブに足した。
- #96: `read_capped` を 1 つにして `info` と `run` で共有した。

## やったこと

[解説書](../../doc/sessions/S255-audit-issues/解説書.md) の表のとおり。触ったもの: `.claude/hooks/save_session_log.py`（と試験）、`log/sessions` の 1 枚（4 か所）、`src/Python/mod/globe_mode/{widget,__init__}.py`（と試験 2 本）、`src/Svelte/src/lib/plot/csv.ts`（と試験）、`src/VSCode/globe-mode/package.json`（と試験）、`.github/{scripts/ci_plan.py,scripts/test_ci_plan.py,workflows/ci.yml}`、`globe-mode-fmi-import`（`lib.rs`・`tests/unpack.rs`）、`globe-mode-fmt`（`layout.rs`・`tests/format.rs`）、ADR-0006・architecture の 2.5 章。

## 検証

- 試験: `test_save_session_log.py` 3 本、pytest（src/Python/tests）48 本 pass・4 本 skip（widget の extra が無い分。anywidget を入れて widget の 2 本も pass）、`test_columns.mjs` pass、VS Code の `save.test.mjs` 2 本 pass、`test_ci_plan.py` 29 本 pass、`globe-mode-fmi-import` の `unpack` 6 本 pass、`globe-mode-fmt` 21+22 本 pass。
- clippy 1.100（`globe-mode-fmt`・`globe-mode-fmi-import`、`--all-targets -D warnings`）: 警告 0。
- feature で `ci.yml`（手動）を回した: ジョブ `desktop` が success。python は widget の試験の注釈の誤りで落ち（直した）、再実行で確かめる。
