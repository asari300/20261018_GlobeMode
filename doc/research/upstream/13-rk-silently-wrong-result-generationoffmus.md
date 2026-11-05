# DRAFT (not filed) - An RK run silently returns a constant (wrong) temperature where BDF stops with an error

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

`Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs` fails with BDF because the directional derivative cannot be built during initialization. With an RK solver (`--solver rk-like`) the run finishes but `directCapacity.heatCapacitor.T` stays at 293.15 K for the whole run, with no error and no warning. The result is silently wrong. Either the RK path should fail for the same reason as BDF, or the derivative should be built.

## Steps to reproduce

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs --source-root <MSL root> --solver rk-like
```

(the flag spelling is from GlobeMode's notes [not verified]; no `.mo` file). Plot `directCapacity.heatCapacitor.T`.

## Expected

The temperature changes with the heat flow of the model (the reference result does), or the run fails.

## Actual

`T = 293.15` for the whole run (GlobeMode S012; the run is outside the tolerance tube of the reference, 2 of 3 signals match).

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still reproduces on both binaries, and the flag spelling that the draft marked [not verified] is checked: `rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Thermal.HeatTransfer.Examples.GenerationOfFMUs --source-root <MSL root> --solver rk-like -o out.csv` finishes (501 time points) and `directCapacity.heatCapacitor.T` is 293.15 K in every row, while `directCapacity.T`, which is the same capacitor, goes from 293.15 K to 365.49 K in the same file (v0.10.4; v0.10.2 gives the same last row). With `--solver bdf` both fail with `[EX001] directional derivative is unavailable: algebraic projection sensitivity matrix is singular`.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Yes. `auto` does not fall back to RK when the directional derivative cannot be built (`GM-U0004`, S013), so GlobeMode stops with an error instead of returning the wrong values.

## Notes

- Source: `doc/research/msl-sim-failures.md` (candidate 6, sections "S012" and "S013"). Why the RK path does not need the derivative that BDF fails to build is not understood [not verified].
