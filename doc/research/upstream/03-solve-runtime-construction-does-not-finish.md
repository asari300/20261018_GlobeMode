# DRAFT (not filed) - Solve IR lowering and SolveRuntime construction take minutes to forever on MSL Fluid examples

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Status (S162, 2026-11-05): **the symptom (does not finish) is gone in Rumoca v0.10.4**: both examples now stop with an error within 33 s and 11 s. Neither example simulates yet, for other reasons (below). Do not file this draft; it is kept as a record.

## Summary

`Modelica.Fluid.Examples.DrumBoiler.DrumBoiler` and `Modelica.Fluid.Examples.Explanatory.MomentumBalanceFittings` do not finish in `rumoca sim` within 240 s. In GlobeMode's profile of DrumBoiler (S016) the time goes to two places:

1. Lowering (117 s to 440 s depending on the model): dominated by `Box<[RegisteredAssertion]>::clone` and `CallScopedActionCollector::insert`.
2. Runtime construction: `SolveRuntime::construct` -> `root_condition_rows` (`crates/rumoca-solver/src/runtime/solve_runtime.rs:938`) -> `target_assignment_shapes_with_output_offsets` (`crates/rumoca-eval-solve/src/prepared/construction.rs:68`) -> `crates/rumoca-ir-solve/src/refresh/assignment_shape.rs:100-118`. That code rebuilds a prefix dependency set for every store position. There are 31 root conditions with 597,000 operations in total (about 65,000 each); the first one did not finish in 900 s. The Cranelift JIT (`regalloc2`) is also slow on these.

## Steps to reproduce

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Fluid.Examples.DrumBoiler.DrumBoiler --source-root <MSL root>
```

No `.mo` file (MSL example). GlobeMode confirmed with the CLI that both models exceed 240 s (S011); the stack/profile for DrumBoiler is from `perf record` on GlobeMode's own process (S016).

## Expected

Construction time roughly linear in the number of operations; the model runs.

## Actual

No result in 240 s (CLI) and no completion in 900 s for the first root-condition shape in GlobeMode.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

CLI, `rumoca sim "Modelica 4.1.0/package.mo" -m <model> --source-root <MSL root>`, wall time on this machine (RSS about 1.5 GB):

| Model | v0.10.2 | v0.10.4 |
| --- | --- | --- |
| `Modelica.Fluid.Examples.DrumBoiler.DrumBoiler` | no result in 270 s (timeout) | stops after 33 s with `[EX001] solve-IR evaluation failed: algebraic projection did not converge at event boundary: worst scaled residual row=1 target=evaporator.h_l value=3.029227e1 ratio=3.029227e1 norm=8.208480e5 ...` (after a NaN-tracing re-run) |
| `Modelica.Fluid.Examples.Explanatory.MomentumBalanceFittings` | no result in 270 s (timeout) | stops after 11 s with `[ER002] unresolved type reference: 'Medium.ThermodynamicState'` at `Fluid/Fittings.mo:1762` (`BaseModelNonconstantCrossSectionArea`, a protected `parameter Medium.ThermodynamicState state_nominal = Medium.setState_pTX(...)`) |

So the lowering and runtime-construction time reported in the Summary no longer occurs on these two models. The two new stops are different problems and are not drafted: the `DrumBoiler` one is a numerical failure at an event (the first model of its kind in GlobeMode's list), and the `MomentumBalanceFittings` one is a name-resolution failure that v0.10.2 did not report (v0.10.2 reached the simulation and then timed out), so it may be a change in v0.10.4 [not investigated]. The other Fluid examples were not re-run. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

None. GlobeMode does not add a pre-flight limit because the construction step is hit again in the real run (ADR-0018). The run is stopped from outside by its wall-clock budget.

## Notes

- Source: `doc/research/msl-sim-failures.md` ("Upstream candidates" 3 and 10; section "S016").
- The quadratic behaviour is our reading of the code and the profile; we did not patch it to confirm [not verified].
