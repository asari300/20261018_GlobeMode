# log/review — レビュー履歴

監査役（Codex。副次的に `/local-review` の Bonsai）が出した指摘と、その採否だけを残す。コードの変更はここに書かない。

- 1 レビュー 1 枚。名前は `<日付>-sNNN-<topic>-<レビュアー>.md`（例 `2026-10-31-s033-protocol-facade-codex.md`）。雛形は [_TEMPLATE.md](_TEMPLATE.md)。
- レビュアーの出力は**原文のまま**貼る。採否と理由は実装側（Claude か人）が書く。指摘を直したら、そのコミットの hash を書く。
- 書くもの: 対象のコミット範囲（`git diff <base>..<head>`）、レビュアーのモデルと版、指摘、採否、結論。
- 書かないもの: 差分そのもの、ファイルの全文、メールアドレス・ホームのパス・トークン。コミットの前に `python3 .claude/hooks/save_session_log.py --check log/review/*.md` で取りこぼし 0 件を確かめる。
- 採否の種類: **採用**（直した）/ **保留**（候補に積んだ。`doc/roadmap.md` の候補の番号を書く）/ **不採用**（理由を書く。レビュアーの誤りなら、何を誤ったかまで）。
- `/finish-session` の 5（コミット）の前に書く。レビューを受けなかったセッションは枚数 0 でよい。

## フォルダで残したレビュー（別セッションの調査）

別セッションが書いた調査（元は `doc/sessions/S136・S149〜S152` の名前で PR・ブランチにあったが、S 番号が main の索引と衝突したため S192 でここへ移した。本文は元のまま、リンクと実行コマンドのパスだけ直した）。索引 `doc/sessions/README.md` には載せない。

| フォルダ | 内容 | 経過 |
| --- | --- | --- |
| [2026-11-03-review-map/](2026-11-03-review-map/README.md) | 全体のレビュー地図（2026-11-03 の main の snapshot）。PR #18 | 地図のみ |
| [2026-11-03-review-fmt-panics/](2026-11-03-review-fmt-panics/review_jp.md) | 整形器の panic 2 件（PR #17）。作業記録 [2026-11-03-review-fmt-panics.md](../work/2026-11-03-review-fmt-panics.md)、[独立レビュー](2026-11-03-s151-review-fmt-panics-independent.md) | S192 で直した |
| [2026-11-03-review-fmt-crlf/](2026-11-03-review-fmt-crlf/review_jp.md) | CRLF の複数行ブロックコメント（issue #19）。作業記録 [2026-11-03-s150-review-fmt-crlf.md](../work/2026-11-03-s150-review-fmt-crlf.md) | S192 で直した |
| [2026-11-03-review-fmt-structure/](2026-11-03-review-fmt-structure/review_jp.md) | `redeclare model extends` の修飾（issue #20）と `end` の不一致（issue #21） | #20 は S167 で修正済み、#21 は S192 で直した |
