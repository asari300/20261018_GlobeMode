# DRAFT (not filed) - A failed algebraic projection at a trial point is remembered as an unrecoverable failure

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

In the model-exchange host's derivative evaluation, when an integrator evaluates the right-hand side at a *trial* point (a rejected Runge-Kutta stage, a Newton iterate, a step-halving retry) and the algebraic projection diverges there, the host records the failure as unrecoverable (`fail` in `crates/rumoca-solver/src/fmi_me/integrator/derivative.rs:176-196`). Only components for which `discards_failed_trials()` is true (those that switch reduced charts, `chart_switch.rs:12`) treat it as "discard this trial point". Everywhere else, a plugin that shortens its step after the failure still ends with an error. The result is that stiff circuits fail with `algebraic projection did not establish coordinate convergence` for explicit RK solvers (`dopri45`, `tsit45`) and for implicit RK (`esdirk34`) in cases where the same trajectory is easy for BDF.

## Steps to reproduce

```bash
rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.Analog.Examples.SimpleTriacCircuit --source-root <MSL root> --solver dopri45 --stop-time 2e-5
```

(flag names are from GlobeMode's notes; the exact CLI flags are [not verified]). For an implicit RK: `Modelica.Electrical.Analog.Examples.ThyristorBehaviourTest` with an ESDIRK34 plugin (GlobeMode `--solver esdirk34`), which fails at t in (5.3e-5, 5.5e-5] (GlobeMode S022/S024).

## Expected

A diverging projection at a trial point lets the integrator reject the trial and shrink the step; the run fails only if the step goes below the minimum.

## Actual

Rumoca's own `dopri45` fails for the first output interval (`--stop-time 2e-5`) and runs out of a 60 s budget at the default stop time. In GlobeMode's plugin the state diverges 0 -> 2.6e4 -> 2.9e5 -> 1.0e6 -> -1.5e8 in the first step (h = 2e-5) and the projection diverges at the halved trial.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still reproduces, and the CLI flags that the draft marked [not verified] are now checked: `rumoca sim "Modelica 4.1.0/package.mo" -m Modelica.Electrical.Analog.Examples.SimpleTriacCircuit --source-root <MSL root> --solver rk-like --t-end 2e-5` fails on v0.10.2 and v0.10.4 with the same text: `[EX001] algebraic projection did not establish coordinate convergence: worst scaled residual row=36 target=simpleTriac.thyristor1.Anode.i value=7.154881e-1 ratio=7.154881e9 ...`. The CLI has no `dopri45`; `rk-like` is its explicit Runge-Kutta solver. In the source, `fail` in `crates/rumoca-solver/src/fmi_me/integrator/derivative.rs` still latches every failure unless `discards_failed_trials` is true (the file changed only by moving the flag into the struct; same behaviour). The ESDIRK34 case (`ThyristorBehaviourTest`) was not re-run: it needs GlobeMode's plugin.

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05), next to the v0.10.2 binary. Rumoca main (`590ac739`, 2026-11-05) was not built. [not verified on main]

## Workaround in GlobeMode

Partial. Users are told to use `bdf` or `esdirk34` for stiff circuits. `ThyristorBehaviourTest` still fails under `esdirk34`. A guard that returns NaN for trial points far from the accepted point helped in one case but was not adopted (the threshold is arbitrary).

## Notes

- Source: `doc/research/msl-sim-failures.md` (candidates 13 and 19; sections "S021" and "S024"). It is an explicit-RK limitation in part (stiffness), which is why GlobeMode does not treat it as a plugin bug; the request here is only the unrecoverable bookkeeping.
- Our hypothesis that the host's projection starts from the previous solution and therefore keeps failing near the accepted point is from a single measurement [not verified].
