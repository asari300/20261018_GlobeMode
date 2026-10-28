# S026 `implicit-fixed-step`

| | |
| --- | --- |
| 日付 | 2026-10-28 |
| ブランチ | `feature/s026-implicit-fixed-step` |
| worktree | `.git-worktree/s026-implicit-fixed-step` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S026-implicit-fixed-step/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「進めて。サブエージェントは Sonnet/high」。ロードマップの候補から次を選んだ。

## 決めたこと

- 候補 1 のうち管から外れた 3 本（`RotationalSample`・`LogicalSample`・`Dimmer_R`）は、すでに「直せない」と分類されていたので選ばなかった。候補 6（残りの計画のソルバ）から、固定刻みの陰的な `impeuler`・`trapezoid` を選んだ。

- 道はサブエージェント（Sonnet・high）の調べで、既存の固定刻みのプラグイン（`plugins/fixed.rs`）に θ 法を足すことにした。diffsol の SDIRK を自前の係数表と固定刻みで使う道は、刻みの調整を外せるかを確かめていない（そちらは使わなかった）。
- Newton 法のヤコビ行列は部品の方向微分から密に作る（依存を足さない）。状態が数百の系では遅いので `shortcut:` を書いた。
- `msl:sim` は既定の `auto` だけを回すので、新しいソルバは通らない。`fixed.rs` の陽的な道は形を変えただけなので回していない（既定のまま進めた判断）。

## やったこと

- `globe-mode-ir/src/sim.rs`: `SolverId::{Impeuler, Trapezoid}`。固定刻みに含めた。
- `globe-mode-sim/src/integrators.rs`: `ThetaMethod`（`IMPEULER`・`TRAPEZOID`）、`ThetaStepper`（Newton 法、上限 20 回、修正量 ≤ 1e-12·(1+|y|)）、密な LU（`solve_dense`）と単体の試験 3 本。
- `globe-mode-sim/src/{solvers,settings}.rs`: 2 つを実装済みにし、試験の一覧を直した。
- `globe-mode-exec/src/rumoca/plugins{.rs,/fixed.rs}`: 固定刻みの方法を `Method`（陽的・θ 法）にした。Newton 法が解けなければ `AdvanceExhausted` で失敗にする。
- `globe-mode-exec/tests/integrators.rs`: 次数の試験に 2 つを足し、`Decay` の閉じた形との一致、硬い減衰（k = 1000、hλ = -100）での安定を足した。
- `globe-mode-cli/src/main.rs`: ヘルプ。文書は `architecture.md`・`features.md`・`roadmap.md`・ADR-0014 の注。

## 検証

- `Decay` の x(1) の誤差: `impeuler` 1.77e-2（h = 0.1）・9.01e-3（h = 0.05）で次数 0.97、`trapezoid` 3.07e-4・7.67e-5 で次数 2.00。硬い減衰で増幅率（1/101、-49/51）と 1e-13 以内で一致し、陽的 Euler は発散。
- コードの検査（`mise run check`、fmt / clippy / test / deny / `uv sync --locked`）: 498 本通過・失敗 0・無視 10（S025 は 493 本）
- `wasm:node`: ok。`wasm:compare`: 198 件中 195 件が一致、既知の差 3、不一致 0（S025 は 162 件。新しいソルバ 2 つの分が増えた）
- `msl:compare`・`msl:funnel`・`msl:sim`: 回していない（frontend は変えていない。`msl:sim` は既定の `auto` だけで、新しいソルバを通らない）
- `check_docs.py`: 文書 144 件 / 相対リンク 884 件 / 切れ 0 件。`render_slides.py`: デッキ 32 件 / 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 密なヤコビ行列と毎回の作り直し（状態が数百の MSL の回路では遅い）。疎の形（`state_jacobian_columns`）と LU の使い回しは必要になってから。
- Newton 法の失敗は単体の試験だけで、`run()` から GM-E11002 として出ることは確かめていない。
- `state_jacobian_columns()` が無い部品では、方向微分の拒否が `DerivativeRefused` として出る（`esdirk34`・`trbdf2` の道と文言が違う）。
- MSL の例題では回していない。残りの計画のソルバ（`radau5`・`esdirk23` など）。
