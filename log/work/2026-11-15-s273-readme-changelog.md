# S273 `readme-changelog`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s273-readme-changelog` |
| worktree | `.git-worktree/s273-readme-changelog` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S273-readme-changelog/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

2026-11-14〜15 に main に入った機能を README（日本語と英語の要約）と CHANGELOG に反映する。コードは変えない。main に無いものは書かない。

## 決めたこと

- 対象は `git log --merges --since=2026-11-14 origin/main` に出る S249・S250・S251・S253・S254・S256・S257・S259・S261・S262・S263・S266・S268（と S255・S267 の一行）。
- `integrate-1115`（S258・S260・S264・S265）は main に無いので書かず、「できていないもの」に線形化の UI・Python を残した。
- README の古い「Diagram / Icon ビューと SSP は未」を、Icon・Diagram（S244〜）と SSP の読み取り（S253〜）に合わせて直した。

## やったこと

- `README.md`: 英語の「What works」と日本語の「できること」に 3 項目、「できていないもの」の更新、CLI の例に `fmu run`・`fmu cosim`・`linearize`・`blt`、Python の `blt`、限界の表の FMI の行。
- `CHANGELOG.md`: P4・P5・P8 と Track B の節に追記、既知の限界の更新。
- `doc/sessions/S273-readme-changelog/`、索引、この記録。

## 検証

- コードの検査: コードは変えていないので省く。
- `check_docs.py`: スライド全件 OK、リンク 4691 件中の切れ 0 件。markdownlint（MD013 の行の長さを除く）: 0 件。

## 残したこと

`integrate-1115` が main に入ったら、線形化の UI・Python、Python の cosim、VS Code の Diagram の編集、部品の追加を足す。
