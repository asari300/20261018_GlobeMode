# S139 `sim-sdirk-regression`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s139-sim-sdirk-regression` |
| worktree | `.git-worktree/s139-sim-sdirk-regression` |
| Phase | P3（実行系） |
| セッション文書 | `doc/sessions/S139-sim-sdirk-regression/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

S126 の疑い。S119 の後、session の道の SDIRK で `HBridge_TrianglePWM_RL`・`HBridge_DC_Drive`（`GM-E11002`）と sdirk34hw の `ThyristorBridge2Pulse_DC_Drive`（`GM-E11003`）が新しく落ちた。S119 の「事象のあとの最初の刻みの上限」を切り替えて短い区間で前後を回し、原因かを決める。原因なら最小の直しを入れ、ThyristorBridge 2 本の改善を失わないことを同じ短い区間の棄却回数で確かめる。全体は `msl-sim.yml` に逃がす。新しいソルバ・diffsol の patch・Rumoca の改変、`globe-mode-fmu`・`fmi`・`lsp`・`wasm`・`src/Svelte`、`doc/msl-*.md` の全体の更新、workflow の定義は触らない。main へのマージ・push・worktree の削除は親がする。

## 決めたこと

- 計測は `rk.rs` に一時的に入れた環境変数（`GM_NO_LAST_STEP` で上限を切る、`GM_PROF` で統計と隙間を `eprintln`）で行い、コミットしていない。コミットしたのは `land_on_stop` と `advance` の呼び出し、試験 1 つだけ。
- 原因は S119 の上限（上限を切ると HBridge 3 本は完走）。仕組みは、直前の刻み（出力の格子に切り詰められた値の繰り返し）が停止時刻の手前 1.15e-14 に着地して残りが `min_timestep` 未満になる（`Step size is too small`）か、先 5.3e-15 に着地して diffsol の切り詰め（`100·eps·(|t|+|h|)`）の内側でホストの境界を越える、の 2 つ。
- 直しは上限の側でなく着地の側。上限の効き方（棄却の減り方）を変えないため。条件は「丸めの 8 倍より大きく `min_timestep` の 10 倍未満」。
- 最初の版（隙間の大きさだけの条件、丸め以下も書き換える）は 2mPulse の esdirk34 が t = 0.0383 で `GM-E11003` になった（隙間 0 のときも `latest - t` に書き換えて 1 ulp 変わり、事象の反復が揺れた）ので、丸めの 8 倍以下は触らない条件を足した。
- `sdirk34hw` の `GM-E11003` は、上限なしでも 2mPulse（0〜2 s）で起きる（t = 0.1017）ので S119 だけが原因ではない。2Pulse は手元の 2 s では再現せず、CI の 15 s で確かめる。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: `land_on_stop` と、`DiffsolRk::advance` の `set_stop_time` 直後の呼び出し、単体試験 `lands_on_stop_only_for_a_small_gap`。
- `doc/adr/0013-exec-backend.md`・`doc/roadmap.md`・`doc/sessions/README.md`・セッション文書。

## 検証

- 手元の前後は解説書 2.1・2.2 の表（release、`--tolerance 1e-7`）。HBridge_TrianglePWM_RL 3 ソルバは直す前に失敗、直した版で完走。HBridge_DC_Drive esdirk34 0〜24 s: 直す前は t = 3.7139999999999005 で `GM-E11002`、直した版は積分が 24 s まで通った（受理 279,600 歩。CSV の書き出しだけ `/tmp`（tmpfs 3.9 GB）の満杯で失敗した。大きい CSV を消して続けた）。ThyristorBridge 2 本の esdirk34 0〜2 s は、受理・誤差の失敗・Newton の失敗が S119 と同じ（2mPulse 58,254・11,221・4,865、2Pulse 33,605・4,032・2,513）。
- `cargo test -p globe-mode-exec -p globe-mode-sim`: `globe-mode-exec` 22・`globe-mode-sim` 13+31+106 など、失敗 0（新しい試験 `lands_on_stop_only_for_a_small_gap` を含む）。
- `mise run wasm:compare`: 387 / 400 一致、既知の差 13、一致しない 0（638.7 s、wasm の線形メモリ 5 MiB。S119 と同じ）。
- `msl-sim.yml`: run 41666815770（`solvers=["esdirk34","sdirk34hw"]`、コミット `90d99ae`）、4 ジョブ success。`HBridge_TrianglePWM_RL` は esdirk34・sdirk34hw とも終了時刻まで（管 OK 7/7）、`HBridge_DC_Drive` は `GM-E11002` が消えて時間切れ（`GM-E11005`、180.85 s）、ThyristorBridge 2 本の esdirk34 は終了時刻まで（47.0 s、160.4 s）、`sdirk34hw` の 2Pulse は `GM-E11003`（t = 13.04）のまま・2mPulse は時間切れ（181.9 s）。`esdirk34` 295・管 286 / 294、`sdirk34hw` 293・283 / 292（S126 の表より各 1 本増）。
- コードの検査（`mise run check`）: `then_some` への書き換え（clippy `unnecessary_lazy_evaluations`）のあとの `mise run check` が EXIT 0（試験 642 passed、失敗 0、ignored 11、deny・`uv sync --locked` も通った。load average 30〜44 の中で約 70 分）。最初の 1 回は clippy で落ちたので直した
- `check_docs.py`: 文書の相対リンク 切れ 0 件（スライドは全件 OK）。`render_slides.py` は全デッキの実行が手元の負荷で途中で止まったので S139 のデッキだけを回し、7 枚 × 3 で問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件（親が写した）
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークンのパターンで検索）

## 残したこと

- 事象の反復（`EventIteration`）が刻みの 1 ulp の違いで収束したりしなかったりする件。Rumoca の host の側。
- `HBridge_TrianglePWM_RL` は上限で歩数が約 2 倍（esdirk34 で 4,014 から 9,109）。上限を事象の直後の 1 歩に限る案は S119 の改善の測り直しが要る。
- 手元の負荷が高く（load average 20 前後）、時間の測定は使っていない。回数だけで比べた。
