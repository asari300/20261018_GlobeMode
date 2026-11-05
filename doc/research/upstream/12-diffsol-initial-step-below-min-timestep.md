# DRAFT (not filed) - The initial step-size estimate can fall below min_timestep, and the first step() fails without trying a step

Target: diffsol (https://github.com/martinjrobins/diffsol)

## Summary

`RkState::set_step_size` (`ode_solver/state.rs:1039-1100`, Hairer's estimate) computes `h0 = 0.01 d0 / d1` (norms of y and f, weighted by atol/rtol) and `h = min(100 h0, h1)` without comparing against `min_timestep`. When the solver is (re)started from a state that is almost zero with a large derivative, the estimate is far below the minimum step and the first `step()` returns "Step size is too small" without evaluating a single right-hand side or Jacobian. Restarting after an event is where it happens in Modelica models.

## Steps to reproduce

Attached `12-diffsol-initial-step-below-min-timestep.mo` (`TinyStateRestart`): `x(start = 3e-11)`, `der(x) = if time < 1 then 0 else 1e5`. The integrator restarts at the time event t = 1 with `x` about 3e-11 and `der(x)` = 1e5. Use an RK solver with the Hairer estimate (ESDIRK34, TR-BDF2 or tsit45 in GlobeMode). A standalone diffsol program is [not yet written].

MSL form: `Modelica.Electrical.Analog.Examples.CauerLowPassSC` with esdirk34 stops at t = 1.05 (|y| about 1e-22, derivatives -3.8e4, 4.3e3, 2.3e3 give d0 = 2.5e-5, d1 = 1.25e11, h0 = 2e-18, h = 1.994e-16; `min_timestep` at t about 1 is 1e-13).

## Expected

The estimate is clamped at `min_timestep` (or the first step is tried and fails normally).

## Actual

`Step size is too small at time = 1.05` without any attempted step. In the same model the restart at t = 1 has an exactly zero state; the estimate takes the `h0 = 1e-6` branch and works, which hid the problem.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Code reading only:

- Vendored diffsol 0.13.2 in Rumoca v0.10.4: `ode_solver/state.rs` (`set_step_size`, lines 1039-1100) is byte-identical to v0.10.2's and to Rumoca main's: no comparison with `min_timestep`.
- diffsol main (`883d9a09`, `crates/diffsol/src/ode_solver/state.rs`, `set_step_size`): the same Hairer estimate (`h = min(100 h0, h1)`, `h0 = 1e-6` when `d0` or `d1` is below 1e-5) with no clamp at `min_timestep`; the `min_timestep` default became the scalar's epsilon (PR #402, 2026-10-23), which changes the threshold but not the missing clamp. Whether the `TinyStateRestart` reduction still stops there on main was not run. [not verified]

## Environment

- diffsol 0.13.2 as vendored in Rumoca v0.10.2 (`vendor/diffsol/`, rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`), driven through a custom SDIRK/RK plugin in GlobeMode (`globe-mode-exec`, `esdirk34` / `trbdf2`).
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- **Not verified against upstream diffsol main** (https://github.com/martinjrobins/diffsol). The vendored copy may differ from upstream. Reduce to a standalone diffsol program before filing.
- diffsol 0.13.2 as vendored in Rumoca v0.10.4 (`vendor/diffsol/`, unchanged version; the tag commit above). Re-checked on 2026-11-05 (S162) by reading the vendored source and diffsol's own main (`crates/diffsol`, commit `883d9a09238c2e0b465f0f1549c462b8c38ea1b2`, 2026-11-02; diffsol-v0.18.0, 2026-10-20, is the latest diffsol release). Nothing was run against diffsol main. [not verified by running]

## Workaround in GlobeMode

Yes (S023). At the end of `initial_state()` the estimate is raised to 10 times `min_timestep` if it is smaller (sign kept). The step grows with each acceptance (2e-13 to 2e-7 in a dozen steps). With it `CauerLowPassSC` passes with esdirk34 and trbdf2 (differences from BDF at most 5.4e-9 and 1.2e-8 relative).

## Notes

- Source: `doc/research/msl-sim-failures.md` (candidate 15, section "S023").
