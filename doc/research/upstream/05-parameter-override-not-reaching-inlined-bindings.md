# DRAFT (not filed) - Runtime parameter overrides do not reach bindings that instantiate inlined from a same-name modifier

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

`SimOptions::param_overrides` (runtime override of a tunable parameter) changes only the outer parameter when an inner component receives it through a same-name modifier such as `signalSource(final f = f)` (MSL `SineVoltage`: `f`, `phase`, `startTime`; `StepVoltage`: `startTime`). The instantiate phase writes the outer modifier's value into the inner binding, so the DAE no longer links the inner parameter to the outer one. The override is accepted without any warning and the result is silently wrong. Time-event times are one visible case: the DAE phase folds `time < startTime` with the binding default (`time_event_instant` / `constant_context()`), so the static time event stays at the default time.

## Steps to reproduce

1. Compile the attached `05-parameter-override-not-reaching-inlined-bindings.mo` (model `InlinedParameters`) with the library API and run with the runtime override `src.startTime = 0.5` (`SimOptions::param_overrides`).
2. Read `y` at t = 1.

MSL form: a `SineVoltage V(V = 1, f = 1)` between a resistor and ground, override `V.f = 2`, read `V.v(0.05)`.

The reproduction goes through the library API; we are not sure whether the `rumoca` CLI exposes runtime overrides [not verified].

## Expected

`y(1) = 0.5` for `src.startTime = 0.5` (default 1 gives `y(1) = 0`). For the MSL form: `V.v(0.05) = sin(2 pi 2 0.05) = 0.588`.

## Actual

`y(1) = 0` (the override has no effect on `r.startTime`). MSL form: `V.v(0.05) = 0.309`, which is the `f = 1` value (GlobeMode S024, 2026-10-28).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

Yes (S024, ADR-0022). When overrides are given, GlobeMode clones a library-only session, embeds the overrides with `Session::set_structural_overrides`, and recompiles. This costs about 2 s and 0.4 GB more on `CauerLowPassSC`. See draft 06 for the problems found on that path.

## Notes

- Source: `doc/research/msl-sim-failures.md` (sections "S023", "S024", candidates 16 and 18). The cause was read at `rumoca-phase-instantiate` `mod_env.rs:694` and `:722` (`resolve_single_part_ref_expr`); exact lines were not traced to the end [not verified].
- Folding of time events by `constant_context()` (candidate 16) is one manifestation of the same cause, not an independent bug.
