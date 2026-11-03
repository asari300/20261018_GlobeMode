# DRAFT (not filed) - Rumoca loses a field of a record constant declared in a package (ED008 unresolved Flat reference)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S072): **Rumoca internal error on valid Modelica.** omc accepts it.

Status (S115, 2026-11-03): **fixed in Rumoca v0.10.4.** `Rep26.A`, `Rep26.B` and `Rep26.C` compile with the v0.10.4 release binary. Do not file; this draft is kept as a record. (The `i` group of ED008, 18 examples, was never reduced and is not covered by this check.)

## Summary

A package declares `constant GasProperties dryair(MM = 0.029, cp = 1006);` (a record constant). Reading `dryair.cp` from a function of the package, from a model (`Air.dryair.cp`), or in a parameter binding stops the DAE construction with:

```text
[ED008] unresolved Flat reference `Rep26.Air.dryair.cp`
```

A scalar constant of the same package (`Air.cpAir`) and a record constant declared in the model itself both compile.

In real code this is `IBPSA.Media.Air` (`constant GasProperties dryair(...)`, read as `dryair.cp` in `enthalpyOfDryAir` and others), with copies in Buildings and AixLib, and the same shape in `Modelica.Media.IdealGases.Common.SingleGasesData.Air.R_s` and ThermofluidStream's media. It is the largest group of ED008 in GlobeMode's run of seven public libraries: 47 of 108 name `<Library>.Media.Air.dryair.cp`, 6 `SingleGasesData.Air.R_s`, 18 name a loop variable `i` (a different shape, not reduced).

## Steps to reproduce

Compile the attached `26-ed008-field-of-package-record-constant.mo` (self-contained):

```bash
rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.A    # function of the package reads dryair.cp
rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.B    # model reads Air.dryair.cp
rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.C    # parameter bound to Air.dryair.cp
rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.B0   # control: scalar package constant
rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.B1   # control: record constant in the model
```

## Expected

All five compile. A constant in a package may be referenced by its dotted name (MLS 3.6 section 5.3.2, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-02), and a record component of a constant is a constant (section 3.8.2, https://specification.modelica.org/maint/3.6/operators-and-expressions.html, checked 2026-11-02).

## Actual

```text
$ globe check 26-ed008-field-of-package-record-constant.mo --model Rep26.B
26-ed008-field-of-package-record-constant.mo:26:36: unsupported[GM-U0002] S5 方程式数と変数数: unresolved Flat reference `Rep26.Air.dryair.cp`
    note: rumoca: ED008
Rep26.B: NG（S5 方程式数と変数数 で停止）

$ rumoca compile 26-ed008-field-of-package-record-constant.mo -m Rep26.B
  [ED008] unresolved Flat reference `Rep26.Air.dryair.cp`
```

`Rep26.A` and `Rep26.C` give the same code; `Rep26.B0` and `Rep26.B1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all five "completed successfully".

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Fixed: `Rep26.A`, `Rep26.B` and `Rep26.C` compile (`Compilation successful!`, exit 0); `Rep26.B0` and `Rep26.B1` still compile. With v0.10.2 the three still give `[ED008] unresolved Flat reference 'Rep26.Air.dryair.cp'`. The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 run (GitHub Actions run 41250636800), `doc/research/libraries-2026-11-01.md`: ED008 108 of 1,781 failures. Library: IBPSA 4.0.0 (master `2adc9194`), `IBPSA/Media/Air.mo` line 25 (declaration); reproduced with `IBPSA.Airflow.Multizone.Examples.Orifice`.
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca internal error (`GM-U0002`).

## Notes

- Where, in rev `276e82b` (names only): the error is `ToDaeError::UnresolvedReference` in `crates/rumoca-phase-dae/src/errors.rs` (about line 74, help text "Flat IR must carry a declared, fully resolved coordinate identity"). The record constant of a package apparently is not given a Flat variable (or is not expanded into its fields) when only a field is referenced, while a scalar package constant is inlined. [Inference from the controls; not traced.]
- The `i` group (for example `IBPSA.Fluid.Sensors.Examples.MoistAirEnthalpyFlowRate`) and ThermofluidStream's `...Constants.h_off` were not reduced.
