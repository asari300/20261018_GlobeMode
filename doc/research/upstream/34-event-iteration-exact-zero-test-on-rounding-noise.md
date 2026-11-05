# DRAFT (not filed) - Event iteration cycles forever when two relation roots sit at rounding noise (`value == 0.0` is an exact test)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S158): **bug** (a sign test on rounding noise). The same code is in Rumoca v0.10.2 (rev `276e82bd`), v0.10.4 and main (`590ac739`, 2026-11-05).

## Summary

`rumoca-solver`, `runtime/solve_runtime/relation_memory.rs`, decides the side of a relation root with exact comparisons to zero:

- `mode_is_consistent` (line 708 in v0.10.4 and main): `value == 0.0 || relation_memory_value_from_root(value) == *side`
- `release_reversed_relation_overrides` (line 949): `|| roots[*index] == 0.0`

When the continuous state at an event time is exactly balanced, the algebraic projection returns root values that are rounding noise (1e-29 to 1e-35), not exact zeros, and their signs flip with the mode that was just pinned. The mode search then "finds" a consistent mode, the pinned side is released in the next pass because the noise changed sign, and the iteration cycles with period 2 until `fixed_point_cap` (32) is used up:

```text
[EventIteration] event condition equations did not converge with fixed pre at t=13.040017098156884
```

The cycle is detected (`on_relation_surface`; `y` differs by 1e-29 between passes) and `resolve_relation_cycle` returns `Found` each time, so the "relation surface" exit is never taken.

## Steps to reproduce

The case was found in a GlobeMode run (the integrator is GlobeMode's own `sdirk34hw` plugin; Rumoca's CLI does not have it). The attached `34-event-iteration-exact-zero-test-on-rounding-noise.mo` only fixes the experiment settings of the MSL example:

```bash
globe simulate -L data/msl 34-event-iteration-exact-zero-test-on-rounding-noise.mo -m Repro34 --solver sdirk34hw
# error[GM-E11003] ... t = 13.040017098 ... [EventIteration] event condition equations did not converge with fixed pre
```

Whether the Rumoca CLI (its own BDF) reaches the same exactly balanced state was not tried. [not verified] The state that triggers it is a coincidence of the step sequence: the same model with GlobeMode's `esdirk34` finishes the 15 s run (GitHub Actions run 41666815770), and `sdirk34hw` with `--stop-time 13.2` passes too.

## What happens (trace, GlobeMode S158)

Model `Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive`, Tolerance 1e-7, three events in a 1.2 microsecond window (firing at 13.04001588, then a state event at 13.0400170981568). At the located event time the two inductor currents are bit-exact opposites: `dcpm.la.i = 19.95342803660142`, `lMains.i = -19.95342803660142` (sum 0.0).

1. Mode search over the candidate roots (index 1 and 4): in every mode the roots are `(+4.1e-30, -4.1e-35)`; mode `[0, 1]` is accepted and pinned as an override.
2. The next pass re-projects: roots `(-3.3e-34, +3.3e-29)`, both signs flipped. The override is "reversed" and released (it is kept only at an exact zero).
3. The next search accepts `[1, 0]` and the roots flip back. Period 2, `y` changes by 1e-29 per pass, `p` toggles six entries (the thyristor `off` memories).

## Expected

When a root is zero up to the projection's own noise, the relation should keep its side (MLS Appendix B, which the Rumoca source comments cite for event iteration), the way the existing relation-surface rule does for a cycle that is detected. A tolerance relative to the magnitude of the operands would do; GlobeMode used an absolute `1e-24` only to confirm the diagnosis.

## Confirmation (GlobeMode, local patch, not proposed as is)

With both comparisons changed to `value.abs() <= 1e-24`, the same run goes to the end (StopTime 15, 87,498 output rows, 64.7 s). The rest of the MSL examples were not re-run with the patch. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, MSL 4.1.0; the code lines above were compared with v0.10.4 and main `590ac7398fc42a383dedf4bf10ccc023353f8c54` through the GitHub API on 2026-11-05 (the two comparisons are unchanged). The run itself was not repeated on v0.10.4 or main. [not verified]
- Linux x86_64 (WSL2, Ubuntu); GlobeMode `globe` release build.

## Workaround in GlobeMode

None. `sdirk34hw` fails on this one MSL example (`GM-E11003`); `esdirk34`, `auto` and the other solvers do not hit it. The failure is recorded in `doc/msl-solvers.md` and tested as a known limit (`msl_sdirk34hw_thyristor_bridge_event_iteration_is_a_known_limit`, `#[ignore]`).

## Notes

- Draft 02 (the sampler endpoint has no rounding floor) is the same kind of omission in a different place.
- A failing event is not specific to `sdirk34hw`: any integrator whose interpolated state lands bit-exactly on the balance can reach it. `esdirk34` also passes through a state with `la.i + lMains.i == 0.0` (t = 13.2000171) without failing; which pass settles depends on the noise.
