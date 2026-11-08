# DRAFT (not filed) - The `fmi2` / `fmi3` C profile rejects an `initial equation` on a state ("rows over declaration seeds")

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S237): **limitation that is hit by ordinary models** (not a wrong result; the export is refused). `initial equation T = T_start;` for a state `T` is the standard spelling of `T(start = T_start, fixed = true)`, and the latter is exported. Reproduced with Rumoca v0.10.2 (rev `276e82bd`) inside GlobeMode and with the v0.10.4 release binary (`rumoca compile --target fmi2`).

## Summary

A model with an `initial equation` that sets a state from a parameter cannot be written as a `fmi2` / `fmi3` source FMU:

```text
FMI C profile: C initialization evaluates residual rows only on the settled
algebraic refresh; rows over declaration seeds need row isolation and a forward
initial Jacobian
```

The message comes from `validate_projection` in `rumoca-ir-solve/src/fmi/parameter_updates.rs`: the initialization has residual rows and none of them is `UnownedCoordinate(Algebraic)`, `SolvedThroughAlgebraicRefresh` or `SurplusAlgebraicCheck`. The same model with `T(start = T_start, fixed = true)` goes through (the start is "assigned from settled parameters", the other branch of the same file).

In the Modelica Standard Library this is the pattern of `Modelica.Thermal.HeatTransfer.Components.HeatCapacitor` (`initial equation if initType == FixedInitial then T = T_start`) and the many models built on it. In GlobeMode's check of 20 small MSL 4.1.0 examples, 6 are refused with one of the three initialization messages of the same file (`rows over declaration seeds` 2, `can only assign parameters and state starts` 3, `cannot run the homotopy continuation` 1); this draft is about the first.

## Steps to reproduce

```bash
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.A --target fmi2   # control: fixed start values, exports
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.E --target fmi2   # one state, initial equation T = T_start
rumoca compile 35-fmi-c-profile-initial-equation-on-state.mo -m Rep35.F --target fmi2   # two connected capacitors
```

## Expected

`E` and `F` export like `A`. An `initial equation` whose left side is a state and whose right side reads only parameters is, by MLS 8.6 (initialization), the same condition as `fixed = true` with `start` set to that expression. The C profile already accepts the second form.

## Actual

`A`: `FMI 2 ME+CS source FMU written`. `E` and `F`: the message above (v0.10.4 release binary and v0.10.2 as a library).

## Environment

- Rumoca v0.10.2 (rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) as a library, and the v0.10.4 release binary (2026-11-08).
- Linux x86_64 (WSL2, Ubuntu). main not checked [not verified].

## Workaround in GlobeMode

Partial. The same models run as an FMU of GlobeMode's own runtime (P5b, `globe-mode-fmu`, which initializes through `globe-mode-exec`): `E` gives `T(0.9) = 94.176`, as `100 * exp(-0.9 / 15)`. P5a (Rumoca's template) still refuses them.

## Notes

- The other two messages of the same function (`can only assign parameters and state starts`, `cannot run the homotopy continuation`) are hit by `Modelica.StateGraph.Examples.FirstExample` and `Modelica.Blocks.Examples.PID_Controller`. Minimal models for them were not made [not verified].
- The model could be rewritten before the C projection (a state set by `state = expr(parameters)` becomes a start with `fixed = true`), which would be a front-end change on the DAE. Whether that fits Rumoca's design is for the maintainers to say.
