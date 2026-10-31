# DRAFT (not filed) - Initialization solves initial equations with relation defaults and never re-solves when a relation flips at t = 0

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

When an initial equation depends (through other variables) on a relation such as `v < 0`, the initialization solves it with the default value of the relation (false) and does not re-solve after the relation has been evaluated at the initial state. The simulation then starts from an inconsistent initial value. In the MSL this gives a wrong trajectory for `Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6` (`filter.x` starts at 0.066, reference 563.19), through `CriticalDamping` with `initType = SteadyState`. The attached file reduces it to a first-order filter.

## Steps to reproduce

```bash
rumoca sim 04-init-equations-solved-with-relation-default.mo -m InitRelationBranch
```

(the single-file invocation form is [not verified]; the Rumoca binary was not available in the drafting session). Then read `x` at t = 0. The MSL form: `... -m Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6 --source-root <MSL root>`.

## Expected

At t = 0, `v = -0.5 < 0`, so `u = 1`, and the steady-state initial equation `der(x) = 0` gives `x(0) = 1`. (OpenModelica 1.27.1 gives 1 on this pattern, as observed in GlobeMode S016.)

## Actual

`x(0) = 2`: the initialization solved with `v < 0` = false (`u = 2`). Rumoca CLI v0.10.2 gives the same value (GlobeMode S016).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

Detection only. After initialization GlobeMode re-solves the initialization system once more and warns (`GM-W10004`) if the values move (S018, ADR-0020). The wrong start value is still used.

## Notes

- Source: `doc/research/msl-sim-failures.md` (section "S016", upstream candidate 11 and the R1 reduction).
- The reduction file is a copy of `src/Modelica/GlobeModeValidation/Simulation/InitRelationBranch.mo` as a top-level model.
