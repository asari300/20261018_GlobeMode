# S107 `features-libraries-sync`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s107-features-libraries-sync` |
| worktree | `.git-worktree/s107-features-libraries-sync` |
| Phase | P5・P6（文書だけ） |
| セッション文書 | `doc/sessions/S107-features-libraries-sync/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

親のセッションからの指示（サブエージェント）。正典の文書を最近のセッションに追いつかせる。features.md を S095〜S103 に合わせる、libraries.md を run 41365960542 の artifact `libraries` で写し直して前回との差を解説書に表で書く、README に S094〜S103 の成果が無ければ英・日 1 行ずつ。roadmap.md・コードは触らない。cargo は回さない。main へのマージ・push・worktree の削除は親。

## 決めたこと

- features.md は既存の 8 行に書き足し、新しい行は足さない（自前の FMU ランタイムは FMI 2.0 Export の P5b に当たる）。
- S099（`fmu-exec-api`）は main の索引に無い（未取り込み）ので書かない。S104〜S106 も同じ。
- libraries.md は artifact をそのまま写し、末尾の手書きの「元の run」の行だけを今回の run に書き換えて残した。
- README は状態の段落の末尾に 1 文（英・日）と、「今後必要になりそうな機能」の FMI の行（P5 の準備中 → 書き出しは済、読み込みは未）。

## やったこと

- `doc/features.md`: 8 行（connect グラフ S101、FMI 2.0 Export S095、FMI 3.0 Export S098、Android S092・S100・S103、ブラウザ S093、marimo S078・S097・S102、FMPy・fmusim S094・S098、Rumoca との照合 S096）。
- `doc/libraries.md`: run 41365960542（schedule、2026-11-02 00:13 UTC、`a778ea92`、success）の artifact を写した。
- `README.md`: 3 行（英の状態、日の状態、FMI の行）。
- `doc/sessions/S107-features-libraries-sync/`、索引の行、この作業記録。

## 検証

- コードの検査（fmt / clippy / test / deny）: 回していない（文書だけ、指示で cargo は回さない）
- libraries.md の前回との差: 7 ライブラリとも例題・OK・NG・GM-U・panic・実行した・実行できた・版の差 0。時間の列だけ違う（Buildings の検査 1:13 → 1:04 など）
- `check_docs.py`: デッキはすべて OK / リンク 2117 件中の切れ 0 件
- `render_slides.py`: デッキ 1 件 / 問題 0 件（ライト・ダーク・スマートフォン 7 枚ずつ。高負荷で 30 s の時間切れが 5 回、6 回目で通った。main の `.venv` の Python で回した。この worktree の `uv sync` は maturin で cargo を回すので止めた）
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件

## 残したこと

- S099・S104〜S106 が main に入ったら、features.md の FMI・Android（release の APK）・marimo（WASM で MSL）の行を直す。
- features.md の P4 の行（ライブラリツリー・プロットなど）は S094 より前の成果も書かれていない行がある（範囲の外）。
