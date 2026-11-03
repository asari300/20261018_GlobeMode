# S113 `upstream-recheck-jp`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s113-upstream-recheck-jp` |
| worktree | `.git-worktree/s113-upstream-recheck-jp` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S113-upstream-recheck-jp/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

- upstream の 28〜32 は最新 Rumoca で再確認し、提出はしないが upstream の内容を更新し、日本語バージョンの issue を `_jp.md` とスライド 1 枚での解説を `_jp.html` で出力する（ユーザーが肉眼で確認する）
- AixLib の `specificEntropyOfpTX` は同様に issue 4 点セットを作成、提出はしない（ユーザーが肉眼で判断）

親のエージェントからの指示: Rust と `src/Python` は触らない、cargo は回さない、main へのマージ・push・worktree の削除は親がやる。

## 決めたこと

- **「最新」はリリース v0.10.4**（2026-11-02 公開、タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`）。配布バイナリを main のチェックアウトの `data/rumoca-latest/` に置き、`.gitignore` に `data/rumoca-latest/` を足した。main（`91d6e936`）は 10 コミット先だが、ソースからのビルドはしない指示なので【未確認】と書いた。
- **28 は件名を絞る**: v0.10.4 で `M`・`M2` が通るので、残る `M1`・`M3`・`M5`（replaceable な package を通す形）を件名にした。直った 2 本は対照として本文に残した。
- **33 は AixLib 向け**: 英語の `.md`、自己完結の `.mo`（`Rep41`）、`_jp.md`、`_jp.html`。AixLib が既に CI のホワイトリストに載せていること（issue #1291）を備考に書き、出し方はユーザーに任せた。
- **`_jp.html` は下書きの隣**に置いた。`check_docs.py` のスライドの検査は `doc/sessions/*/*.html` だけなので、同じ `check_slides` を直接呼んで確かめた。`check_docs.py` そのものは変えていない。
- **`render_slides.py` を 1 行直した**: 1 枚のデッキは → キーと `#S03` の検査を省く（1 枚では必ず落ちるため）。

## やったこと

- `doc/research/upstream/28`〜`32-*.md`: 「Re-check on Rumoca v0.10.4 (S113, 2026-11-03)」の節と環境の行を足した。28 は件名・判定・手順の注記も直した。
- `doc/research/upstream/28`〜`33-*_jp.md`・`*_jp.html`: 新規 12 ファイル。
- `doc/research/upstream/33-aixlib-specific-entropy-of-ptx-missing.md`・`.mo`: 新規。
- `doc/upstream.md`: 英語の要約、状態の凡例、28〜32 の行の状態と日本語版へのリンク、33 の行、提出の順の 10・15 位と 33 の行。
- `doc/research/upstream/README.md`: 表の 28〜32 の状態、33 の行、「Rumoca v0.10.4 での再確認と AixLib 向けの 33（S113）」の節。
- `doc/roadmap.md`: P3 の行と「S113 で済んだこと」。`doc/sessions/README.md`: 索引の行。
- `.claude/skills/session-docs/render_slides.py`: 上の 1 行。`.gitignore`: `data/rumoca-latest/`。

## 検証

- 28〜32 の再現 24 本を v0.10.2 と v0.10.4 で: 変わったのは 28 の `M`・`M2`（ET004 → 通る）の 2 本。残り 22 本はコードとメッセージが同じ（v0.10.2 の失敗 14・対照 10 が、v0.10.4 では失敗 12・対照 10・通るようになった 2）。
- 33: omc 1.27.2 は AixLib 3.0.1 + MSL 4.0.0 で `setState_psX` を呼ぶモデルを 670 行のエラーで断り、`setState_pTX` の対照を通す（約 9 s、最大 RSS 0.9 GB）。`Rep41.M` は omc・Rumoca 2 版とも断り、`M0` は通る。AixLib main の該当ファイルは取得物と差 0。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。
- `check_docs.py`: デッキ（`doc/sessions/*/*.html`）すべて OK / 文書 470 件・相対リンク 2,331 件中の切れ 0 件
- `_jp.html` 6 枚の `check_slides`: 6 件とも OK
- `render_slides.py`（main の `.venv/bin/python -I`、1 デッキずつ）: S113 の `slides.html` と `_jp.html` 6 件の計 7 件で問題 0 件。機械の負荷（1 分の平均 6〜21）で Chromium のスクリーンショットが 30 s で時間切れになることがあり、28・29・30 は 2〜3 回目で通った（はみ出しの失敗は 0）。最初の版の 33 は `<pre>` の行が右で切れていたので、コードの行を 56 桁以内に縮めた
- セッション記録の `--check`: 親が写すので対象外
- ステージした差分の漏洩検査（メールアドレス・ホームのパス・トークン）: 0 件

## 残したこと

- ライブラリの例題そのものは v0.10.4 で回していない【未確認】。28 の影響は 4 → 3 本の見込み。
- 17〜27（と 01〜16）は v0.10.4 で確かめ直していない。v0.10.2 → v0.10.4 は 221 コミットで、次元・record の直しが入っている。
- `data/rumoca-latest/` は main のチェックアウトにあり、この `.gitignore` が main に入るまで main で未追跡に見える。
