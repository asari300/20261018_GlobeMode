# S366 `features-cleanup`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s366-features-cleanup` |
| worktree | `.git-worktree/s366-features-cleanup` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S366-features-cleanup/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S356 の「次回」の 2 件（`doc/features.md` の重なった行を 1 機能 1 行に畳む。README の既知の限界の「参照・リネームの範囲は開いている文書の中（S166）」が古いかを確かめて事実に合わせる）と、ほかの README・features の S335〜S364 との食い違い。markdownlint 0、リンク切れ 0。cargo は回さない。

## 決めたこと

- 重なった 4 機能は、どちらの版が上位かを語単位の差分で見て、上位の版に足りない分だけ足した（整形・lint に S192、Desktop に S176）。情報は消していない。経緯の S 番号は残した。
- 参照・リネームの範囲は、LSP（S230・S235）と UI（S161・S166・S235）で分けて書いた。UI は `Backend.references`・`rename` → wasm の `references_at`・`rename_at`、または sidecar の `globe/references`・`globe/rename` で、どれも 1 文書。確かめられなかったものは無い（UI の実機の操作は見ていない。コードの読みだけ）。
- 数字（538 本・367 本・518 本）は `doc/frontend-diff.md` の表に合わせた。推測で足した数字は無い。

## やったこと

- `doc/features.md`: 「整形・lint」「Text ビュー」「Desktop」「Android」を各 1 行に畳み、表を割っていた空行 2 つを無くした。「補完・定義ジャンプ・参照・リネーム・ホバー」の S166 の時点の記述と S230・S235 の記述を分けた。S357・S358（CLI）、S360（call hierarchy）、S363（Text ビュー）を足した。
- `README.md`: 英語・日本語の「編集」、日本語の既知の限界、「今後必要になりそうな機能」の 2 行、「できていないもの」の数字（英語・日本語）、VS Code の節。
- `CHANGELOG.md`: 既知の限界の 518 本を 548 本に。
- `doc/sessions/README.md`: S366 の行。`doc/sessions/S366-features-cleanup/`、この記録。

## 検証

- コードは変えていないので `mise run check` は省く。cargo は回していない。Playwright も回していない（UI は読みで確かめた）。
- `markdownlint-cli2`（MD013 を除く既定）: README・CHANGELOG・features・roadmap・sessions README・新しい解説書・この記録で 0 件。重なった行を畳んだ結果、それまで表の外の段落だった features の 3 行（Diagram / Icon・Documentation・パラメータダイアログ）が表に戻り、1 行の末尾の `|` の前の空白（MD060）を足した。
- `check_docs.py`: スライド全部 OK・リンク 文書 1325 件 / 相対リンク 5495 件 / 切れ 0 件。
- `render_slides.py`（自分のデッキだけ）: デッキ 1 件 / 問題 0 件（desktop-light・desktop-dark・mobile 各 6 枚）。1 回目は `Page.screenshot` が 30 s で時間切れ、2 回目で通った。
- 文書の漏洩検査（メールアドレス・ホームのパス・トークン）: 追加した行で 0 件。

## 残したこと

- UI の参照・リネームを複数ファイルに広げること（応答の形を変える必要がある）。
- S338（Rumoca 0.10.4）が入ったあとの README の Rumoca の版と数字。
