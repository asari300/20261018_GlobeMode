# S292 `readme-changelog-2`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s292-readme-changelog-2` |
| worktree | `.git-worktree/s292-readme-changelog-2` |
| Phase | 文書だけ |
| セッション文書 | `doc/sessions/S292-readme-changelog-2/` |
| セッション記録 | なし（このセッションの専用の記録が `log/local/` に無いため省く） |

## 依頼

S273 の後に main に入った機能を README（日本語と英語の要約）と CHANGELOG に反映する。コードは変えない。

## 決めたこと

- 対象は origin/main の 0a27686e までに入った S274・S275・S276・S277・S278・S279・S280・S281・S282・S283・S285・S286・S288。S284 は main に無いので書かない。
- S258・S260・S264・S265 は S273 の後の統合で CHANGELOG にも README にも入っていることを確かめた（足していない）。
- roadmap・features は各セッションが更新済みなので触らない。

## やったこと

- `README.md`: 英語の What works と日本語のできること（ssp-export・Desktop のコシミュレーション・固有値と安定性・stages・図の削除・Track B をつないだ範囲）、できていないものの更新、CLI の例に `globe fmu ssp-export` と `globe stages`、限界の表の FMI の行。引数は clap の定義（`fmu.rs`・`main.rs`）と照らした。
- `CHANGELOG.md`: 運用（S288）、P4（S280・S282、S281・S283）、P5（S274・S277）、P7（S285・S286）、Track B（S275・S276・S279・S278）、既知の限界。
- `doc/sessions/S292-readme-changelog-2/`、索引、この記録。

## 検証

- コードの検査: コードは変えていないので省く。
- `check_docs.py`: スライド全件 OK、リンク 4831 件中の切れ 0 件。`render_slides.py`（自分のデッキ）: 問題 0 件。markdownlint（MD013 を除く）: 0 件。
- ステージした差分のメールアドレス・ホームのパス・トークン: 0 件（下のコミット前に確認）。

## 残したこと

S284 が main に入ったら Track B に一行。
