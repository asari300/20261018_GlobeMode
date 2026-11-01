# S064 auto-esdirk-fallback

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s064-auto-esdirk-fallback` |
| worktree | `.git-worktree/s064-auto-esdirk-fallback` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S064-auto-esdirk-fallback/` |
| セッション記録 | 親が写す（このセッションは親から渡されたサブエージェント） |

## 依頼

S055 の持ち越し。`auto` の回し直しに esdirk34 を足す。順序は手元の小さい集合で比べて決める。`Dimmer_RL` の文言も対象にするか決める。ADR-0028。全例題の CI で本数が減らないことを確かめる（減れば原因を書いて親に判断を委ねる）。

## 決めたこと

- 順序は BDF → esdirk34 → dopri45。dopri45 を先にすると、硬い系で予算を使い切って esdirk34 に時間が残らない（`Bridge2m_DC_Drive`・`Dimmer_RL`）。dopri45 だけが救える `TwoMass` は、esdirk34 が 3 秒以内に落ちて dopri45 に回る。案 A は実装して比べていない（dopri45 が予算の最後まで使うのは測定済みの 192 s / 201 s の時間切れから決まる）。
- esdirk34 が時間切れ以外で止まれば、規則に当たらない失敗でも dopri45 を試す（取りこぼさない）。時間切れなら試さない。
- `Dimmer_RL` の「made no progress beyond roundoff」を対象に足した（`MeIntegrationError::Contract`。型に印が無いので文言）。
- 最初に「残りの予算を渡す」版を作った（依頼の「回し直しごとに残り時間を渡す」）。CI で `Lightning` が減ったので、元の予算を渡す版に戻した。親に伝える: 依頼の配分は守れなかった。
- 診断: dopri45 は `GM-I11002` のまま、esdirk34 は新しい `GM-I11003`。msl-sim の欄を `auto → esdirk34` などにした。
- ADR-0015 は Superseded にせず、先頭に追記だけ（(a) の切り替え先を拡げるだけで、(b) などは生きている）。

## やったこと

- `globe-mode-exec/src/rumoca.rs`: `run_auto` を書き直し（回し直しの列、`Fallback::NoProgress`・`Other`、`fallback_diagnostic` に from/to）、試験を 2 本足した（順序、`no_progress`）。`globe-mode-core`: `RUN_SOLVER_FALLBACK_ESDIRK`。`xtask/src/sim.rs`: `solver_used` と数え方。説明文は `globe-mode-sim/src/solvers.rs`、xtask の `main.rs`・`solvers.rs`。
- 文書: ADR-0028、ADR 索引、ADR-0015 の追記、architecture.md、roadmap.md（S064 の節、S055 の次の候補、候補 2）、doc/msl-sim.md（run の artifact。「前回との差」は S055 の結果に直した）、セッション文書、sessions/README.md。

## 検証

- 手元（release、`globe simulate --solver auto`、S050 の許容誤差、`--timeout 180`、21 本。他のセッションの負荷で秒数は目安）:
  - `Dimmer_RL`: `GM-U0005` 91 s → esdirk34 で終了 93 s。
  - dopri45 で終えていた 7 本（`Lightning`・`Bridge2Pulse_RLV`・`Bridge2mPulse_RLV`・`CenterTap2Pulse_RLV`・`CenterTap2mPulse_RLV`・`CenterTapmPulse_RLV`・`ChopperBuckBoost`）→ esdirk34 で終了（1.1〜2 倍の時間）。`TwoMass` は esdirk34 が失敗 → dopri45 で終了。
  - 変わらない失敗: `Bridge2Pulse_DC_Drive`（esdirk34 `GM-E11003`、dopri45 時間切れ）、`Bridge2mPulse_DC_Drive`（手元は時間切れ。esdirk34 単独で 162 s、ランナーは 94 s）、`HeatingNPN_NORGate` ほか 5 本。
  - BDF で通る 5 本（`SinCosEncoder`・`ThyristorBehaviourTest`・`IMC_DOL`・`SMPM_Braking`・`OneTank`）は回し直しなし。
  - `Lightning` を `--timeout 30` で: 前は dopri45 で終了 116 s、後は esdirk34 が時間切れ 121 s（msl-sim は 180 s で回し直す）。
- CI（`msl-sim` ジョブ。比べる相手は S055 の run 41247200921）:
  - run 41282411061（残りの予算を渡す版）: 実行できた 311（+2）、管 302 / 310。`Lightning` が `GM-U0005` で減った。
  - run 41287267439（採った版）: 実行できた 309 → **310**、管に収まった 301 / 308 → **301 / 309**、減ったモデル 0、`GM-U0005` 1 → 0、回し直し 12（1 本通った）→ 15（5 本通った）。ワーカーの時間 2807 s → 2897 s（ランナーの速さでぶれる）。
- `cargo test -p globe-mode-exec -p globe-mode-sim -p globe-mode -p xtask`: 全部通る（`globe-mode` の `Simulation` の試験を含む）。`cargo clippy -p globe-mode-exec -p globe-mode-core -p globe-mode-sim -p xtask --all-targets -- -D warnings`: 警告 0。`cargo fmt --check`: 差分 0。
- `mise run check`・`wasm:node`・`wasm:compare`・`render_slides.py`・`check_docs.py` の最終: 親が回す。

## 残したこと

- `ThyristorBridge2mPulse_DC_Drive`（esdirk34 で 94 s）は時間切れの境目。予算の分け方の決定が要る。
- `auto` の全体の予算（以前から BDF と回し直しがそれぞれ全部の予算）。Rumoca の dopri45 が最初の予算 30 s を超えて走る件が先。
- `Dimmer_RL` の `y_arg` の外れの原因。`known-deviations` には入れていない。
- 手元のディスク: `/tmp`（tmpfs 3.9 GB）が満杯になり、S055 のスクラッチの CSV と build（`o2.csv`・`out_*.csv`・`tgt`、約 1.2 GB）を消した。
- `msl-sim.yml` の `solvers` を `["auto"]` にしても `auto` がもう 1 度回る（約 1 時間）。`gh run cancel` で止めた。`solvers=[]` は matrix が空で失敗するはず【未確認】。
