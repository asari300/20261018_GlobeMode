# DRAFT (not filed) - An array dimension that is a constant of another package fails (ET004), or EF024 through a replaceable package that is not redeclared

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S091): **bug** (valid Modelica rejected). omc accepts it.

## Summary

A model component whose array dimension is a constant of another package fails:

```text
[ET004] unevaluable array dimensions for 'x': dimension expression could not be evaluated: [Expression(P.n)]
```

The same constant through a `replaceable package Medium = P` that keeps its default gives:

```text
[EF024] flat variable is missing structured identity: n
```

It fails the same way with the name qualified from the root (`Rep36.P.n`), with `P` as a top-level package in another file, with a `parameter Integer m = P.n` used as the dimension, with the constant inherited by `extends`, with a binding, and inside a component that uses the default of its replaceable package. It passes when the dimension is a constant of the model itself, when the package is declared inside the model, and when the package reaches the component through `redeclare package Medium = P`. That is why MSL's fluid components (`Boundary_pT` with `X[Medium.nX]`, always redeclared) pass, while a model-level `Real X[Medium.nX]` with MSL's `Modelica.Media.Water.ConstantPropertyLiquidWater` fails with EF024 for `nX`.

## Steps to reproduce

Compile the attached `28-et004-ef024-dimension-from-package-constant.mo` (self-contained):

```bash
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M    # ET004: Real x[P.n]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M1   # EF024: replaceable package Medium = P; Real x[Medium.n]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M2   # ET004: parameter Integer m = P.n; Real x[m]
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M3   # ET004: component using the default of its replaceable package
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M0   # control: constant of the model
rumoca compile 28-et004-ef024-dimension-from-package-constant.mo -m Rep36.M4   # control: the same component with redeclare
```

## Expected

All six compile. A constant declared in a package is a constant expression and may be used as an array dimension (MLS 3.6 section 3.8.1 and section 10.1, https://specification.modelica.org/maint/3.6/operators-and-expressions.html and https://specification.modelica.org/maint/3.6/arrays.html, checked 2026-11-02); the name `P.n` is found by the normal lookup (section 5.3.2, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-02), and the default of a replaceable package is used when it is not redeclared (section 7.3).

## Actual

```text
$ globe check 28-et004-ef024-dimension-from-package-constant.mo --model Rep36.M
... error[GM-E2001] S2 型と次元: unevaluable array dimensions for 'x': dimension expression could not be evaluated: [Expression(P.n)]

$ globe check 28-et004-ef024-dimension-from-package-constant.mo --model Rep36.M1
... unsupported[GM-U0002] S4 接続: flat variable is missing structured identity: n
```

The Rumoca CLI gives ET004 for `M`, `M2`, `M3` and EF024 for `M1`; `M0` and `M4` compile (exit 0). OpenModelica 1.27.2 (`loadFile`, `checkModel`): all six "completed successfully".

## Seen in libraries

In GlobeMode's library run (GitHub Actions run 41365960542, 2026-11-02, Rumoca v0.10.2, MSL 4.1.0), 3 examples stop on this shape:

- `Buildings.HeatTransfer.Conduction.BaseClasses.Examples.Temperature_u`: `Real ud[Buildings.HeatTransfer.Conduction.nSupPCM]` (ET004).
- `IBPSA.Fluid.HeatExchangers.Examples.WetCoilEffectivenessNTUMassFlow` and the same model in Buildings: the component `UAFroRated` of `WetCoilUARated` keeps the default `replaceable package MediumA = IBPSA.Media.Air`, and its `parameter MediumA.ThermodynamicState staAir` has `X[nX]` (ET004 for `hex.UAFroRated.staAir.X`). In a copy of IBPSA with `redeclare package MediumA = IBPSA.Media.Air` added to `UAFroRated`, the ET004 goes away (the example then stops at a different, unrelated error: a `for` range `Medium.nXi`).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. In a model, redeclare the package in the component that has the dimension, or use a constant of the model itself.

## Notes

- Using the constant only in an expression (`Real y = P.n*time`) compiles; only its use as a dimension fails.
- Why EF024 instead of ET004 when the path goes through the replaceable package was not traced. [not verified]
