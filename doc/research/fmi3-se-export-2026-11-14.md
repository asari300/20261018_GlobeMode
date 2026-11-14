# FMI 3.0 Scheduled Execution（SE）の Export の調査（S251）: クロックの無い連続モデルを「周期クロック 1 つ・パーティション 1 つ」で出せるか

- 確認日: 2026-11-14（特に断らない限り）
- 目的: [ADR-0031](../adr/0031-fmi-export-p5a-first.md) の持ち越し「FMI 3.0 の SE」の最初の一切れを決める。結果は [ADR-0031 の S251 の追記](../adr/0031-fmi-export-p5a-first.md)。
- 出典はすべて fmi-standard の tag `v3.0.2` のソース（`docs/*.adoc`・`headers/`・`schema/`）。`https://raw.githubusercontent.com/modelica/fmi-standard/v3.0.2/` の下を取って読んだ。組版後の HTML（<https://fmi-standard.org/docs/3.0.2/>）は読んでいない【未確認】。

## 1. 結論

| 問い | 答え |
| --- | --- |
| SE の FMU に Clock は要るか | **要る**。「FMU は関連する Clock を `modelDescription.xml` に定義することでモデルパーティションを公開する」「入力 Clock ごとに 1 つのパーティション」（`docs/5_2_scheduled_execution_api.adoc`、`5_4_scheduled_execution_schema.adoc`）。Clock が無いとパーティションが無く、importer が動かす対象が無い |
| クロックの無い連続モデルは出せるか | 出せる（形としては）。周期の入力 Clock（`intervalVariability="constant"`・`intervalDecimal`）を 1 つ足し、そのパーティション 1 つが `fmi3ActivateModelPartition(clock, t)` で前の時刻から `t` まで内部で積分する。仕様の SE は離散時間の計算（`f_activate(•x_d, w_d, u_d, p, k_i)`、`5_2` の表）を想定していて、連続の状態を FMU の中で積分する使い方を明示に認める・禁ずる文は読んだ範囲に無い【未確認】。XSD は変数の型・variability と SE を結び付けていない（`fmi3Variable.xsd`、`fmi3AttributeGroups.xsd`） |
| `priority` | SE ではすべての Clock で**必須**（`5___scheduled_execution.adoc`「For Scheduled Execution this attribute is mandatory for every Clock」）。XSD 上は optional（`fmi3ClockAttributes`）なので、書く側が守る |
| 周期 Clock の属性 | `intervalVariability`（XSD で必須。`constant`・`fixed`・`tunable`・`changing`・`countdown`・`triggered`）、`intervalDecimal`、`shiftDecimal`（既定 0）、`priority`。`<ScheduledExecution>` は共通の capability flag 以外の属性を持たない（`5_4`）。`modelIdentifier` は必須 |
| 呼び出しの形 | `fmi3InstantiateScheduledExecution(instanceName, instantiationToken, resourcePath, visible, loggingOn, instanceEnvironment, logMessage, clockUpdate, lockPreemption, unlockPreemption)`（`headers/fmi3FunctionTypes.h`）。`fmi3ActivateModelPartition(instance, clockReference, activationTime)`。同じ Clock・同じ時刻で 2 度呼んではならず、同じ Clock への連続の呼び出しは `activationTime` が**狭義単調増加**（`5_2`）。`Discard` は `Error` と同じに扱う |
| 変数の読み書き | `fmi3Set*` → `fmi3ActivateModelPartition` → `fmi3Get*` の順を推奨。`fmi3Get*` は `fmi3Set*` の後に `ActivateModelPartition` を挟まず呼んではならない。重い計算は `ActivateModelPartition` の中に置く（`5_2`） |
| 出力 Clock・countdown Clock | モデルが `fmi3ClockUpdateCallback` を呼ぶ道。1 つの周期 Clock だけなら要らない（この最初の一切れでは使わない） |
| fmusim で回せるか | **回せない**。v0.15.0（`docs/compatibility.md`）も v0.16.0（最新、2026-11-14 に確認）も「Model Exchange and Co-Simulation」だけ。Reference-FMUs の `Clocks.fmu`（SE）は fmusim の対象外【推測。fmusim のソース `simulate/fmi3.rs` に Scheduled の語が無いこと（v0.15.0）から】 |
| FMPy で回せるか | `simulate_fmu` は SE を扱わない（`src/fmpy/__init__.py` v0.3.32 の `fmi_types` は CoSimulation・ModelExchange だけ）。**低水準の `fmpy.fmi3.FMU3ScheduledExecution`**（`src/fmpy/fmi3.py` の 1987 行目付近。`instantiate`・`activateModelPartition`）が使える。GlobeMode の試験はこれを Python から直接呼ぶ |
| Rumoca のテンプレート（P5a）に SE はあるか | ADR-0031 のとおり `fmi2`・`fmi3` は ME と CS のみ（manifest の `[capabilities]` は事象・クロックを扱わない）。P5a で SE を出す道は無い |

## 2. 参考にした実物: Reference-FMUs v0.0.42 の `Clocks.fmu`

手元の `data/reference-fmus/3.0/Clocks.fmu`（[fmi-2026-11-01.md](fmi-2026-11-01.md) の取得物）の `modelDescription.xml` は `<ScheduledExecution modelIdentifier="Clocks"/>` と、`<Clock name="inClock1" valueReference="1001" causality="input" intervalVariability="constant" intervalDecimal="1.0" priority="0"/>` ほか。Clock の値参照は変数の値参照と別の番号帯（1001〜）に置いてある。`ModelVariables` の `time` は `independent` の Float64。GlobeMode の最初の一切れはこの `constant` の入力 Clock の 1 つだけを書く。

## 3. 最初の一切れの決め方（推測を含む）

- **P5b（自前のランタイム）で出す**。1 つの周期 Clock に 1 つのパーティションを割り当て、`fmi3ActivateModelPartition` の中で CS の `do_step` と同じもの（固定刻みの RK4 と事象処理）を回す。CS・ME と同じ cdylib・同じ `resources/*.mo` の道を使えるので、新しい計算は要らない。
- Clock の周期は FMU が決める（`intervalDecimal`）。importer が `activationTime` を並べる。周期は `experiment(Interval)`、無ければ 0.01。刻みは importer が選ぶ（FMU の積分の内部の刻みは CS と同じ上限 1e-3）。
- 持ち越し: 複数の Clock・パーティション、出力 Clock、countdown / triggered の Clock、`fmi3GetClock`・`fmi3GetIntervalDecimal`、`lockPreemption`・`unlockPreemption`、P5a（Rumoca）での SE、fmusim と fmuCheck の検査（回せない・開発終了）。
