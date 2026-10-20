# S015 `lowering-limit`

| | |
| --- | --- |
| 日付 | 2026-10-20 |
| ブランチ | `feature/s015-lowering-limit` |
| worktree | `.git-worktree/s015-lowering-limit` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S015-lowering-limit/` |
| セッション記録 | `log/sessions/2026-10-19-8faf1abd-17ac-42de-b09c-7ff1bc0bc15d.md`（S013 の後半・S014 と同じセッション） |

## 依頼

「進めて」（S014 の締めの後）。S014 の報告で次に挙げた 2 つ（lowering が終わらない 7 本を早く見切る、120 s 近いモデルが負荷で揺れる）を自分で選んだ。

## 決めたこと

既定のまま進めた判断（ユーザーと決めたことは無い）。設計は [ADR-0017](../../doc/adr/0017-lowering-limit-and-hard-timeout-retry.md)。

- CPU 時間で止める案はやめた。ワーカーは 17 スレッドのうち 1 本だけが CPU を使い、16 論理コアは 8 コア × 2 のハイパースレッドなので、負荷で CPU 時間も延びる。
- 上限を決める前に、外から止めた 7 本を上限 600 s で 1 本ずつ回した（空いた機械）。3 本は lowering が終わらず、2 本は約 520 s で `GM-U0004`（EL005）、`EngineV6` は 169 s で終わり、`DrumBoiler` は lowering が約 117 s で終わってプリフライトが 481 s で終わらない。
- lowering の段の上限は最初 90 s にしかけたが、`EngineV6` を見て 150 s にした（全例題の実行を途中で止めて回し直した）。終わるものの lowering は空いた機械で最長 64 s（`EngineV6`）、S014 の負荷の下では `FullRobot` が約 118 s。120 s より短い上限では終わるものと終わらないものを分けられないので、「早く見切る」はしなかった。
- 外から止めたもの（lowering の段の上限で止めたものを除く）を、時間切れと同じく予算 180 s で回し直す。150 s は 120 s より長いので、1 回目で外から止めたものはすべて回し直し、回し直しの中で lowering の上限が効く。lowering が終わらない例題は 1 本あたり約 155 s 余分にかかる。
- 分類は「外から停止」のまま。`GM-U` の診断（製品のコード）にはしない。
- `mise.toml` の `msl:sim` の説明は直さなかった（S012 の記述として正しく、ツールの定義の変更は CI の全体検査になるため）。

## やったこと

- xtask の `sim.rs`: `request` に lowering の段の上限（`lowering_kill`）と段ごとの秒、`Record::stage_times`、`timed_out` に外から止めたもの（`lowering_cut` を除く）、報告の文言と「lowering の段の長いもの」の表。テストに偽のワーカーで lowering の上限が先に効くこと、回し直しの対象の判定を足した。
- xtask の `main.rs`: `--lowering-kill-seconds`（既定 150）。
- 文書: ADR-0017（新規）、ADR の索引、architecture.md と CLAUDE.md の `msl:sim` の記述、roadmap.md（S015 の節、S016 の候補 4 を「プリフライトが終わらない例題」に）、research/msl-sim-failures.md（節「S015 で調べたこと」）、doc/msl-sim.md（回し直し）。

## 検証

- `mise run check`: 1 回で通った（テスト 487 本成功・10 本は ignore、cargo deny は advisories・bans・licenses・sources ok）
- xtask のテスト: 41 本成功
- `msl:sim`（2026-10-20 12:06〜13:08、負荷の平均 1.1〜2.3）: 356 本のうち 309 本が実行でき（S014 から +2）、301 / 308 本（97.7%）が管に収まった（+1）。外から停止 9 → 6 本（lowering の段の上限で 5 本、`DrumBoiler` はプリフライトで 240 s）。回し直し 12 本のうち実行できたのは `EngineV6` の 1 本（管は 2 / 4 で外れ）。`LightningSegmentedTransmissionLine` は 1 回目で終わった（88 s）。`FullRobot` は S013 と同じ `GM-E11001`。全体 3733 s（S014 は負荷の下で 3627 s）。
- 7 本の 600 s の測り（S014 の xtask、`/usr/bin/time`）: [調査の記録](../../doc/research/msl-sim-failures.md) の節「S015 で調べたこと」の表のとおり。
- `check_docs.py`: デッキ OK、文書 106 件・相対リンク 727 件中の切れ 0 件。`render_slides.py`: 7 枚 × 3 表示で問題 0 件（スクリーンショットを目で見た）
- セッション記録の `--check`: 6 件中、取りこぼしのあるファイル 0 件
- ステージした差分の漏洩検査: 0 件（main と origin/main の差分も 0 件）

## 残したこと

- `DrumBoiler` のプリフライト（`SolveRuntime::eval_at` の t0 の評価）が 481 s 以上終わらない。原因は調べていない。プリフライトは警告しか出さないので、上限を掛けられる（S016 の候補 4）。
- `EngineV6` が新しく管から外れた（`filter.x[1]`・`filter.x[2]` が t = 0 から、はみ出し 562）。既知の外れの一覧には入れていない。
- lowering が終わらない 5 本は上流（EL005 の類）。msl:sim は 1 本あたり約 155 s 長くなった。
- 1 回目の上限 120 s と lowering の上限 150 s はどちらも壁時計なので、負荷の高い機械では引き続き揺れる（回し直しで拾える分だけ減った）。
