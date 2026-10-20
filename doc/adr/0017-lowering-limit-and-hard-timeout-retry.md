# ADR-0017 MSL の実行では lowering の段に別の上限を掛け、外から止めたものも回し直す

- 状態: Accepted（2026-10-20、S015）
- 関係: [ADR-0013](0013-exec-backend.md) の「時間切れは外から止める」と、S012 の時間切れの回し直しを広げる。どちらも覆さない。製品の実行系（`globe-mode-exec`）は変えない

## 背景

- `cargo xtask msl-sim` は、1 本の応答を 120 s（`--kill-seconds`）待ち、無ければワーカーを止める。回し直すのは協調型の時間切れ（`GM-E11005`）だけで、外から止めたものは回し直さなかった。
- S014 で段を知らせるようにしたところ、S013 で外から止めた 7 本は 7 本とも lowering（Rumoca の構造解析と Solve IR への変換）の段で 120 s を使い切っていた。lowering は Rumoca の協調型の予算の外にある。
- 120 s 近くかかるモデル（`LightningSegmentedTransmissionLine`、`FullRobot`）は、機械の負荷で外からの停止に変わる。S014 の 2 回目の全例題の実行では、これで実行できた本数が 1 本減った。16 論理コア（8 コア × 2）の機械で負荷の平均が 10〜12 だと、1 本の時間が 2 倍近くになる。ワーカーの CPU を使うのはほぼ 1 スレッドで、CPU 時間も同じだけ延びる（ハイパースレッドの相手と取り合うため）ので、CPU 時間で測っても揺れは消えない。
- Rumoca 自身の MSL のハーネスは、段ごとに別の上限を掛ける（lowering の段 `ir_solve*` は `--solve-timeout-seconds`、既定 10 s。`crates/rumoca-test-msl/src/bin/rumoca-sim-worker.rs:63-66,700-712`、rev 276e82bd、2026-10-20 確認）。

## 決定

### (a) lowering の段に別の上限（`--lowering-kill-seconds`、既定 150 s）

- 親は、ワーカーが lowering の段に入ってから 150 s で次の段に進まなければ、ワーカーを止める。報告の文言は「lowering が 150 s で終わらないので止めた」。1 回目は全体の 120 s が先に来るので、この上限が効くのは主に (b) の回し直し（240 s）の中。
- 150 s にしたのは、終わるものの lowering が長く、負荷で 2 倍近くになるため。S013 で外から止めた 7 本を、空いた機械（負荷の平均 0.1〜4）で上限 600 s にして回した（xtask は S014 のもの）。

  | 例題 | 600 s での結果 |
  | --- | --- |
  | `MomentumBalanceFittings`・`MeasuringTemperature`・`NonCircularPipes` | lowering が 600 s で終わらない（CPU 時間も壁時計とほぼ同じで、待っているのではなく計算し続けている） |
  | `DrumBoiler` | lowering は約 117 s で終わり、S10 のプリフライト（t0 の評価）が 481 s で終わらない |
  | `InverseParameterization`・`EngineV6_analytic` | lowering が約 520 s で終わり、`GM-U0004`（EL005。構造解析・Solve IR の変換で計算できない） |
  | `EngineV6` | 終了時刻まで 169 s（準備（lowering・プリフライト・組み立て）95 s、積分 58 s）。管は 2 / 4 |

  S015 の全例題の実行（空いた機械）で測った段ごとの秒では、終わるもので lowering が最も長いのは `EngineV6` の 64 s（ほかに `FullRobot` 44 s、`Engine1b_analytic` 14 s）。`DrumBoiler` の lowering も 117 s で終わる（その後のプリフライトで止まる）。S014 の負荷の下では `FullRobot` の lowering が約 118 s だった。150 s は `EngineV6` の約 2.3 倍。
- 120 s より短い上限で「早く見切る」ことはしない。終わるものの lowering（負荷の下の `FullRobot` の約 118 s、`DrumBoiler` の 117 s）と、終わらないものの区別が 120 s の内側ではつかないため。
- 結果の分類は「外から停止」のまま（`GM-U` の診断にはしない）。`GM-*` は製品の診断で、製品の実行系には lowering の上限が無い（止めるのは利用者）。報告の「外から止めたモデルの段」で lowering の段として数える。600 s で 2 本が `GM-U0004` で終わったので、lowering が終わらないものは未対応（上流の EL005 の類）の見込みが高い。

### (b) 外から止めたもの（lowering の段の上限で止めたものを除く）も回し直す

- S012 の回し直し（予算 180 s、外から止めるのは 240 s）の対象に、外から止めたものを足す。負荷で 120 s を超えたもの（`LightningSegmentedTransmissionLine` など）と、全体で 140〜170 s かかる `EngineV6` を拾う。
- lowering の段に上限の秒数以上いて止めたものは回し直さない。既定の 150 s では 1 回目に上限が効かないので、1 回目に外から止めたものはすべて回し直し、回し直しの中で lowering の段の上限で止める（240 s を待たない）。
- その分、lowering が終わらない例題は 1 本あたり約 275 s（1 回目 120 s + 回し直し約 155 s）かかり、S014 の 120 s より長い。「早く見切る」より、終わるものを落とさないことを選んだ。

### (c) 段ごとの秒を記録する

- 親は段の知らせの間隔で段ごとの秒を測り、JSON の `stage_times` に残す（同じ段に 2 度入ったら足す）。報告に「lowering の段の長いもの」の表（15 行）を足し、次に上限を決める材料にする。

## 結果

- MSL の全例題の実行（[msl-sim.md](../msl-sim.md)、2026-10-20、空いた機械で負荷の平均 1.1〜2.3）: 356 本のうち **309 本**が実行でき、**301 / 308 本**（97.7%）が管に収まった（S014 は 307 本・300 / 306 本）。
  - 増えた 2 本: `EngineV6`（回し直しで 139 s。lowering 64 s。管は 2 / 4 で外れ）と `LightningSegmentedTransmissionLine`（1 回目で終わった。S014 は負荷で外から停止）。`FullRobot` は S013 と同じ `GM-E11001`（初期化の失敗）に戻った。
  - 外から止めたのは 6 本: lowering の段の上限（150 s）で 5 本、`DrumBoiler` は回し直しで lowering が 117 s で終わり、S10 のプリフライトで 240 s に届いた。
  - 回し直しは 12 本（時間切れ 5 本・外から停止 7 本）。実行できたのは `EngineV6` の 1 本。
  - 全体の時間は 3733 s（S014 は負荷の下で 3627 s）。lowering が終わらない 5 本は回し直しで 1 本あたり約 155 s 余分にかかる。
- **S014 の「7 本とも lowering の段」は 120 s の時点の見え方で、`DrumBoiler` は lowering の後のプリフライト（GlobeMode の t0 の評価）で止まる**ことが分かった（600 s では段に入ってから 481 s）。プリフライトは警告しか出さないので、GlobeMode の側で上限を掛けられる。S016 の候補にした。
- 製品の実行系（`globe-mode-exec`）は変えていない。上限と回し直しは `cargo xtask msl-sim` の中だけ。
