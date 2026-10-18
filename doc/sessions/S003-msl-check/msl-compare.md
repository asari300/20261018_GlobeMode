# MSL 4.1.0 での globe check と Rumoca の CLI の突き合わせ

`cargo xtask msl-compare` が生成した。手で編集しない。

| | |
| --- | --- |
| MSL | 4.1.0（sha256 `5409cfa17797…`） |
| Rumoca | CLI v0.10.2（GlobeMode の依存も同じ rev `276e82bd`） |
| 結果 | **20 本中 20 本が一致（両方とも成功 17 本）** |

- 一致の判定: 成否が同じで、両方とも成功したときは DAE の（状態 / 代数 / 出力）と連続方程式の数が同じ。DAE の数は、配列を 1 個として数える（Rumoca の CLI の `-v` と同じ粒度）。
- 連続方程式の数は、Rumoca の CLI が `-v` で表示する `[rumoca] Continuous equations (f_x)` と同じ数え方（`--emit dae-json` の `continuous_equation_operations` の数ではない。こちらは操作の数で、配列や for を含むと多くなる）。
- 「スカラー」は配列を展開した方程式数 = 変数数で、S5 の釣り合いの検査に使う数。GlobeMode だけが表示する。
- 失敗の欄は、GlobeMode は「段階 GM コード（Rumoca のコード）」、Rumoca の CLI は「終了コード（エラーコード）」。

| # | モデル | globe check（状態 / 代数 / 出力） | Rumoca の CLI（状態 / 代数 / 出力） | 判定 |
| --- | --- | --- | --- | --- |
| 1 | `Electrical.Analog.Examples.ChuaCircuit` | OK 3 / 41 / 0・連続方程式 44（スカラー 44 = 44） | OK 3 / 41 / 0・連続方程式 44 | 一致 |
| 2 | `Electrical.Analog.Examples.CauerLowPassAnalog` | OK 7 / 61 / 1・連続方程式 69（スカラー 69 = 69） | OK 7 / 61 / 1・連続方程式 69 | 一致 |
| 3 | `Electrical.Analog.Examples.HeatingRectifier` | OK 2 / 46 / 1・連続方程式 49（スカラー 49 = 49） | OK 2 / 46 / 1・連続方程式 49 | 一致 |
| 4 | `Electrical.Analog.Examples.CharacteristicIdealDiodes` | OK 0 / 74 / 3・連続方程式 77（スカラー 80 = 80） | OK 0 / 74 / 3・連続方程式 77 | 一致 |
| 5 | `Electrical.Analog.Examples.Rectifier` | OK 5 / 105 / 8・連続方程式 119（スカラー 128 = 128） | OK 5 / 105 / 8・連続方程式 119 | 一致 |
| 6 | `Mechanics.Rotational.Examples.First` | OK 8 / 45 / 1・連続方程式 54（スカラー 54 = 54） | OK 8 / 45 / 1・連続方程式 54 | 一致 |
| 7 | `Mechanics.Rotational.Examples.CoupledClutches` | NG S5 GM-E5002 (ED001) | NG exit 1 (ED001) | 一致 |
| 8 | `Mechanics.Rotational.Examples.Friction` | NG S5 GM-E5002 (ED001) | NG exit 1 (ED001) | 一致 |
| 9 | `Mechanics.Translational.Examples.SignConvention` | OK 6 / 32 / 3・連続方程式 41（スカラー 41 = 41） | OK 6 / 32 / 3・連続方程式 41 | 一致 |
| 10 | `Mechanics.Translational.Examples.Oscillator` | OK 5 / 43 / 2・連続方程式 50（スカラー 50 = 50） | OK 5 / 43 / 2・連続方程式 50 | 一致 |
| 11 | `Mechanics.Translational.Examples.Damper` | OK 9 / 50 / 0・連続方程式 59（スカラー 59 = 59） | OK 9 / 50 / 0・連続方程式 59 | 一致 |
| 12 | `Thermal.HeatTransfer.Examples.TwoMasses` | OK 2 / 16 / 2・連続方程式 20（スカラー 20 = 20） | OK 2 / 16 / 2・連続方程式 20 | 一致 |
| 13 | `Thermal.HeatTransfer.Examples.Motor` | OK 2 / 32 / 4・連続方程式 37（スカラー 41 = 41） | OK 2 / 32 / 4・連続方程式 37 | 一致 |
| 14 | `Blocks.Examples.PID_Controller` | OK 10 / 59 / 17・連続方程式 83（スカラー 89 = 89） | OK 10 / 59 / 17・連続方程式 83 | 一致 |
| 15 | `Blocks.Examples.Filter` | NG S5 GM-E5001 (ED019) | NG exit 1 (ED019) | 一致 |
| 16 | `Blocks.Examples.InverseModel` | OK 3 / 7 / 5・連続方程式 15（スカラー 15 = 15） | OK 3 / 7 / 5・連続方程式 15 | 一致 |
| 17 | `Electrical.Machines.Examples.DCMachines.DCPM_Start` | OK 13 / 159 / 28・連続方程式 200（スカラー 200 = 200） | OK 13 / 159 / 28・連続方程式 200 | 一致 |
| 18 | `Mechanics.MultiBody.Examples.Elementary.Pendulum` | OK 7 / 255 / 0・連続方程式 87（スカラー 934 = 934） | OK 7 / 255 / 0・連続方程式 87 | 一致 |
| 19 | `Fluid.Examples.Tanks.EmptyTanks` | OK 4 / 152 / 2・連続方程式 61（スカラー 138 = 138） | OK 4 / 152 / 2・連続方程式 61 | 一致 |
| 20 | `StateGraph.Examples.FirstExample` | OK 0 / 8 / 2・連続方程式 10（スカラー 55 = 55） | OK 0 / 8 / 2・連続方程式 10 | 一致 |
