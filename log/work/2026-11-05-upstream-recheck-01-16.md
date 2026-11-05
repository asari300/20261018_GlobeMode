# S162 upstream-recheck-01-16

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s162-upstream-recheck-01-16` |
| worktree | `.git-worktree/s162-upstream-recheck-01-16` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S162-upstream-recheck-01-16/` |
| セッション記録 | （親が写す） |

## 依頼

`doc/features.md` の「Rumoca との差分照合」に残る「01〜16 の再確認は未」を片付ける。上流の下書き 01〜16 を Rumoca の最新のリリースの配布バイナリで確かめ直し、S113・S115 と同じ書式で本文・日本語版・1 枚の解説・一覧を直す。issue は出さない。

## 決めたこと

- 「最新」は v0.10.4（2026-11-05 でも最新）。main（`590ac739`）はビルドせず、ソースの比較にだけ使った。
- CLI で回せない件の確かめ方: 05 は Python 束縛の wheel（`simulate(params=)`）、06・10〜12・14 はソースの比較（Rumoca の v0.10.2・v0.10.4・main と diffsol main）。実行していない件は本文に【未確認】と書いた。
- `_jp` は再現が残る件だけ（01・02・05〜09・12〜16 の 12 件）。直った 03・04 と diffsol main で直った 10・11 は S115 に倣って作らない。
- 02・15 は「一部／ほとんど直った」とし、残る形（`SimpleTriacCircuit` の 1e-12、`Dimmer_RL`）だけを出す方針にした。

## やったこと

- `doc/research/upstream/01〜16-*.md` に「Re-check on Rumoca v0.10.4 (S162, 2026-11-05)」の節と環境の行、必要なら Status を足した。
- `_jp.md` と `_jp.html` を 12 組足した（`_jp.html` は生成スクリプトで作り、手元の `check_slides` で検査）。
- `doc/upstream.md`（表の状態・提出の順・回避を外す条件・英語の要約）、`doc/research/upstream/README.md`、`doc/features.md`、`doc/roadmap.md` を直した。
- スライド・解説書・索引の行。

## 検証

数字で書く。

- main のチェックアウトに `_jp.md` を 12 枚、誤って書いた（親の指摘で worktree へ `mv`。main の `git status` は空に戻った）。
- CLI の回し: 15 は 9 モデル × 2 版 × 2 許容誤差 = 36 回。03 は 2 モデル × 2 版。04 の MSL の形（`EngineV6`）は v0.10.4 で `filter.x` = 563.19。
- v0.10.4 で新しく見つかった停止: `MomentumBalanceFittings` の `ER002 Medium.ThermodynamicState`（11 s。v0.10.2 は 270 s で時間切れ）、`DrumBoiler` の `EX001`（33 s）。どちらも未調査で、GlobeMode の検査の本数には入れていない。
- コードの検査: Rust は触らないので `mise run check` は回していない。
- `check_docs.py`: デッキ OK（S162 を含む）／リンク 3427 件中の切れ 0 件。`_jp.html` 12 件は `check_slides` を直接呼んで問題 0 件
- `render_slides.py`: S162 のデッキと `_jp.html` 12 件（3＋4＋4＋3 の組で回した）で問題 0 件
- セッション記録の `--check`: （親が写すので、親が確かめる）
- ステージした差分の漏洩検査: メールアドレス・ホームのパス・トークンらしき文字列 0 件（`data/rumoca*` の symlink はステージしていない）

## 残したこと

- Rumoca の rev を v0.10.4 に上げる判断。上げたら 04 の GM-W10004、02・15 の auto の切り替えが外せるかを `msl:funnel`・`msl:sim` で見る。
- 03 の後に出た 2 つの停止の調査。
- 06・12・14 は実行していない。issue にする前に Rust のテスト、diffsol 単体のプログラム、wasm32 のビルドで確かめる。
