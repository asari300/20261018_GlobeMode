# DRAFT (not filed) - A conditional component `if area.use_in` is kept when the redeclared class of `area` sets `use_in = false`

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S373): **bug**. In `Holder h(redeclare model Area = AreaFinal)` the replaceable model `Area` is replaced by `AreaFinal`, which sets `use_in = false` in the modification of its base class (`extends PartArea(final use_in = false)`). `Holder` declares `Real u = 1 if area.use_in;`. The condition is `false` (MLS 3.7 section 4.4.5: a conditional component is removed when its condition is false; section 7.3: `redeclare` replaces the class), so `h.u` must not exist. Rumoca flattens it and keeps `h.u`; the flat model has `h.area.use_in` and `h.u`. omc 1.27.2 has only `h.area.use_in = false`.

Status (S373, 2026-11-16): reproduces on v0.10.2 (the pin of GlobeMode) and v0.10.4 (the latest release). Rumoca main was not built.

## Summary

`PartArea` has `parameter Boolean use_in = true;`. `AreaFinal` extends it with `final use_in = false`. `Holder` has `replaceable model Area = PartArea; Area area;` and `Real u = 1 if area.use_in;`. With `Holder h(redeclare model Area = AreaFinal)` (`A`) the flat model (`--emit flat-json`, key `variables`) contains `h.u`. The controls are removed correctly: the component has the class itself instead of a replaceable one (`B`), `use_in` set by a modification on the component (`C`), a flag of the model itself (`D`). The value of `h.area.use_in` is evaluated right (`false`); only the conditional component is decided with the default of the replaceable class (`true`).

## Steps to reproduce

```bash
# run it from a directory that holds only this file (the compiler also reads sibling .mo files)
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.A --emit flat-json   # redeclare model Area = AreaFinal
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.B --emit flat-json   # control: AreaFinal area
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.C --emit flat-json   # control: h(area(use_in = false))
rumoca compile 44-conditional-component-ignores-redeclared-class.mo -m Rep44.D --emit flat-json   # control: a flag of the model
```

Look at the keys of `variables`.

## Expected

`Rep44.A` has only `h.area.use_in`, like `B`, `C` and `D`.

## Actual

| model | keys of `variables` (v0.10.2) | keys of `variables` (v0.10.4) |
| --- | --- | --- |
| `A` (`redeclare model Area = AreaFinal`) | `h.area.use_in`, `h.u` | `h.area.use_in`, `h.u` |
| `B` (control: `AreaFinal area`) | `area.use_in` | `area.use_in` |
| `C` (control: `h(area(use_in = false))`) | `h.area.use_in` | `h.area.use_in` |
| `D` (control: a flag of the model) | `flag` | `flag` |

omc 1.27.2 `instantiateModel(Rep44.A)`: `final parameter Boolean h.area.use_in = false;` and no `u`.

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-16, checked in S359). Rumoca main was not built.
- Linux x86_64 (WSL2, Ubuntu). omc 1.27.2.

## Workaround in GlobeMode

None needed. GlobeMode's own flat model (`globe-mode-flat`) evaluates the condition with the redeclared class and drops `h.u`. The two differ on the examples that use the form, and `cargo xtask diff-frontend` reports the difference in the variable names. GlobeMode does not copy Rumoca's behavior.

## Notes

- Found in AixLib 3.0.1 (commit `e5df9a8d`): `AixLib.Airflow.WindowVentilation.Examples.VentilationFlowRateSimpleOpening`. The empirical models (`ASHRAE`, `Caciolo`, ...) redeclare `replaceable model OpeningArea = AixLib.Airflow.WindowVentilation.OpeningAreas.OpeningAreaSimple`, which extends `PartialOpeningArea(final use_opnWidth_in = false, ...)`. The partial flow model declares `final parameter Boolean use_opnWidth_in = openingArea.use_opnWidth_in;` and `RealInput opnWidth_in if use_opnWidth_in`. GlobeMode's flat model has no `opnWidth_in`; Rumoca keeps it in each of the 12 components (353 variables against 365). The AixLib model was not run through the CLI (the CLI stops at `PKG-008` for the AixLib tree, a directory that has both a subdirectory and a `.mo` file of the same name `RadiatorTypes`), so the cause in the AixLib example is read from the S369 comparison through the library API (v0.10.2) and the small model above [not verified with the v0.10.4 CLI on the AixLib model].
- The sash form of the same AixLib package (`VentilationFlowRateSashOpening`, `redeclare model OpeningArea = …OpeningAreaSashVDI2078 (use_opnWidth_in = false, …)`) is draft 40 (the class modification is dropped); whether 40 and this draft have the same root is not known.
- Source: GlobeMode S373, MLS 3.7 https://specification.modelica.org/maint/3.7/ (conditional components and `redeclare`; the section numbers were not re-read [not verified]).
