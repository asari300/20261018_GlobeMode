# S019 `resettle-followups`

| | |
| --- | --- |
| 日付 | 2026-10-23 |
| ブランチ | `feature/s019-resettle-followups` |
| worktree | `.git-worktree/s019-resettle-followups` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S019-resettle-followups/` |
| セッション記録 | `log/sessions/2026-10-22-b4423585-4c94-49e2-bb23-ae4c23d0d96e.md`（S017・S018 と同じセッション） |

## 依頼

S018 の締めの報告のあと「つづけて」。S018 で次回に残したこと（ADR-0020 の案 D、レビューの指摘 1・2・4〜7）を片付ける。あわせて「target は 2 つ前のセッションのものを消す」とユーザーが決めた。

## 決めたこと

- ユーザーと決めたこと:
  - target は 2 つ前のセッションのものを消す（RPi のディスク対策）。私の提案で、コピー（`cp -a`）をやめて移動（`mv`）にした。コピーすると target が 2 つになり、SD カード（117 GB）が 91% を超えるため。
  - **次回から作業は PC に戻す**（RPi では `msl:sim` が PC の約 2 倍かかり、時間切れの本数が機械で変わる）。重い検査（`msl:funnel`、`msl:sim`、`wasm:node`、`wasm:compare`）は PC に回す。
- 既定のまま進めたこと: 解き直しを省く条件は「`update_relation_memory_from_state` が false」にした。ADR-0020 の案 D（根の条件が無いモデル）を含み、根の条件があっても t0 で反転しないモデルまで省ける。ADR-0021 に書いた。

## やったこと

- `globe-mode-exec/src/rumoca.rs`: 関係式の記憶が変わらなければ `resettle` を None で返す。`resettle` を別の `catch_unwind` に入れた。`differs` に差の式をまとめた。`Resettle.moved` から 1 回目の値を落とした。単体試験 `resettle_warns_only_where_the_run_started_away_from_the_second_solve` を足した。
- `globe-mode-exec/tests/simulate.rs`: doc コメントの取り違えと本数（13 本）を直した。
- `doc/adr/0021-resettle-only-when-relations-flip.md` を足し、ADR-0020 の状態の行と ADR の索引を直した。
- セッション文書 `doc/sessions/S019-resettle-followups/`。

## 検証

- `mise run check`（RPi、`CARGO_BUILD_JOBS=2`）: 通った。`cargo test` 489 本が通り、失敗 0、無視 10。fmt、clippy、deny、`uv sync --locked` も通った。最初の 1 回は `cargo fmt --check` の整形の違いで落ち、`cargo fmt` で直した。次の 1 回はセッションが切れて途中で止まった。
- `an_initialization_that_reads_a_relation_flipped_at_t0_is_warned`: 通った（案 D を狭めた後も再現のモデルは警告される）。
- `check_docs.py`: デッキ全件 OK、相対リンク 806 件中の切れ 0 件。`render_slides.py`: 問題 0 件。
- 4 つの検査は S020（2026-10-24〜25、WSL x86_64、RAM 3 GB + swap 16 GB）で回した。詳しくは [S020 の作業記録](2026-10-24-s020-pc-heavy-checks.md)。
  - `msl:funnel`: 566 本: OK 369・NG 13・GM-U 184・panic 0（S017 と同じ）。
  - `msl:sim`: 356 本: 実行できた 309・管に収まった 301 / 308（97.7%）・実行 NG 41・外から停止 6・panic 0（S015 の PC の数字と同じ）。**GM-W10004 は EngineV6 の 1 本だけ**（回し直しで終了時刻まで行き、管は 2 / 4）。ほかの 355 本では 0 本。
  - EngineV6 単体: GM-W10004 が `filter.x[1] = 0.065992 → 563.186238` ほか 7 個で出た（S018 と同じ値）。
  - `LightningSegmentedTransmissionLine` のプリフライト 15.1 s（S018 の RPi では 33.6 s。PC は約 2 倍速いので、費用そのものが下がったかは分からない。関係式の記憶が変わるので解き直しは省けていない）。プリフライトの段の合計は 356 本で 45.2 s（S018 の RPi 98.5 s）。
  - `wasm:node`: 通った。`wasm:compare`: 126 件中 123 件が一致、既知の差 3 件、一致しない 0 件。
  - ADR-0021 は直さない（EngineV6 は検出でき、ほかで GM-W10004 は出なかった）。

## 残したこと

- ~~次の PC のセッションの最初に、上の 4 つの検査を回す~~ → S020 で済ませた（上の「検証」）。
- swap が 8 GB から 5.7 GiB に下がった理由は不明のまま（今回の再起動のあと 8.0 GiB に戻っていた）。RPi を使わなければ不要。
