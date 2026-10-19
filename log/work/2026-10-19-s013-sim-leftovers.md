# S013 `sim-leftovers`

| | |
| --- | --- |
| 日付 | 2026-10-19 |
| ブランチ | `feature/s013-sim-leftovers` |
| worktree | `.git-worktree/s013-sim-leftovers` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S013-sim-leftovers/` |
| セッション記録 | `log/sessions/2026-10-19-36fc92f3-f36e-4690-bc9d-a4b8815a6435.md`（開始〜全例題の実行）、`log/sessions/2026-10-19-8faf1abd-17ac-42de-b09c-7ff1bc0bc15d.md`（再開〜締め） |

## 依頼

「進めて。別のエージェントが /ponytail-review を実行中。衝突しないように注意して」。S012 の「次回」の候補 1（止まった例題・管の外れの残り、`auto` の初期化での切り替えの見直し）を、自分で選んで進めた。

2026-10-20 に「再開して」で再開し、締めた（全例題の実行は 2026-10-19 に終わっていた）。

## 決めたこと

- `auto` は方向微分が作れない失敗（`GM-U0004`）では、組み立て・初期化でも dopri45 に切り替えない（[ADR-0015](../../doc/adr/0015-auto-and-reference-tolerance.md) の (a)。ADR-0014 (d) の一部を覆す）。
- `cargo xtask msl-sim --tolerance-scale` を足し、`mise run msl:sim` は 0.1 で回す。注釈に Tolerance が無いモデルは 1e-6 のまま（参照結果と同じ規則。最初に既定にも掛ける形で回しかけてやめた）。`globe simulate` の既定は変えない（ADR-0015 の (b)）。
- `DoublePendulumInitTip` を既知の外れの一覧に足した（許容誤差を 1/10 にすると 3/4 → 2/4）。

## やったこと

- `globe-mode-exec/src/rumoca.rs`: `Fallback::InitialLinearization` と `directional_derivative_unavailable` を消した。テストを直した。
- xtask の `main.rs`・`sim.rs`: `--tolerance-scale`（ワーカーへ渡す、報告の設定の行と JSON に書く）。`mise.toml` の `msl:sim` に `--tolerance-scale 0.1`。
- 文書: ADR-0015（新規）、ADR-0014 の状態と (d)、ADR の索引、architecture.md、roadmap.md（S013 の節、S014 の候補）、research/msl-sim-failures.md（節「S013 で直したもの」）、CLAUDE.md の `msl:sim` の説明、doc/msl-sim.md（回し直し）。

## 検証

- `mise run msl:sim`（`--tolerance-scale 0.1`、2026-10-19 12:44 UTC）を S012 の結果と両方にある 356 本で比べた:
  - 実行できた 309 → **308**（−1。`GenerationOfFMUs` が `GM-U0004` で止まる）
  - 参照結果と比べた 308 → 307、管に収まった 299 → **301**（+2。`DemonstrateSignalExtrema`・`DemoPowerSupplyWithBuffer`）。98.0%。収まらなくなったものは無い
  - 管の外れ 6 本（既知 3・既知でない 3）。時間切れの回し直し 6 本のうち 1 本が実行でき、管に収まった
  - panic 0、外から停止 7、ワーカーの異常終了 0
- `msl:funnel` は回していない（検査の段階・Rumoca の rev は変えていない。`target/msl-funnel.json` は S012 のもの）。
- `mise run check`: fmt・clippy（`-D warnings`）0 件、テスト 485 本通過・0 失敗・10 本 ignore（S012 と同じ本数。消したのはテストの中の assert だけ）、cargo deny ok、`uv sync --locked` ok。
- `mise run wasm:node` ok。`mise run wasm:compare`: 117 件のうち 114 件が一致、既知の差 3 件、一致しない 0 件（S012 と同じ）。
- `check_docs.py`: 全デッキ OK、相対リンク 679 件のうち切れ 0 件。`render_slides.py`: S013 の 6 枚 × 3 表示で問題 0 件。図の箱の文字のはみ出しとスマートフォンの表の長さは目で見て直した。

## 残したこと

- 既知でない管の外れ 3 本（`Clocked` の `RotationalSample`・`LogicalSample`、`Dimmer_R`）。許容誤差では変わらない。
- Rumoca の Auto とは、方向微分が作れないときの扱いが 1 本ずれる（ADR-0015 の結果）。
- ほかの候補は [roadmap.md](../../doc/roadmap.md#次のセッション) の S014 の候補。
