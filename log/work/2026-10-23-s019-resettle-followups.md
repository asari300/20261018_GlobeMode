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
- **通していない**: `msl:funnel`、`msl:sim`、`wasm:node`、`wasm:compare`（PC に回した）。MSL での GM-W10004 の本数、EngineV6 の警告、Lightning の費用は未確認。

## 残したこと

- **次の PC のセッションの最初に**、上の 4 つの検査を回して、この記録の「検証」に数字を足す。見ること: EngineV6 で `filter.x = 0.066 → 563` が警告される、MSL で GM-W10004 が 0 本のまま、`LightningSegmentedTransmissionLine` のプリフライトが下がる。結果が違えば ADR-0021 を直す。
- swap が 8 GB から 5.7 GiB に下がった理由は不明のまま（今回の再起動のあと 8.0 GiB に戻っていた）。RPi を使わなければ不要。
