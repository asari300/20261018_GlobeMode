# S197 `desktop-review-fixes`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s197-desktop-review-fixes` |
| worktree | `.git-worktree/s197-desktop-review-fixes` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S197-desktop-review-fixes/` |
| セッション記録 | （親が写す） |

## 依頼

Desktop のレビュー（Opus/medium）の指摘 2〜6 を直す。1・7 は S196（`tauri-backend.ts`）、8 は TS も触るので持ち越し。

## 決めたこと

- 検査に予算を付けない決まり（ADR-0018）は変えず、中断だけを予算の有無に関係なく効かせる（`Running` を常に登録）。
- tauri に依存しない部分（MSL の切り替え、method の許可）を `src/shell.rs` に出した。理由: 手元に webkit2gtk が無く Desktop のクレートがビルドできないので、`worker.rs`・`msl.rs`・`shell.rs` を別の小さなクレートに取り込んで試験を回すため。
- 許す method は UI の `method: "…"` と完全に一致させ、試験で突き合わせる（UI が足すと試験が落ちる）。
- CSP は index.html の meta と同じ＋`connect-src` の `ipc: http://ipc.localhost`。試験で一致を確かめる。
- `exportFmu` の `outDir` の限界は ADR-0023 に書いた（直さない）。

## やったこと

`src/Rust/apps/desktop/src/{worker,msl,main,shell}.rs`、`tauri.conf.json`、ADR-0023、レビューの記録、このセッションの文書、CHANGELOG・roadmap。

## 検証

- 手元: `cargo fmt --check` 通過。別クレート経由の試験 13 件通過（中断 1・MSL の失敗と排他 1・取り消し 1・method の一致 1・CSP の一致 1 を足した）。中断の試験は `Running` の登録を外すと落ちることを確かめた（指摘 2 の再現）。
- clippy と Desktop のクレートの試験、bundle、smoke は GitHub Actions: 下の run。
- `check_docs.py`: 切れ 0 件。
