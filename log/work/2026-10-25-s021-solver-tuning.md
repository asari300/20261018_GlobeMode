# S021 `solver-tuning`

| | |
| --- | --- |
| 日付 | 2026-10-25 |
| ブランチ | `feature/s021-solver-tuning` |
| worktree | `.git-worktree/s021-solver-tuning` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S021-solver-tuning/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md`（S016 の後半・S020 と同じセッション） |

## 依頼

S020 の締めで示した S021 の候補 4 つのうち、ユーザーが「2」（自前の積分器の詰め）を選んだ（2026-10-25）。roadmap の「次のセッション」の 2: `esdirk34` が bdf で通る 5 本で落ちる（diffsol の非線形の失敗・刻みが小さすぎる）、`tsit45`・`esdirk34` の 5 本で射影が収束しない（GM-E11002）、固定刻みの既定の刻み（出力の間隔）では `heun` の精度が足りない（`Rotational.Examples` 7 本で 3 / 7）。陽的 RK もヤコビ行列つきで作っている（diffsol の `ClosureNoJac` が非公開）。

あわせて S020 の漏れ 2 つ（作業記録の漏洩検査の下書き、roadmap の S020 の節）をこのセッションで直す。

## 決めたこと

ユーザーと決めたこと:

- S021 の題は候補 2（自前の積分器の詰め）。ほかの候補（止まった例題の残り、ブラウザ、ADR-0021 の費用の前後比較）は次回以降。

既定のまま進めたこと（理由つき）:

- **esdirk34・trbdf2 の Newton 法の失敗の上限を外す**（`ode_options.max_nonlinear_solver_failures = usize::MAX`）。diffsol の SDIRK は失敗を積分全体で累積して数え（`vendor/diffsol/src/ode_solver/runge_kutta.rs:849-872`）、BDF は 1 歩の中で数える（`bdf.rs:1356-1359`）。切り替えのたびに数回失敗する回路では 50 回をすぐ使い切る。上限を外しても、失敗ごとに刻みを 0.3 倍にする規則と刻みの下限（`min_timestep`）が 1 歩の暴走を止める。diffsol（Rumoca の vendor）を patch する手もあるが、1 行の設定で済むのでそうした。上流への報告の候補 12 に書いた。
- **tsit45 の射影の失敗は直さない**。硬い回路（サイリスタ・ダイオード）で最初の刻みから状態が発散し、試しの点で代数の射影が発散すると Rumoca の host が復帰不能の失敗として記憶する（`fmi_me/integrator/derivative.rs:176-196`。chart を切り替える部品だけが「棄却」になる）。Rumoca 自身の `dopri45` も同じ所で失敗する。陽的 RK がこの硬さに向かないのが原因で、GlobeMode のプラグインの不具合ではない。上流への報告の候補 13 に書いた。
- **heun の既定の刻み（出力の間隔）は変えない**。`Rotational.Examples` 7 本で 3 / 7 のまま。`ElasticBearing` は既定の刻みで発散して 1e139 になる（硬いモデルに陽的 2 次の固定刻みを使うと不安定になる。rk4 は同じ刻みで安定）。精度は刻み幅で決まると文書に書いてあり、`--step` で選べる。既定を小さくすると `Decay` の試験（刻み = 出力の間隔）や既定の意味が変わるので触らない。
- **回帰の試験はダイオードの最小のモデルで**。`GlobeModeValidation.Simulation.DiodeSwitching`（膝の鋭いダイオード、漏れ抵抗、1 ms ごとに向きが変わる電流、98 回の切り替え）。直す前の binary で esdirk34 が t = 0.0027、trbdf2 が t = 5e-6 で「nonlinear solver failures (50)」になることを確かめてから、直した binary で通ることを確かめた。試験は esdirk34 だけ（trbdf2 は debug ビルドで 60 s の予算を超える。release では 1.7 s）。終点はダイオードの膝の平らな所（t = 0.0985）に置いた（斜面の途中だと wasm とネイティブの bdf が 2.3e-4 ずれ、`wasm:compare` が一致しないと数えた）。
- **`msl:sim` の全体は `--solver esdirk34` で回し、既定（`auto`）では回さない**。直したのは SDIRK の問題を組む 1 行で、`auto`（bdf → dopri45）の道は通らない。既定の全体（69 分）はユーザーの端末が要るので省いた。
- **S020 の target と data は `mv` で引き継いだ**（S019 の決まり）。`msl:funnel` のビルドと参照結果の取得を省けた。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: SDIRK の問題を組むときに `max_nonlinear_solver_failures = usize::MAX`（コメントに理由）。
- `src/Modelica/GlobeModeValidation/Simulation/DiodeSwitching.mo`（新規）と `package.order`: 回帰の最小のモデル。
- `src/Rust/mod/globe-mode-exec/tests/integrators.rs`: `sdirk_survives_more_than_fifty_newton_failures`（esdirk34 が終了時刻まで進み、切り替え 98 回、v(0.0985) が bdf と 1e-4 以内）。
- `doc/research/msl-sim-failures.md`: 「S021 で直したもの」（esdirk34 の原因と直し、tsit45 の原因、OvervoltageProtection の誤った結果、heun）と上流への報告の候補 12・13。
- `doc/architecture.md`: ソルバの節に S021 の 1 行。`doc/roadmap.md`: S020・S021 の節、S019 の残しの消し込み、候補 2 の更新。
- S020 の漏れ: `log/work/2026-10-24-s020-pc-heavy-checks.md` の漏洩検査の行を 0 件に、roadmap に S020 の節。
- セッション文書 `doc/sessions/S021-solver-tuning/`、索引の行。

## 検証

数字で書く。通っていない項目は「通っていない」と書く。

- `mise run check`（13:28〜14:25、57 分。RAM 3 GB の debug ビルドが重い）: `test_ci_plan`・fmt・clippy・test・deny・`uv sync --locked` すべて通過。テストは **490 本が通過・0 失敗・10 ignore**（S019 は 489 本。新しい `sdirk_survives_more_than_fifty_newton_failures` を含む）。その後に試験の期待値と単一要素のループを直し、`cargo fmt --check`・`cargo clippy -p globe-mode-exec --all-targets -- -D warnings`・`cargo test -p globe-mode-exec --test integrators sdirk`（1 本通過）を通した。
- **直す前後の確認（`DiodeSwitching`、release）**: 直す前は esdirk34 が t = 0.0027、trbdf2 が t = 5e-6 で `Exceeded maximum number of nonlinear solver failures (50)`。直した後は bdf 0.053737・esdirk34 0.053724・trbdf2 0.053739（v(0.0985)。差 1.3e-5・2e-6）、切り替え 98 回。
- **`Electrical.Analog.Examples` 54 本、`--solver esdirk34`、予算 30 s、回し直し無し**: 直す前 **実行 42・管 39**（GM-E11002 が 10 本、GM-E11005 が 2 本）→ 直した後 **実行 45・管 42**。通るようになった: `AmplifierWithOpAmpDetailed`（管 5/5）、`OpAmps.Differentiator`（管 OK）。実行できるが管から外れる: `OvervoltageProtection`（0/1。`CL.v` が t = 0.0184 のツェナーの降伏から参照とずれ、終了時刻で -1.64 V（参照 -0.0006）。直す前も同じ軌道で t = 0.3088 に止まっていたので、直しが誤りを作ったのではない。trbdf2 はここで射影の発散（`zDiode.v` = 1.8e25）で止まる）。`DifferenceAmplifier` は失敗から時間切れ（30 s）に変わった。残る GM-E11002 は `CauerLowPassSC`（`Step size is too small at time = 1.05`）、`HeatingRectifier`・`ThyristorBehaviourTest`（射影の収束）、bdf でも落ちる `HeatingNPN_NORGate`・`HeatingPNP_NORGate`・`InvertingSchmittTrigger`。
- **同 54 本、`--solver tsit45`**（コードは変えていない）: 実行 41・管 38（S012 は 40・36。予算と機械が違う）。射影の失敗 7 本（`SimpleTriacCircuit`・`ThyristorBehaviourTest`・`HeatingRectifier`・`OvervoltageProtection`・NPN・PNP・Schmitt）、時間切れ 6 本。
- **`Rotational.Examples` 7 本、`--solver heun`（既定の刻み = 出力の間隔）**: 実行 7・**管 3**（S012 と同じ）。`Backlash` 3/4、`First` 3/4、`FirstGrounded` 3/4、`ElasticBearing` 0/6（`shaft.phi` が 1e139 に発散。bdf・rk4 は 445.56）。
- **`msl:sim` の全体 356 本、`--solver esdirk34`、予算 30 s、回し直し無し**（14:31〜）: **356 本: 実行できた 272・管に収まった 262 / 271（96.7%）・実行 NG 76（うち時間切れ 31）・外から停止 8・panic 0**（57 分。既定の auto は S020 で 309・301 / 308。直す前の esdirk34 の全体の数字は無いので比較は Analog 54 本で）
- `mise run wasm:node`: 通った（53 s。ビルド済み）。`mise run wasm:compare`: 1 回目は 135 件中 130 件が一致・既知の差 3・**一致しない 2**（`DiodeSwitching` の bdf・auto で wasm とネイティブが 2.3e-4 ずれる。終点が斜面の途中だったため）。モデルの終点を平らな所に直して回し直し: 135 件中 132 件が一致・既知の差 3・一致しない 0（47 s）
- `check_docs.py`: デッキ全件 OK。文書 128 件 / 相対リンク 832 件 / 切れ 0 件。render_slides: [render] デッキ 27 件 / 問題 0 件
- セッション記録の `--check`: 9 件を検査、取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（ステージした差分。main と origin/main の差分はマージ時に確かめる）

## 残したこと

- esdirk34 は `OvervoltageProtection` で終了時刻まで進むが誤った軌道になる（ツェナーの降伏の分岐。黙って誤った結果）。trbdf2 は同じ所で止まる。SDIRK の段の Newton 法が強い非線形の代数ループで別の枝に落ちる見込み【未確認】。diffsol の ESDIRK34 の誤差の見積もりが硬い問題で甘い可能性も【未確認】。
- esdirk34 の残り: `CauerLowPassSC`（t = 1.05 で刻みが下限）、`HeatingRectifier`・`ThyristorBehaviourTest`（射影の収束）。tsit45 の硬い回路は直せない（候補 13）。
- heun の既定の刻みは変えていない。硬いモデルでは発散して「終了時刻まで」になる（`ElasticBearing`）。固定刻みの発散（値が有限のまま大きくなる）を知らせる検査は作っていない。
- `msl:sim` の既定（`auto`）の全体は回していない（直した道を通らない）。`msl:funnel`・`msl:omc` も回していない（検査の段階・Rumoca の rev は変えていない）。
- 上流への報告の候補 12（diffsol の SDIRK の失敗の数え方）・13（陽的 RK の試しの点の射影の発散）はユーザーが出す。
