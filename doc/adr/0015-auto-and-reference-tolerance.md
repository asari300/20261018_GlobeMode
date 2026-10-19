# ADR-0015 `auto` は方向微分が作れないときに切り替えない。MSL の突き合わせは参照結果と同じ許容誤差（注釈の 0.1 倍）で回す

- 状態: Accepted（2026-10-19、S013）
- 関係: [ADR-0014](0014-session-route-wasm.md) の決定 (d) のうち「組み立て・初期化で方向微分が作れないときは RK に切り替える（S011 の規則）」を覆す。(d) のほかの部分（積分の失敗とサンプラの食い違いで dopri45 に回し直す、`GM-I11002`）はそのまま。[ADR-0013](0013-exec-backend.md) の MSL の突き合わせの設定を変える

## 背景

- S011 から、`auto` は BDF の組み立て・初期化で方向微分が作れないと RK（dopri45）に切り替えていた。Rumoca の Auto（`rumoca-sim` の `select_auto_integrator`）と同じ規則。
- MSL の全例題（S012）で、この規則に当たったのは `Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs` の 1 本だけ。RK で終了時刻まで進むが、`directCapacity.heatCapacitor.T` が 293.15 K のまま動かず、参照（t = 0.1 で 309.8 K）から外れた（[research/msl-sim-failures.md](../research/msl-sim-failures.md) の 26）。`GM-I11002` で切り替えを知らせてはいたが、結果は誤り。
- MAP-LIB の参照結果は、Tolerance が experiment annotation にあればその 0.1 倍、無ければ 1e-6 で作られている（[research/openmodelica-dymola.md](../research/openmodelica-dymola.md) の節「S011 で調べた参照結果と比較の方法」。534 本の `creation.txt` を集計）。S012 までの `msl-sim` は注釈の値のまま回していた。S012 の調査で、1/10 にすると `DemonstrateSignalExtrema` と `DemoPowerSupplyWithBuffer` が管に収まると分かっていた。

## 決定

### (a) `auto` は方向微分が作れない失敗では切り替えない

- 組み立て・初期化でも、積分の途中でも、方向微分が作れない失敗（`GM-U0004`）では dopri45 に切り替えない。BDF の失敗をそのまま返す。
- 切り替えるのは、BDF が積分で失敗したとき（`GM-E11002`）と、サンプラが食い違ったとき（`MeSessionError::Contract` の文言）の 2 つだけ（ADR-0014 (d) のまま）。
- RK で回したい人は `--solver dopri45` を明示する（結果は参照と比べて確かめる）。
- 理由: 誤った結果を返すより、未対応（`GM-U0004`）で止まるほうが害が小さい。MSL で救えていたのは 1 本だけで、その 1 本も誤りだった。Rumoca と規則がずれるが、`auto` は GlobeMode の名前であり、Rumoca の CLI と比べるのは `bdf`（型付きの道）で行う（ADR-0014 (a)）。

### (b) `cargo xtask msl-sim` の許容誤差を参照結果と同じ規則にする

- `--tolerance-scale <倍率>` を足す。experiment annotation に数の字面の Tolerance があるモデルだけ、その倍率を掛けた値を `SimSettings::tolerance` に渡す。注釈に無いモデル（と字面でないもの）は既定の 1e-6 のまま（参照結果と同じ規則）。
- `mise run msl:sim` は `--tolerance-scale 0.1` で回す（結果は「結果」の節）。
- `globe simulate` の既定は変えない（experiment annotation の値のまま）。これは比べ方の設定で、利用者の実行の設定ではない。

### 選ばなかった案

- 切り替えた結果に印を付けて残す: `GM-I11002` は既に出しており、それでも結果が使われてしまう。止めるほうが確か。
- 注釈が無いモデルの既定 1e-6 にも倍率を掛ける: 参照結果の作り方とずれる。S013 で最初にこの形で回しかけて気づき、やめた。
- `globe simulate` の既定を注釈の 0.1 倍にする: MLS の experiment annotation の意味（Tolerance はモデルの作者が決める）から外れる。

## 結果

- `mise run msl:sim`（`--tolerance-scale 0.1`、既定 `auto`、時間切れは 180 s で回し直す）を S012 の結果と比べた（[msl-sim.md](../msl-sim.md)、両方にある 356 本）:
  - 実行できた 309 → **308**（−1）。止まったのは `GenerationOfFMUs` で、誤った結果を返さず `GM-U0004` で止まるようになった（(a) の狙いどおり）。
  - 管に収まった 299 → **301**（+2）。`DemonstrateSignalExtrema` と `DemoPowerSupplyWithBuffer` が新しく収まった（(b)）。管に収まらなくなったものは無い。
  - 参照結果と比べた 307 本のうち 301 本（98.0%）。外れた 6 本のうち 3 本は既知の外れの一覧にある（S013 で `DoublePendulumInitTip` を足した。許容誤差を 1/10 にすると 3/4 → 2/4 に悪くなるので、倍率では救えない）。
- `auto` で dopri45 に回し直したのは 15 本、そのうち dopri45 で実行できたのは 8 本。
- 方向微分が作れずに止まる例題は、Rumoca の Auto なら RK で回る。Rumoca の CLI と `auto` の本数が 1 本ずれるのは、この決定による。
