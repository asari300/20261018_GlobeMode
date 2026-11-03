# DRAFT (not filed) - A broken function in a medium that the model never uses, reached through another subclass's redeclaration, fails the model (ER002)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S108): **bug** (Rumoca checks a class that the model does not instantiate). omc accepts the model. The broken function itself is a real error in the library (see Notes).

## Summary

A partial model `PFP` has `replaceable package Medium1 = PM`. Model `M` extends it with `redeclare package Medium1 = Good` and calls `Medium1.f(time)`. A different model `Engine` extends the same `PFP` with `redeclare package Medium1 = Bad`. `Bad` has a function `T_psX` whose nested function calls `specificEntropyOfpTX`, which is not declared anywhere; `T_psX` is reached only from `Bad.g`. `M` uses none of `Engine`, `Bad`, `g` or `T_psX`, yet compiling `M` gives:

```text
[ER002] unresolved function call: 'specificEntropyOfpTX'
```

`M` compiles when it does not call through `Medium1` (`M0`), and when its base class has no other subclass that redeclares `Medium1 = Bad` (`M1`).

## Steps to reproduce

Compile the attached `32-er002-unused-function-of-other-subclass-medium.mo` (self-contained):

```bash
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M       # ER002
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M0      # control: no call through Medium1
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.M1      # control: same call, base without the other subclass
rumoca compile 32-er002-unused-function-of-other-subclass-medium.mo -m Rep40.Engine  # ER002 (Engine does not call g either)
```

## Expected

`M`, `M0` and `M1` compile. Flattening `M` instantiates only `M`, its base class `PFP` with `Medium1 = Good`, and the functions it calls (MLS 3.6 section 5.6, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-03). Another subclass of `PFP` and its medium are not part of `M`. `Engine` also does not call `g`, so OpenModelica accepts it as well.

## Actual

```text
$ globe check 32-er002-unused-function-of-other-subclass-medium.mo --model Rep40.M
... NG（S1 名前解決 で停止）   rumoca: ER002
```

The Rumoca CLI gives ER002 for `M` and `Engine`; `M0` and `M1` compile (exit 0). `globe check` gives the same. OpenModelica 1.27.2 (`loadFile`, `checkModel`): `M`, `M0`, `M1` and `Engine` "completed successfully".

## Seen in libraries

In GlobeMode's library run (GitHub Actions run 41365960542, 2026-11-02, Rumoca v0.10.2, MSL 4.1.0), 39 examples of AixLib 3.0.1 stop with `unresolved function call: 'specificEntropyOfpTX'`, in `Airflow.Multizone.Examples` (6), `Airflow.FacadeVentilationUnit`, `Examples.Tutorial.SimpleHouse.SimpleHouse6`, `Fluid.Chillers` (5), `Fluid.HeatPumps` (7), `Fluid.HeatExchangers` (3), `Fluid.Geothermal.Borefields` (5), `Fluid.DistrictHeatingCooling` (2), `Fluid.FMI.ExportContainers` (2), `Fluid.Interfaces` (2), `Fluid.MassExchangers`, `Fluid.Examples` (2) and `Obsolete.Year2024` (2).

The call is in `AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPCombustionMixtureGasNasa.T_psX.f_nonlinear` (line 670), reached from its `setState_psX`. These examples use components that extend `AixLib.Fluid.Interfaces.PartialFourPortInterface` and call `Medium1.setState_phX(...)`, with ordinary media (for example `AixLib.Media.Air`, or `Modelica.Media.Air.SimpleAir` in a reduced test). `AixLib.Fluid.BoilerCHP.ModularCHP.BaseClasses.ExhaustHeatExchanger` extends the same `PartialFourPortInterface` with `redeclare package Medium1 = Medium3` (`CHPFlueGasLambdaOnePlus`, which extends `CHPCombustionMixtureGasNasa`). With the Rumoca CLI and one source root holding AixLib and MSL 4.1.0:

- `model W3 extends AixLib.Fluid.Interfaces.PartialFourPort(redeclare package Medium1 = Modelica.Media.Air.SimpleAir, redeclare package Medium2 = Modelica.Media.Air.SimpleAir); Medium1.ThermodynamicState s = Medium1.setState_phX(1e5, 1e5, Medium1.X_default); end W3;` gives the ER002.
- With `ExhaustHeatExchanger.mo` replaced by an empty model, `W3` passes name resolution (it then stops on an unrelated balance check, as expected for that test).
- With `ExhaustHeatExchanger.mo` replaced by only `extends AixLib.Fluid.Interfaces.PartialFourPort(redeclare package Medium1 = AixLib.DataBase.CHP.ModularCHPEngineMedia.CHPFlueGasLambdaOnePlus)`, the ER002 comes back.

GlobeMode's `globe check` reads only the closure of the files the model uses (since S054, GlobeMode ADR-0025), so it does not see these 39 examples fail with ER002 (for example `AixLib.Airflow.Multizone.Examples.ZonalFlow` stops later, at EI032). The library run reads all of AixLib, so it does.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

Partial: the native and wasm `--library` routes read only the closure of the model (GlobeMode ADR-0025), which avoids this in the examples above. The full-library read of the library run does not.

## Notes

- The library is not valid either: `CHPCombustionMixtureGasNasa` copies MSL's `MixtureGasNasa.T_psX` but not the protected function `specificEntropyOfpTX` it calls, and neither `PartialMixtureMedium` nor `PartialMedium` declares it. Calling `setState_psX` of a CHP medium would fail in any tool. That is for AixLib to fix (GlobeMode does not report it); this draft is only about Rumoca reaching it from models that do not use it.
- How Rumoca reaches `Bad` from `M` (it looks like it resolves every redeclaration of `PFP.Medium1` in the loaded classes) was not traced in the code. [not verified]
