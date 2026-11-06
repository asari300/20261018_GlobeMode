# DRAFT (not filed) - Rumoca rejects valid `annotation(Evaluate = true)` on a non-parameter component with ER070

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **Rumoca rejects valid Modelica** (a hard error where the specification says "no effect"). The library code is legal; omc accepts it. The error text also cites a wrong section number.

## Summary

`Modelica.Blocks.Sources.Constant Dzero(final k = 0) if not with_D annotation(Evaluate = true)` (a block instance, not a parameter) is rejected with `[ER070] annotation Evaluate is only allowed on parameter or constant components; 'Dzero' is not parameter or constant (MLS §18.6)`. The annotation is meant for the parameter `k` inside the block; Dymola, OpenModelica and others accept it on the component. Rumoca already treats the same annotation on function locals as a warning (`WR006`, "has no effect"), and the comment next to that code quotes the right sentence of the specification; the component case keeps a hard error.

In real code this is `IBPSA.Controls.Continuous.LimPID` (`Dzero`), which every PID user hits: 258 examples in GlobeMode's run of seven public libraries (Buildings 173, AixLib 58, the remaining 27 in IBPSA by subtraction; 250 of the 258 name `Dzero`).

## Steps to reproduce

Compile the attached `18-er070-evaluate-on-non-parameter-component.mo` (needs the MSL for `Modelica.Blocks.Sources.Constant`):

```bash
rumoca compile 18-er070-evaluate-on-non-parameter-component.mo -m Rep18 --source-root <MSL root>
rumoca compile 18-er070-evaluate-on-non-parameter-component.mo -m Rep18Plain --source-root <MSL root>   # same model without the annotation
```

## Expected

Both compile. MLS 3.6 section 18.3 "Symbolic Processing", Annotation 18.1 `Evaluate` (https://specification.modelica.org/maint/3.6/annotations.html, checked 2026-11-01, text read from the page): "The annotation Evaluate can occur in the component declaration, its type declaration, or a base class of the type-declaration. ... In the case of hierarchical components it is applied to all components, overriding any Evaluate-setting for specific components. The annotation Evaluate only has effect for a component declared with the prefix parameter." So a component declaration is a listed place for it; on a non-parameter it has no effect, and for a hierarchical component (a block instance) it is applied to the parameters inside. The specification names no error. (Section 18.6 is "Graphical Objects" in 3.6; the section cited by the message is wrong.)

## Actual

```text
$ globe check 18-er070-evaluate-on-non-parameter-component.mo --library data/msl --model Rep18
18-er070-...mo:7:5: error[GM-E1001] S1 名前解決: annotation Evaluate is only allowed on parameter or constant components; 'Dzero' is not parameter or constant (MLS §18.6)
    note: rumoca: ER070
Rep18: NG（S1 名前解決 で停止）

$ rumoca compile 18-er070-evaluate-on-non-parameter-component.mo -m Rep18 --source-root data/msl
  [ER070] annotation Evaluate is only allowed on parameter or constant components; 'Dzero' is not parameter or constant (MLS §18.6)
```

`Rep18Plain` compiles in both. OpenModelica 1.27.2 (`loadModel(Modelica)`, `loadFile`, `checkModel`): both `Rep18` and `Rep18Plain` "completed successfully" (2 equations, 2 variables).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged: `Rep18` gives the same `[ER070] annotation Evaluate is only allowed on parameter or constant components; 'Dzero' is not parameter or constant (MLS §18.6)` (with `--source-root` pointing at MSL 4.1.0); `Rep18Plain` compiles (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Modelica Standard Library 4.1.0 (the attached file only needs `Modelica.Blocks.Sources.Constant`).
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 classification, `doc/research/libraries-2026-11-01.md`: ER070 258 of 1,781 failures (Buildings 173, AixLib 58). Library: IBPSA 4.0.0 (master `2adc9194`), `IBPSA/Controls/Continuous/LimPID.mo` line 134; copies in Buildings 14.0.0 and AixLib 3.0.1.
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None (the annotation could be stripped before checking, but GlobeMode does not rewrite sources). Recorded as a Rumoca limit.

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-resolve/src/semantic_checks/annotations.rs`, `check_component_evaluate_annotations` (about lines 70-125). Components with a `parameter`/`constant` prefix return early; function locals get warning `WR006` (about lines 98-110); every other class kind gets the hard `ER070`. The doc comment on that function already says why function locals are only a warning ("the annotation Evaluate only has effect for a component declared with the prefix parameter"), and notes that MSL 4.1.0 `Electrical.Machines.SpacePhasors.Functions.ToSpacePhasor` relies on it.
- Suggested change: treat a non-parameter component the same way (warning `WR006`, annotation dropped). The same function also rejects `Evaluate` on a class and on an `extends` clause (`check_non_component_evaluate_annotations`); the specification explicitly allows it "in its type declaration, or a base class of the type-declaration", which is the same family of cases. [Not tested here.]
- The cited section number is wrong in both messages (`MLS §18.6` is "Graphical Objects"; `Evaluate` is in 18.3).
