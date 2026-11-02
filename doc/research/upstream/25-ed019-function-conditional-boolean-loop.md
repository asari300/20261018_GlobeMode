# DRAFT (not filed) - Rumoca cannot compile a function whose `if` branch assigns and then loops (ED019 `function conditional`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S072): **unsupported valid Modelica** (Rumoca reports it as unsupported, not as an error in the model). omc accepts it.

## Summary

A function with a `Boolean` output whose `if`/`else` has, in one branch, an assignment followed by a `for` loop is rejected when the DAE is built:

```text
[ED019] unsupported Flat semantic owner `function conditional`: `isIncreasing` requires assignments or nested conditionals in every checked branch
```

The same shape with a `Real` output compiles, and so does the `Boolean` function with the loop moved out of the `if`.

In real code this is `IBPSA.Utilities.Math.Functions.isMonotonic` (called from `splineDerivatives`, which also fails with the same message), with copies in Buildings and AixLib. It is the largest group of ED019 in GlobeMode's run of seven public libraries: 44 of 125 ED019 failures have this message (`splineDerivatives`, `isMonotonic` and others), 12 have `function shape proof`, 12 `compact dependent-domain transition`, the rest smaller groups.

## Steps to reproduce

Compile the attached `25-ed019-function-conditional-boolean-loop.mo` (self-contained):

```bash
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M    # fails
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M0   # control: Real output
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M1   # control: loop outside the if
```

## Expected

All three compile. Assignments, `if` statements and `for` statements are ordinary statements of a function's algorithm section (MLS 3.6 sections 11.2 and 12.2, https://specification.modelica.org/maint/3.6/statements-and-algorithm-sections.html and https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02). The specification does not restrict which statements a branch may contain.

## Actual

```text
$ globe check 25-ed019-function-conditional-boolean-loop.mo --model Rep25.M
25-ed019-function-conditional-boolean-loop.mo:13:16: unsupported[GM-U0001] S5 方程式数と変数数: unsupported Flat semantic owner `function conditional`: `isIncreasing` requires assignments or nested conditionals in every checked branch
    note: rumoca: ED019
Rep25.M: NG（S5 方程式数と変数数 で停止）

$ rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M
  [ED019] unsupported Flat semantic owner `function conditional`: ...
```

`Rep25.M0` and `Rep25.M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all three "completed successfully".

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 run (GitHub Actions run 41250636800), `doc/research/libraries-2026-11-01.md`: ED019 125 of 1,781 failures. Library: IBPSA 4.0.0 (master `2adc9194`), `IBPSA/Utilities/Math/Functions/isMonotonic.mo`; reproduced with a model that calls `isMonotonic` and with `IBPSA.Airflow.Multizone.BaseClasses.Examples.WindPressureProfile`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-dae/src/construction/analysis/function_conditionals.rs`, `validate_conditional_branch_shape` (from about line 496); the error is raised at about line 549 for any branch statement that is neither an assignment nor a nested `if` matched to a plan. Why the `Real` output takes another path was not traced. [not verified]
- `splineDerivatives` (array `Real` output, branches with `assert` and loops) fails with the same message; its smallest failing shape was not isolated (two simpler array versions compile). It may be a second shape of the same check.
- The `function shape proof` group was reduced in S080; see "Minimal reproduction: `function shape proof`" below.

## Minimal reproduction: `function shape proof` (S080, checked 2026-11-02)

This is a second, unrelated ED019 shape. It is filed together with the one above only because both are ED019; split it into its own issue if preferred.

### Summary

A function whose input dimension is a package constant (`input Real x[n]`) fails when it is called from a model of a package that *extends* the defining package with a modifier of that constant (`extends P(n = 2)`):

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

Without the modifier (`extends P;`), or with `input Real x[:]`, it compiles.

In real code this is MSL 4.0.0 `Modelica.Media.Interfaces.PartialSimpleMedium.specificEnthalpy_pTX`, which declares `input MassFraction X[nX]` and is called by `BaseProperties` (`h = specificEnthalpy_pTX(p, T, X)`); `Modelica.Media.Water.ConstantPropertyLiquidWater` extends `PartialSimpleMedium` with modifiers (`mediumName = ...`, which `substanceNames` and so `nS`/`nX` depend on). Every model that instantiates `ConstantPropertyLiquidWater.BaseProperties` with MSL 4.0.0 fails, for example only MSL `Modelica.Fluid.Sources.MassFlowSource_T` plus `FixedBoundary`, or `AixLib.Obsolete.YearIndependent.FastHVAC.Examples.Sensors.SensorVerification` (the error points at `Modelica/Media/package.mo`). MSL 4.1.0 declares `X[:]` in these functions and compiles; changing only line 6035 of MSL 4.0.0 `Modelica/Media/package.mo` (`specificEnthalpy_pTX`) from `X[nX]` to `X[:]` also makes the MSL-only model compile.

### Steps to reproduce

Compile the attached `25b-ed019-function-shape-proof-modified-package-constant.mo` (self-contained):

```bash
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M    # fails
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M0   # control: extends P without modifier
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M1   # control: callee declares x[:]
```

### Expected

All three compile. `n` is a constant, so `x[n]` is a valid dimension (MLS 3.6 section 10.1, https://specification.modelica.org/maint/3.6/arrays.html, checked 2026-11-02), and a modifier in an `extends` clause sets the value of the inherited constant (MLS 3.6 section 7.2, https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-02). Inside `Q`, `n = 2` and `B.x` has two elements, matching `f`.

### Actual

```text
$ globe check 25b-ed019-function-shape-proof-modified-package-constant.mo --model Rep27.M
25b-...mo:10:20: unsupported[GM-U0001] S5 方程式数と変数数: unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
    note: rumoca: ED019
Rep27.M: NG（S5 方程式数と変数数 で停止）

$ rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M
  [ED019] unsupported Flat semantic owner `function shape proof`: dependent
  │ extent is not an exact Integer expression over proven shape axes
 10 │       input Real x[n];
```

`Rep27.M0` and `Rep27.M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all three "completed successfully" (`Rep27.M`: 3 equations, 3 variables).

### Notes

- Where, in rev `276e82b` (names only): `crates/rumoca-phase-dae/src/construction/function_shapes/mod.rs`, `proven_extent` (about line 2400). The extent `n` of the callee is evaluated in a shape environment that apparently does not see the modified value from the `extends` clause. [Reading only; not stepped through.]
- The modifier may also be a scalar (`extends P(n = 2)` in the reproduction). In MSL it is indirect: `mediumName` changes `substanceNames = {mediumName}`, then `nS = size(substanceNames, 1)` and `nX = nS`.
- Counts: in GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), 12 examples have the `dependent extent ...` message (ThermofluidStream `myMedia` examples 5, AixLib expansion valves and compressors 4, `OccupancySchedule` in IBPSA/AixLib/Buildings 3). Those run with MSL 4.1.0, so they are not the MSL 4.0.0 `X[nX]` case; whether they are the same shape (a dimension that depends on a constant modified in an `extends`) was not checked. [not verified] The one example that changes between MSL 4.1.0 and 4.0.0 (S067) is `SensorVerification`, and it is this shape (the Rumoca v0.10.2 CLI gives the same ED019 for it with AixLib at `e5df9a8d` and MSL 4.0.0).
