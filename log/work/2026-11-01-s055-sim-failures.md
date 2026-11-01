# S055 `sim-failures`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s055-sim-failures` |
| worktree | `.git-worktree/s055-sim-failures` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S055-sim-failures/` |
| セッション記録 | [`log/sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md`](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md) |

## 依頼

S050 のソルバ別の表（run 41194959752）で目立った 2 件の原因を数字で出し、直せるものだけ直す。`cerk23`・`cerk34` の時間切れ（`GM-E11005`）27 本（`cerk45` は 22、`bdf` は 2）と、`bdf` 単独の内部エラー（`GM-U0005`）10 本。親セッションから渡された子の作業（並列のエージェント）。

## 決めたこと

- **`cerk` は直さない**（分類と案内だけ足す）。27 本のうち 24 本は陰的ソルバが終える硬い系で、陽的な方法の刻み幅は次数 3 の `cerk23` から 8 の `dopri853` までほぼ同じ桁（安定限界）。実装の誤りの証拠は見つからなかった。他のソルバの数値を変えない約束なので、diffsol の刻み幅制御の差し替えもしなかった。
- **GM-E11005 に陽的ソルバ向けの補足を足す**（`explicit_timeout_hint`）。数値は変えない。
- **`bdf` の `GM-U0005` は直さない**。検査は Rumoca の host、補間は Rumoca の BDF。native の `bdf` は GlobeMode のコードを通らない。上流の下書き 15 を足した（出すのはユーザー）。
- **`auto` の切り替え先を `esdirk34` にする案は実装しない**（`auto` の挙動を変えるので ADR-0015 の改訂が要る）。次の候補に書いた。
- **`msl-sim.yml` に入力 `solvers`**。既存のジョブの中身は変えず、`on.workflow_dispatch.inputs` と matrix の `solver` の式だけ。`msl-sim`（全体）と `split-all` は絞っても回る（既存ジョブの中身を変えない指示のため `if` を足していない）。
- **`doc/msl-solvers.md` は更新しない**（表は全ソルバ同時の run で作る決まり）。

## やったこと

- `gh run download 41194959752` で 14 ソルバの JSON を取り、時間切れ 27 本を分類（陰的が終えた 24、どれも終えない 2、dopri45 だけ 1）。
- `rk.rs`・`session.rs` に一時的な出力を入れ（**コミットしていない、戻した**）、刻み幅・誤差ノルム・`bdf` の補間の食い違いを見た。
- `bdf` の 7 本を許容誤差 1e-6・3e-7・1e-7 で回した。
- 変更したファイル:
  - `src/Rust/mod/globe-mode-exec/src/rumoca.rs`（`explicit_timeout_hint` と試験）
  - `.github/workflows/msl-sim.yml`（入力 `solvers`）
  - `doc/research/msl-sim-failures.md`（節「S055 で調べたこと」）
  - `doc/research/upstream/15-…md` と `README.md`
  - `doc/roadmap.md`（S055 の節）、`doc/sessions/README.md`、`doc/sessions/S055-sim-failures/`

## 検証

- `cargo test -p globe-mode-sim`: 106 件 OK。`cargo test -p globe-mode-exec --lib`: 19 件 OK。`cargo clippy -p globe-mode-exec -- -D warnings`: 警告 0。
- 数字（research の節に出典つき）:
  - `cerk23` の時間切れ 27 本: 陰的 8 本のどれかが終えた 24（`bdf` が終えた 21、32 秒以内）、どれも終えない 2、dopri45 だけ 1。dopri45 が終えて diffsol の陽的が終えない 7。
  - 刻み幅（cerk23）: `DCPM_Drive` 2.6e-8（終了まで 7.7e7 歩）、`SmoothStep` 2.9e-9（9.2e5 歩）、`SMPM_Braking` 2.4e-10（3.3e9 歩）。`IMC_Inverter` の t ≈ 0.12〜0.15 の刻み幅は cerk23 2.6e-6〜9.5e-6、cerk45 2.1e-6〜5.3e-6、dopri853 6.9e-6〜1.4e-5。
  - `bdf` の `GM-U0005` 10 本: 9 本は左端の補間の食い違い（上限の 1.1〜445 倍）、1 本は歩が丸めの幅に潰れる。`auto` は 7 本を救い、2 本は dopri45 が時間切れ、1 本（`Dimmer_RL`）は切り替わらない。
  - 許容誤差を変えた `bdf` の 7 本: 1e-7 は 0/7 が通る、3e-7 は 4/7、1e-6 は 4/7。単調でない。
- CI（run 41247200921、`["cerk23","cerk34","bdf"]`、全ジョブ成功。コミット c07a133）: `bdf` 301・管 293/300・`GM-U0005` 10・3305 s、`cerk34` 286・275/285・時間切れ 27・9421 s は S050 と全モデルで同じ結果。`cerk23` は実行できた 288 → 290、管 279/287 → 280/289、時間切れ 27 → 25。`IMC_Inverter`（run_seconds 177.9 s）と `SMPM_VoltageSource`（164.4 s）が回し直しの予算 180 s の境目で通った。
- `check_docs.py`: デッキ OK、相対リンク切れ 0 件。`mise run check`・`render_slides.py`・`wasm:*`・`msl:*` は親が回す（手元は RAM 3 GB で、他のセッションと同時に動かしたため回していない）。

## 残したこと

- 陽的 RK の刻み幅制御が `dopri45` との差の原因かは【未確認】。`OneTank` は積分器の中でなく host のイベント処理に数秒かかっていた。
- `auto` の切り替え先の案（`esdirk34`）。
- 上流の下書き 15 を出すかどうか（ユーザー）。
- 秒数・歩数/秒は他のセッションのビルドと重なった機械での値（load average 10〜70）。刻み幅と歩数は負荷に依らない。
