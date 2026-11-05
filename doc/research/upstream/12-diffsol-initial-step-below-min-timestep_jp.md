# 下書き（未提出）12: 最初の刻みの見積もりが `min_timestep` を下回り、最初の `step()` が一度も試さずに失敗する

対象: diffsol（<https://github.com/martinjrobins/diffsol>）。英語の本文は [12-diffsol-initial-step-below-min-timestep.md](12-diffsol-initial-step-below-min-timestep.md)、再現は [.mo](12-diffsol-initial-step-below-min-timestep.mo)、1 枚の解説は [_jp.html](12-diffsol-initial-step-below-min-timestep_jp.html)。

判定（GlobeMode、S023。S162 でソースを読んで確認）: **diffsol の穴**。Rumoca v0.10.4 が同梱する diffsol 0.13.2 は v0.10.2 と同一で、diffsol の main（2026-11-02）にも `min_timestep` との比較が無い。実行はしていない【未確認】。

## 概要

`RkState::set_step_size`（`ode_solver/state.rs`、Hairer の見積もり）は `h0 = 0.01 d0 / d1`（y と f の、atol/rtol で重みづけた大きさ）と `h = min(100 h0, h1)` を `min_timestep` と比べずに決める。ほぼ 0 の状態で導関数が大きい状態から（再）開始すると、見積もりが最小の刻みよりずっと小さくなり、最初の `step()` が右辺もヤコビ行列も 1 回も評価せずに「Step size is too small」を返す。Modelica では事象の後の再開で起きる。

## 再現手順

添付の `TinyStateRestart`（`x(start = 3e-11)`、`der(x) = if time < 1 then 0 else 1e5`）を、Hairer の見積もりを使う RK（GlobeMode の ESDIRK34・TR-BDF2・tsit45）で解く。t = 1 の時間事象で `x` が約 3e-11、`der(x)` が 1e5 の状態から再開する。diffsol 単体のプログラムは【未作成】。MSL の形は `Modelica.Electrical.Analog.Examples.CauerLowPassSC` を esdirk34 で（t = 1.05 で止まる。`h` は約 2e-16、t ≈ 1 の `min_timestep` は 1e-13）。

## 期待

見積もりを `min_timestep` で下から抑える（または最初の刻みを試して普通に失敗する）。

## 実際

`Step size is too small at time = 1.05`。刻みを 1 回も試さない。t = 1 の再開では状態が厳密に 0 なので `h0 = 1e-6` の枝を通り、問題が隠れていた。

## ソースの再確認（S162）

- Rumoca v0.10.4 が同梱する diffsol 0.13.2 の `ode_solver/state.rs` は、v0.10.2・Rumoca main（`590ac739`）とバイト単位で同一。
- diffsol の main（`883d9a09`、2026-11-02。`crates/diffsol`）の `set_step_size` も同じ Hairer の見積もりで、`min_timestep` との比較は無い。`min_timestep` の既定がスカラのイプシロンになった（PR #402、2026-10-23）ので閾値は変わるが、下から抑える処理は無い。
- diffsol の最新のリリースは diffsol-v0.18.0（2026-10-20）。

## 環境

- diffsol 0.13.2（Rumoca v0.10.4 の `vendor/diffsol/`）。GlobeMode の SDIRK/RK plugin（`globe-mode-exec`、`esdirk34` / `trbdf2`）から使う。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。
- diffsol 本家での実行は【未確認】。出す前に diffsol 単体のプログラムに縮め、main で確かめる。

## GlobeMode 側の回避

あり（S023）。`initial_state()` の終わりで、見積もりが `min_timestep` の 10 倍より小さければ 10 倍に持ち上げる（符号は保つ）。刻みは受理のたびに伸びる（2e-13 から 2e-7 へ 12 歩ほど）。これで `CauerLowPassSC` が esdirk34・trbdf2 で通る（BDF との差は相対で最大 5.4e-9 と 1.2e-8）。

## 備考

- 10・11 は diffsol の main で直っている（10 は確実、11 は直っているように見える）。12 は残る唯一の diffsol の件。出すなら diffsol 単体のプログラムを添えて diffsol に出す。
