# DRAFT (not filed) - Rumoca fails with EF025 on calls of a function built from two `extends` clauses (e.g. MSL `Files.loadResource`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **Rumoca rejects valid Modelica, for the `loadResource` shape (reproduced with MSL only and with a self-contained model).** The rest of the 138 EF025 failures (calls through a replaceable Medium) were *not* reduced to a minimal model; see Notes.

## Summary

Calling `Modelica.Utilities.Files.loadResource("modelica://...")` in a parameter binding stops with `[EF025] missing exact function-selection identity for 'Modelica.Utilities.Files.loadResource': exposed function has no unique exact extends implementation`. In MSL, `loadResource` has no body of its own: it has two `extends` clauses, one for the interface (`...PartialModelicaServices.ExternalReferences.PartialLoadResource`) and one for the implementation (`ModelicaServices.ExternalReferences.loadResource`, which is the external function). The same pattern written by hand (`extends PartialF; extends Impl;`) fails the same way. omc accepts both.

`loadResource` is how IBPSA, Buildings and AixLib build file paths for weather data and tables: 51 of the 138 EF025 failures in GlobeMode's run of seven public libraries name it (Buildings, AixLib, IBPSA). Most of the remaining 85 call functions of a replaceable `Medium` (for example `Medium.setState_pTX`); those are not covered by the attached reproduction.

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
- Medium shape: a plain `replaceable package Medium = Modelica.Media.Air.SimpleAir` with `Medium.setState_pTX(...)` and `Medium.specificEnthalpy(...)` compiles (tried in S070). The failing Medium calls in IBPSA/Buildings/AixLib were not reduced; the S063 report only has the count. Before filing, run one failing library example (for instance an IBPSA source with `Medium.setState_pTX`) with the S1 problem of draft 17 out of the way and keep the exact function name from the message.
