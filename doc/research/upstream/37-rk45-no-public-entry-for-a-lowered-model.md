# DRAFT (not filed) - `rumoca_sim::rk45` has no public entry for an already lowered model (`simulate_artifact` is `pub(crate)`), so a caller that lowers for its own use lowers twice

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S359): **API request**, not a bug. No result is wrong. The lowering of the DAE to Solve IR, which is the longest stage of the preparation for some models, runs once more than the caller needs, and the public API gives the caller no way to avoid it. Read from the sources of v0.10.2, v0.10.4 and main; no run was timed for this draft. The same source reading (2026-11-15, S344) is the origin of GlobeMode issue #46.

## Summary

`rumoca_sim::rk45` has three public ways to run a model: `simulate` (also `simulate_dae`), `simulate_with_diagnostics`, and `SimulationSession`. Each takes a `&Dae` and lowers it inside (`lower_runtime_artifact` -> `lower_runtime_fmi_artifact` -> `lower_correlated_for_simulation_with_overrides`). The function that runs the integrator on a model that is already lowered, `rk45::simulate_artifact(artifact, opts, execution_backend)`, is `pub(crate)`. So is the function that turns a `LoweredSolveModel` into the `(MeModelArtifact, Option<MeExecutionBackend>)` it takes (`finish_runtime_fmi_artifact`), and the function that picks the execution backend (`admitted_native_execution_backend`).

The BDF side already has the shape a caller needs. `build_simulation_with_stage_timing_and_lowered_model(dae, opts, begin_stage, observe_lowered_model)` calls `observe_lowered_model(&LoweredSolveModel)` once, between the lowering and the creation of the runtime component, and returns a `PreparedSimulation` that `run_prepared_simulation` runs. A caller that needs the lowered model for something of its own (GlobeMode builds a `SolveRuntime` from it and evaluates the model at its start values, to warn about a non-finite start before the run) lowers once with BDF.

With rk45 the same caller lowers twice: once through the public `lower_correlated_for_simulation_with_overrides` for its own use, and once inside `rk45::simulate`. There is no `PreparedSimulation` for rk45 and no observer.

## Steps to reproduce

This is a missing entry, so the "reproduction" is the call that cannot be written. [Read from the source; not compiled.]

```rust
let opts = rumoca_sim::SimOptions { solver_mode: rumoca_sim::SimSolverMode::RkLike, ..Default::default() };

// What a caller can do today: lower for its own use ...
let lowered = rumoca_sim::lower_correlated_for_simulation_with_overrides(&dae, &opts)?;
let _start_values = lowered.model().initial_y.clone();      // the caller's own use of the lowered model
// ... and then it has to lower again, inside the call:
let result = rumoca_sim::rk45::simulate(&dae, &opts)?;

// What it cannot do: run rk45 on `lowered`.
// `rumoca_sim::rk45::simulate_artifact(artifact, &opts, backend)` is `pub(crate)`.
```

Where the visibility is:

| Rumoca | `rk45::simulate_artifact` | `build_simulation_with_stage_timing_and_lowered_model` (BDF) |
| --- | --- | --- |
| v0.10.2 (`276e82bd`) | `crates/rumoca-sim/src/rk45.rs:44`, `pub(crate)` | `diffsol.rs:184`, `pub` |
| v0.10.4 (`9c8c56fc`) | `rk45.rs:71`, `pub(crate)` | `diffsol.rs:186`, `pub` |
| main (`b55eafce`, 2026-11-13) | `rk45.rs:115`, `pub(crate)` (also `simulate_artifact_with_observation`, `:123`) | `diffsol.rs:203`, `pub` |

[The line numbers were read from the git checkouts of v0.10.2 and v0.10.4 and, for main, from the GitHub contents API on 2026-11-16.]

A model for trying the other calls is [37-rk45-no-public-entry-for-a-lowered-model.mo](37-rk45-no-public-entry-for-a-lowered-model.mo) (`Rep37.A`; any model that simulates shows the same call pattern). `rumoca sim 37-rk45-no-public-entry-for-a-lowered-model.mo -m Rep37.A --solver rk-like` runs it with rk45 on v0.10.2 and v0.10.4 (501 time points). The CLI does not show the second lowering.

## Expected

A public way to run rk45 on a lowered model, in whichever shape fits Rumoca's design:

1. make `rk45::simulate_artifact` public and give the caller a public way to build its arguments from a `LoweredSolveModel` (today `finish_runtime_fmi_artifact` and `admitted_native_execution_backend` are `pub(crate)`; `rumoca_phase_solve::fmi::finish_fmi_component` and `MeModelArtifact::new` are public, but a hand-built artifact would lack what `finish_runtime_fmi_artifact` adds in v0.10.4: the execution backend and the execution receipt), or
2. give rk45 the shape of the BDF path: a `build_simulation_with_stage_timing_and_lowered_model`-style function that returns a prepared object with a `run()`.

## Actual

No such entry. The caller lowers twice.

## Re-check on Rumoca v0.10.4 and main (S359, 2026-11-16)

The latest release is still v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, published 2026-11-02; `git ls-remote` and the releases API on 2026-11-16). `rk45::simulate_artifact` is `pub(crate)` in v0.10.4 and in main (`b55eafce2475be1721324d56e1c51ddecfe7dbdb`, 2026-11-13). The lowering path is the same function in both rk45 and BDF (`lower_correlated_for_simulation_with_overrides`). No cache of the lowering was found in `crates/rumoca-sim/src/solve_lowering/`; `rumoca-phase-solve` was not searched for one [not verified].

## Environment

- Rumoca v0.10.2 (rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, the rev GlobeMode pins), v0.10.4 (https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4) and main (`b55eafce`), compared by reading the source.
- Linux x86_64 (WSL2, Ubuntu). Nothing was built for this draft.

## Workaround in GlobeMode

None. GlobeMode's native `dopri45` (rk45) lowers twice (its own preflight through `lower_correlated_for_simulation_with_overrides`, then `rk45::simulate`), and `auto` lowers again for each retry (BDF, esdirk34, dopri45; up to three times). In S344 the retries were made to stop when the time budget is spent (ADR-0041), but the duplicate lowering of a run that still has budget remains. The suggested fix in GlobeMode's issue #46 (run dopri45 through GlobeMode's session route, `rumoca_solver::fmi_me`, so that it lowers once) was measured in S218 (#43) for BDF and dropped: on MSL `EngineV6` the typed BDF route finished in 202 s and the session route stopped at the 170 s budget with a timeout (GlobeMode ADR-0013, S218 addendum). It was not measured for rk45 [not verified].

## Notes

- Cost: the lowering is the longest stage for some models and it is not under Rumoca's time budget (the budget starts when the host is created). GlobeMode's MSL runs on v0.10.2 (S015, 2026-10-20, an idle machine) measured the lowering stage at 64 s for `EngineV6`, 44 s for `FullRobot` and 14 s for `Engine1b_analytic`; `FullRobot` took about 118 s under machine load. These are the cost of one lowering. The cost of the second lowering of a dopri45 run was not measured separately [not verified].
- The same request has an initialization twin: draft 38 (a prepared BDF component is initialized three times). Draft 14 already notes that making `admitted_native_execution_backend` public is a related visibility request; if that is wanted too, 1 above would cover it.
- Source: GlobeMode issue #46 and its comments (S226, S344), `doc/sessions/S344-solver-budget-issues/解説書.md`, ADR-0041.
