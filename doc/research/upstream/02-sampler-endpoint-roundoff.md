# DRAFT (not filed) - "accepted-interval left endpoint sampler" contract failure with tight tolerance or BDF

Target: Rumoca (https://github.com/CogniPilot/rumoca)

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

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.

## Workaround in GlobeMode

Yes. (1) The tolerance is raised to 1e-10 when it is lower, with warning `GM-W10003` (S012). (2) `--solver auto` retries with dopri45 when the sampler disagrees (S012, info `GM-I11002`). These hide the symptom; the underlying bound is unchanged.

## Notes

- Source: `doc/research/msl-sim-failures.md` ("Upstream candidates", item 2). Relevant code at the rev: `crates/rumoca-solver/src/fmi_me/session.rs:486,1414`, `crates/rumoca-solver/src/fmi_me/root.rs:156-160,177-207,340-358` (as read in GlobeMode S011).
- GlobeMode observed four models with this symptom in S011 and zero after the workaround.
