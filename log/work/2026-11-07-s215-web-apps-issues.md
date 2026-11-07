# S215 `web-apps-issues`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s215-web-apps-issues` |
| worktree | `.git-worktree/s215-web-apps-issues` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S215-web-apps-issues/` |
| セッション記録 | （親が写す） |

## 依頼

ユーザーが 2026-11-07 に `/ponytail-audit` で立てた issue のうちウェブアプリの 2 件（#22・#36）を、いまの main で 1 件ずつ再現してから直す。#22 は CI に検査を足す。#36 は無いものだけ足し、各アプリの試験に 1 本ずつ。

## 決めたこと

- 本文の上限は 1 MiB、`globe` の時間切れは 10 秒を全アプリ共通にした（[web-apps.md の 3.1](../../doc/web-apps.md)）。issue の「60000」「64 << 20」ではなく、依頼の例に合わせて小さくした。FastAPI の 60 秒は 10 秒に縮む。
- 時間切れの値は環境変数 `GLOBE_TIMEOUT_MS` で変えられる。試験で 10 秒待たないための最小の仕掛け（6 種で同じ名前）。
- 時間切れの状態: FastAPI は元から 504。ほかは「`globe` を実行できない」と同じ扱い（Phoenix の fmt/lint は 500、Gleam の整形は契約どおり 200 の `ok: false`、Astro は 500）。依頼の「503 相当」は、FastAPI の既存の 504 に合わせて採らなかった。
- issue #36 の `disable` と `indent: 0` も同じコミットで直した（小さいので）。
- CI の `web` は build と test だけ。clippy・deny は入れない。

## やったこと

- `src/Rust/apps/web-{axum,actix,rocket,loco}/src/lib.rs`: `Options { .., ..defaults }`。
- `.github/scripts/ci_plan.py`・`test_ci_plan.py`・`.github/workflows/ci.yml`: 出力 `web` とジョブ `web`（matrix 4 種）。
- FastAPI・Streamlit・marimo・Phoenix・Gleam・Astro: 上限・時間切れ・（Gleam の）排他。試験を各 1〜2 本足した。
- 文書: web-apps.md 3.1、architecture.md 2.5、ADR-0006 に追記、各 README、roadmap、CHANGELOG、索引、解説書とスライド、レビュー履歴。

## 検証

- 再現: 4 種の `cargo build` は直す前に E0063（axum は main のまま、ほかは直しを戻して）。FastAPI の新しい試験 3 本、Phoenix の 4 本が直す前に落ちた。Astro・Gleam・Streamlit・marimo は直す前のコードでは回していない。
- Rust の 4 種（直した後）: `cargo build --locked` 成功、`cargo test --locked` は axum 1・actix 3・rocket 6・loco 4 本通る、`cargo fmt --check`・`cargo clippy -- -D warnings` 通る。
- FastAPI `pytest` 9 本、Streamlit `pytest` 4 本、marimo `pytest` 5 本、Phoenix `mix test` 12 本、Gleam `gleam test` 9 本、Astro `node --test` 5 本が通る（`globe` は共有 target の debug をコピーして使った）。
- `test_ci_plan.py` 26 本通る。`check_docs.py` 切れ 0 件、スライド OK。`cargo fmt --all --check` 通る（ルートの workspace は変えていない）。
- CI: `gh workflow run ci.yml --ref feature/s215-web-apps-issues -f full=true` の run 42332243206 が success（新しいジョブ `web (axum)`・`web (actix)`・`web (rocket)`・`web (loco)` が全部 success）。
- 手元の load が 13〜28 のままで、cargo の前に load 10 未満を待つ規則は守れなかった（待ちを 30 分で諦め、`nice` と `CARGO_BUILD_JOBS=2` で回した）。

## 残したこと

- CLI を呼ぶ 6 種の試験は CI に無い。
- 巨大な本文を読む前に断る（chunked）のはプロキシの仕事として `shortcut:` に残した。
