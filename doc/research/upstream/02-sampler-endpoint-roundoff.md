# DRAFT (not filed) - "accepted-interval left endpoint sampler" contract failure with tight tolerance or BDF

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Status (S162, 2026-11-05): **partly fixed in Rumoca v0.10.4.** `ChopperBuckBoost_DutyCycle` now completes (the BDF correction below); the missing rounding floor for `SimpleTriacCircuit` with `Tolerance = 1e-12` still reproduces. File only the second model.

## Summary

Two MSL examples stop with a contract error from the model-exchange host: the sampler value at the left endpoint of an accepted interval differs from the integrator's value by more than a bound that has no rounding floor.

- `Modelica.Electrical.Analog.Examples.SimpleTriacCircuit`: the model sets `Tolerance = 1e-12`; the difference is 1.0e-12 and the bound is 1e-12, so it fails by a hair. The bound should probably not go below a multiple of machine epsilon times the magnitude.
- `Modelica.Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle`: with BDF the difference is 1.8e-4 against a bound of 1.2e-5; the same model passes with an RK solver.

## Steps to reproduce

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.Analog.Examples.SimpleTriacCircuit --source-root <MSL root>
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle --source-root <MSL root>
```

Not confirmed with the CLI. GlobeMode observed both through the Rumoca library API (S011), with the BDF plugin of `rumoca_solver_diffsol`. Whether the CLI reproduces it is [not verified].

## Expected

A tolerance of 1e-12 (an unusual but legal annotation) either works or is clamped to what double precision can resolve; the BDF run does not violate the sampler contract.

## Actual

The run ends with `MeSessionError::Contract` (`accepted-interval left endpoint sampler ...`).

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Now confirmed with the CLI (the draft said "not confirmed with the CLI"). All runs `rumoca sim "Modelica 4.1.0/package.mo" -m <model> --source-root <MSL root>` with the bundled MSL 4.1.0:

| Model | Options | v0.10.2 | v0.10.4 |
| --- | --- | --- | --- |
| `ChopperBuckBoost_DutyCycle` | default (and `--solver bdf`) | `[EX001] accepted-interval left endpoint sampler state 0 is 11.732514634476857, but the checked point carries 11.732335121155664; the disagreement exceeds 0.000012` (difference 1.8e-4 against 1.2e-5, as drafted) | completes (16017 time points) |
| `SimpleTriacCircuit` | default (and `--solver bdf`) | completes | completes |
| `SimpleTriacCircuit` | `--solver bdf --rtol 1e-12 --atol 1e-12` | `[EX001] ... state 2 is 0.9216330044421757, but the checked point carries 0.9216330044411705; the disagreement exceeds 0.000000000001` | `[EX001] ... state 1 is 0.3699294501858618, but the checked point carries 0.36992945018473467; the disagreement exceeds 0.000000000001` |

So the first part of the Summary still holds: with a tolerance of 1e-12 the difference (1.0e-12 and 1.1e-12) is just above a bound that has no rounding floor. The CLI's defaults do not use the model's `Tolerance = 1e-12` as such (the default run completes), so the tolerance has to be passed explicitly. The second part is fixed: v0.10.4 vendors a diffsol with a BDF patch that keeps the continuous extension of the accepted step (`vendor/diffsol/RUMOCA_PATCH.md`, "The fifth correction"), which removes the BDF disagreement described in draft 15.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Yes. (1) The tolerance is raised to 1e-10 when it is lower, with warning `GM-W10003` (S012). (2) `--solver auto` retries with dopri45 when the sampler disagrees (S012, info `GM-I11002`). These hide the symptom; the underlying bound is unchanged.

## Notes

- Source: `doc/research/msl-sim-failures.md` ("Upstream candidates", item 2). Relevant code at the rev: `crates/rumoca-solver/src/fmi_me/session.rs:486,1414`, `crates/rumoca-solver/src/fmi_me/root.rs:156-160,177-207,340-358` (as read in GlobeMode S011).
- GlobeMode observed four models with this symptom in S011 and zero after the workaround.
