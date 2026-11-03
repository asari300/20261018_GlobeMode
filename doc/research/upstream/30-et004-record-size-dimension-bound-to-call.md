# DRAFT (not filed) - A record parameter with `eta[size(y, 1)]` bound to a function call fails (ET004)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S091): **bug** (valid Modelica rejected). omc accepts it.

## Summary

A record `R` has `parameter Real y[:]` and `parameter Real eta[size(y, 1)]`. A function `f` returns `R` with `output R r(y = {0, 1}, eta = zeros(2))`. The model parameter `parameter R p = f(2)` gives:

```text
[ET004] unevaluable array dimensions for 'p.eta': dimension expression could not be evaluated: [Expression(FunctionCall { comp: size, args: [y, UnsignedInteger("1")], ... })]
```

The same record with the fields given directly (`parameter R p(y = {0, 1}, eta = {2, 2})`) compiles.

## Steps to reproduce

Compile the attached `30-et004-record-size-dimension-bound-to-call.mo` (self-contained):

```bash
rumoca compile 30-et004-record-size-dimension-bound-to-call.mo -m Rep38.M    # ET004
rumoca compile 30-et004-record-size-dimension-bound-to-call.mo -m Rep38.M0   # control
```

## Expected

Both compile. The dimensions of a component with `:` are taken from its binding (MLS 3.6 section 10.1, https://specification.modelica.org/maint/3.6/arrays.html, checked 2026-11-02); here the binding is the function's output, whose sizes are fixed by its modifiers (section 12.4.5, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
$ globe check 30-et004-record-size-dimension-bound-to-call.mo --model Rep38.M
... error[GM-E2001] S2 型と次元: unevaluable array dimensions for 'p.eta': dimension expression could not be evaluated: [Expression(FunctionCall { comp: size, ...
```

The Rumoca CLI gives ET004 for `M` (exit 1); `M0` compiles (exit 0). OpenModelica 1.27.2 (`loadFile`, `checkModel`): both "completed successfully".

## Seen in libraries

In GlobeMode's library run (GitHub Actions run 41365960542, 2026-11-02), 15 examples stop with this message: `Fluid.Movers.Examples.MoverContinuous`, `MoverParameter`, `MoverStages`, `PumpsSeries` and `SpeedControlled_y_pumpCurves` in each of IBPSA, Buildings and AixLib. `Fluid.Movers.Data.Generic` has `final parameter efficiencyParameters_yMot motorEfficiency_yMot_generic = motorEfficiencyCurve(...)`, the record has `y[:]` and `eta[size(y, 1)]`, and the function declares `output efficiencyParameters_yMot motorEfficiency_yMot(y = {...}, eta = zeros(nPoi))`. The library itself was not patched to confirm. [not verified]

## Re-check on Rumoca v0.10.4 (S113, 2026-11-03)

Unchanged: `M` gives `[ET004] unevaluable array dimensions for 'p.eta'` (exit 1); `M0` compiles (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Re-checked on 2026-11-03 (S113) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4). Rumoca main (`91d6e936648387d04648ec9c52237479eaec46d2`, 10 commits after v0.10.4) was not built; none of those 10 commit titles mentions name lookup, dimensions or bindings. [not verified on main]

## Workaround in GlobeMode

None.

## Notes

- With `y[2]` (fixed) and `eta[size(y, 1)]` and the same function call, the error changes to `[ED008] unresolved Flat reference 'y'`; with `y[:]` and `eta[2]`, to `unresolved component dimension for p.y: :`. Both look like the same missing step (sizes of a record bound to a call), not traced. [not verified]
