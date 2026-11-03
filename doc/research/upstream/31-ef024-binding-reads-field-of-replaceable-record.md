# DRAFT (not filed) - A binding that reads a field through a `replaceable` record (component or class) fails (EF024)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S108): **bug** (valid Modelica rejected). omc accepts it.

## Summary

A binding that reads a field through something declared `replaceable` gives `[EF024] flat variable is missing structured identity: <name>`. Two forms:

1. **Replaceable record component.** Record `Stage` has `replaceable parameter NV nomVal`. Record `Coil` has `parameter Stage sta` and `parameter Real mSmall = sta.nomVal.m`. A model with `parameter Coil c(sta = Stage(nomVal = NV(m = 2)))` gives EF024 for `m`; when `sta` is also `replaceable` (as in the library) the message names `nomVal`.
2. **Replaceable record class.** In a package, `replaceable record ST Real a = 2.5; end ST;` and a function with a protected binding that reads a field of an `ST`: `protected ST st; Real da = st.a;` (or of an input `input ST s; ... Real t = s.a;`). Calling the function gives EF024 for `a`.

Without `replaceable` both compile, and so does reading the same field in the model (form 1) or in the algorithm.

## Steps to reproduce

Compile the attached `31-ef024-binding-reads-field-of-replaceable-record.mo` (self-contained):

```bash
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M    # EF024: m       (form 1)
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M2   # EF024: nomVal  (form 1, sta also replaceable)
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F    # EF024: a       (form 2, protected record variable)
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F1   # EF024: a       (form 2, input)
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M0   # control: nomVal not replaceable
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.M1   # control: field read in the model
rumoca compile 31-ef024-binding-reads-field-of-replaceable-record.mo -m Rep39.F0   # control: record class not replaceable
```

## Expected

All seven compile. `replaceable` only allows a later redeclaration (MLS 3.6 section 7.3, https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-03); a field access such as `sta.nomVal.m` or `st.a` is an ordinary component reference (section 5.3, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-03), and a protected variable of a function may have a binding (section 12.4.4, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-03).

## Actual

```text
$ globe check 31-ef024-binding-reads-field-of-replaceable-record.mo --model Rep39.M
... unsupported[GM-U0002] S4 接続: flat variable is missing structured identity: m
```

The Rumoca CLI gives EF024 for `M` (`m`), `M2` (`nomVal`), `F` and `F1` (`a`); `M0`, `M1` and `F0` compile (exit 0). `globe check` gives the same codes. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all seven "completed successfully".

## Seen in libraries

In GlobeMode's library run (GitHub Actions run 41365960542, 2026-11-02, Rumoca v0.10.2, MSL 4.1.0), 8 examples stop on these two forms.

Form 1, 7 examples of Buildings 14.0.0 (`flat variable is missing structured identity: nomVal`):

- `Buildings.Fluid.DXSystems.BaseClasses.Examples.DryCoil`
- `Buildings.Fluid.DXSystems.Cooling.BaseClasses.Examples.ApparatusDewPoint`, `ApparatusDryPoint`, `DXCooling`, `SpeedSelect`, `SpeedShift`, `WetCoil`

The record `Buildings.Fluid.DXSystems.Cooling.AirSource.Data.Generic.DXCoil` has `replaceable parameter ...BaseClasses.Stage sta[nSta]` and `parameter MassFlowRate m_flow_small = 0.0001*sta[nSta].nomVal.m_flow_nominal`; `Stage` has `replaceable parameter ...NominalValues nomVal`. In a copy of Buildings with that binding changed to `m_flow_small = 0.0001`, `SpeedSelect` passes all of GlobeMode's checks (S0 to S10), and `DryCoil` and `WetCoil` move on to a different, unrelated error (EF004, if-equation branches with different equation counts).

Form 2, 1 example of AixLib 3.0.1 (`flat variable is missing structured identity: T`): `AixLib.Media.Refrigerants.Examples.BranchingDynamicPipes`. The Rumoca CLI points at `AixLib/Media/Refrigerants/R134a/R134a_IIR_P1_395_T233_455_Horner.mo` line 1247, `SaturationProperties sat = setSat_T(state.T)` in `redeclare function extends thermalConductivity`, where `state` is a `ThermodynamicState` (a `replaceable record` of `Modelica.Media.Interfaces.PartialMedium`). The same medium file has other bindings of this form (`SmoothTransition st; ... Real dp = st.d_pT;`, `SmoothTransition` is a `replaceable record`); calling `density_pT` or `temperature_ph` of the medium gives EF024 for `d_pT` and `T_ph`. The library copy was not edited to confirm the example passes. [not verified]

## Re-check on Rumoca v0.10.4 (S113, 2026-11-03)

Unchanged: `M` (`m`), `M2` (`nomVal`), `F` and `F1` (`a`) give EF024; `M0`, `M1` and `F0` compile (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Re-checked on 2026-11-03 (S113) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4). Rumoca main (`91d6e936648387d04648ec9c52237479eaec46d2`, 10 commits after v0.10.4) was not built; none of those 10 commit titles mentions name lookup, dimensions or bindings. [not verified on main]

## Workaround in GlobeMode

None. In a library, read the field in the algorithm (functions) or avoid `replaceable` on the record that the binding reads through.

## Notes

- Inheritance is not needed for form 2: the function and the record are in one package that nobody extends.
- The array of records (`sta[nSta]`) is not needed for form 1; a scalar `sta` fails the same way.
- Draft 29 (a protected binding that calls a function of a replaceable package) is also a binding of a function, but its repro has no replaceable record; whether the two share a cause was not traced. [not verified]
