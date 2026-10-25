# S020 `pc-heavy-checks`

| | |
| --- | --- |
| 日付 | 2026-10-24〜25 |
| ブランチ | `feature/s020-pc-heavy-checks` |
| worktree | `.git-worktree/s020-pc-heavy-checks` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S020-pc-heavy-checks/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md`（S016 の後半と同じセッション。`/clear` を挟む） |

## 依頼

RPi での S019 を pull したので、WSL で作業を続ける。S019 が PC に回した 4 つの検査（`msl:funnel`、`msl:sim`、`wasm:node`、`wasm:compare`）を回し、本数を記録する。確かめること: EngineV6 で GM-W10004（`filter.x = 0.066 → 563`）、MSL で GM-W10004 が 0 本、`LightningSegmentedTransmissionLine` のプリフライトが下がる。違えば ADR-0021 を直す。

## 決めたこと

- ユーザーと決めたこと:
  - `msl:sim` が Claude Code に 2 回止められた（セッションが待ち状態のときにメモリが少ないと背景のコマンドを殺す。Linux の OOM ではない）あと、「先に `wasm:node`・`wasm:compare` を回し、`msl:sim` は後にする」を選んだ。3 回目はユーザーが自分の端末で `nohup mise run msl:sim &> log/mise/msl-sim.log &` と回した（Claude Code は関与しないので止まらない）。
  - 1 回目はユーザーが main のチェックアウトで回して、`target/msl-funnel.json` が無くすぐ失敗した。main に `log/mise/msl-sim.log`（未追跡）と `data/msl/`（`.gitignore` 済み）が残っている。私は触っていない。
- 既定のまま進めたこと:
  - `log/mise/` を `.gitignore` に足した（ユーザーが置いた mise のログの置き場。コミットしない）。
  - ADR-0021 は直さない。EngineV6 は検出でき、MSL のほかの 355 本で GM-W10004 は出なかった。
  - 前回の `target/msl-sim.json` はこの PC に無い（RPi の target は mv で渡す決まり）ので、`msl:sim` の報告に前回との差は無い。数字は S015（PC）と S018（RPi）の作業記録と手で比べた。

## やったこと

- `doc/msl-funnel.md`・`doc/msl-sim.md`: 再生成（本数は変わらない。日時と秒数だけ）。
- `log/work/2026-10-23-s019-resettle-followups.md`: 「検証」に 4 つの検査の数字を足し、「残したこと」を消し込んだ。
- `.gitignore`: `log/mise/` を足した。
- セッション文書 `doc/sessions/S020-pc-heavy-checks/`、索引の行。Rust のコードは触っていない。

## 検証

数字で書く。通っていない項目は「通っていない」と書く。

- `mise run msl:funnel`（release、WSL: RAM 3 GB + swap 16 GB、09:25〜10:13 の 48.5 分、取得とビルドを含む）: **566 本: OK 369・NG 13・GM-U 184・panic 0**。S019（RPi）と同じ本数。`/usr/bin/time` の最大 RSS 2.1 GiB（RAM の上限で swap に出た分は含まない）。
- `mise run msl:sim`: 1 回目は 249 / 356 本まで進んだところで、Claude Code がメモリ不足のために止めた（10:13 に開始、DrumBoiler と MeasuringTemperature が lowering で 120 s の上限に当たった直後）。2 回目（16:27 開始）は 347 / 356 本まで進んで、また同じ理由で止められた（回し直しの段に入る前）。
- `mise run msl:sim` の 3 回目（ユーザーの端末、2026-10-24 23:01 UTC 開始、全体 4132 s = 69 分）: **356 本: 実行できた 309・管に収まった 301 / 308（97.7%）・実行 NG 41（うち時間切れ 4）・外から停止 6・panic 0**。S015（PC、2026-10-20）と同じ本数。回し直し 13 本のうち実行できた 2・管に収まった 1。P3 の完了条件は満たす。
  - **GM-W10004 は 1 本**: `EngineV6`（回し直しで終了時刻まで、管 2 / 4 で `filter.x[1]`・`filter.x[2]` が t = 0 から外れ、はみ出し 562）。S018 の RPi では EngineV6 が外から止まって 0 本だったが、PC では報告に入った。ほかの 355 本では 0 本。
  - プリフライトの段の合計 45.2 s（356 本、中央値 0.007 s）。最大は `LightningSegmentedTransmissionLine` 15.1 s、次いで `EngineV6` 10.3 s、`Engine1b_analytic` 1.7 s。S018 の RPi（合計 98.5 s、Lightning 33.6 s）の約半分だが、PC は RPi の約 2 倍速いので、ADR-0021 で費用が下がったかはこの数字からは言えない。Lightning は関係式の記憶が変わるので解き直しは省かれていない。
  - `LightningSegmentedTransmissionLine` は 1 回目は 120 s の応答の上限で止まり、回し直しで 105 s で終了時刻まで（管 3 / 3）。`EngineV6_analytic` と Fluid の 5 本は lowering の 150 s の上限（S015 と同じ）。
- `mise run wasm:node`（16:11〜16:22、ビルド込み 635.6 s）: 通った。Decay を bdf・dopri45・auto で（誤差 2.7e-8 / 1.9e-9 / 2.7e-8）、MSL の ChuaCircuit を bdf で（4599 ms、メモリ 513 MiB）、panic の文言も読めた。wasm 14.5 MB。
- `mise run wasm:compare`（70.4 s）: **126 件中 123 件が一致、既知の差 3 件、一致しない 0 件**。`InitRelationBranch` は 9 ソルバすべてで wasm とネイティブの両方が GM-W10004 を出す。
- **EngineV6 単体**（`globe simulate -L data/msl -m Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6 --stop-time 0.01 --tolerance 1e-7`、WSL で準備 103.2 s・実行 2.7 s、全体 165 s、最大 RSS 1.9 GiB）: ADR-0021 の条件（関係式の記憶が変わったときだけ解き直す）でも **GM-W10004 が出た**: `filter.x[1] = 0.065992 → 563.186238`、`filter.x[2]`、`filter.y`、`filteredEngineTorque`、`$state_coordinates[3]`・`[4]`、`$formal_derivative.1.filter.x[1] = 27487.73 → 0`（計 7 個）。S018 の値と同じ。
- コードの検査（fmt / clippy / test / deny）: Rust のコードは触っていない。
- `check_docs.py`: デッキ全件 OK、相対リンク 818 件中の切れ 0 件。`render_slides.py`: デッキ 26 件、問題 0 件
- セッション記録の `--check`: 9 件を検査、取りこぼし 0 件
- ステージした差分の漏洩検査: （finish-session で埋める）

## 残したこと

- `msl:sim` を Claude Code の背景で回すと、セッションが待ち状態のときにメモリ不足で止められる（2 回）。この PC（RAM 3 GB）で回すときは、ユーザーの端末で `nohup` で回すか、`CLAUDE_CODE_DISABLE_BG_SHELL_PRESSURE_REAP=1` を付けて Claude Code を起こす。
- main に残ったユーザーの `log/mise/msl-sim.log`（未追跡）は、マージの前に消すか移す。
- ADR-0021 の費用の効果（Lightning のプリフライト）は、同じ機械での前後の比較が無いので未確認。確かめるなら S017 の rev と S019 の rev を同じ PC で回して比べる。
- `msl:omc` は omc が無いので回していない。
