# S047 `msl-reports-ci`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s047-msl-reports-ci` |
| worktree | `.git-worktree/s047-msl-reports-ci` |
| Phase | P3（S037・S043 の続き） |
| セッション文書 | `doc/sessions/S047-msl-reports-ci/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「続けて作業せよ」（S043 と同じ依頼の続き）。S043 で残した、GitHub Actions の結果の取り込み。

## 決めたこと

- 報告は 2 回目の run 41174276501 の artifact を使う（1 回目 run 41170414005 の JSON がキャッシュから戻り、前回との差の節が載るため）。
- 手元では回し直さない（ランナーで 2 回回して一致。手元は RAM 3 GB）。

## やったこと

- `doc/msl-funnel.md`・`doc/msl-sim.md` を artifact の版に差し替えた（`xtask` が生成したもの。手で編集していない）。ランナーのパスは含まれていない（grep で 0 件）。
- roadmap の S047 の節、CLAUDE.md の `msl:sim` の行に `split-all` を一文、セッション文書、索引。

## 検証

- run 41170414005（msl-sim ジョブ 04:54〜06:18 UTC、85 分）: 成功。
- run 41174276501: msl-sim ジョブ 79 分・split-all ジョブ 61 分、どちらも成功。
- funnel: 566 本: OK 369・NG 13・GM-U 184・panic 0、S6 の Rumoca との照合 381/381（2026-10-24 の手元と同じ）。検査 743.6 s、最大 RSS 7.2 GiB。
- sim: 356 本: 実行できた 309・管に収まった 301/308（97.7%）、panic 0（手元と同じ）。回し直し 13 → 12 本（実行できた 2 → 1、管に収まった 1 → 0）。lowering の段で止まった 6 → 5 本、S10 のプリフライトで止まった 0 → 1 本。2 回の run の間の差は 0。
- split-all: 566/566 で全部読みと分割読み込みの `CheckReport` が一致（`GM_SPLIT_FRESH=1`、2931.69 s）。

- `check_docs.py`: 文書 226 件 / 相対リンク 1152 件 / 切れ 0 件。
- `render_slides.py`: 手元では通っていない。Chromium のスクリーンショットが最小のページでも 30 s で時間切れになる（S043 の 14 時台は通っていた。`--disable-gpu`・`channel='chromium'`・`--use-gl=swiftshader` でも安定しない）。ユーザーの判断で、CI の `docs` ジョブ（同じ `render_slides.py`）に任せた。

## 残したこと

- 手元の Chromium が止まる原因（WSL の GPU かメモリか）は突き止めていない。

- 回し直しが 1 本減ったのは、`Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine` がランナーでは 1 回目（予算 30 s・応答 120 s）で終わったため（手元では応答の上限で外から止め、回し直しで実行できていた）。実行できた本数 309 は変わらない。
- `msl:omc` は回していない（ランナーに omc が無い）。
