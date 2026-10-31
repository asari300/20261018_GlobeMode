# DRAFT (not filed) - Solve IR lowering and SolveRuntime construction take minutes to forever on MSL Fluid examples

Target: Rumoca (https://github.com/CogniPilot/rumoca)

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

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

None. GlobeMode does not add a pre-flight limit because the construction step is hit again in the real run (ADR-0018). The run is stopped from outside by its wall-clock budget.

## Notes

- Source: `doc/research/msl-sim-failures.md` ("Upstream candidates" 3 and 10; section "S016").
- The quadratic behaviour is our reading of the code and the profile; we did not patch it to confirm [not verified].
