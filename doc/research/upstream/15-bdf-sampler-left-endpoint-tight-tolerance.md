# DRAFT (not filed) - BDF continuous extension fails the host's left-endpoint check at rtol = 1e-7 (and sometimes 1e-6)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Status (S162, 2026-11-05): **mostly fixed in Rumoca v0.10.4.** The nine examples that stopped with the left-endpoint contract error now complete with `--solver bdf` at rtol = atol = 1e-7 and 1e-6; `Dimmer_RL` (the roundoff message) still stops. A new report would cover only `Dimmer_RL`.

## Summary

With the default `bdf` integrator (`rumoca-solver-diffsol`), 9 MSL 4.1.0 examples stop with `MeSessionError::Contract` (`accepted-interval left endpoint sampler state <i> is <a>, but the checked point carries <b>; the disagreement exceeds <bound>`) at a relative tolerance of 1e-7. The disagreement is 1.1x to 445x the bound. A tenth example (`Modelica.Electrical.PowerConverters.Examples.ACAC.Dimmer_RL`) stops with `accepted step from t=5.0416408911668364 to t=5.041640891166937 made no progress beyond roundoff`. Draft 02 covers two other models with the same first message (`SimpleTriacCircuit`, `ChopperBuckBoost_DutyCycle`); this draft adds the pattern across tolerances.

The host check (`fmi_me/root.rs`, `accept_step` and `state_consistency_bound`) requires `|sampler(t_left) - checked_left| <= max(abs_tol, rel_tol * max(nominal, |x0|, |x1|))`, with `rel_tol` equal to the integrator's own relative tolerance. The BDF dense output one step back is only accurate to about the local error of the step, which diffsol tests with an RMS norm over all states. A component whose local error is a few times the RMS average passes the integrator's test and fails the host's per-component check.

## Steps to reproduce

GlobeMode's `globe simulate` calls the Rumoca library; the tolerance is the model's `Tolerance` annotation times 0.1 (GlobeMode's reference-matching choice, ADR-0015) or `--tolerance` directly:

```bash
globe simulate -L data/msl -m Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_RLV_Characteristic --solver bdf --tolerance 1e-7
```

Not confirmed with the Rumoca CLI [not verified]. Observed through the library API.

## Expected

Either the BDF sampler agrees with its own accepted endpoints within the bound the host demands, or the host's bound is not tighter than what the integrator's error test guarantees (for example the bound uses the integrator's RMS-weighted tolerance).

## Actual

Seven of the examples, `bdf` at three tolerances (GlobeMode, 2026-11-01; "pass" means the run reaches the stop time):

| Model | 1e-6 | 3e-7 | 1e-7 |
| --- | --- | --- | --- |
| `ThyristorBridge2Pulse_RLV_Characteristic` | pass | pass | contract error (1.1x) |
| `ThyristorCenterTap2Pulse_RLV_Characteristic` | pass | pass | contract error (1.3x) |
| `ThyristorCenterTap2mPulse_RLV_Characteristic` | contract error | contract error | contract error (10.3x) |
| `ThyristorCenterTapmPulse_RLV_Characteristic` | pass | pass | contract error (445x) |
| `ChopperBuckBoost_DutyCycle` | contract error | pass | contract error (7.6x) |
| `ThyristorBridge2mPulse_DC_Drive` | contract error | event iteration failure | contract error (5.3x) |
| `ThyristorBridge2mPulse_RLV_Characteristic` | pass | contract error | contract error (1.5x) |

The outcome is not monotone in the tolerance. A trace of `ThyristorBridge2Pulse_RLV_Characteristic` shows the failing step is a regular long step (h = 2.1e-4) 8 ms after the last event reset, with the offending state at 10.8x the bound at that step.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Checked with the CLI (`rumoca sim "Modelica 4.1.0/package.mo" -m <model> --source-root <MSL root> --solver bdf --rtol <tol> --atol <tol>`, the settings GlobeMode's `msl-sim` uses). Outcome per model; "contract" is `[EX001] accepted-interval left endpoint sampler state ...`:

| Model | v0.10.2, 1e-7 | v0.10.2, 1e-6 | v0.10.4, 1e-7 | v0.10.4, 1e-6 |
| --- | --- | --- | --- | --- |
| `ThyristorBridge2Pulse_RLV_Characteristic` | contract (1.1e-7) | completes | completes | completes |
| `ThyristorCenterTap2Pulse_RLV_Characteristic` | completes | completes | completes | completes |
| `ThyristorCenterTap2mPulse_RLV_Characteristic` | contract (1.0e-6) | contract (5.0e-6) | completes | completes |
| `ThyristorCenterTapmPulse_RLV_Characteristic` | contract (4.7e-5) | completes | completes | completes |
| `ChopperBuckBoost_DutyCycle` | contract (9.1e-6) | contract (1.8e-4) | completes | completes |
| `ThyristorBridge2mPulse_DC_Drive` | `EX001` event condition equations did not converge (t = 0.10166) | not recorded | completes (40,531 points) | completes (50,093 points, sliding-mode warning) |
| `ThyristorBridge2mPulse_RLV_Characteristic` | contract (3.9e-6) | completes | completes | completes |
| `ThyristorBridge2Pulse_DC_Drive` | contract | contract | completes | completes |
| `Dimmer_RL` | `EX001` accepted step ... made no progress beyond roundoff (t = 5.0416) | not finished in 240 s | same roundoff stop (t = 5.0416) | same roundoff stop (t = 6.2014) |

The CLI numbers differ a little from the GlobeMode table under "Actual" (for example `ThyristorCenterTap2Pulse_RLV_Characteristic` completes on v0.10.2 here), because GlobeMode drives the host through the library API, but the pattern is the same. The cause is the one named in the Summary: the BDF dense output one step back. v0.10.4 vendors a diffsol with a patch that keeps the continuous extension of the accepted step (`vendor/diffsol/RUMOCA_PATCH.md`, "The fifth correction": the interpolation table is rescaled to the next, much smaller step, which amplifies rounding error by about (h_old / h_new)^k / k!). With it the host's left-endpoint check passes on all nine.

Open: `Dimmer_RL` stops with `accepted step from t=5.041640891166859 to t=5.041640891166959 made no progress beyond roundoff` (v0.10.4, 1e-7). This is the separate roundoff defect mentioned in the last note (the BDF step floor below the host's roundoff width). The patch does not change it.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode; diffsol 0.13.2 with Rumoca's `vendor/diffsol` patch.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Partial. `--solver auto` retries with dopri45 on the sampler message (S012, info `GM-I11002`); it rescues 7 of the 10 (S050 table, run 41194959752). Two (`ThyristorBridge2Pulse_DC_Drive`, `ThyristorBridge2mPulse_DC_Drive`) then time out because explicit dopri45 is too slow on those stiff circuits. `Dimmer_RL` is not retried because `fallback()` only matches the sampler message; retrying would not help it either (every explicit solver times out on it).

## Notes

- Source: `doc/research/msl-sim-failures.md`, section "S055 で調べたこと".
- Related: draft 02 (same message, other models) and draft 13.
- The roundoff message (`Dimmer_RL`) may be a separate defect: the BDF step floor (`scaled_step_floor`) is below the host's roundoff width.
