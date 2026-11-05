# DRAFT (not filed) - Initialization solves initial equations with relation defaults and never re-solves when a relation flips at t = 0

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Status (S162, 2026-11-05): **fixed in Rumoca v0.10.4 for the reduction** (`x(0) = 1`). Do not file; this draft is kept as a record. The MSL form is also fixed (below).

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

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Fixed for the reduction. The single-file form that the draft marked [not verified] works: `rumoca sim 04-init-equations-solved-with-relation-default.mo -m InitRelationBranch -o out.csv` (the CLI writes `.csv` or `.html`). The first row of the CSV (`time,s,v,u,x`):

| Rumoca | `u` at t = 0 | `x` at t = 0 |
| --- | ---: | ---: |
| v0.10.2 | 1 | 2 |
| v0.10.4 | 1 | 1 |

`u` is 1 in both (the relation is evaluated), but v0.10.2 starts `x` from 2, the value solved with the relation default; v0.10.4 starts from 1 (the expected value).

The MSL form, `rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Mechanics.MultiBody.Examples.Loops.EngineV6 --source-root <MSL root> -o out.csv` (about 5 minutes on this machine), first row of `filter.y` (and `filter.x[1]`): v0.10.2 `0.06599` (the wrong start value of the draft), v0.10.4 `563.186` (the reference value 563.19).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Detection only. After initialization GlobeMode re-solves the initialization system once more and warns (`GM-W10004`) if the values move (S018, ADR-0020). The wrong start value is still used.

## Notes

- Source: `doc/research/msl-sim-failures.md` (section "S016", upstream candidate 11 and the R1 reduction).
- The reduction file is a copy of `src/Modelica/GlobeModeValidation/Simulation/InitRelationBranch.mo` as a top-level model.
