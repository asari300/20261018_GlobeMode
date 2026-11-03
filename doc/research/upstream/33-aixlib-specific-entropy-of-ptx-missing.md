# DRAFT (not filed) - `CHPCombustionMixtureGasNasa.T_psX` calls `specificEntropyOfpTX`, which is not declared (copied from MSL without its protected helper)

Target: AixLib (https://github.com/RWTH-EBC/AixLib)

Verdict (GlobeMode, S113): **library bug** (a call to an undeclared function). OpenModelica 1.27.2 and Rumoca reject a model that calls it.

## Summary

`AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa` copies `T_psX` from MSL's `Modelica.Media.IdealGases.Common.MixtureGasNasa`. Its nested function `f_nonlinear` calls `specificEntropyOfpTX(p=p, T=u, X=X)` (line 670, and the `inverse` annotation on line 676), but `specificEntropyOfpTX` is not declared in `CHPCombustionMixtureGasNasa`, nor in `Modelica.Media.Interfaces.PartialMixtureMedium` or `PartialMedium` that it extends. In MSL it is a protected function of `MixtureGasNasa` itself, which `CHPCombustionMixtureGasNasa` does not extend. So every call of `setState_psX` (line 89 to 100), and of the inherited defaults that go through it (`specificEnthalpy_psX`, `temperature_psX`, `density_psX`, `isentropicEnthalpy` with `exact = true`), fails for all the CHP media (`CHPFlueGasLambdaOnePlus`, `NaturalGasMixture_*`, `LiquidFuel_*`).

OpenModelica:

```text
[.../AixLib/DataBase/CHP/ModularCHPEngineMedia/CHPCombustionMixtureGasNasa.mo:670:7-670:51:writable] Error: Function specificEntropyOfpTX not found in scope f_nonlinear.
```

## Steps to reproduce

With AixLib 3.0.1 (or main, see Environment) and MSL 4.0.0, check this model:

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

The same model with `setState_pTX(1e5, 300 + time, Medium.reference_X)` instead checks successfully.

The attached `33-aixlib-specific-entropy-of-ptx-missing.mo` reduces the shape without AixLib: `Rep41.Original` stands for `MixtureGasNasa` (with the protected `specificEntropyOfpTX`), `Rep41.Copy` for `CHPCombustionMixtureGasNasa` (only `T_psX` copied):

```bash
omc: loadFile("33-aixlib-specific-entropy-of-ptx-missing.mo"); checkModel(Rep41.M); checkModel(Rep41.M0);
rumoca compile 33-aixlib-specific-entropy-of-ptx-missing.mo -m Rep41.M    # ER002
rumoca compile 33-aixlib-specific-entropy-of-ptx-missing.mo -m Rep41.M0   # control
```

## Expected

`specificEntropyOfpTX` is declared where `f_nonlinear` can find it. A function call is looked up by the normal name lookup from the calling scope (MLS 3.6 section 5.3 and section 12.4, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html and https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-03); here no enclosing class declares it.

A possible fix is to copy MSL's protected function into `CHPCombustionMixtureGasNasa` (MSL 4.0.0, `Modelica/Media/IdealGases/Common/package.mo`, `MixtureGasNasa.specificEntropyOfpTX`, line 1340), or to let `T_psX.f_nonlinear` call `specificEntropy(setState_pTX(p, u, X))`, which the package already redeclares. Not tried. [not verified]

## Actual

- OpenModelica 1.27.2, `checkModel(S113AixCheck)`: the error above (AixLib 3.0.1, MSL 4.0.0, checked 2026-11-03). The control with `setState_pTX` "completed successfully".
- `Rep41.M`: OpenModelica gives `Function specificEntropyOfpTX not found in scope f_nonlinear.`; Rumoca v0.10.2 and v0.10.4 give `[ER002] unresolved function call: 'specificEntropyOfpTX'`. `Rep41.M0` passes in both.

## Environment

- AixLib 3.0.1 (tag v3.0.1, commit `e5df9a8da27833ee391c2f7a0adbcd80278f47d2`, 2026-06-23), MSL 4.0.0.
- AixLib main on 2026-11-03 is the same commit `e5df9a8d`, and `CHPCombustionMixtureGasNasa.mo` there is identical (line 670 still calls `specificEntropyOfpTX`). The file was last changed on 2022-10-04 (`7d576dbf11`, "Fix equations and uses #1336").
- OpenModelica 1.27.2; Rumoca v0.10.2 and v0.10.4 (prebuilt `rumoca-linux-x86_64`).
- Linux x86_64 (WSL2, Ubuntu).

## Workaround in GlobeMode

Not applicable (library bug). In GlobeMode it shows up only through the Rumoca behaviour of draft 32: with the whole of AixLib loaded, 39 examples that never use a CHP medium stop with ER002 on this call. `globe check` reads only the closure of the model (GlobeMode ADR-0025) and does not hit it.

## Notes

- No AixLib model calls `setState_psX` or the other s-based functions of a CHP medium (searched in AixLib 3.0.1, `Fluid/BoilerCHP` and `DataBase/CHP`), so the AixLib examples run in OpenModelica and Dymola. Only a user model that calls them, and the check of the package itself, fail.
- AixLib already lists `AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa` in `ci/whitelist/om_check_whitelist.txt` and `ci/whitelist/dymola_check_whitelist.txt` (line 49 of each, main, checked 2026-11-03), and it is in the list of issue #1291 ("Incorrect AixLib models in connection with the IBPSA merge", closed 2024-07-26, https://github.com/RWTH-EBC/AixLib/issues/1291). Whether that check fails for this reason was not checked. [not verified] An issue that names `specificEntropyOfpTX` or `T_psX` was not found (GitHub issue search, 2026-11-03).
