# S317 `readme-changelog-4`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s317-readme-changelog-4` |
| worktree | `.git-worktree/s317-readme-changelog-4` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S317-readme-changelog-4/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S304 の後に main に入った利用者に見える機能（S297・S301・S303・S305〜S309・S311・S313）を README（日本語と英語の要約）と CHANGELOG に反映する。コードは変えない。

## 決めたこと

- 対象は origin/main（e35cded3）に入ったもの。S310 は未取り込みなので書かない。S312 は内部の診断なので書かない。S301・S308 は CHANGELOG に既にあり、README に足した。
- native の検査は「試し（既定は Rumoca）」。S6 まで、実行に未接続、wasm・UI から選べない、と書いた。「できていないもの」の Track B の記述を直した。
- S303 は運用・計画（P0）の項に 1 行（S255 と同じ置き場）。

## やったこと

- `README.md`: 英語と日本語に native の検査、ブラウザ・Android の MSL の分割配信、VS Code のキー操作とパラメータ、できていないもの、CLI の例、Android の MSL の同梱の説明。
- `CHANGELOG.md`: 冒頭の「足した範囲」、P0（S303）、P4（S306・S309）、Track B（S297・S305・S307・S311・S313）、既知の限界。
- `doc/sessions/S317-readme-changelog-4/`、索引、この記録。

## 検証

- コードの検査: コードは変えていないので省く。
- 数字は各セッションの merge のメッセージ・[doc/frontend-diff.md](../../doc/frontend-diff.md)・ADR-0039 から写した。
- check_docs・markdownlint の結果は報告に書く。

## 残したこと

S310 が main に入ったら CHANGELOG の運用の項に足す。
