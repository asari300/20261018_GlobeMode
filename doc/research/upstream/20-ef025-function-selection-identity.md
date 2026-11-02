# DRAFT (not filed) - Rumoca fails with EF025 on calls of a function built from two `extends` clauses (e.g. MSL `Files.loadResource`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **Rumoca rejects valid Modelica, for the `loadResource` shape (reproduced with MSL only and with a self-contained model).** S080 reduced the replaceable-Medium shape too (a different message of the same code); see "Minimal reproduction: two Medium packages" below.

## Summary

Calling `Modelica.Utilities.Files.loadResource("modelica://...")` in a parameter binding stops with `[EF025] missing exact function-selection identity for 'Modelica.Utilities.Files.loadResource': exposed function has no unique exact extends implementation`. In MSL, `loadResource` has no body of its own: it has two `extends` clauses, one for the interface (`...PartialModelicaServices.ExternalReferences.PartialLoadResource`) and one for the implementation (`ModelicaServices.ExternalReferences.loadResource`, which is the external function). The same pattern written by hand (`extends PartialF; extends Impl;`) fails the same way. omc accepts both.

`loadResource` is how IBPSA, Buildings and AixLib build file paths for weather data and tables: 136 of the 138 EF025 failures in GlobeMode's run of seven public libraries name it (Buildings, AixLib, IBPSA; recounted in S080 from GitHub Actions run 41365960542 on 2026-11-02 — the earlier "51" was one library's row). The other 2 are the Medium shape below.

## Steps to reproduce

Compile the attached `20-ef025-function-selection-identity.mo`:

```bash
rumoca compile 20-ef025-function-selection-identity.mo -m Rep20Load --source-root <MSL root>        # MSL only
rumoca compile 20-ef025-function-selection-identity.mo -m Rep20TwoExtends.M                        # no MSL needed
rumoca compile 20-ef025-function-selection-identity.mo -m Rep20Load0 --source-root <MSL root>      # control (no call), compiles
```

## Expected

Compiles. The call is a plain function call to a function declared in MSL 4.1.0 (`Modelica/Utilities/Files.mo`, `function loadResource` with `extends ...PartialLoadResource; extends ModelicaServices.ExternalReferences.loadResource;`). MLS 3.6 section 7.1.1 "Multiple Inheritance" (https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-01) lets the flattened base classes share elements: a declaration of a base class must either not exist yet or "Be exactly identical to any element of the flattened enclosing class with the same name and the same level of protection (public or protected) and same contents. In this case, the first element in order (can be either inherited or local) is kept." The `uri` input and `fileReference` output of the interface function are identical to those of the implementation, so the two `extends` are legal, and the MSL (and omc) rely on it.

## Actual

```text
$ globe check 20-ef025-function-selection-identity.mo --library data/msl --model Rep20Load
...mo:5:31: unsupported[GM-U0002] S4 接続: missing exact function-selection identity for `Modelica.Utilities.Files.loadResource`: exposed function has no unique exact extends implementation
    note: rumoca: EF025
$ globe check 20-ef025-function-selection-identity.mo --model Rep20TwoExtends.M
...mo:37:26: unsupported[GM-U0002] S4 接続: missing exact function-selection identity for `f`: exposed function has no unique exact extends implementation
    note: rumoca: EF025
$ rumoca compile 20-ef025-function-selection-identity.mo -m Rep20Load --source-root data/msl
  [EF025] missing exact function-selection identity for `Modelica.Utilities.Files.loadResource`: exposed function has no unique exact extends implementation
$ rumoca compile 20-ef025-function-selection-identity.mo -m Rep20TwoExtends.M
  [EF025] missing exact function-selection identity for `f`: exposed function has no unique exact extends implementation
```

OpenModelica 1.27.2: `checkModel(Rep20Load)` and `checkModel(Rep20TwoExtends.M)` both "completed successfully" (1 equation, 1 variable).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 classification, `doc/research/libraries-2026-11-01.md`: EF025 138 of 1,781 failures; 136 name `loadResource` (51) or a Medium function; the S067 rerun with MSL 4.0.0 gives the same counts.
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit; the examples stop at S4 (`GM-U0002`, an internal Rumoca error, not a model error).

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-flatten/src/pipeline/function_overrides_and_dims/function_selection.rs`, `exact_function_implementation` (about lines 81-105). When `function_alias_requires_exact_selection(class_def)` holds, `resolve_function_extends_target_def_id` must return exactly one extends target; for a function with two `extends` clauses (interface plus implementation) it does not, and the "no unique exact extends implementation" error is raised. [Reading only; the two helper functions were not stepped through.]
- A function that extends exactly one function (or is a plain `redeclare function extends`) works; the failure needs two `extends` of function classes.
- Medium shape: a plain `replaceable package Medium = Modelica.Media.Air.SimpleAir` with `Medium.setState_pTX(...)` and `Medium.specificEnthalpy(...)` compiles (tried in S070). The failing shape is reduced below (S080).

## Minimal reproduction: two Medium packages (S080, checked 2026-11-02)

The same code with another message. It may be filed as a separate issue.

### Summary

A model with two package aliases, each naming a different package that extends one base package *with a modifier*, and a model of the base package that calls the base's function unqualified (`y = f(x)` in `BaseProperties`), fails:

```text
[EF025] missing exact function-selection identity for `f`: function source package has multiple exact override package selections
```

With one alias, with base packages extended without modifiers, or with the same package twice, it compiles. Two components that each redeclare their own `Medium` also compile.

In real code this is `ThermofluidStream.Media.myMedia.Examples.MixtureGases` (`Medium1` = `CombustionAir`, `Medium2` = `SimpleNaturalGas`, both extending the NASA mixture package with modifiers; the message names `molarMass`) and `Buildings.DHC.Plants.Steam.BaseClasses.Examples.ControlVolumeEvaporation` (`Buildings.Media.Steam` and `Buildings.Media.Specialized.Water.TemperatureDependentDensity`, both extending MSL `PartialMedium`/`PartialPureSubstance` with modifiers; the message names `Modelica.Fluid.Sources.BaseClasses.PartialSource.Medium.setState_pTX`). These are the 2 EF025 failures that are not `loadResource`.

### Steps to reproduce

Compile the attached `20b-ef025-two-medium-aliases-modified-extends.mo` (self-contained):

```bash
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M    # fails
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M0   # control: one Medium
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M1   # control: extends without modifiers
rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M2   # control: the same package twice
```

### Expected

All four compile. Inside `M1.BaseProperties` the name `f` is looked up in the enclosing package, which is `A` (MLS 3.6 section 5.3, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-02); `b1.y = 2*time` and `b2.y = 3*time`. The other alias in the same model does not take part in that lookup.

### Actual

```text
$ globe check 20b-ef025-two-medium-aliases-modified-extends.mo --model Rep28.M
20b-...mo:17:16: unsupported[GM-U0002] S4 接続: missing exact function-selection identity for `f`: function source package has multiple exact override package selections
    note: rumoca: EF025
Rep28.M: NG（S4 接続 で停止）

$ rumoca compile 20b-ef025-two-medium-aliases-modified-extends.mo -m Rep28.M
  [EF025] missing exact function-selection identity for `f`: function source
  │ package has multiple exact override package selections
```

`Rep28.M0`, `Rep28.M1` and `Rep28.M2` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all four "completed successfully" (`Rep28.M`: 4 equations, 4 variables).

### Notes

- Where, in rev `276e82b` (names only): `crates/rumoca-phase-flatten/src/pipeline/function_overrides_and_dims/function_selection.rs`, `exact_override_package_for_source_package` (about lines 223-292). Both aliases' packages contain the source package `PM`, the unqualified call has no alias prefix to pick one by name, and two different packages remain as candidates, so the error is raised. The candidate list seems to be the model's override packages, not the package that encloses the calling model. [Reading only; not stepped through.]
- Both library examples were re-run with the Rumoca v0.10.2 CLI in S080 (ThermofluidStream at `a2faa411`, Buildings 14.0.0 at `ad48324b`, MSL 4.1.0) and give the same EF025 message.
