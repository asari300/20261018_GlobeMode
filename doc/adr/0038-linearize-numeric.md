# ADR-0038 線形化は `ModelInstance` の上の数値の中心差分で行い、解析ヤコビ行列は使わない

- 状態: Accepted（2026-11-15、S254）
- 関係: [ADR-0032](0032-p5b-own-runtime.md) 系の `ModelInstance`（状態・入力を外から与えて微分を返す口。S099・S114）、[features.md](../features.md) の線形化（P8）

## 背景

P8 の線形化（features.md「A/B/C/D」）は「Rumoca の機能をアダプタ経由で使う」と書いていた。実際に使える入口を調べると、Rumoca の公開 API（`SolveRuntime`）には微分と出力を評価する関数はあるが、状態・入力に対するヤコビ行列や `linearize` に当たる公開の関数は、Rumoca の rev `276e82b` の `pub fn *jacobian*`・`pub fn linearize` の検索（2026-11-15）で見つからなかった（出てくるのは vendor の diffsol の積分器用のもので、DAE の A・B・C・D ではない）。

## 決定

1. **数値の中心差分で A・B・C・D を出す。** `ModelInstance::linearize`（`globe-mode-exec`）が、状態と入力（top-level の `input` の Real）を 1 つずつ `±h` ずらし、`derivatives`（`dx/dt = f`）と `variable_values`（top-level の `output` の Real = `y`）の差を `2h` で割る。`h = eps^(1/3)·max(1, |v|)`（中心差分の最適な刻み。代数ループの許容誤差 1e-10 を `√eps` の刻みで割ると 1e-2 の雑音になるので `√eps` は使わない）。代数変数は `derivatives`・`variable_values` が毎回解き直すので、線形化の側は触らない。
2. **線形化の点は「初期化の直後」か「そこまで実行した状態」。** `Workspace::linearize(model, overrides, time)` は、`time` が無いか開始時刻なら `ModelInstance::new` の直後の点。後の時刻なら開始時刻から `time` まで `run` で実行し、最後の状態の値を `set_states`・`set_time` で置く。入力を外から駆動するモデルは実行できない（GM-U0004）ので後の時刻は使えない。平衡点を求めること（trim）はしない。
3. **口は facade と CLI だけ。** `globe linearize [files] -m <model> [-L lib] [--time T] [--param NAME=VALUE] [--json]`。JSON は `{model, time, x, u, y, x0, u0, y0, A, B, C, D}`（行列は行ごとの配列）。Python・UI・LSP は次の WP。

## 結果

- 良い: Rumoca の内部に触らず、試験で解析的に分かる行列（線形系、振り子 `A = [0 1; -cos θ -c]`）が 1e-6 で合う。同じ口で FMU の `ModelInstance` を使うので、後から Python などに同じ関数を出せる。
- 悪い: 誤差は 1e-8 程度で、事象の境（不連続）をまたぐ点では値が合わない。状態の数だけ 2 回ずつ評価するので、状態が数千のモデルでは遅い（解析ヤコビ行列なら 1 回）。後の時刻の線形化は、入力のあるモデルで使えない。入力・出力は Real だけ（Integer・Boolean・配列の要素は含まない）。
- 解析ヤコビ行列を使える公開の道が Rumoca に出たら、同じ `Linearization` を返す実装に差し替える。
