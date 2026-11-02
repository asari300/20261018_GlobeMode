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
- `splineDerivatives` (array `Real` output) fails with the same message; reduced in S083 to `25g` (see the last section).
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
- Counts: in GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), 12 examples have the `dependent extent ...` message (ThermofluidStream `myMedia` examples 5, AixLib expansion valves and compressors 4, `OccupancySchedule` in IBPSA/AixLib/Buildings 3). Those run with MSL 4.1.0, so they are not the MSL 4.0.0 `X[nX]` case; only 1 of them (ThermofluidStream `SimpleLiquidWater`) is this shape; the other 11 are other shapes (S083, see the next section). The one example that changes between MSL 4.1.0 and 4.0.0 (S067) is `SensorVerification`, and it is this shape (the Rumoca v0.10.2 CLI gives the same ED019 for it with AixLib at `e5df9a8d` and MSL 4.0.0).

## The remaining ED019 messages in MSL 4.1.0 runs (S083, checked 2026-11-02)

In GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), three ED019 messages were not yet reduced: `dependent extent is not an exact Integer expression over proven shape axes` (12 examples), `` `X`/`b`: axis 1 is variable-size and neither a binding nor an assignment sizes it `` (5), and `function conditional` in `splineDerivatives` (25: IBPSA 8, AixLib 9, Buildings 8). They come from six different shapes in MSL 4.1.0 and the libraries, plus one example that is the 25b shape. Each was reduced by copying the failing function out of the library and removing parts until the error went away; the source line was found with the Rumoca CLI on the example, and checked by changing that line in a copy of the library.

All of the attached reproductions are self-contained. In each, `M` fails in both `globe check` and the Rumoca v0.10.2 CLI with the same code, and the controls `M0`/`M1` compile. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in every file. These shapes may be separate issues; they are listed together here because they share the ED019 code.

| File | Message | Smallest failing shape | Compiles when | Where in the libraries | Examples |
| --- | --- | --- | --- | --- | ---: |
| 25b (above) | `dependent extent is not an exact ...` | `input Real x[n]`, `n` changed by `extends P(n = 2)` | `x[:]`, or no modifier | ThermofluidStream `myMedia.Interfaces.PartialSimpleMedium.specificEnthalpy_pTX` (a copy of the MSL 4.0.0 `X[nX]`) | 1 (`SimpleLiquidWater`) |
| `25c-ed019-function-shape-proof-size-of-call-sized-constant.mo` (`Rep29`) | `dependent extent is not an exact ...` | a function declares `protected Integer n = size(c, 1) - 1`, where the package constant `c[:]` is bound to a call whose output size depends on an input (`output Real p[n + 1]`) | `c` has a literal binding, or `size(c, 1)` is used without the `Integer` local | MSL `Media.Incompressible.TableBased.specificEntropy` (`npol = size(poly_Cp, 1) - 1`, `poly_Cp = Polynomials.fitting(...)`); the ThermofluidStream copy | 2 (`TestGlycol`, `InverseIncompressible_sh_T`); MSL's own 2 also |
| `25d-ed019-function-shape-proof-dimension-before-input.mo` (`Rep30`) | `dependent extent is not an exact ...` | `input Real x[n]` declared before `input Integer n` | `input Integer n` first, or `x[:]` | `Controls.SetPoints.OccupancySchedule.getOutput` (`occupancy[nRow]` before `nRow`) in IBPSA, AixLib, Buildings | 3 |
| `25e-ed019-function-shape-proof-record-local-dimension.mo` (`Rep31`) | `dependent extent is not an exact ...` | a record with `parameter Integer nc = 2; parameter Real[nc] a` used as a function's protected local (also with `constant`) | literal dimension, or the record as a model component | MSL `Media.R134a.R134a_ph` (`Common.EOSIdealCoeff`, `EOSResidualCoeff`, locals in `fid_R134a`, `fres_R134a`); the ThermofluidStream copy | 6 (AixLib expansion valves and compressors 4, ThermofluidStream `R134a1`/`R134a2` 2); MSL `R134a1`/`R134a2` also |
| `25f-ed019-function-shape-proof-binding-reads-later-constant.mo` (`Rep32`) | `` `b`: axis 1 is variable-size ... `` | `protected constant Real b[:] = {c, 2}` declared before `constant Real c` | `c` declared first, or a literal binding | `Media.Steam.temperature_ph` (`b[:]` reads `TMean`, `TSD` declared after it) in IBPSA, AixLib, Buildings | 3 (`SteamTemperatureEnthalpyInversion`) |
| `25h-ed019-function-shape-proof-flexible-array-from-call.mo` (`Rep34`) | `` `X`: axis 1 is variable-size ... `` | `protected Real[:] X;` sized by `X := g(x)` (also with the binding `= g(x)`) | `Real[2] X` | MSL `Media.Air.ReferenceMoistAir.Xsaturation` (`MassFraction[:] X; X := massFractionSaturation(state)`); the ThermofluidStream copy | 2 (ThermofluidStream `MoistAir`, `MoistAir2`); MSL `ReferenceAir.MoistAir`, `MoistAir2` also |
| `25g-ed019-function-conditional-loop-only-branch.mo` (`Rep33`) | `function conditional`: `... requires assignments or nested conditionals in every checked branch` | the output array is defined in a `for` loop, then `if b then for ... loop d[i] := 0; end for; end if;` | `d := y` instead of the first loop, or an assignment in the branch | `Utilities.Math.Functions.splineDerivatives` ("Ensure monotonicity") in IBPSA, AixLib, Buildings | 25 |

Checks on the libraries (Rumoca CLI or `globe check`, which reports the file):

- The four AixLib examples and MSL `R134a1`: the error points at MSL `Media/R134a.mo` line 53 (`parameter Real[nc] a`). In a copy of MSL 4.1.0 with the record dimensions written as literals (`Real[5] a`, `Real[21] c, d, t, n`), MSL `R134a1` and `R134a2` get past this error (they stop later at a different ED008, `state_h`).
- `IBPSA.Controls.SetPoints.Examples.OccupancySchedule`: in a copy of IBPSA with `input Integer nRow` moved before `input Real occupancy[nRow]`, the ED019 goes away (it stops later at a different ED019, `multi-output equation`, in the initial equation).
- ThermofluidStream `SimpleLiquidWater`: changing only `X[nX]` to `X[:]` in `myMedia/Interfaces.mo` line 2323 makes the example pass all of GlobeMode's checks.
- MSL `Incompressible.Examples.TestGlycol`: the CLI points at `Math/Polynomials.mo` line 137 (`output Real p[n + 1]` of `fitting`); with a model that only calls `Glycol47.specificEntropy`, the same message comes out, and the self-contained reduction is 25c.

### Expected

All compile. Array dimensions of function components may depend on inputs and on constants (MLS 3.6 section 12.4.5); default values and bindings in a function are evaluated in dependency order, not declaration order, as long as they are acyclic (sections 12.4.1 and 12.4.4); a protected array with `:` may be sized by an assignment (section 12.4.5); and assignments, `if` and `for` are ordinary statements of an algorithm section (section 11.2). (https://specification.modelica.org/maint/3.6/functions.html and https://specification.modelica.org/maint/3.6/statements-and-algorithm-sections.html, checked 2026-11-02.)

### Notes

- 25c gives the target message only when the `Integer` local has the same name as the constant the call's argument reads through an alias (`nc = n`, local `n`), as in MSL (`npolHeatCapacity = npol`, local `npol`). With another local name (`Rep29.M2`) the message is `dependent extents cannot call runtime function`. S091 checked the names with eight variants of 25c (the callee's input `n` or `m`, the package constant `n` or `q` with `nc` bound to it, the local `n`, `m`, `q` or `k`): the target message comes out exactly when the local has the name of the package constant (`n`/`n` or `q`/`q`), whatever the callee's input is called; otherwise the message is `dependent extents cannot call runtime function`. So the binding `nc = n` seems to be resolved in the scope of the function `f`, where `n` is the local `Integer`, instead of the package `P` where it is written (MLS 3.6 section 5.3). This reading is an inference from the messages; the code was not read. [not verified]
- 25d also fails when the block is used through `extends` (`Rep30.M2`). With a model-level parameter `n` passed as the argument, the same function compiled during the reduction; not traced. [not verified]
- 25g is the same check as the `function conditional` shape above (a branch that holds only a `for` loop), but with a `Real` array output. It replaces the earlier note that `splineDerivatives` was not isolated.
