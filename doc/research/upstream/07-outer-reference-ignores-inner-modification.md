# DRAFT (not filed) - An outer reference reads the type default instead of the inner declaration's modification

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

When a component refers to `outer System system` and the matching `inner System system(energyDynamics = Dynamics.FixedInitial)` modifies a parameter, Rumoca evaluates the outer reference with the *type default* (`Free`) rather than the inner declaration's modification. Initial equations guarded by `if massDynamics == Dynamics.FixedInitial` then disappear from the DAE. This is the pattern of the MSL Fluid examples (11 of them use `inner Modelica.Fluid.System system(energyDynamics = FixedInitial)`): the initial equations of `PartialDistributedVolume` (e.g. `mediums.p = ps_start`) are missing.

## Steps to reproduce

Compile the attached `07-outer-reference-ignores-inner-modification.mo` (model `OuterModification`) and count the initial equations in the DAE: two (`volume.p = volume.ps_start`, array of 2) are expected; GlobeMode's structural analysis finds 0 of 2 (GlobeMode S009).

MSL form: any Fluid example that declares `inner Modelica.Fluid.System system(energyDynamics = FixedInitial)`; in GlobeMode the S8 (initialization balance) check of these examples is short of initial equations, and for the `NonCircularPipes` examples omc reports two equations as redundant that are absent from Rumoca's DAE (GlobeMode S009).

## Expected

2 initial equations (the `if` branch is taken).

## Actual

0 initial equations. Changing the type default of `energyDynamics` to `FixedInitial` gives 2 of 2 (checked in GlobeMode S009, so the cause is the default being used).

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still reproduces on both binaries. `rumoca compile 07-outer-reference-ignores-inner-modification.mo -m OuterModification --emit dae-json`: `initialization_equation_operations` has 0 entries (v0.10.2 and v0.10.4). With the type default of `energyDynamics` changed to `Dynamics.FixedInitial` (control) it has 1 entry, the array equation `p = ps_start` of 2 scalars, in both. Both versions also print `warning[WD001]: the branch of this equation is selected at translation, so parameter volume.massDynamics cannot be set: its branches differ in structure (MLS 3.7 §8.3.4)`.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

None. GlobeMode records the case as a known limit (validation model `GlobeModeValidation.Init.OuterModification` and a db test that fails when Rumoca is fixed); the analysis reports an under-determined initialization (`GM-W8001`) for the affected models.

## Notes

- Source: `doc/roadmap.md` (S009 section), `doc/research/rumoca.md`.
- This is the same family as draft 08 (modification scoping), but a different code path.
