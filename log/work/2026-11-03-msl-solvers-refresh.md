# S126 `msl-solvers-refresh`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s126-msl-solvers-refresh` |
| worktree | `.git-worktree/s126-msl-solvers-refresh` |
| Phase | P3・P8 |
| セッション文書 | `doc/sessions/S126-msl-solvers-refresh/` |
| セッション記録 | （親が写す） |

## 依頼

S119 が main に入った後のソルバの表 `doc/msl-solvers.md` と `doc/msl-sim.md` を、`msl-sim.yml` の全体の run（既定の全ソルバ）で更新する。前回（S050 の表、S119 の msl-sim.md）との差、ThyristorBridge 2 本の行を解説書に書く。2mPulse が時間切れに戻っても扱いは変えず事実として書く。Rust は触らず、cargo も手元の msl:sim も回さない。マージ・push・worktree の削除は親。

## 決めたこと

- `msl-sim.yml` の concurrency は `group: msl-sim`・`cancel-in-progress: false`。S119 の esdirk34 だけの run 41589277425 が `solvers (esdirk34)` を回していたので、終わるのを待ってから `gh workflow run msl-sim.yml --ref main` を出した（待機中の 2 つ目は cancelled になるため）。
- 出したときの main は `97be9cc`（S119 のあとに S123・S130 も入った）。依頼は「S119 の後」なので、そのまま main の先頭で回した。
- artifact は全文が生成物で、手で書いた前書きが無いので、そのまま写した（写す前にリンクとスクリプトの有無を見た）。
- 表の数だけでは S050 からの差の中身がわからないので、ソルバごとの artifact の JSON を S050 の run と突き合わせた（`/tmp` の使い捨てのスクリプト。コミットしない）。
- SDIRK の新しい失敗は直さず、【未確認】として持ち越した（依頼の範囲外）。

## やったこと

- `doc/msl-solvers.md`・`doc/msl-sim.md`: run 41616928442 の artifact `msl-solvers`・`msl-sim` に置き換えた。
- `doc/sessions/S126-msl-solvers-refresh/`（slides.html・解説書.md）、`doc/sessions/README.md` の索引の行、`doc/roadmap.md` の「S126 で済んだこと」。

## 検証

- run 41616928442: 全 17 ジョブ success（`msl-sim` 10:09〜11:29 UTC、`solvers` 11:29〜14:29 UTC）。
- `auto`: 実行できた 312・管 303 / 311（S119 と同じ）。356 本の判定・止まった理由・ソルバの段・管は S119 の報告と全部同じ。回し直し 15 → 14 本。全体 4113.8 → 3721.4 s。
- `ThyristorBridge2mPulse_DC_Drive`: 完走、積分 125.64 s（S119 の run は 177.14 s）。`ThyristorBridge2Pulse_DC_Drive`: 完走、積分 46.20 s（63.42 s）。
- ソルバの表（S050 → S126）: `auto` 309 → 312、`dopri45` 292 → 293、`sdirk34hw` 294 → 292、`tsit45` 287 → 285、`cerk23` 288 → 287、`cvode` 294 → 293、ほか 8 本は実行できた本数が同じ。
- esdirk34 は S119 の run の `msl-sim-solver-esdirk34` と全モデルの判定が同じ。
- コードの検査（fmt / clippy / test / deny）: Rust を触っていないので回していない。
- `check_docs.py`: デッキ OK（S126 を含め NG 0）/ リンク 2,672 件中の切れ 0 件
- `render_slides.py`: 手元では通らず（Page.screenshot が 30 s で時間切れ 2 回。別の WP の cargo のビルド中で、S119 のデッキでも同じ）、CI の docs で確認
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）

## 残したこと

- session の道の SDIRK で `HBridge_TrianglePWM_RL`・`HBridge_DC_Drive` が `GM-E11002`、`sdirk34hw` で `ThyristorBridge2Pulse_DC_Drive` が `GM-E11003` に変わった。S119 の前の版で SDIRK を回して原因を確かめる。
- `ThyristorBridge2mPulse_DC_Drive` は予算 180 s の境目のまま（ランナーで 125〜177 s）。
