# S115 `upstream-recheck-17-27`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s115-upstream-recheck-17-27` |
| worktree | `.git-worktree/s115-upstream-recheck-17-27` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S115-upstream-recheck-17-27/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

親のエージェントから: S113 の持ち越し。下書き 17〜27（と 25b〜25h）を Rumoca v0.10.4 で再確認し、状態を更新する。S113 が 28〜32 で行ったのと同じ手順・形式。まだ再現する件に `_jp.md` と 1 枚の `_jp.html`、直った件は `doc/upstream.md`・`doc/research/upstream/README.md` の状態を「v0.10.4 で修正」にして提出の順から外す。提出はしない（ユーザーが日本語版を肉眼で見て判断する）。01〜16 は対象外。Rust は触らない、cargo は回さない、main へのマージ・push・worktree の削除は親がやる。

## 決めたこと

- **バイナリは S113 と同じ**: v0.10.2 は main の `data/rumoca/`、v0.10.4（`9c8c56fc`）は `data/rumoca-latest/`。worktree の `data/` に main の取得物への symlink を張った（ステージしない）。
- **直った件（25・25b・25c・25g・26）は消さずに残す**: 英語の本文の Verdict の下に「Status (S115): fixed in Rumoca v0.10.4」と再確認の節を足し、`_jp` は作らない。upstream.md の状態の凡例に「v0.10.4 で修正」を足した。
- **20 は 20b の形だけを出す**: `loadResource` の形（`Rep20Load`・`Rep20TwoExtends.M`）が v0.10.4 で通り、20b（`Rep28.M`）が残る。本数は 138 → 2 で、提出の順を 5 位 → 13 位にした。`_jp` は 20b の形で書いた。
- **25e は `parameter` の形だけを出す**: `MC`（`constant`）は v0.10.4 で通る。MSL 4.1.0 の `EOSIdealCoeff` は `parameter Integer nc=5; parameter Real[nc] a`（`Media/R134a.mo` の 52〜53 行で確かめた）なので、ライブラリの形は残るほう。
- **提出の順**: 24 が 6 位 → 5 位、16 以降が 1 つずつ繰り上がる。直った 5 件は「—」の行にまとめた。S113 の 28〜32 の `_jp.html` の「提出の順は N 位」も直した。
- `_jp.html` は 11 枚を同じ型から作った（作業用のスクリプトは `/tmp` に置き、コミットしない）。`<pre>` の行は 56 桁以内（S113 の注意）。

## やったこと

- `doc/research/upstream/17`〜`27-*.md`（16 件）: 「Re-check on Rumoca v0.10.4 (S115, 2026-11-03)」の節と環境の行。直った件と一部直った件（20・25e）は Verdict の下に Status の行。
- `doc/research/upstream/*_jp.md`・`*_jp.html`: 新規 22 ファイル（17・18・19・20・21・24・25d・25e・25f・25h・27）。
- `doc/research/upstream/28`〜`32-*_jp.html`: 提出の順の数字。
- `doc/research/upstream/README.md`: 表の 17〜27 の状態と日本語版へのリンク、冒頭の経緯、「Rumoca v0.10.4 での再確認（17〜27、S115）」の節。
- `doc/upstream.md`: 英語の要約、本文の冒頭、状態の凡例、表の 17〜27 の状態、提出の順、回避の節の 1 文。
- `doc/roadmap.md`: P3 の行と「S115 で済んだこと」。`doc/sessions/README.md`: 索引の行。`doc/sessions/S115-upstream-recheck-17-27/`: スライドと解説書。

## 検証

- 再現 62 モデル × 2 版（v0.10.2・v0.10.4）: v0.10.4 で通るようになった 11（`Rep20Load`・`Rep20TwoExtends.M`・`Rep25.M`・`Rep27.M`・`Rep29.M`・`Rep29.M2`・`Rep31.MC`・`Rep33.M`・`Rep26.A`・`B`・`C`）、両方で落ちる 20、両方で通る 31、v0.10.4 で新たに落ちる 0。残る 20 本のコードとメッセージは v0.10.2 と同じ（17 の `DefId(78)` も同じ）。
- MSL を読む 3 本（`Rep18`・`Rep20Load`・`Rep20Load0`）は機械の負荷（1 分の平均 27〜34）で 1 回に数分かかった。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。
- `check_docs.py`: デッキすべて OK / 文書 497 件・相対リンク 2,498 件中の切れ 0 件
- `_jp.html` 17 枚（新規 11・S113 の 6）の `check_slides`: 17 件とも OK
- markdownlint（MD013・MD034 を除く）: 新しい行での警告 0 件（20 の英語の本文の見出しの重複、roadmap・索引の既存の警告は前から）
- `render_slides.py`（main の `.venv/bin/python -I`、1 デッキずつ、1 分の平均が 10 未満になってから開始）: S115 の `slides.html` と新しい `_jp.html` 11 件の計 12 件で問題 0 件（はみ出し 0）。Chromium のスクリーンショットが 30 s で時間切れ・接続切れになることがあり（負荷は途中で 3〜21 に揺れた）、slides・20・21・27 などは 2〜4 回目で通った
- セッション記録の `--check`: 親が写すので対象外
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件

## 残したこと

- 01〜16（実行系・ソルバ・diffsol）は v0.10.4 で確かめ直していない。MSL の例題の実行が要る。
- ライブラリの例題そのものは v0.10.4 で回していない【未確認】。直った件の本数（`loadResource` 136、25 の 44、25g の 25、26 の 47 など）が消えるかは、GlobeMode の Rumoca の rev を上げて `msl:funnel`・`libraries.yml` で確かめる。
- Rumoca の main（v0.10.4 より後）は【未確認】。
