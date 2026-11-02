# S088 `sim-dimmer-thyristor`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s088-sim-dimmer-thyristor` |
| worktree | `.git-worktree/s088-sim-dimmer-thyristor` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S088-sim-dimmer-thyristor/` |
| セッション記録 | （親が写す） |

## 依頼

S064 の持ち越し 3 件。(2) `Dimmer_RL` の `y_arg` の外れの原因（`Dimmer_R` と同じか）を調べ、実行系で直せれば直し、直せなければ既知の外れに理由付きで足す。(1) `ThyristorBridge2mPulse_DC_Drive` の esdirk34 の時間の内訳を測り、予算の分け方の 1 行で収まれば直す（ADR-0028 に追記）、収まらなければ数字を残して持ち越し。(3) `auto` の全体の予算は時間があれば計測だけ。手元は 1〜2 本だけ回し、全体は `msl-sim.yml` を feature ブランチで回す。main へのマージ・push は親が行う。

## 決めたこと

- **`Dimmer_RL` と `Dimmer_R` は同じ原因**と判定し、2 本とも `src/Modelica/msl-sim-known-deviations.txt` に足した（実行系は変えない）。外れるのは、サイリスタが止まって第 1 高調波の振幅が漏れ電流の大きさ（参照 2.67e-6 V）まで落ちる 0.02 s の窓の `y_arg` だけ。点弧の時刻は `Dimmer_R` の最初の 11 回が 1e-6 s まで一致、電流と導通区間の信号は 4〜5 桁一致。ほぼ 0 の振幅の位相は `Mean` の積分の残差で決まる（参照の側も、許容誤差を 1e-8 にすると 9/11 → 4/11 に悪くなる）。`Dimmer_RL` は負荷電圧に `L·di/dt` が入るので、止まっている間の振幅がこちらは 4〜8e-5 V（参照の 15〜30 倍）になり、位相が 1.06 と 1.70 rad に分かれる（外れ 19206 点、止まっている約 1.9 s ぶん）。
- research の「点弧の時刻のずれが窓の平均に入る見込み【未確認】」（`Dimmer_R`）は、点弧の時刻が一致したので外れ。
- **`ThyristorBridge2mPulse_DC_Drive` は直さない**。`run_auto` は回し直しの各段に元の予算をそのまま渡すので、分け方の 1 行で esdirk34 に渡る時間は増えない。esdirk34 単独の時間そのものが境目にある（下の数字）。
- (3) は計測だけ。
- 実行系の Rust は変えていないので、`mise run check`・`wasm:node`・`wasm:compare` は回していない。

## やったこと

- `src/Modelica/msl-sim-known-deviations.txt`: `Dimmer_R`・`Dimmer_RL` の 2 行。
- `doc/msl-sim.md`: CI の artifact の報告をそのまま写した。
- 文書: `doc/sessions/S088-sim-dimmer-thyristor/`、索引、roadmap の S064 の持ち越しと S088 の節、research の `Dimmer_R` の行。

## 検証

- 手元（release の `globe simulate`、`--tolerance 1e-7`）: `Dimmer_RL` esdirk34 77 s で終了（他のセッションのビルドで負荷 24）。`auto --timeout 180` は負荷で時間切れ（232 s）。`Dimmer_R` auto で終了。比べたのは MAP-LIB の参照結果（`dac67829`）の 2 本だけを取った CSV。
- `ThyristorBridge2mPulse_DC_Drive`（手元、負荷 1 前後のとき）: BDF 7.5 s で `GM-U0005`、esdirk34 単独 159.8 s で終了（点 114031、格子の外 37528 ＝ イベント約 3.75 万回、状態 18、最大 RSS 0.95 GB）。負荷 239 のときは esdirk34 が 900 s で時間切れ（数字に使わない）。
- CI `msl-sim.yml` run 41403194603（`-f 'solvers=["esdirk34"]'`）: `msl-sim` success・`split-all` success。356 本: 実行できた 310（前回 310）・管に収まった 301 / 309（前回 301 / 309）・実行 NG 40（うち時間切れ 4）・外から停止 6・panic 0。前の `doc/msl-sim.md` とモデルごとの判定・管の結果が 355 行すべて同じ。既知の外れ 3 → 5。`ThyristorBridge2mPulse_DC_Drive` は 187.5 s で時間切れ（積分の段 185.0 s、最初の試行 37.5 s）。`solvers (esdirk34)` は手元で esdirk34 単独の数字が取れたので止めた（cancelled）。
- 前回の `target/msl-sim.json` はキャッシュから戻らず（S064 のブランチのキャッシュ）、報告に「前回との差」が無い。差は報告の表を手で突き合わせた。
- (3) 最初の試行（予算 30 s）で 30 s を超えたもの 7 本: 53.3 s（`ThyristorBridge2mPulse_RLV_Characteristic`）、47.6 s（`ThyristorBridge2Pulse_DC_Drive`、BDF → esdirk34 → dopri45）、37.5 s、32.3 s、31.3 s、30.9 s、30.6 s。応答の上限 120 s で外から止めたもの 8 本。
- コードの検査（fmt / clippy / test / deny）: 回していない（Rust を変えていない）。`wasm:node`・`wasm:compare` も同じ理由で回していない。
- `check_docs.py`: デッキ 91 件 OK / 文書 365 件・相対リンク 1762 件中の切れ 0 件。`render_slides.py`（S088 の 1 デッキ、desktop-light・dark・mobile の 8 枚ずつ）: 問題 0 件
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークン）

## 残したこと

- `ThyristorBridge2mPulse_DC_Drive` の時間切れ（イベントごとのやり直しを軽くするか、回し直しの予算を見直す）。ヤコビ行列の作り直しの回数は数える口が無く数えていない。
- `auto` の全体の予算の規則（計測だけ）。
- ほぼ 0 の振幅の位相を比べ方の側で除くかは決めていない。
- main の作業ツリーに未追跡の `target-shared/` がある（このセッションのものではない。触っていない）。
