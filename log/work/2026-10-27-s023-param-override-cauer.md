# S023 `param-override-cauer`

| | |
| --- | --- |
| 日付 | 2026-10-27 |
| ブランチ | `feature/s023-param-override-cauer` |
| worktree | `.git-worktree/s023-param-override-cauer` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S023-param-override-cauer/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「進めて。サブエージェントはSonnet/high」。S022 の残しのうち、`globe simulate --param V.startTime=…` が効かない件と、esdirk34 の `CauerLowPassSC`（t = 1.05 のイベントの後の再開で刻みが縮む）の計測を選んだ。

## 決めたこと

- 調査はユーザーの指示でサブエージェント 2 本（Sonnet、effort high）に分けた。A は `--param` の原因（`rk.rs` に触らない）、B は `CauerLowPassSC` の計測と直し（`rk.rs` の一時的な計測は最後に外す）。
- **RK の最初の刻みを下限の 10 倍まで持ち上げる**（`rk.rs` の `initial_state()`）。B の計測で、t = 1.05 の再開の後に右辺を 1 度も呼ばずに `step()` が「Step size is too small」を返すことが分かった。diffsol の `set_step_size` の見積もり `h0 = 0.01·‖y‖/‖f‖` が、|y| ≈ 1e-22・|f| ≈ 4e4 で 2e-18、刻み 2e-16 になり、下限 1e-13 を割る。S022 の仮説（試しの点の NaN・host の失敗の記憶）は外れ。使えない刻みを上げるだけなので、通っていた例題の出力は変わらない（`OvervoltageProtection`・`HeatingRectifier` はバイト単位で同じ）。10 倍にしたのは、下限そのものだと最初の歩が棄てられたときにまた割るため。
- **回帰の試験は新しい小さなモデル** `TinyStateRestart`（x = 3e-11 から t = 1 で der(x) が 0 → 1e5）。直す前の esdirk34・trbdf2・tsit45 で同じ所で落ちるのを、作る前に確かめた（t = 0 から始めると下限が小さく再現しない）。
- **`--param` の時間イベントは直さない**。A の調査と私が読んだコードで、Rumoca の DAE の段の `time_event_instant`（`expression_events.rs:621`）が `constant_context()`（`analysis.rs:1410`、調整できるパラメータも既定値で畳む）で時刻を求め、静的な時間イベントにすることを確かめた。手元で直すなら `Session::set_structural_overrides` でコンパイルし直す道で、frontend・キャッシュ・wasm に届くので別のセッションで ADR を書いてから。上流の候補 16。警告（新しい GM-W）も入れていない。
- `msl:sim` の全体は `--solver esdirk34` だけ。既定の `auto`（Rumoca の BDF と dopri45）は `initial_state()` を通らないので回していない。`msl:funnel` も回していない（検査の段は変えていない）。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: `initial_state()` の最後で、刻みが `min_timestep` の 10 倍より小さければ持ち上げる（9 行、理由の注釈つき）。
- `src/Modelica/GlobeModeValidation/Simulation/TinyStateRestart.mo`・`package.order`: 最小の再現。
- `src/Rust/mod/globe-mode-exec/tests/integrators.rs`: `rk_restarts_from_a_tiny_state_with_a_large_derivative`（esdirk34・trbdf2・tsit45 で x(2) = 1e5）。
- `doc/research/msl-sim-failures.md`: 「S023 で直したもの」（再開の刻み、`--param` の原因と直す道、上流の候補 15・16）。S022 の `CauerLowPassSC` の段落を直した。
- `doc/architecture.md`（RK の最初の刻み）、`doc/roadmap.md`（S023 の節）、`doc/sessions/S023-param-override-cauer/`、索引。

## 検証

- `TinyStateRestart`: 直す前の binary は esdirk34・trbdf2・tsit45 とも失敗、直した後は 3 つとも x(2) = 100000.00000000001。
- `cargo test -p globe-mode-exec --release --test integrators`: 9 本通過。
- `CauerLowPassSC`（B の計測）: esdirk34 は t = 60 まで 75 s、bdf との差は全信号で |x−y|/(1+|y|) ≤ 5.4e-9、trbdf2 は 1.2e-8。`msl-sim --wall-seconds 120` で積分 39.6 s、管に収まる（1 / 1）。
- `Electrical.Analog.Examples` 54 本（予算 30 s）: esdirk34 は 実行 48・管 46、trbdf2 は 48・45（どちらも S022 と同じ。違いは `CauerLowPassSC` が GM-E11002 → 時間切れの 1 本）。tsit45 は S021 の 41・38 → 42・39（違いは `LightningSegmentedTransmissionLine` が時間切れ → 通る。予算の境目）。
- `msl:sim` 全体 356 本の esdirk34（予算 30 s、約 44 分）: 実行 282 → 286、管 273 / 281 → 277 / 285（97.2%）、実行 NG 67 → 64（時間切れ 24 → 26）、外から停止 7 → 6、panic 0。新たに通った 4 本は `PowerConverters.Examples.ACDC` のサイリスタの整流器（`HalfControlledBridge2mPulse`・`ThyristorBridge2mPulse_R`・`ThyristorCenterTap2mPulse_R`・`ThyristorCenterTapmPulse_R`）で、すべて管に収まる。管の外れは 8 本のまま。
- `wasm:node`: 通った。`wasm:compare`: 144 件中 141 件が一致・既知の差 3・一致しない 0。
- コードの検査（fmt / clippy / test / deny）: `mise run check` で 491 本通過・0 失敗・10 ignore、fmt・clippy・deny OK（最初の回は `integrators.rs` の fmt で落ち、`cargo fmt` で直して回し直した。約 66 分）
- `check_docs.py`: 全デッキ OK / 文書 134 件・相対リンク 850 件中の切れ 0 件。`render_slides.py` 29 デッキ・問題 0 件（S05 の図の文字のはみ出しを直して撮り直し、S03・S05 を目で確かめた）
- セッション記録の `--check`: 検査 9 件・取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- `--param` で時間イベントの時刻が変わらない（Rumoca の DAE の段。上流の候補 16）。手元の直しは `set_structural_overrides` の道（ADR が要る）。それまでの警告も入れていない。
- `ThyristorBehaviourTest` の esdirk34（消弧の逆回復で射影が発散）。
- 上流への報告の候補 15・16（ユーザーが出す）。diffsol の本家で `set_step_size` が同じかは確かめていない。
- 既定の `auto` の `msl:sim`、`msl:funnel`・`msl:omc` は回していない（`auto` の道と検査の段は変えていない）。
