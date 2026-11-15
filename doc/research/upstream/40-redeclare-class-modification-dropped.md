# DRAFT (not filed) - The modification in `redeclare model M = Impl(show = true)` is dropped, so a conditional component guarded by `show` is missing

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S362): **bug**. In `Holder h(redeclare model M = Impl(show = true))` the short class definition `M = Impl(show = true)` modifies the class `Impl` that `M` stands for (MLS 3.7 section 7.2: a modification can occur in a short class definition, with the example `redeclare type Voltage = Real(unit = "V")`; section 7.3: `redeclare` replaces the class). Rumoca flattens `h.m.show` as `false` (the default of `Impl`), so a component declared with `if show` is not created. omc 1.27.2 and the same modification written on the component both give `show = true`.

Status (S362, 2026-11-16): reproduces on v0.10.2 (the pin of GlobeMode) and v0.10.4 (the latest release). Rumoca main was not built.

## Summary

`replaceable model M = Impl;` in a `Holder`, and `Holder h(redeclare model M = Impl(show = true))` in the model that uses it. `Impl` extends a `Base` with `parameter Boolean show = false;` and `Real y = 2 if show;`. The flat model (`--emit flat-json`) of the redeclared form has `h.m.show` bound to the literal `false` and no variable `h.m.y`. The control forms, `Holder h(m(show = true))` and a direct `Impl m(show = true)`, bind `show` to `true` and keep `y`.

## Steps to reproduce

```bash
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.A --emit flat-json   # redeclare model M = Impl(show = true)
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.B --emit flat-json   # control: h(m(show = true))
rumoca compile 40-redeclare-class-modification-dropped.mo -m Rep40.C --emit flat-json   # control: Impl m(show = true)
```

Look at `variables`: the keys and the `binding` of `h.m.show`.

## Expected

`Rep40.A` has `h.m.show = true` and the variable `h.m.y`, like `B` and `C`.

## Actual

| model | `show` | variables |
| --- | --- | --- |
| `A` (`redeclare model M = Impl(show = true)`) | `false` | `h.m.show`, `h.m.x` |
| `B` (`h(m(show = true))`, control) | `true` | `h.m.show`, `h.m.x`, `h.m.y` |
| `C` (`Impl m(show = true)`, control) | `true` | `m.show`, `m.x`, `m.y` |

Same on v0.10.2 and v0.10.4. omc 1.27.2 `instantiateModel(Rep40.A)`: `final parameter Boolean h.m.show = true;` and `Real h.m.y = 2.0;`.

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-16, checked in S359). Rumoca main was not built.
- Linux x86_64 (WSL2, Ubuntu). omc 1.27.2.

## Workaround in GlobeMode

None needed. GlobeMode's own flat model (`globe-mode-flat`) applies the modification and keeps `h.m.y`, so it differs from Rumoca on the examples that use the form. Track B records the difference (`doc/adr/0039-native-cst-parser.md`, notes S355 and S362) and does not copy Rumoca's behavior.

## Notes

- MSL 4.1.0 (`Fluid/Pipes.mo`, `Fluid/Examples/InverseParameterization.mo` and `TraceSubstances.mo` read, 2026-11-16): in `Modelica.Fluid.Examples.InverseParameterization`, `pipe1` has `redeclare model FlowModel = Modelica.Fluid.Pipes.BaseClasses.FlowModels.NominalTurbulentPipeFlow(show_Res = true, …)` and `pipe2` has `redeclare model FlowModel = …FlowModels.NominalLaminarFlow(show_Res = true, …)`. The declarations with `if show_Res` are `Res[n]` and `m_flows_turbulent[n-1]` of `PartialStaggeredFlowModel` (both pipes), `dps_fg_turbulent[n-1]` of `NominalTurbulentPipeFlow` (`pipe1` only) and `pathLengths_nominal[n-1]` of `NominalLaminarFlow` (`pipe2` only; it is unconditional in `NominalTurbulentPipeFlow`); `StaticPipe` has `n = 2`. GlobeMode's flat model has 6 more variables (513 against 507; `pipe1`: `Res`, `m_flows_turbulent`, `dps_fg_turbulent`; `pipe2`: `Res`, `m_flows_turbulent`, `pathLengths_nominal`) and the DAE 8 more scalar equations and unknowns (287 against 279; 4 per pipe). The 2 other examples whose variable names differ in the S355 comparison, `RoomCO2` and `RoomCO2WithControls` of `Modelica.Fluid.Examples.TraceSubstances`, use `…FlowModels.DetailedPipeFlow(show_Res = true)`, which adds only `Res` and `m_flows_turbulent` (`DetailedPipeFlow` has no `dps_fg_turbulent`; 293 against 291 and 748 against 744 variables); the cause is the same. Whether Rumoca's own balance of these examples would be affected is not known.
- Source: GlobeMode S355 (found with a small model) and S362 (this draft, the DAE comparison of `InverseParameterization`), MLS 3.7 https://specification.modelica.org/maint/3.7/inheritance-modification-and-redeclaration.html (sections 7.2 and 7.3; the headings and the example of 7.2 were read, 2026-11-16).
