# DRAFT (not filed) - A parameter of the predefined enumeration `AssertionLevel` (or `StateSelect`) is dropped from the flat model

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S373): **minor bug** (a missing variable, no wrong equation found). A component `parameter AssertionLevel level = AssertionLevel.warning;` is an ordinary parameter of the predefined enumeration type `AssertionLevel` (MLS 3.7 section 8.3.7 describes `assert` and `AssertionLevel`; the section number was not re-read [not verified]). The flat model of Rumoca has no variable `level`. The same holds for `parameter StateSelect sel = StateSelect.prefer;`. A parameter of an enumeration that the user declares is kept (control `C`). omc 1.27.2 keeps both.

Status (S373, 2026-11-16): reproduces on v0.10.2 (the pin of GlobeMode) and v0.10.4 (the latest release). Rumoca main was not built.

## Summary

The flat model (`--emit flat-json`, key `variables`) of `A` and `B` has only `x`; `C` has `level` and `x`. A reference to the dropped parameter in an equation is replaced by its value (`x = if level == AssertionLevel.warning then time else 0` becomes `(AssertionLevel.warning == AssertionLevel.warning)`), so the model still compiles. What is lost is the variable itself: it cannot be modified from outside after the flattening, it is not in the list of parameters, and the set of variable names differs from other tools.

## Steps to reproduce

```bash
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.A --emit flat-json   # AssertionLevel
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.B --emit flat-json   # StateSelect
rumoca compile 43-predefined-enumeration-parameter-dropped.mo -m Rep43.C --emit flat-json   # control: a user enumeration
```

Look at the keys of `variables`.

## Expected

`Rep43.A` has `level` and `x`; `Rep43.B` has `sel` and `x`; like `C` and like omc.

## Actual

| model | keys of `variables` (v0.10.2) | keys of `variables` (v0.10.4) |
| --- | --- | --- |
| `A` (`AssertionLevel`) | `x` | `x` |
| `B` (`StateSelect`) | `x` | `x` |
| `C` (control: user enumeration) | `level`, `x` | `level`, `x` |

omc 1.27.2 `instantiateModel(Rep43.A)`: `parameter enumeration(warning, error) level = AssertionLevel.warning;`; `Rep43.B`: `parameter enumeration(never, avoid, default, prefer, always) sel = StateSelect.prefer;`.

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-16, checked in S359). Rumoca main was not built.
- Linux x86_64 (WSL2, Ubuntu). omc 1.27.2.

## Workaround in GlobeMode

None needed. GlobeMode's own flat model (`globe-mode-flat`) keeps the parameters of `AssertionLevel` (like omc) and drops those of `StateSelect` only to keep the set of variable names equal to Rumoca's (`flatten.rs`, `ty == "StateSelect"`). `cargo xtask diff-frontend` reports the difference in the names for `AssertionLevel`.

## Notes

- Found in ThermofluidStream 1.4.1 (commit `a2faa411`): `ThermofluidStream.DropOfCommons` declares `parameter AssertionLevel assertionLevel = AssertionLevel.error;`. `ThermofluidStream.Idealized.Examples.TUMExercisesThermodynamicCycles.Exercise3OttoEngine.BaseModel` and `Exercise4DieselEngine.BaseModel` (they contain `inner DropOfCommons dropOfCommons`) have the variable `dropOfCommons.assertionLevel` in GlobeMode's flat model and not in Rumoca's (22 against 21 and 19 against 18 variables; the equation counts are 0 and 0). They are 2 of the 25 examples that Rumoca flattens in that library (the S373 look at the run of S369).
- Source: GlobeMode S373, from the `libraries.yml` comparison of S369.
