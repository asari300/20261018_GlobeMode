# S027 `sdirk-tableaus`

| | |
| --- | --- |
| 日付 | 2026-10-29 |
| ブランチ | `feature/s027-sdirk-tableaus` |
| worktree | `.git-worktree/s027-sdirk-tableaus` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S027-sdirk-tableaus/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「進めて。サブエージェントは Sonnet/high」。ロードマップの候補 6（残りの計画のソルバ）の続きとして、diffsol の SDIRK に自前の係数表を差す `esdirk23`・`esdirk45`・`sdirk34hw` を選んだ。

## 決めたこと

- 道: 係数表を `globe-mode-sim::sdirk` に置き、`plugins/rk.rs` で diffsol の `Tableau::new` にして、`esdirk34`・`trbdf2` と同じ SDIRK の道に渡す（サブエージェント Sonnet・high の調べ）。
- 係数の出典（2026-10-29 確認）: `esdirk23` は Jørgensen ら（arXiv:1803.01613 §3.2）。diffsol の `tr_bdf2` と同じ方法で、終点の値は `trbdf2` と 12 桁以上一致する（名前の受け口として残した）。`esdirk45` は Kennedy–Carpenter 2019 の ESDIRK5(4)7L[2]SA2 で、値は OrdinaryDiffEq.jl の `ESDIRK547L2SA2Tableau`（MIT）から写した。写し元の `c₄ = 49/453` は行の和（49/353）と合わないので誤記と判断して 49/353 にした。NASA/TM-2016-219173 の表 25 は c₆ > 1 の別の版で使わなかった。`sdirk34hw` は Hairer–Wanner II の SDIRK4（OrdinaryDiffEq.jl の `Hairer4Tableau` と照合）。論文と本の表そのものは見ていない【未確認】。
- `sdirk34hw` は補間の係数（beta）も渡す。diffsol の既定の Hermite 補間は左の傾きを段 0 から取り、ESDIRK でしか正しくない（渡す前は BouncingBall の h(2) が 0.157、正しくは 0.2607）。
- 写し間違いの守り: 根つき木で 6 次までの次数条件を数値で確かめる単体の試験（Euler を 1 次、RK4 を 4 次と判定する自己試験つき）。
- `wasm:compare` の BouncingBall の 3 件（新しいソルバ）は、既存の既知の差（esdirk34・trbdf2・tsit45）と同じ道・同じ所で、成功と診断のコードは一致するので、既知の差に足した（既定のまま進めた判断）。

## やったこと

- `globe-mode-sim/src/sdirk.rs`（新規）: 3 つの係数表と次数条件の試験 8 本。`lib.rs`・`solvers.rs`・`settings.rs`。
- `globe-mode-ir/src/sim.rs`: `SolverId` の 3 つ。
- `globe-mode-exec/src/rumoca/plugins/rk.rs`: `Method::Custom` と `custom_tableau`。`plugins.rs`・`session.rs`、`tests/integrators.rs`（適応刻みの一覧と BouncingBall に足した）。
- `globe-mode-cli/src/main.rs`: ヘルプ。`globe-mode-wasm/node/compare.mjs`: 既知の差に 3 件。
- 文書: `architecture.md`（表と出典の注）、`features.md`、`roadmap.md`。

## 検証

- 試験のモデル: `Decay` x(1) の誤差 esdirk23 3.2e-7・esdirk45 1.1e-10・sdirk34hw 4.9e-8、`Oscillator` 2.0e-6・1.4e-9・4.3e-10、`BouncingBall` h(2) はどれも 0.26074173（bdf と同じ）。3 つとも `TinyStateRestart` も通る。
- コードの検査（`mise run check`）: 506 本通過・失敗 0・無視 10（S026 は 498 本）
- `wasm:node`: ok。`wasm:compare`: 252 件中 246 件が一致、既知の差 6、不一致 0（足す前は不一致 3。上の判断）
- `msl:compare`・`msl:funnel`・`msl:sim`: 回していない（frontend は変えていない。`msl:sim` は既定の `auto` だけで、新しいソルバを通らない）
- `check_docs.py`: 文書 147 件 / 相対リンク 890 件 / 切れ 0 件。`render_slides.py`: デッキ 33 件 / 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- `esdirk45` の刻みの間の値は 3 次の Hermite 補間（終点は 5 次）。
- 約 60 個の分数は 1 つずつ読み直していない（次数条件の試験に任せた）。
- MSL の例題では回していない。残りの計画のソルバ（`radau5`・`dopri853`・`ida`・`cvode`・`cerk*`・`qss`）。
- サブエージェントが 1 回、main のチェックアウトの `target/` でビルドした（4.7 GB、`.gitignore` 済み。main の状態はきれいなまま）。
