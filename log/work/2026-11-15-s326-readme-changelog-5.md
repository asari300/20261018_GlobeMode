# S326 `readme-changelog-5`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s326-readme-changelog-5` |
| worktree | `.git-worktree/s326-readme-changelog-5` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S326-readme-changelog-5/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S317 の後に main に入った利用者に見える機能（S312・S315・S316・S318〜S323・S325）を README と CHANGELOG に反映する。コードは変えない。

## 決めたこと

- 対象は origin/main に入ったもの。S310 は未取り込みなので書かない。
- CLI の引数は `globe-mode-cli/src/main.rs`・`fmu.rs` の clap の定義と照らした（`--solvers`・`--parallel`・`--compare`・`--reference`・`--max-order`・`ssp-import`・`ssp-export --ssv`）。
- S312 と S325 は 1 行にまとめ、Rumoca と両方で通った分は方程式・釣り合い・状態・正則がすべて一致と書いた。

## やったこと

- `README.md`: 英語と日本語に、複数ソルバ、SSP の往復、キーボードの結線、Track B の一致、marimo の見本、CLI の例。
- `CHANGELOG.md`: 冒頭の範囲、P0（S320・S321）、P4（S316・S322・S323）、P5（S315・S318）、P6（S319）、Track B（S312・S325）、既知の限界。
- `doc/sessions/S326-readme-changelog-5/`、索引、この記録。

## 検証

- コードは変えていないので省く。数字は各セッションの merge のメッセージと ADR から写した。

## 残したこと

S310 が main に入ったら CHANGELOG の運用の項に足す。
