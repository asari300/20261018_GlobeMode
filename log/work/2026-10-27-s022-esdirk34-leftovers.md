# S022 `esdirk34-leftovers`

| | |
| --- | --- |
| 日付 | 2026-10-27 |
| ブランチ | `feature/s022-esdirk34-leftovers` |
| worktree | `.git-worktree/s022-esdirk34-leftovers` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S022-esdirk34-leftovers/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md`（S016 の後半・S020・S021 と同じセッション） |

## 依頼

S021 の締めの報告のあと、ユーザーが「進めて。サブエージェントは Sonnet/high」（2026-10-27）。S021 が残した esdirk34 の件を続ける: `OvervoltageProtection` で終了時刻まで進むが黙って誤った軌道になる（ツェナーの降伏から参照とずれ、CL.v(0.4) = -1.64 V、参照 -0.0006 V）。残りの 3 本（`CauerLowPassSC` の刻みの下限、`HeatingRectifier`・`ThyristorBehaviourTest` の射影の収束）の原因の分類。調査はサブエージェント（Sonnet、effort high）2 本に分けた。

## 決めたこと

ユーザーと決めたこと:

- S022 は S021 の残し（esdirk34）を続ける。調査はサブエージェント（Sonnet、effort high）で回す（ユーザーの指示「進めて。サブエージェントは Sonnet/high」）。

既定のまま進めたこと（理由つき）:

- **SDIRK のヤコビ行列を刻みの始めの受理した状態で取る**（`rk.rs` のヤコビ行列の閉包が diffsol の渡す点を無視し、`anchor` を使う）。diffsol の SDIRK は `reset_jacobian(op, &state.y, t)`（`runge_kutta.rs:637`、`sdirk.rs:261,277`）で段の未知数 `x` の位置に状態 `y` を渡し、`op/sdirk.rs` の `jacobian_inplace` が `phi + c·y`（状態の約 1.4 倍）で `f'` を呼ぶ。ツェナーの指数ではその点の J が 1e15（正しい点では 5e5）で、Newton 法の更新量 `r / (1 − hγJ)` と、同じ行列で解く埋め込みの誤差の見積もりが 1e12 分の 1 に潰れる。一歩分だけ古い行列になるが、SDIRK はもともと刻みの中で行列を使い回す。vendor の diffsol を patch する手もあるが、閉包の 1 か所で済む。上流への報告の候補 14。
- **試しの点の番人は入れない**。受理点から 1e3 倍以上離れた試しの点で host を呼ばず NaN を返す案（サブエージェントが試した）は許容誤差 1e-5 の残りの失敗に効いたが、閾値が恣意的で、0 の近くから始まる状態を誤って棄てうる。
- **回帰の試験は `DiodeSwitching` の許容を締める**（1e-4 → 1e-5）。直す前の差 1.3e-5 で落ち、直した後は 1.3e-6。MSL は試験で使えず、ツェナーの最小モデルを新たに足すより小さい。
- **`CauerLowPassSC`・`ThyristorBehaviourTest` は分類だけ**。どちらも計測（刻みの列、NaN の回数）が要り、確信が低い。
- **`msl:sim` の全体は `--solver esdirk34` だけ**。`auto` の道は直しを通らない。
- **S021 の `target/` と `data/` を `mv` で引き継いだ**（S019 の決まり）。

## やったこと

- `src/Rust/mod/globe-mode-exec/src/rumoca/plugins/rk.rs`: `anchor: Rc<RefCell<Vec<f64>>>` を足し、`rebuild`・`reset_in_place`・`advance`（受理した状態）で更新。`build_problem` のヤコビ行列の閉包が渡された点を使わず `anchor` で `directional_derivative_into` を呼ぶ（コメントに理由）。
- `src/Rust/mod/globe-mode-exec/tests/integrators.rs`: `sdirk_survives_more_than_fifty_newton_failures` の許容を 1e-5 に。
- `doc/research/msl-sim-failures.md`: 「S022 で直したもの」（機構、直し、残りの 3 本の分類、`--param V.startTime` が効かない件）と上流への報告の候補 14。
- `doc/architecture.md`: ソルバの節に S022 の 1 文。`doc/roadmap.md`: S022 の節、S021 の残しの消し込み。
- セッション文書 `doc/sessions/S022-esdirk34-leftovers/`、索引の行。

## 検証

数字で書く。通っていない項目は「通っていない」と書く。

- **`OvervoltageProtection`（release）**: esdirk34 の CL.v(0.4) は直す前 -1.64 → 直した後 -0.00061562（bdf -0.00061563）。t = 0.02 の CL.v・zDiode.i・Rv.i が bdf と 5 桁で一致（5.3956・-0.02141・0.02411）。trbdf2 は -0.00061640（直す前は射影の発散で停止）。`--tolerance 1e-7`（直す前は GM-U0005）も通る。`--tolerance 1e-5` は esdirk34・trbdf2 とも t ≈ 0.225 で射影の発散のまま。直す前に `--tolerance 1e-8` にすると正しいが 32 s（直した後 0.15 s）。
- **`DiodeSwitching`**: esdirk34・trbdf2・bdf とも v(0.0985) = 0.05373688、k = 98。bdf との軌道の差 1.3e-6・1.9e-6。
- **`Electrical.Analog.Examples` 54 本、`--solver esdirk34`、予算 30 s、回し直し無し**: S021 の 実行 45・管 42 → **実行 48・管 46**（231 s）。新たに通った: `OvervoltageProtection`（管 1/1）・`HeatingRectifier`・`DifferenceAmplifier`（S021 は時間切れ）。管の外れ 2 本は S021 と同じ（`DemoPowerSupplyWithBuffer` 2/4、`Multivibrator` は既知）。残る失敗 6 本: `CauerLowPassSC`（刻みの下限）・`ThyristorBehaviourTest`（射影）・bdf でも落ちる `HeatingNPN_NORGate`・`HeatingPNP_NORGate`・`InvertingSchmittTrigger`・時間切れ `LightningSegmentedTransmissionLine`。
- **同 54 本、`--solver trbdf2`**（S022 で初めて計測）: 実行 48・管 45（325 s）。外れは上の 2 本と `CompareLineTrunks`（`tLine4.p2.v` の 2 点、はみ出し 8e-4）。失敗は esdirk34 と同じ 4 本と時間切れ 2 本（`Lightning`・`SimpleTriacCircuit`）。
- **`msl:sim` の全体 356 本、`--solver esdirk34`、予算 30 s、回し直し無し**: 356 本: 実行できた 282・管に収まった 273 / 281（97.2%）・実行 NG 67（うち時間切れ 24）・外から停止 7・panic 0（50 分。S021 は 272・262 / 271。管に入った: `OvervoltageProtection`・`PlanarFourbar`。新たに実行できて外れた: `Dimmer_RL`（S021 は時間切れ。`vH01.y_arg` の位相、既知の `Dimmer_R` と同じ種類）。既定の auto は S020 で 309・301 / 308）。
- `mise run wasm:node`: 通った（54 s）。`mise run wasm:compare`: 135 件中 132 件が一致・既知の差 3・一致しない 0。
- `mise run check`: （18:05〜18:47、42 分）fmt・clippy・test・deny・`uv sync --locked` すべて通過。テストは **490 本が通過・0 失敗・10 ignore**（`sdirk_survives_more_than_fifty_newton_failures` は許容 1e-5 で通過）。
- `check_docs.py`: デッキ 全件 OK / 文書 131 件・相対リンク 840 件・切れ 0 件。render_slides: [render] デッキ 28 件 / 問題 0 件（S03〜S05 を目で確認、はみ出し無し）
- セッション記録の `--check`: 9 件を検査、取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（main と origin/main の差分もマージ時に 0 件）

## 残したこと

- `CauerLowPassSC`: t = 1.05（スイッチトキャパシタの BooleanPulse の立ち下がり、V.v が t = 1 で立ち上がった後の最初の電荷の移動）の後の再開で刻みが下限まで縮む。許容誤差 3e-6 以上・V.V ≤ 0.1 で通り、2e-6 以下・V.V ≥ 1 で落ちる。tsit45・dopri45・bdf は通る。`advance` は `step()` の誤りを先に返すので、host が記憶した失敗（試しの点の NaN）が「刻みが小さすぎる」の文言で出ている可能性【未確認】。次は再開の後の刻みの列と NaN の回数を計測する。
- `ThyristorBehaviourTest`: esdirk34 だけ消弧の逆回復（t ∈ (5.3e-5, 5.5e-5]、vAK が -1.4 → -7.0）で射影が発散。直した後は trbdf2 が通る。host の射影が硬い導通の混合（5 桁）で失敗する見込み【未確認】。
- `--tolerance 1e-5` の `OvervoltageProtection` は esdirk34・trbdf2 とも t ≈ 0.225 で射影の発散（Newton 法が CL.v = 7.6 から -4e9 へ飛ぶ。候補 13 の機構）。
- `globe simulate --param V.startTime=0.5` が効かない（`--list-parameters` には載る。`V.V` は効く）。時間イベントに使うパラメータの上書き。別の件として次回以降。
- `msl:sim` の既定（`auto`）の全体、`msl:funnel`・`msl:omc` は回していない（検査の段階・Rumoca の rev は変えていない）。
- 上流への報告の候補 14 はユーザーが出す。
