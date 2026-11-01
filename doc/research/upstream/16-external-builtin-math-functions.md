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

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, as pinned by GlobeMode.
- Modelica Standard Library 4.0.0 (Math/package.mo), 13 `external "builtin"` functions.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

Yes (S068, ADR-0029). After compiling, GlobeMode serializes the checked `Dae` (it implements serde), replaces every `call` node of a function that is `external "builtin"`, whose symbol is one of the built-ins above and whose arguments are exactly its inputs in order, by a `builtin` node with the same operands, and reads the `Dae` back (Rumoca re-checks it). This works but relies on the serde wire shape, which is not a stable API. A fix inside Rumoca is either to translate such a call to `PureBuiltin` when the DAE is constructed, or to evaluate `ExternalLanguage::Builtin` in the numeric evaluator and the lowering.

## Notes

- Source: `doc/research/external-builtin-2026-11-01.md`.
- Impact measured by GlobeMode (S067): with MSL 4.0.0, 70 examples of Buildings, AixLib, IBPSA and DeltaRobot fail only for this reason.
