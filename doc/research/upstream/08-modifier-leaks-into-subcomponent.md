# DRAFT (not filed) - A modifier on a component (part(theta1(fixed = true))) also applies to the same-named variable of a subcomponent

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

In the pattern of MSL `Mechanics.MultiBody.Parts.RollingWheelSet`, the component `Part` sets `theta1(fixed = true)` itself and sets `joint.theta1(fixed = false)` for its inner `Joint`. When the example instantiates `Part part(theta1(fixed = true))`, Rumoca also gives `part.joint.theta1` and `part.joint.der_theta1` `fixed = true`: the outer modification leaks onto the same-named variables one level down. The initialization then has two redundant initial conditions (both aliases fixed). The MSL examples `RollingWheelSetDriving` and `RollingWheelSetPulling` show it; GlobeMode's initialization matching reports 7 redundant conditions there, which omc does not.

## Steps to reproduce

Compile the attached `08-modifier-leaks-into-subcomponent.mo` (model `ModifierScope`) and list the `fixed` attributes after flattening: `part.joint.theta1` and `part.joint.der_theta1` should be `fixed = false`. Without the outer modifier (`Part part;`) the result is correct (2 of 2 initial conditions, GlobeMode S009).

## Expected

`part.theta1.fixed = true`; `part.joint.theta1.fixed = false`.

## Actual

`part.joint.theta1.fixed = true` and `part.joint.der_theta1.fixed = true` (as seen in GlobeMode's initialization matching; the flattened attributes themselves were not dumped in the drafting session [not verified]).

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still reproduces, and the flattened attributes that the draft could not dump ([not verified] under "Actual") are now read with `rumoca compile 08-modifier-leaks-into-subcomponent.mo -m ModifierScope --emit flat-mo`: both v0.10.2 and v0.10.4 print `part.joint.theta1(start = 0, fixed = true)` and `part.joint.der_theta1(start = 0, fixed = true)`; expected `fixed = false` for both. (v0.10.4 quotes the dotted names, `'part.joint.theta1'`, and prints `StateSelect.prefer` where v0.10.2 printed `StateSelect.Prefer`; unrelated.)

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a known limit (validation model `GlobeModeValidation.Init.ModifierScope` and a db test that fails when Rumoca is fixed).

## Notes

- Source: `doc/roadmap.md` (S009 section).
