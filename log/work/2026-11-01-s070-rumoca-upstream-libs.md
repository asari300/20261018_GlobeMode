# S070 `rumoca-upstream-libs`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s070-rumoca-upstream-libs` |
| worktree | `.git-worktree/s070-rumoca-upstream-libs` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S070-rumoca-upstream-libs/` |
| セッション記録 | （親が写す） |

## 依頼

S063 の持ち越し。公開ライブラリ 7 つの失敗 1,781 本の上位 5 つ（ER132・ER070・EI032・EF025・EF004）が「ライブラリの誤りか Rumoca の穴か」を、最小の再現・MLS 3.6・omc で判定し、上流（Rumoca）への英語の下書き 17〜21 にする。製品コードは変えない。issue は出さない。

## 決めたこと

- 下書きは 17〜21（16 は並行の S68 が使う）。S 番号は S070 が空いていたのでそのまま（`doc/sessions/README.md` と `git ls-remote --heads origin` で確認。S068 は `feature/s068-external-builtin`、S069 の worktree もある）。番号の衝突は無い。
- 再現の `.mo` のモデル名は `Rep17`〜`Rep21` で始める。Rumoca の CLI は同じフォルダの `.mo` を全部読むため、他の再現と名前が重なると `EM001` になる（19 と 21 に同じ `RepA` があり、実際に当たったので直した）。
- Rumoca の CLI は `cargo xtask fetch-rumoca`（取得済みの release 版 v0.10.2）を使った。GlobeMode の `globe` は共有 target の既存の release を使い、ビルドし直していない。
- EF025 の replaceable な Medium の関数は、最小の再現が作れなかったので「不明」と書き、README の状態に「要再現」を付けた（無理に合法・違法を決めない）。
- EF004 は 1 つの下書きに原因 2 つ。出すときの分け方はユーザーに任せる。

## やったこと

- `doc/research/upstream/17〜21-*.md` と `.mo`（5 組）。各 md は Summary / Steps to reproduce / Expected / Actual / Environment / Workaround in GlobeMode / Notes。Rumoca の箇所はファイル名と行番号だけ（rev `276e82b`）。
- `doc/research/upstream/README.md`: 表に 17〜21、節「公開ライブラリの失敗から」。16 の行は触っていない。
- `doc/research/libraries-2026-11-01.md`: 「原因の中身」の 5 行の【未確認】を判定に直し、「→ 上流 N」のリンクを付けた。
- `doc/roadmap.md`: S070 の節、S063 の持ち越しを済に（ER023 は残す）。
- `doc/sessions/S070-rumoca-upstream-libs/`（slides.html・解説書.md）、`doc/sessions/README.md` の行。

## 検証

数字で書く。

- 再現できた件: 5 件中 5 件（EF025 は `loadResource` と自己完結の版。Medium の関数の分は再現できず）。失敗するモデル 17 → 1、18 → 1、19 → 2、20 → 2、21 → 3（EF004）。対照（通るはずのモデル）は全部通った。
- `globe check`（release）と Rumoca v0.10.2 の CLI で、失敗の Rumoca のコードが全部一致（ER132・ER070・EI032・EF025・EF004）。
- omc 1.27.2 の `checkModel`: 失敗するモデルと対照のすべてが成功（17〜21 の全部）。
- MLS 3.6 の本文は `specification.modelica.org/maint/3.6/` のページから取り出して確かめた（確認日 2026-11-01）。見つけた違い: Rumoca のメッセージの「MLS §18.6」は Graphical Objects で、Evaluate は §18.3（Annotation 18.1）。`getInstanceName` は §3.7.4.6（依頼の §3.7.5 ではない）。
- 5 コードの本数の合計 1,115 / 1,781（最初の診断で数えた本数）。ER070 の IBPSA 27 本は 258 − 173 − 58 の引き算（表に直接の数字は無い）。
- コードの検査（fmt / clippy / test / deny）: 製品コードを変えていないので回していない（親が念のため `mise run check`）。
- `check_docs.py`: 下のコミット前に実行（結果は親への報告に書く）。
- セッション記録の `--check`・漏洩検査: 親が行う（セッション記録の写しは親）。

## 残したこと

- 下書きは Rumoca の main（0.10.3 以降）で確かめてから出す（ユーザー）。出したら README に URL と日付を足す。
- EF025 の replaceable な Medium の関数（約 85 本）の再現。ER132 を避けて IBPSA の呼び出しを調べる必要がある。
- ER023・ER003（ClaRa）、EI006、ED019、ED008 の判定。
- 下書き 17 の「`builtin_def_ids` が組み込みの関数を含まない」は読んだだけの推測（デバッガで確かめていない）。19・20・21 の Rumoca の箇所も同じ。本文に [inference] と書いた。
- `render_slides.py` は親が回す。
