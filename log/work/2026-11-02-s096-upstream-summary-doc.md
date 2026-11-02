# S096 `upstream-summary-doc`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s096-upstream-summary-doc` |
| worktree | `.git-worktree/s096-upstream-summary-doc` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S096-upstream-summary-doc/` |
| セッション記録 | [2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

上流（Rumoca・diffsol）への issue の下書き 01〜30 を、提出する前に doc ディレクトリにまとめる（ユーザーの判断、2026-11-02）。`doc/upstream.md` に 1 つの表と「提出の順」「提出のときの手順」「GlobeMode 側の回避を外す条件」。下書き 25 を形ごとに分ける。文書だけ。main へのマージ・push は親。

## 決めたこと

- 表は 35 行（01〜21、24、25、25b〜25h、26〜30）。22・23 は正当な拒否で下書きが無いので表の外に 1 行。
- 状態は「未提出」（CLI か自己完結の `.mo` で再現済み: 01・03・17〜30 の 21 行）と「Rumoca main で再確認が要る」（API だけ・`[not verified]`・diffsol: 02・04〜16 の 14 行）。提出済み 0。
- 提出の順は影響の本数順（指示どおり）。回避のある件は本数が多くても後に回すと注記した（16 は 70 本で 7 番目）。
- 25 は `function conditional` だけに戻し、S080・S083 の節の内容を 25b〜25h の `.md` に移した。`.mo` の 1 行目のコメントを `draft 25b` などに直した。
- CLAUDE.md の正典の行は 1 語（`upstream.md`）で済んだので足した。

## やったこと

- 新規: `doc/upstream.md`、`doc/research/upstream/25b〜25h-*.md`（7 枚）、セッション文書 2 つ、この作業記録。
- 変更: `doc/research/upstream/25-*.md`（89 行減）、`doc/research/upstream/README.md`（冒頭のリンク、25 の行を 8 行に、経緯の文の言い換え）、`25b〜25h-*.mo` の 1 行目、`README.md`（文書の表に 1 行）、`CLAUDE.md`（正典の行）、`doc/sessions/README.md`（S070 の下に 1 行）。

## 検証

- コードの検査: Rust の変更 0 のため回していない。
- `check_docs.py`: S096 のデッキ OK / 文書 404 件・相対リンク 1,979 件中の切れ 0 件（1 回目は `[#番号](URL)` の 1 件を直した）。
- `render_slides.py`: S096 のデッキだけを 1 回試し、240 s で出力 0（時間切れ）。CI に任せる。
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件。

## 残したこと

- 04・05・07・08 の影響の本数は数えていない【未確認】。
- 25f・25h・25g の「Actual」のメッセージは S083 の記録から写した。CLI の全文は出すときに写し直す（各下書きの Notes に書いた）。
- Rumoca main での再確認はしていない（出すのはユーザー）。
