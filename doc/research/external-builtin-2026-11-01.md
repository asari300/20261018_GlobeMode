# `external "builtin"` の数学関数を実行する（S068）

- 確認日: 2026-11-01。Rumoca rev `276e82bd`（v0.10.2）のソース（`~/.cargo/git/checkouts/rumoca-*/276e82b/crates/`）を読んだ。
- 目的: S067 の持ち越し。MSL 4.0.0 の `Modelica.Math.sin` などが `external "builtin" y = sin(u);` で、実行系が断る（`GM-U0004`）。公開ライブラリ 7 つを MSL 4.0.0 で回すと、funnel が OK の例題のうち 70 本がこれだけで実行できない（[libraries-2026-11-01.md](libraries-2026-11-01.md) の「MSL 4.0.0 で回し直した差（S067）」）。

## Rumoca のどこで断るか

| 場所 | 何をするか |
| --- | --- |
| `rumoca-ir-dae/src/model/external_functions.rs` | `ExternalLanguage::{C, Fortran77, Builtin}`。宣言の言語を分類して `ExternalFunctionBody` に持たせる（実行しない） |
| `rumoca-eval-dae/src/numeric.rs` の `external_function_failure` | 数値の評価。external の呼び出しはすべて `NumericEvaluationErrorKind::ExternalFunction`（`external <言語> function '<名前>' calls '<記号>', which this runtime cannot execute`）で断る |
| `rumoca-eval-dae/src/projection.rs` | スカラー射影。同じ文言の `ProjectionError::ExternalFunction` |
| 実行系の lowering（solve-IR） | `EL005`（構造解析の射影）、`EX002`（lowering）。GlobeMode は `GM-U0004` にする |

組み込みの先は DAE にある。`PureBuiltin` に `Sin Cos Tan Asin Acos Atan Atan2 Sinh Cosh Tanh Exp Log Log10 Sqrt Abs` がある（`rumoca-ir-dae/src/expression/nodes.rs`）。

## 道の選び方

小さい順に調べた。

| 道 | 結果 |
| --- | --- |
| (a) Rumoca の公開 API に写す設定や hook | 無い。言語を分類するだけで、実行は全部断る |
| (b) `Dae` を lowering の前に書き換える | **採った**。`Dae` は不変で、構築 API は借用の印付きの閉包（`Dae::construct`）だけ。ただし `Dae` は serde の `Serialize` / `Deserialize` を実装し、読み戻すときに Rumoca が構築と同じ検査をやり直す（`model/wire.rs`）。直列化した値（`serde_json::Value`）の `call` ノードを `builtin` ノードに替えて読み戻せる |
| (c) frontend で MSL 4.0.0 の `Math/package.mo` を文字列置換 | 採らない。版と文字列に結びつき、他のライブラリの同じ書き方に効かない |

[ADR-0029](../adr/0029-external-builtin-rewrite.md) に判断と限界を書いた。

### 書き換えの形

直列化した `Dae` の `storage.functions[i].external` が `{language: "builtin", symbol: "sin", arguments: [{input: e}], result: 0}`、`storage.expressions.nodes` の `call` が `{owner, function: i, output: 0, operand_count: 1, derivative: null}`、`builtin` が `{builtin: "sin", operand_count: 1}`。被演算子は `expressions.operands` に呼び出しの順に詰まっているので、`call` を同じ `operand_count` の `builtin` に替えれば並びは変わらない。置き換えるのは、言語が `builtin`、記号が上の表にある、本体の文が空、入力の数が引数の数に一致、出力が 1 つ、external の引数が「この関数の k 番目の入力」を順に渡すだけの関数。

## 前後の数

### 小さなモデル（`GlobeModeValidation.Simulation.ExternalBuiltin`）

`external "builtin"` の自前の関数 `mySin`（`sin`）と `myExp`（`exp`）を呼ぶ。直す前は `GM-U0004`（`external builtin function 'mySin' calls 'sin', which this runtime cannot execute`）。直した後は bdf・dopri45 で `x(1) = 1 - cos 1`、`z(1) = e` と一致する（`cargo test -p globe-mode-exec --test simulate external_builtin`）。wasm（Node）でも実装のある 20 ソルバすべてがネイティブと一致した（`wasm:compare --models ExternalBuiltin`: 20 / 20 件が一致）。

### MSL 4.0.0 の例題（`globe simulate -L data/msl-4.0.0`、debug ビルド）

| 例題 | 直す前 | 直した後 |
| --- | --- | --- |
| `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum` | `GM-U0004` | 終了時刻 5 まで進んだ |
| `Modelica.Mechanics.MultiBody.Examples.Elementary.Surfaces` | `GM-U0004` | 終了時刻 5 まで進んだ |
| `Modelica.Electrical.Machines.Examples.InductionMachines.IMC_DOL` | `GM-U0004` | 実行できた |
| `Modelica.Electrical.Polyphase.Examples.TransformerYD` | `GM-U0004` | `GM-U0004`（別の原因: `affine derivative coefficient is not compile-time numeric`） |

## 公開ライブラリ（`libraries.yml -f msl=4.0.0`）

@@CI@@

## 限界

- serde の形は Rumoca の公開契約ではない。書き換えが成り立たない（形が変わった）ときは元の `Dae` のまま（`GM-U0004` に戻るだけ）。
- `external_tables`（読み込み済みの表）は直列化の形から外して読み戻し、付け直す（最初の版は表を持つ `Dae` を書き換えず、AixLib の 6 本ほどが直らなかった）。
- 変換は `CompiledModel` を作るとき（`compile_strict`）だけ。`check` の判定（S0〜S10）は元の `Dae` で行うので、判定の数は変わらない。
