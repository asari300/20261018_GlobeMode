# DRAFT (not filed) - `external "builtin"` functions that only call a Modelica built-in (`sin`, `exp`, ...) cannot be executed

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

A function whose body is `external "builtin" y = sin(u);` is accepted by the front end and the DAE (`ExternalLanguage::Builtin`, symbol `sin`), but every runtime path refuses to execute it: `external builtin function 'mySin' calls 'sin', which this runtime cannot execute` (`EL005` from structural analysis, `EX002` from the solve-IR lowering). The Modelica Standard Library 4.0.0 writes 13 functions of `Modelica.Math` this way (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `exp`, `log`, `log10`), so any 4.0.0 model that reaches one of them (including `Modelica.Blocks.Math.Sin` and the MultiBody frame functions) cannot be simulated. MSL 4.1.0 rewrote them as `algorithm y := .sin(u);` and is not affected.

The DAE already has the target of the translation: `PureBuiltin` has `Sin`, `Cos`, `Tan`, `Asin`, `Acos`, `Atan`, `Atan2`, `Sinh`, `Cosh`, `Tanh`, `Exp`, `Log`, `Log10`, `Sqrt`, `Abs`.

## Steps to reproduce

[16-external-builtin-math-functions.mo](16-external-builtin-math-functions.mo) (self-contained, independent of the MSL version). With the MSL itself, any 4.0.0 example that calls `Modelica.Math.sin`, for example `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum` with `MODELICAPATH` set to MSL 4.0.0. Observed through GlobeMode (`globe simulate -L <msl-4.0.0> -m ...`), which calls Rumoca's library API. Not confirmed with the Rumoca CLI [not verified].

## Expected

The model runs, with `mySin(u)` equal to `sin(u)` (MLS section 12.9.1: a function with `external "builtin"` is one of the built-in functions of the language specification).

## Actual

```text
[EL005] DAE structural proof failed: checked DAE scalar projection failed: external builtin function `mySin` calls `sin`, which this runtime cannot execute
```

The message comes from `rumoca-eval-dae/src/numeric.rs` (`external_function_failure`), `rumoca-eval-dae/src/projection.rs` (`ProjectionError::ExternalFunction`) and the solve-IR lowering. Language `builtin` is classified in `rumoca-ir-dae/src/model/external_functions.rs` (`ExternalLanguage::from_declared`) but never executed.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still reproduces; the message changed. `rumoca sim` on `16-external-builtin-math-functions.mo` (`-m ExternalBuiltin`):

| Rumoca | message |
| --- | --- |
| v0.10.2 | `[EL005] DAE structural proof failed: checked DAE scalar projection failed: external builtin function 'mySin' calls 'sin', which this runtime cannot execute` |
| v0.10.4 | `[EL001] unsupported checked DAE semantics: external builtin function 'mySin' calls 'sin', which the Solve runtime cannot execute` |

MSL 4.0.0 `Modelica.Mechanics.MultiBody.Examples.Elementary.Pendulum`: v0.10.4 `[EL001] unsupported checked DAE semantics: external builtin function 'Modelica.Math.cos' calls 'cos', which the Solve runtime cannot execute` (v0.10.2: the `[EL005]` form).

The addendum (S073) is partly addressed: for a parameter binding (`final parameter Real c = mycos(til)`), v0.10.4 now names the cause, `[EL001] unsupported checked DAE semantics: parameter 'c' depends on other parameters, so its binding must be re-applied after parameter changes, and its expression has no checked initialization-update lowering: external builtin function 'mycos' calls 'cos', which the Solve runtime cannot execute`, where v0.10.2 gave the generic `[EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid`. The root cause (`external "builtin"` is not executed) is unchanged. The Buildings examples named in the addendum were not re-run. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, as pinned by GlobeMode.
- Modelica Standard Library 4.0.0 (Math/package.mo), 13 `external "builtin"` functions.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Yes (S068, ADR-0029). After compiling, GlobeMode serializes the checked `Dae` (it implements serde), replaces every `call` node of a function that is `external "builtin"`, whose symbol is one of the built-ins above and whose arguments are exactly its inputs in order, by a `builtin` node with the same operands, and reads the `Dae` back (Rumoca re-checks it). This works but relies on the serde wire shape, which is not a stable API. A fix inside Rumoca is either to translate such a call to `PureBuiltin` when the DAE is constructed, or to evaluate `ExternalLanguage::Builtin` in the numeric evaluator and the lowering.

## Notes

- Source: `doc/research/external-builtin-2026-11-01.md`.
- Impact measured by GlobeMode (S067): with MSL 4.0.0, 70 examples of Buildings, AixLib, IBPSA and DeltaRobot fail only for this reason.

## Addendum (S073, 2026-11-02): in a parameter binding the failure is a generic contract error

When the `external "builtin"` call sits in a parameter binding instead of an equation, the same root cause surfaces as a generic solve-IR contract error, without naming the function:

```modelica
model InParam "same mycos as in the reproduction file"
  parameter Real til = 1.5707963267948966;
  final parameter Real c = mycos(til);
  Real x(start = 1, fixed = true);
equation
  der(x) = -c * x;
end InParam;
```

```text
[EL005] invalid Solve IR contract: pure-call argument or slot interface is invalid
```

The equation form (`der(x) = -mycos(time) * x`) gives the clear message above (`external builtin function 'mycos' calls 'cos', which this runtime cannot execute`). The message comes from `rumoca-phase-solve/src/error.rs` with `SolveProgramConstructionError::InvalidCallInterface` (`rumoca-ir-solve/src/typed_program/program/construction_error.rs`), which has many raise sites in `rumoca-phase-solve/src/lower/typed_functions.rs`; which one fires was not traced [not verified]. With MSL 4.0.0 this is how `Buildings.HeatTransfer.Convection.Examples.Interior` and `Buildings.HeatTransfer.Windows.BaseClasses.Examples.InteriorConvection` (Buildings 14.0.0, `final parameter Real cosTil = Modelica.Math.cos(til)`) fail. Suggested fix besides executing `builtin`: report the parameter path with the same `cannot execute` message.
