# DRAFT (not filed) - A modifier `vol(massDynamics = massDynamics)` passes the default of the enclosing parameter evaluated in the scope of `vol`, so the `if` equation on it picks the wrong branch

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S373): **bug**. In `Boiler b(energyDynamics = Dyn.Steady)` the enclosing model passes `massDynamics` down with `Vol vol(massDynamics = massDynamics)`. The default of `b.massDynamics` is `energyDynamics`, so `b.vol.massDynamics` is `Steady` (the right-hand side of a modifier names an element of the enclosing class: the example of MLS 3.7 section 7.2.3, `class C4 extends C3(…, x5 = x3, …)`, passes a sibling element this way; read on 2026-11-16). Rumoca flattens `if massDynamics == Dyn.Steady then 0 = f else der(m) = f` of `vol` to the `else` branch, as if `b.massDynamics` were `Dynamic`, the default of `energyDynamics`. omc 1.27.2 gives `0.0 = b.vol.f`.

Status (S373, 2026-11-16): reproduces on v0.10.2 (the pin of GlobeMode) and v0.10.4 (the latest release). Rumoca main was not built.

## Summary

`Decl` declares `parameter Dyn energyDynamics = Dyn.Dynamic;` and `parameter Dyn massDynamics = energyDynamics;`. Both `Boiler` and `Vol` extend `Decl`. `Boiler` has a component `Vol vol(massDynamics = massDynamics)`, and the model uses `Boiler b(energyDynamics = Dyn.Steady)`. `Vol` has an `if massDynamics == Dyn.Steady then 0 = f else der(m) = f` equation. The flat model (`--emit flat-mo`) of `A` contains `der(b.vol.m) = b.vol.f;` where `0 = b.vol.f;` is expected. The controls pass: `Vol5` that declares the two parameters itself instead of extending `Decl` (`B`), a modifier that passes only `energyDynamics` (`C`), and `massDynamics` set directly on `b` (`D`).

## Steps to reproduce

```bash
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.A --emit flat-mo   # wrong branch
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.B --emit flat-mo   # control: Vol5 declares the parameters itself
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.C --emit flat-mo   # control: only energyDynamics is passed down
rumoca compile 42-modifier-passes-default-binding-in-wrong-scope.mo -m Rep42.D --emit flat-mo   # control: massDynamics set on b
```

Look at the equation of `b.vol` (or `v` in the controls): `0 = ...` or `der(...) = ...`.

## Expected

`A` has `0 = b.vol.f;` like `B`, `C` and `D`.

## Actual

| model | equation of `vol` (v0.10.2) | equation of `vol` (v0.10.4) |
| --- | --- | --- |
| `A` (`vol(massDynamics = massDynamics)`) | `der(b.vol.m) = b.vol.f;` | `der(b.vol.m) = b.vol.f;` |
| `B` (control: `Vol5` declares the parameters) | `0 = b.vol.f;` | `0 = b.vol.f;` |
| `C` (control: only `energyDynamics` passed) | `0 = b.vol.f;` | `0 = b.vol.f;` |
| `D` (control: `massDynamics` set on `b`) | `0 = b.vol.f;` | `0 = b.vol.f;` |

omc 1.27.2 `instantiateModel(Rep42.A)`: `final parameter ... b.vol.massDynamics = Rep42.Dyn.Steady;` and `0.0 = b.vol.f;`.

Only the combination where the enclosing model and the component both inherit the default `massDynamics = energyDynamics` from the same base, and the enclosing model passes the parameter down, goes wrong. It looks as if the default binding `energyDynamics` of `b.massDynamics` is evaluated in the scope of `vol` (`vol.energyDynamics` is `Dynamic`).

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-16, checked in S359). Rumoca main was not built.
- Linux x86_64 (WSL2, Ubuntu). omc 1.27.2.

## Workaround in GlobeMode

None needed. GlobeMode's own flat model (`globe-mode-flat`) evaluates the chain `b.vol.massDynamics = b.massDynamics = b.energyDynamics` and keeps `0 = f`. The two differ in the DAE of the example below (6 states against 7), and `cargo xtask diff-frontend` reports it. GlobeMode does not copy Rumoca's behavior.

## Notes

- Found in Buildings 14.0.0 (commit `ad48324b`), `Buildings.DHC.Plants.Steam.BaseClasses.Examples.BoilerPolynomial` (S373, from the `libraries.yml` run of S369). `boiSte` is a `BoilerPolynomial` with `energyDynamics = SteadyState`; its `massDynamics` defaults to `energyDynamics` (`LumpedVolumeDeclarations` and `PartialTwoPortTwoMedium` both declare it) and the volume `vol` (`ControlVolumeEvaporation`, extending `PartialSaturatedControlVolume`, which extends `PartialTwoPortTwoMedium` as well) gets `final massDynamics = massDynamics`. The `if massDynamics == SteadyState then 0 = mWat_flow + mSte_flow else der(m) = mWat_flow + mSte_flow` of the volume goes to the `else` branch in Rumoca. The equation and unknown counts are the same (161 and 161, one equation per branch), but Rumoca has `der(boiSte.vol.m)` and counts 7 states against 6 (GlobeMode). The Rumoca structure rows of `boiSte.vol` list `der(m)`, `mWat_flow` and `mSte_flow` for that equation, where the steady branch has only the two flows.
- The same pattern is common (`massDynamics = energyDynamics` in `Buildings.Fluid.Interfaces.LumpedVolumeDeclarations`, passed down as `final massDynamics = massDynamics` in many Buildings and IBPSA models). The number of examples that change is not known; only `BoilerPolynomial` showed up in the 7 libraries' comparison (the states are compared only for the examples that Rumoca balances; 116 of 351 for Buildings) [the count of other affected examples is not verified].
- Source: GlobeMode S373, MLS 3.7 https://specification.modelica.org/maint/3.7/inheritance-modification-and-redeclaration.html (section 7.2.3 and its example were read on 2026-11-16; a sentence that states the lookup scope of the right-hand side was not found in that section [not verified]).
