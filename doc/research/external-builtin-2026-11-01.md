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

直列化した `Dae` の `storage.functions[i].external` が `{language: "builtin", symbol: "sin", arguments: [{input: e}], result: 0}`、`storage.expressions.nodes` の `call` が `{owner, function: i, output: 0, operand_count: 1, derivative: null}`、`builtin` が `{builtin: "sin", operand_count: 1}`。被演算子は `expressions.operands` に呼び出しの順に詰まっているので、`call` を同じ `operand_count` の `builtin` に替えれば並びは変わらない。置き換えるのは、言語が `builtin`、記号が上の表にある、本体の文が空、入力の数が引数の数に一致、出力が 1 つ、external の引数がすべて入力で、番号が増える順（`atan2(u2, u1)` のような入れ替えを弾く。式の番号は Rumoca の内部の番号とずれるので、入力そのものかまでは確かめない）の関数。

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

`gh workflow run libraries.yml --ref feature/s068-external-builtin -f msl=4.0.0 -f 'libraries=["DeltaRobot","IBPSA","AixLib"]'`（Buildings は 70 分かかるので外した）。実行できた本数（funnel が OK の例題のうち）:

| ライブラリ | 4.0.0 の実行できた（S067、run 41295445820） | 4.0.0 + 今回（run 41313259246） | 4.1.0（S058 の run） |
| --- | ---: | ---: | ---: |
| AixLib（OK 118） | 71 | **97** | 98（OK 119） |
| IBPSA（OK 88） | 47 | **66** | 66 |
| DeltaRobot（OK 9） | 2 | **5** | 5 |
| 計 | 120 | **168** | 169 |

- 判定（OK / NG / GM-U / panic）は S067 の 4.0.0 と同じ（AixLib 118 / 360 / 164 / 0、IBPSA 88 / 230 / 135 / 4、DeltaRobot 9 / 7 / 5 / 0）。`check` は元の `Dae` で行うので変わらない。
- IBPSA と DeltaRobot は 4.1.0 の本数に届いた。AixLib は 4.1.0 の 98 に対し 97 で、4.1.0 で OK だった 1 本（`SensorVerification`。4.0.0 では ED019 で OK でなくなる。S067）の差を除くと同じ。AixLib は funnel が OK の 118 本のうち 21 本が実行できないままで、4.1.0 でも同じ原因（`Modelica.Math` 以外）。
- **S067 が「残り 5 本」とした例題**: DeltaRobot の 3 本（`axesRotations` の record の戻り値）は、今回の変換で実行できるようになった（DeltaRobot 2 → 5、4.1.0 と同じ）。S067 は別の原因と書いたが、`axesRotations` が `Modelica.Math` を呼んでいたと思われる【呼び出しの確認はしていない】。Buildings の `HeatTransfer.Convection.Examples.Interior`・`HeatTransfer.Windows.BaseClasses.Examples.InteriorConvection` の 2 本（`invalid Solve IR contract: pure-call argument or slot interface is invalid`）は Buildings を回し直していないので未確認。Buildings の 27 本も CI で回し直していない。

### CI を 4 回回した経緯

| run | 結果 | 見つけたこと |
| --- | --- | --- |
| 41307189067 | AixLib 642 本が全部 NG（GM-E1001 / EM001）、IBPSA 66、DeltaRobot 5 | **S066 の退行**。`RadiatorTypes.mo` と package.mo の無い `RadiatorTypes/` が並ぶ配置を、読み飛ばしの道が両方読んで二重のクラスになっていた。S066 の確認に AixLib は入っていなかった |
| 41309463666 | AixLib 92 | 退行を直した後。表（CombiTable）を持つ `Dae` を書き換えていなかった（`HumansSenTDep` など） |
| 41312015621 | AixLib 94 | 表の対応後。直列化の式の番号が Rumoca の内部の番号とずれ、番号で式を引く確認が失敗する関数があった（`Declination` の `asin`・`cos`） |
| 41313259246 | AixLib 97、IBPSA 66、DeltaRobot 5 | 番号の大小だけを見る確認に変えた後 |


## 限界

- serde の形は Rumoca の公開契約ではない。書き換えが成り立たない（形が変わった）ときは元の `Dae` のまま（`GM-U0004` に戻るだけ）。
- `external_tables`（読み込み済みの表）は直列化の形から外して読み戻し、付け直す（最初の版は表を持つ `Dae` を書き換えず、AixLib の 6 本ほどが直らなかった）。
- 変換は `CompiledModel` を作るとき（`compile_strict`）だけ。`check` の判定（S0〜S10）は元の `Dae` で行うので、判定の数は変わらない。
