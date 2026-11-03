# DRAFT (not filed) - Rumoca rejects a conditional component whose condition reads a constant of another package (EI006)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S072): **Rumoca rejects valid Modelica.** The condition is a parameter expression (it is built from package constants); omc accepts it.

## Summary

`Real X_w if M.nXi > 0;` (where `M` is a package and `nXi` a constant in it) is rejected with `[EI006] conditional component 'X_w' requires parameter expression`. The same happens when the condition reads a `final parameter` whose binding is a package constant (`single = (Medium.nX == 1)`, `x if not single`), and when the condition is a package constant written directly (`x if Medium.nX > 1`). The same condition on a constant or a parameter of the model itself compiles.

In real code this is `IBPSA.Fluid.FMI.Interfaces.FluidProperties` (`X_w if Medium.nXi > 0`) and `IBPSA.Fluid.Sources.MassFlowSource_WeatherData` (`x_pTphi if (not singleSubstance)`), with copies in Buildings and AixLib: 117 examples in GlobeMode's run of seven public libraries (Buildings 54, IBPSA 30, the rest in AixLib; 40 name `X_w`, 27 `x_pTphi`).

## Steps to reproduce

Compile the attached `24-ei006-condition-on-package-constant.mo` (self-contained):

```bash
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.A   # connector, replaceable package constant
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.B   # final parameter bound to a package constant
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C   # package constant written directly
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C0  # control: local constant
rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C1  # control: local parameter
```

## Expected

All five compile. MLS 3.6 section 4.4.5 "Conditional Component Declaration" (https://specification.modelica.org/maint/3.6/class-predefined-types-and-declarations.html, checked 2026-11-02): "The expression must be a Boolean scalar expression, and must be a parameter expression." Section 3.8.3 "Parameter Expressions" (https://specification.modelica.org/maint/3.6/operators-and-expressions.html, checked 2026-11-02) counts constant expressions, parameter variables and operators applied to parameter subexpressions; section 3.8.2 counts constant variables. A package constant compared with a literal is a constant expression.

## Actual

```text
$ globe check 24-ei006-condition-on-package-constant.mo --model Rep24.C
24-ei006-condition-on-package-constant.mo:30:22: error[GM-E3001] S3 instantiate: conditional component `x` requires parameter expression
    note: rumoca: EI006
Rep24.C: NG（S3 instantiate で停止）

$ rumoca compile 24-ei006-condition-on-package-constant.mo -m Rep24.C
  [EI006] conditional component `x` requires parameter expression
```

`Rep24.A` and `Rep24.B` give the same code; `Rep24.C0` and `Rep24.C1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all five "completed successfully" (`Rep24.A` with a warning that the connector is unbalanced, which is a property of the repro, not of the condition).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged: `Rep24.A`, `Rep24.B` and `Rep24.C` give the same `[EI006] conditional component ... requires parameter expression`; `Rep24.C0` and `Rep24.C1` compile (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 run (GitHub Actions run 41250636800), `doc/research/libraries-2026-11-01.md`: EI006 117 of 1,781 failures. Library: IBPSA 4.0.0 (master `2adc9194`), `IBPSA/Fluid/FMI/Interfaces/FluidProperties.mo` line 16; reproduced with `IBPSA.Fluid.FMI.ExportContainers.Examples.FMUs.Fan`.
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit; the examples stop at S3 (`GM-E3001`).

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-instantiate/src/conditional_components.rs`, `decide_condition` (about lines 205-228) evaluates the condition with `evaluate_component_condition_with_outer_values`, and retries once only when the condition mentions an `import` alias. A dotted reference to a constant of a package that is not imported (`Medium.nX`, `M.nXi`) apparently is not resolved by that evaluator, so the condition stays undecided and `mark_disabled_component_if_needed` reports `ConditionalError` (`EI006`). [Inference from the outcome; not stepped through.]
- Other condition names in the library run (`solRad`, `capFil1`, `resExtWallFloor`, `steBal`, `uSha`, ...) were not reduced; they may have other causes.
