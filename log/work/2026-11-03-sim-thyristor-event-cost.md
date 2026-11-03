# S119 `sim-thyristor-event-cost`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s119-sim-thyristor-event-cost` |
| worktree | `.git-worktree/s119-sim-thyristor-event-cost` |
| Phase | P3（実行系） |
| セッション文書 | `doc/sessions/S119-sim-thyristor-event-cost/` |
| セッション記録 | （親が写す） |

## 依頼

S088 の持ち越し。`ThyristorBridge2mPulse_DC_Drive`（と 2Pulse）が `auto` で時間切れ（`GM-E11005`）になる原因、事象 1 回あたりの esdirk34 のやり直しの費用を測って軽くする。まず「事象の処理 / 事象の後の最初の刻み / 通常の刻み」の内訳を出し、最小の対策を 1 つ入れ、数値が変わらないこと（既知の外れと `wasm:compare` の一致数）を確かめ、全例題は `msl-sim.yml` で回す。新しいソルバ・diffsol の patch・Rumoca の改変はしない。main へのマージ・push・worktree の削除は親がする。

## 決めたこと

- 計測は `globe-mode-exec/src/rumoca/plugins/rk.rs` に一時的に入れた（`GM_PROF` のときだけ `eprintln`、回数と `Instant`）。コミットしていない。`globe-mode-sim` の `sdirk.rs`・`integrators.rs`・`solvers.rs` は diffsol の esdirk34 の道を通らないので変えていない。
- 測る区間は 0〜2 s。`--stop-time 0.5`・`1` は直す前から t = 0.0617 s で `GM-E11003` になる（持ち越し）。
- 陽的 RK（`tsit45`・`dopri853`・`cerk*`）には当てない。当てると `wasm:compare` の `DiodeSwitching/tsit45` が分かれた（下の検証）。対象のモデルは esdirk34 なので陽的 RK に当てる理由が無い。
- 対策は「事象のあとの最初の刻みを、直前に diffsol が次に試すつもりだった刻みで上から抑える」の 1 つ（`DiffsolRk::last_step` と `initial_state` の 1 つの `if`）。いちばん重い所（最初の刻み、実行の 28%）に効き、刻みを大きくする向きの変化が無いので選んだ。ヤコビ行列の使い回しは reset と init の合計で 7% しかないので選ばなかった。reset の省略は Rumoca の session の約束を読み替えるので持ち越した。
- ADR は ADR-0013 の「結果」に追記した（予算の規則の ADR-0017 は変えていない）。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: `last_step` を足し、`advance` で受理のたびに `state.h` を覚え、`rebuild`・`reset_in_place` の `initial_state` で Hairer の見積もりを上から抑える。
- `doc/adr/0013-exec-backend.md`・`doc/roadmap.md`・`doc/sessions/README.md`・`doc/msl-sim.md`（Actions の報告）・セッション文書。

## 検証

- 内訳（直す前、0〜2 s、3 回の中央値）: 実行 27.6 s = reset 1.45 s（5%、0.60 ms/回）+ initialize 0.43 s（2%）+ 事象の後の最初の刻み 7.74 s（28%、3,002 回、2.58 ms/回）+ 通常の刻み 10.20 s（37%、54,603 回、0.19 ms/回）+ Rumoca の host 約 7.8 s（28%）。
- 前後（同じバイナリで切り替え、交互に 5 回ずつ）: 誤差の検査の失敗 18,273 → 7,612、Newton 法の失敗 4,824 → 2,662、右辺の評価 439,753 → 379,957、ヤコビ行列の方向微分 36,075 → 35,065、受理した刻み 57,605 → 58,254。最初の刻みの時間 7.74 → 5.48 s、実行 27.6 → 25.6 s（中央値。負荷 5〜35 で揺れる）。状態は 2 s まで格子の上で相対 4e-7 以内。
- `cargo test -p globe-mode-sim -p globe-mode-exec`: 171 passed、3 ignored、失敗 0。
- `mise run wasm:node`: 通過。
- `mise run wasm:compare`: 387 / 400 一致、既知の差 13、一致しない 0（S114 と同じ）。最初は陽的 RK にも上限を当てていて 386 / 400・不一致 1（`DiodeSwitching/tsit45`、相対 1.3e-6。安定の限界の刻みで wasm とネイティブの丸めの差が事象を越えて残る）になったので、上限を SDIRK だけにした。
- `msl-sim.yml`: run 41589277425（`solvers=["esdirk34"]`、コミット `8789b84`）、`msl-sim`・`split-all` success。`ThyristorBridge2mPulse_DC_Drive` が回し直しの esdirk34 で終了時刻まで（積分 177.1 s / 予算 180 s、管 OK 38/38。前は 187.5 s で時間切れ）、`ThyristorBridge2Pulse_DC_Drive` も（63.4 s、管 OK 19/19。前は 197.4 s で時間切れ）。実行できた 310 → 312、管に収まった 301 / 309 → 303 / 311。ほかの 353 本の判定と管は変わらない。2mPulse は予算 180 s に対して 177.1 s。`solvers (esdirk34)` は報告の時点でまだ動いていた。
- コードの検査（`mise run check`）: `mise run check` は test_ci_plan・fmt・clippy が通り、`cargo test --workspace` は単体と結合の試験 46 組が通ったあと、`globe-mode` の doctest が E0460（共有 target で S118 が同時に `globe_mode_db` をビルドしていた）で落ちた。`globe-mode-db`・`globe-mode` を touch して `cargo test --workspace --doc`（17 組 ok、失敗 0）・`cargo deny check`（ok）・`uv sync --locked`（ok）を回し直して通った
- `check_docs.py`: デッキ 122 件 OK / 文書 510 件・相対リンク 2552 件中の切れ 0 件。`render_slides.py` 7 枚 × 3 で問題 0 件
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（main..HEAD のコードのコミットも 0 件）

## 残したこと

- reset の直後に Rumoca の session が `initialize` を呼ぶので、reset のヤコビ行列と LU（実行の 5%）は捨てられている。
- Rumoca の host の部分（根の探索・Event Mode・出力、約 28%）。
- `--stop-time 0.5`・`1` で t = 0.0617 s に `GM-E11003`（直す前から。2 以上では起きない）。
