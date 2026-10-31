# DRAFT (not filed) - EL005 "structurally singular" on higher-index DAEs in the MSL examples

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

`rumoca sim` stops with `[EL005] DAE structural proof failed: structurally singular system` on MSL examples that need index reduction (a differentiated algebraic variable, mechanical/electrical loops with constraints). The smallest one is `Modelica.Blocks.Examples.FilterWithDifferentiation` (14 matched out of 15 equations). In GlobeMode's MSL run (S011) 18 example models stop at this error; none of them is in Rumoca's own `msl_quality_baseline.json` `sim_ok` list, so this looks like a known gap rather than a regression. We are asking whether index reduction (dummy derivatives or similar) is planned, or whether a better message (which variables/equations are unmatched) can be produced.

## Steps to reproduce

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Blocks.Examples.FilterWithDifferentiation --source-root <MSL root>
```

No `.mo` file is needed (MSL example). Confirmed with the Rumoca v0.10.2 CLI binary (GlobeMode S011).

## Expected

The model simulates (it is a regular MSL example; Rumoca's own baseline does not list it under `sim_ok`).

## Actual

```text
[StructuralAnalysis] solve-IR evaluation failed: [EL005] DAE structural proof failed: structurally singular system: 14 matched out of 15 equations and 15 unknowns
```

(message form: `crates/rumoca-phase-structural/src/types.rs:145-147`, `crates/rumoca-phase-solve/src/error.rs:102` at the rev below.) Other models with the same error in GlobeMode S011: transformers, induction machines, FluxTubes, Media (17 more).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

None. GlobeMode reports the run as unsupported (`GM-U0004`) and shows Rumoca's message.

## Notes

- Observed by GlobeMode S011; details in `doc/research/msl-sim-failures.md` ("Upstream candidates", item 1).
- Some other models stop with a different EL005 variant (`DAE system is not computable: ... linear solve requires an aggregate function owner`, e.g. `InverseParameterization`); that is a separate symptom and is not part of this draft.
