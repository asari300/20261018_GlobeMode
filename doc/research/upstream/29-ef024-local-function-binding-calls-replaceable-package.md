# DRAFT (not filed) - In a function declared inside a model, a protected binding that calls a function of the model's replaceable package fails (EF024)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S091): **bug** (valid Modelica rejected). omc accepts it.

## Summary

A model with `replaceable package Medium = P` declares a function `g` whose protected variable has the binding `Real w = Medium.f(u)`. Calling `g` gives:

```text
[EF024] flat variable is missing structured identity: f
```

The same call in the algorithm section (`z := Medium.f(u)`) compiles, and so does the binding with the package's own name (`Real w = P.f(u)`). It also fails when the package is redeclared through `extends M(redeclare package Medium = P)`, which is how the libraries use it.

## Steps to reproduce

Compile the attached `29-ef024-local-function-binding-calls-replaceable-package.mo` (self-contained):

```bash
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M    # EF024
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M1   # EF024 (extends with redeclare)
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M0   # control: call in the algorithm
rumoca compile 29-ef024-local-function-binding-calls-replaceable-package.mo -m Rep37.M2   # control: P.f in the binding
```

## Expected

All four compile. A protected component of a function may have a binding, evaluated before the algorithm (MLS 3.6 section 12.4.4, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02), and `Medium.f` is found by the normal lookup from the function's enclosing model (section 5.3, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-02).

## Actual

```text
$ globe check 29-ef024-local-function-binding-calls-replaceable-package.mo --model Rep37.M
... unsupported[GM-U0002] S4 接続: flat variable is missing structured identity: f
```

The Rumoca CLI gives EF024 for `M` and `M1`; `M0` and `M2` compile (exit 0). OpenModelica 1.27.2 (`loadFile`, `checkModel`): all four "completed successfully".

## Seen in libraries

In GlobeMode's library run (GitHub Actions run 41365960542, 2026-11-02), 18 examples stop with `flat variable is missing structured identity: temperature`: the six `Media.Examples` (`AirProperties`, `SteamProperties`, `WaterProperties`, `PropyleneGlycolWaterProperties`, `Specialized.Water.Examples.TemperatureDependentDensityProperties`, `Specialized.Water.Examples.WaterProperties_pT`) in each of IBPSA, Buildings and AixLib. They extend `Media.Examples.BaseClasses.PartialProperties`, whose protected function `checkState` has `Real TErrAbs = abs(Medium.temperature(state1) - Medium.temperature(state2))`. In a copy of IBPSA with the two bindings moved into the algorithm of `checkState`, `IBPSA.Media.Examples.WaterProperties` passes all of GlobeMode's checks.

## Re-check on Rumoca v0.10.4 (S113, 2026-11-03)

Unchanged: `M` and `M1` give `[EF024] flat variable is missing structured identity: f`; `M0` and `M2` compile (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Re-checked on 2026-11-03 (S113) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4). Rumoca main (`91d6e936648387d04648ec9c52237479eaec46d2`, 10 commits after v0.10.4) was not built; none of those 10 commit titles mentions name lookup, dimensions or bindings. [not verified on main]

## Workaround in GlobeMode

None. Move the call from the binding into the algorithm section.

## Notes

- The other EF024 messages in the same run (`nomVal` 7 in Buildings `Fluid.DXSystems`, `T` 1 and `nXi` 1 in AixLib) were not reduced. [not verified]
