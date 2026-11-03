# 下書き（未提出）33: AixLib の `CHPCombustionMixtureGasNasa.T_psX` が宣言の無い `specificEntropyOfpTX` を呼ぶ

対象: **AixLib**（<https://github.com/RWTH-EBC/AixLib>）。英語の本文は [33-aixlib-specific-entropy-of-ptx-missing.md](33-aixlib-specific-entropy-of-ptx-missing.md)、再現は [.mo](33-aixlib-specific-entropy-of-ptx-missing.mo)、1 枚の解説は [_jp.html](33-aixlib-specific-entropy-of-ptx-missing_jp.html)。

判定（GlobeMode、S113）: **ライブラリの誤り**（宣言の無い関数の呼び出し）。OpenModelica 1.27.2 も Rumoca も、これを呼ぶモデルを断る。

## 概要

`AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa` は MSL の `Modelica.Media.IdealGases.Common.MixtureGasNasa` から `T_psX` を写した。その入れ子の関数 `f_nonlinear` は `specificEntropyOfpTX(p=p, T=u, X=X)` を呼ぶ（670 行と、676 行の `inverse` の注釈）。ところが `specificEntropyOfpTX` は `CHPCombustionMixtureGasNasa` にも、継いでいる `PartialMixtureMedium`・`PartialMedium` にも無い。MSL では `MixtureGasNasa` 自身の protected の関数で、`CHPCombustionMixtureGasNasa` は `MixtureGasNasa` を継いでいない。

このため、CHP の媒質（`CHPFlueGasLambdaOnePlus`、`NaturalGasMixture_*`、`LiquidFuel_*`）で `setState_psX`（89〜100 行）と、それを通る既定の関数（`specificEnthalpy_psX`、`temperature_psX`、`density_psX`、`exact = true` の `isentropicEnthalpy`）を呼ぶと、どのツールでも失敗する。

OpenModelica の出力:

```text
[.../AixLib/DataBase/CHP/ModularCHPEngineMedia/CHPCombustionMixtureGasNasa.mo:670:7-670:51:writable] Error: Function specificEntropyOfpTX not found in scope f_nonlinear.
```

## 再現手順

AixLib 3.0.1（または main。環境を参照）と MSL 4.0.0 で次のモデルを検査する。

```modelica
model S113AixCheck
  package Medium = AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPFlueGasLambdaOnePlus;
  Medium.ThermodynamicState st = Medium.setState_psX(1e5, 7000 + time, Medium.reference_X);
end S113AixCheck;
```

```text
setModelicaPath("<MSL 4.0.0>");
loadFile("<AixLib>/AixLib/package.mo");
loadFile("S113AixCheck.mo");
checkModel(S113AixCheck);
```

`setState_psX` を `setState_pTX(1e5, 300 + time, Medium.reference_X)` に変えると通る（対照）。

AixLib なしの最小の形は添付の `.mo`。`Rep41.Original` が `MixtureGasNasa`（protected の `specificEntropyOfpTX` あり）、`Rep41.Copy` が `CHPCombustionMixtureGasNasa`（`T_psX` だけ写した）に当たる。

```bash
omc: loadFile("33-aixlib-specific-entropy-of-ptx-missing.mo"); checkModel(Rep41.M); checkModel(Rep41.M0);
rumoca compile 33-aixlib-specific-entropy-of-ptx-missing.mo -m Rep41.M    # ER002
rumoca compile 33-aixlib-specific-entropy-of-ptx-missing.mo -m Rep41.M0   # 対照
```

## 期待

`specificEntropyOfpTX` が `f_nonlinear` から見える所に宣言されている。関数の呼び出しは呼ぶ側の有効範囲から通常の探索で引く（MLS 3.6 §5.3・§12.4、<https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html>・<https://specification.modelica.org/maint/3.6/functions.html>、確認日 2026-11-03）。ここでは囲むどのクラスにも宣言が無い。

直し方の候補（試していない【未確認】）:

- MSL の protected の関数（MSL 4.0.0 の `Modelica/Media/IdealGases/Common/package.mo`、`MixtureGasNasa.specificEntropyOfpTX`、1340 行）を `CHPCombustionMixtureGasNasa` に写す。
- `T_psX.f_nonlinear` で、パッケージが既に redeclare している `specificEntropy(setState_pTX(p, u, X))` を呼ぶ。

## 実際

- OpenModelica 1.27.2 の `checkModel(S113AixCheck)`: 上のエラー（AixLib 3.0.1、MSL 4.0.0、確認日 2026-11-03）。`setState_pTX` の対照は completed successfully。
- `Rep41.M`: OpenModelica は `Function specificEntropyOfpTX not found in scope f_nonlinear.`、Rumoca v0.10.2・v0.10.4 は `[ER002] unresolved function call: 'specificEntropyOfpTX'`。`Rep41.M0` はどちらも通る。

## 環境

- AixLib 3.0.1（タグ v3.0.1、コミット `e5df9a8da27833ee391c2f7a0adbcd80278f47d2`、2026-06-23）、MSL 4.0.0。
- **AixLib の main（2026-11-03）は同じコミット `e5df9a8d` で、`CHPCombustionMixtureGasNasa.mo` も同一（670 行は今も `specificEntropyOfpTX` を呼ぶ）。直っていない。** このファイルの最後の変更は 2022-10-04（`7d576dbf11`、「Fix equations and uses #1336」）。
- OpenModelica 1.27.2、Rumoca v0.10.2・v0.10.4（配布バイナリ）。
- Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

対象外（ライブラリの誤り）。GlobeMode では、Rumoca の下書き 32 のふるまいを通してだけ現れる。AixLib 全体を読むと、CHP の媒質を使わない 39 本の例題がこの呼び出しの ER002 で止まる。`globe check` はモデルの閉包だけ読むので出ない（GlobeMode の ADR-0025）。

## 備考

- AixLib のモデルで CHP の媒質の `setState_psX` などの s を使う関数を呼ぶものは無い（AixLib 3.0.1 の `Fluid/BoilerCHP` と `DataBase/CHP` を検索）。そのため AixLib の例題は OpenModelica・Dymola では動く。落ちるのは、それらを呼ぶ利用者のモデルと、パッケージそのものの検査だけ。
- AixLib は `AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa` を既に `ci/whitelist/om_check_whitelist.txt` と `ci/whitelist/dymola_check_whitelist.txt`（main、どちらも 49 行、確認日 2026-11-03）に載せている。issue #1291（「Incorrect AixLib models in connection with the IBPSA merge」、2024-07-26 に閉じた、<https://github.com/RWTH-EBC/AixLib/issues/1291>）の一覧にもある。その検査がこの理由で落ちているかは確かめていない【未確認】。`specificEntropyOfpTX` や `T_psX` を名指しする issue は見つからなかった（GitHub の issue 検索、2026-11-03）。
- 出すか、出すなら「既にホワイトリストにある件の原因と直し方」として出すかはユーザーが決める。
