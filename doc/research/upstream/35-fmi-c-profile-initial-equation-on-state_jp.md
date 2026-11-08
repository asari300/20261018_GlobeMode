# 下書き（未提出）35: `fmi2` / `fmi3` の C profile が状態への `initial equation` を断る（"rows over declaration seeds"）

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [35-fmi-c-profile-initial-equation-on-state.md](35-fmi-c-profile-initial-equation-on-state.md)、再現は [.mo](35-fmi-c-profile-initial-equation-on-state.mo)、1 枚の解説は [_jp.html](35-fmi-c-profile-initial-equation-on-state_jp.html)。

判定（GlobeMode、S237）: **ふつうのモデルが当たる限界**（誤った結果ではなく、書き出しを断る）。状態 `T` への `initial equation T = T_start;` は `T(start = T_start, fixed = true)` の標準的な別の書き方で、後者は書き出せる。Rumoca v0.10.2（rev `276e82bd`、ライブラリ）と v0.10.4 の配布バイナリ（`rumoca compile --target fmi2`）で再現する。

## 概要

状態をパラメータから決める `initial equation` があると、`fmi2` / `fmi3` のソースの FMU を書き出せない。

```text
FMI C profile: C initialization evaluates residual rows only on the settled
algebraic refresh; rows over declaration seeds need row isolation and a forward
initial Jacobian
```

メッセージは `rumoca-ir-solve/src/fmi/parameter_updates.rs` の `validate_projection` から出る。初期化に残差の行があり、そのどれも `UnownedCoordinate(Algebraic)`・`SolvedThroughAlgebraicRefresh`・`SurplusAlgebraicCheck` でないとき。`T(start = T_start, fixed = true)` なら通る（同じファイルの、start を設定済みのパラメータから代入する枝）。

MSL では `Modelica.Thermal.HeatTransfer.Components.HeatCapacitor`（`initial equation if initType == FixedInitial then T = T_start`）がこの形で、それを使うモデルが広く当たる。GlobeMode が MSL 4.1.0 の小さい例題 20 本を調べたところ、6 本が同じファイルの初期化の 3 つのメッセージのどれかで断られた（`rows over declaration seeds` 2、`can only assign parameters and state starts` 3、`cannot run the homotopy continuation` 1）。この下書きは 1 つめ。

## 再現手順

```bash
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.A --target fmi2   # 対照: fixed の start、書き出せる
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.E --target fmi2   # 状態 1 つ、initial equation T = T_start
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.F --target fmi2   # 2 つの熱容量をつないだもの
```

## 期待

`E`・`F` も `A` と同じく書き出せる。左辺が状態で右辺がパラメータだけの `initial equation` は、MLS 8.6（初期化）では、その式を `start` にした `fixed = true` と同じ条件。C profile は後者を受けている。

## 実際

`A`: `FMI 2 ME+CS source FMU written`。`E`・`F`: 上のメッセージ（v0.10.4 の配布バイナリ、v0.10.2 のライブラリ）。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`、ライブラリ）と v0.10.4 の配布バイナリ（2026-11-08）。
- Linux x86_64（WSL2、Ubuntu）。main は【未確認】。

## GlobeMode 側の回避

一部。同じモデルは GlobeMode 自前のランタイムの FMU（P5b、`globe-mode-fmu`。初期化は `globe-mode-exec`）では回る。`E` は `T(0.9) = 94.176`（`100 * exp(-0.9 / 15)`）。P5a（Rumoca のテンプレート）は今も断る。

## 備考

- 同じ関数のほかの 2 つのメッセージ（`can only assign parameters and state starts`、`cannot run the homotopy continuation`）は `Modelica.StateGraph.Examples.FirstExample` と `Modelica.Blocks.Examples.PID_Controller` が当たる。最小のモデルは作っていない【未確認】。
- C の射影の前に DAE を書き換える（`state = expr(パラメータ)` を `fixed = true` の start にする）案もあるが、Rumoca の設計に合うかは開発者の判断。
