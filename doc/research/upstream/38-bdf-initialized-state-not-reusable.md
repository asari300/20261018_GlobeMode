# DRAFT (not filed) - A prepared BDF simulation is leased (initialized) three times when a caller checks the initialization before it runs, and the caller cannot hand the initialized state to the run

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S359): **API request**, small. No result is wrong. The public BDF path (`build_simulation_with_stage_timing_and_lowered_model`, `check_prepared_initialization`, `run_prepared_simulation`) leases the retained FMI component three times, and Rumoca documents each lease as an initialization. The cost was **not measured**. Read from the sources of v0.10.2, v0.10.4 and main. The same reading (2026-11-15, S344) is the origin of GlobeMode issue #45.

## Summary

A caller that wants to know where a BDF run fails (build, initialization, integration) uses the three public steps in this order:

```rust
let (prepared, _timings) = rumoca_sim::build_simulation_with_stage_timing_and_lowered_model(&dae, &opts, |_| {}, |_lowered| {})?;
rumoca_sim::check_prepared_initialization(&prepared)?;        // "did initialization fail?"
let result = rumoca_sim::run_prepared_simulation(&prepared)?; // the run
```

Each step ends up in `MeRetainedComponent::lease`, whose doc comment says "complete an actual initialization handoff or restore source defaults and initialize the next run", and `lease_count` says "Runs this component has been leased for, each one an initialization" (`crates/rumoca-solver/src/fmi_me/session.rs`):

| # | where | what it does (`crates/rumoca-sim/src/diffsol.rs`, v0.10.2 / v0.10.4 / main) |
| --- | --- | --- |
| 1 | the end of `build_simulation_artifact` | `drop(check_prepared_component(&prepared))` (lines 241 / 236 / 223): lease, attach the BDF plugin (`into_session`), drop the session; the result is dropped |
| 2 | `check_prepared_initialization` | `check_prepared_component` again (285 / 302 / 280): lease, attach the plugin, drop the session |
| 3 | `run_prepared_simulation` | `simulate_prepared` (264 / 281 / 276): lease, attach the plugin, `run_to_stop` |

The first one is documented in v0.10.4 and main as a warm-up: "A prepared simulation for repeated runs: instantiated, with every lazily compiled native owner compiled now by one warm-up initialization, so no run pays a compile. The warm-up's own outcome is dropped, because the run that meets a failing initialization reports it." (v0.10.4 has that comment and also a one-shot path, `instantiate_prepared_simulation`, that skips the warm-up. The public one-shot `simulate` still goes through `build_simulation`, so it keeps the warm-up.)

The caller cannot drop the second step without losing the stage. The errors of `lease`, `into_session` and `run_to_stop` leave `simulate_prepared` through `?` with no `at_stage` call, so a failure of the BDF plugin's start (`MeIntegrationError::Numerical`) reaches the caller of `run_prepared_simulation` without a `SimFailureStage`, while failures inside the FMI kernel's initialization carry `MeStage::Initialization` (`crates/rumoca-solver/src/fmi_me/kernel.rs:461,470,501`, v0.10.4). `check_prepared_initialization` runs `into_session` on its own, so a failure there is known to come from initialization. GlobeMode uses that to classify the failure as an initialization failure, on which its `auto` mode does not switch to another solver.

## Steps to reproduce

The cost of the repeated initialization is not shown by a model; the pattern is in the calls above. A model with a nontrivial initialization to try the three calls on is [38-bdf-initialized-state-not-reusable.mo](38-bdf-initialized-state-not-reusable.mo) (`Rep38.A`: one stiff state, an initial equation and an algebraic loop). `rumoca sim 38-bdf-initialized-state-not-reusable.mo -m Rep38.A --solver bdf` runs it on v0.10.2 and v0.10.4 (501 time points). The CLI does not show how many times the component was initialized. [Not measured; not compiled.]

## Expected

A way for the three steps to share one initialization, in whichever shape fits Rumoca's design. For example:

1. `check_prepared_initialization` returns (or keeps) the initialized state and `run_prepared_simulation` continues from it instead of leasing again, or
2. `run_prepared_simulation` puts a `SimFailureStage` (for example `Initialization`) on the error of the plugin's start, so that a caller does not need step 2 to learn where the run failed, or
3. a variant of the build function that skips the warm-up (the one-shot path in v0.10.4 already has the private `instantiate_prepared_simulation`).

## Actual

Three leases. Whether the second and the third solve the initial system again or restore a snapshot was not traced: `MeKernelCell::restart` (`crates/rumoca-solver/src/fmi_me/instance.rs:201`) restores a pristine FMU state when one is kept (`scalar_pristine`) [not verified whether that skips the solve].

## Re-check on Rumoca v0.10.4 and main (S359, 2026-11-16)

The latest release is still v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`). The three steps are the same in v0.10.4 and in main (`b55eafce2475be1721324d56e1c51ddecfe7dbdb`, 2026-11-13; there `check_prepared_component` takes an observation and a `ConstructionPurpose::PreparationCheck`). The v0.10.4 change is the comment and the one-shot path above: the one-shot `simulate_artifact` now skips the warm-up.

## Environment

- Rumoca v0.10.2 (rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, the rev GlobeMode pins), v0.10.4 (https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4) and main (`b55eafce`), compared by reading the source.
- Linux x86_64 (WSL2, Ubuntu). Nothing was built for this draft.

## Workaround in GlobeMode

None. GlobeMode keeps the three steps, because step 2 is what tells it that a failure came from initialization (GlobeMode ADR-0014, S267 addendum). It could not find a model that produces a stage-less failure of the plugin's start (13 small models and MSL `GenerationOfFMUs`, on both of its routes, gave typed or staged errors), so the rule is tested with a constructed error only. The size of the cost on a large model is not known (GlobeMode issue #45 says "not measured").

## Notes

- This is the initialization twin of draft 37 (rk45 has no entry for a lowered model). Both ask for a way to reuse work that the public steps repeat.
- Rumoca's own `auto` mode keeps the BDF probe's prepared component and runs it (`simulate_probed`, `pub(crate)`), so it checks the initialization once and runs once; the public steps have no such reuse.
- Source: GlobeMode issue #45 and its comments (S226, S344), `doc/sessions/S344-solver-budget-issues/解説書.md`, ADR-0014 (S267 addendum).
