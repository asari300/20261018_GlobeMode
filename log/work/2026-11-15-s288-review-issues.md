# S288 `review-issues`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s288-review-issues` |
| worktree | `.git-worktree/s288-review-issues` |
| Phase | P5・P7・P8（保守） |
| セッション文書 | `doc/sessions/S288-review-issues/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

レビューが 2026-11-15 に立てた GitHub issue #98〜#104（7 件）を 1 件ずつ監査し、妥当なものを直す。issue は閉じない。Diagram の編集が `feature/integrate-1116`（S281・S283）で変わっているので、そこから worktree を切る。

## 決めたこと

- 7 件とも issue の再現どおりにコードを読んで妥当と判断。見送り 0、issue へのコメント 0。
- #99 は「読み方を直す」を選んだ（`declarations` が名前ごとに読む）。`delete_part` の複数の名前の拒否は S281 のまま残す。
- #100 の検査は LSP でなく fmt の共有の関数（`coord`・`end_ok`）に置いた。LSP の 3 つの method は fmt を呼ぶだけで、None は `null` になるため。
- #101 に時間の試験は足さない（共有ランナーで落ちる時間の試験を S270 で直した経緯）。動作は既存の往復試験で守る。
- #103 は Python の引数を `timeout` だけにした（issue どおり）。`memory_limit_mib` は dict の spec に書ける。clippy の `too_many_arguments` は `#[allow]` で受けた（pyo3 の口が引数をそのまま並べる）。

## やったこと

`globe-mode-fmt/src/diagram.rs`（#98・#99・#100）、`globe-mode-fmi-import/src/{ssp,cosim}.rs`（#101・#102）、`globe-mode-cli/src/fmu.rs`・`globe-mode-py/src/lib.rs`・`_native.pyi`・`globe_mode/__init__.py`（#103。#104 の Python 側も）、`globe-mode-lsp/src/lib.rs`（#104）。試験を 5 本足し、roadmap・セッション文書・索引を更新。

## 検証

- fmt の試験 31 本・LSP の lib 64 本・fmi-import の lib 14 本・CLI の `fmu_cosim` 11 本（Reference-FMUs を `data/` に symlink して実行）・Python の `test_fmu.py` 含む 9 本が通る。
- 変えた 5 crate の `cargo clippy --all-targets -D warnings` は 0 件、`cargo fmt --check` は差分なし。
- fmt の 3 本は旧い `diagram.rs` に試験だけ足して 3 本とも落ちることを確かめた。LSP・fmi-import・CLI・Python の試験は旧コードでの再実行はしていない（#103 は引数が無く、#102・#104 は誤りが出ない、という構造から）。

## 残したこと

`delete_part` が複数の名前の宣言を断る点、`dx`・`dy`・`x`・`y` が数でないとき 0 とする点（LSP）。
