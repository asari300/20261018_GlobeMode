# DRAFT (not filed) - A function input sized by a package constant fails when the package is extended with a modifier of that constant (ED019 `function shape proof`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S080): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

A function whose input dimension is a package constant (`input Real x[n]`) fails when it is called from a model of a package that *extends* the defining package with a modifier of that constant (`extends P(n = 2)`):

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

Without the modifier (`extends P;`), or with `input Real x[:]`, it compiles.

In real code this is MSL 4.0.0 `Modelica.Media.Interfaces.PartialSimpleMedium.specificEnthalpy_pTX`, which declares `input MassFraction X[nX]` and is called by `BaseProperties` (`h = specificEnthalpy_pTX(p, T, X)`); `Modelica.Media.Water.ConstantPropertyLiquidWater` extends `PartialSimpleMedium` with modifiers (`mediumName = ...`, which `substanceNames` and so `nS`/`nX` depend on). Every model that instantiates `ConstantPropertyLiquidWater.BaseProperties` with MSL 4.0.0 fails, for example only MSL `Modelica.Fluid.Sources.MassFlowSource_T` plus `FixedBoundary`, or `AixLib.Obsolete.YearIndependent.FastHVAC.Examples.Sensors.SensorVerification` (the error points at `Modelica/Media/package.mo`). MSL 4.1.0 declares `X[:]` in these functions and compiles; changing only line 6035 of MSL 4.0.0 `Modelica/Media/package.mo` (`specificEnthalpy_pTX`) from `X[nX]` to `X[:]` also makes the MSL-only model compile.

## Steps to reproduce

Compile the attached `25b-ed019-function-shape-proof-modified-package-constant.mo` (self-contained):

```bash
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M   # fails
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M0  # control: extends P without modifier
rumoca compile 25b-ed019-function-shape-proof-modified-package-constant.mo -m Rep27.M1  # control: callee declares x[:]
```

## Expected

All three compile. `n` is a constant, so `x[n]` is a valid dimension (MLS 3.6 section 10.1, https://specification.modelica.org/maint/3.6/arrays.html, checked 2026-11-02), and a modifier in an `extends` clause sets the value of the inherited constant (MLS 3.6 section 7.2, https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-02). Inside `Q`, `n = 2` and `B.x` has two elements, matching `f`.

## Actual

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

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Where, in rev `276e82b` (names only): `crates/rumoca-phase-dae/src/construction/function_shapes/mod.rs`, `proven_extent` (about line 2400). The extent `n` of the callee is evaluated in a shape environment that apparently does not see the modified value from the `extends` clause. [Reading only; not stepped through.]
- The modifier may also be a scalar (`extends P(n = 2)` in the reproduction). In MSL it is indirect: `mediumName` changes `substanceNames = {mediumName}`, then `nS = size(substanceNames, 1)` and `nX = nS`.
- Counts: in the MSL 4.1.0 libraries run, 12 examples have the `dependent extent ...` message, but only 1 of them is this shape (ThermofluidStream `SimpleLiquidWater`, a copy of the MSL 4.0.0 `X[nX]` in `myMedia/Interfaces.mo` line 2323; changing only that to `X[:]` makes the example pass all of GlobeMode's checks). The other 11 are drafts 25c, 25d and 25e. With MSL 4.0.0 (S067) `SensorVerification` is this shape (the Rumoca v0.10.2 CLI gives the same ED019 for it with AixLib at `e5df9a8d`).
