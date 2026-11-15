# S342 `docs-refresh`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s342-docs-refresh` |
| worktree | `.git-worktree/s342-docs-refresh` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S342-docs-refresh/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S322〜S334 の成果を利用者向けの文書に反映する。CHANGELOG の Unreleased に複数ソルバの比較（S322・S323・S327・S328）と Track B の進捗（S324〜S334）、README（英語の要約を含む）と features に複数ソルバの比較（無ければ）、roadmap の Track B の欄を今の数字と残りの要約にして経緯は ADR-0039 へ。markdownlint の警告 0、`doc/` と `log/` の相対リンク切れ 0。cargo は回さない。

## 決めたこと

- 複数ソルバの比較は README（英語・日本語・CLI の例）・features・CHANGELOG（S322・S323・S328）に既にあったので、新しく足したのは S327 の分だけ。英語の README の限界の 1 文（「相対差を出さない」）だけ S328 より前のままだったので直した。
- 数字は `doc/frontend-diff.md`（S334 の `msl:diff-frontend`）、ADR-0039 の付記、各セッションの索引の行から写した。推測の数字は無い。
- roadmap の Track B の欄は、経緯を消さずに ADR-0039 へ寄せた。ADR にあった数字を照合したところ、無かったのは 67 → 192 → 279 の推移の一部と resolve 467/566 など（形が違うだけのものを除く）で、付記 S342 に推移の表として入れた。
- 古くなっていた roadmap の「残り」（`GM-E` の番号が無い、定義ジャンプ・ホバーが未、feature `native` が無い、B3・B4 の旧い残り）は、S296・S305・S312〜S334 で済んでいたので要約から外し、理由を ADR の付記 S342 に書いた。
- 並行の S335〜S337 が roadmap の同じ欄の数字を変える。取り込み時に要約の形（B1〜B4 の今の数字・残り・製品へのつなぎ）に合わせる。
- markdownlint は、リポジトリに設定が無く既定だと MD013（行の長さ）が全行に出るので、MD013 を切った設定（作業用）で数えた。触ったファイルにあった既存の警告 25 件（表の中の `|`、空行、裸の URL、コードの前後の空白）も直した。

## やったこと

- `CHANGELOG.md`: 冒頭の範囲に S327・S329〜S334、運用・計画に S327、S322・S323 の行に S327 への案内、Track B に S324・S329〜S334、既知の限界の 279 本を 471 本に。
- `README.md`: 英語の複数ソルバの限界の 1 文を S328 に合わせ S327 の注意を足し、日本語に S327 の 1 文、Track B の数字を英語・日本語の 4 か所で 279 → 471 と S334 の一致に。
- `doc/features.md`: Track B の行を S334 まで、複数ソルバの行に S327。表の中の `|` を `\|` に、閉じ損ねた `**` を直した。
- `doc/roadmap.md`: Track B の行（範囲・完了条件）と「機能の穴（Track B）」の行を要約に。空行・裸の URL の警告を直した。
- `doc/adr/0039-native-cst-parser.md`: 付記 S342（推移の表と付記の案内）。コードの前後の空白を直した。
- `doc/sessions/README.md`: S342 の行。S328 の行の `|a-b|` を `\|a-b\|` に。
- `doc/sessions/S342-docs-refresh/`、この記録。

## 検証

- コードは変えていないので `mise run check` は省く。cargo は回していない。
- `markdownlint-cli2`（MD013 を除く既定）: README・CHANGELOG・roadmap・features・ADR-0039・sessions README で 0 件（最初は既存の 25 件）。
- 数字の突き合わせ: roadmap の旧い欄にあった数字の記号（`n/m`・`a → b`）を ADR-0039 と照合し、ADR に無かったものを付記 S342 に入れた。

## 残したこと

- S335〜S341 が main に入ったあとの数字の更新（roadmap・README・CHANGELOG・features の Track B）。
- features の Track B の行は、S335 以降を足していない。
