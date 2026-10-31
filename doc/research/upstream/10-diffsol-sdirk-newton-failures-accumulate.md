# DRAFT (not filed) - SDIRK compares the total number of Newton failures of the whole integration with max_nonlinear_solver_failures

Target: diffsol (https://github.com/martinjrobins/diffsol)

## Summary

In the SDIRK solver, `solve_fail` (`ode_solver/runge_kutta.rs:849-872` in the vendored copy) compares `statistics.number_of_nonlinear_solver_fails`, which accumulates over the whole integration, with `max_nonlinear_solver_failures` (default 50). BDF (`bdf.rs:1269,1356-1359`) compares against the value at the start of the current step, i.e. failures *within one step*. A model with many discontinuities (diodes, thyristors, saturating op-amps) has a few Newton failures at each switch, so a long run stops with `Exceeded maximum number of nonlinear solver failures (50)` even though every step recovers.

## Steps to reproduce

Attached `10-diffsol-sdirk-newton-failures-accumulate.mo` (model `DiodeSwitching`: a square-wave current of period 2 ms into a sharp diode with a leakage resistor, 98 switches, stop time 0.0985) solved with an SDIRK method (ESDIRK34 or TR-BDF2) at tolerance 1e-6. A standalone diffsol program is [not yet written].

MSL form: 7 of the 54 `Modelica.Electrical.Analog.Examples` that BDF solves stop with an ESDIRK34 plugin (e.g. `OpAmps.Differentiator` at step 55, t = 0.0092).

## Expected

The limit applies per step (as in BDF), or the documentation says that it is cumulative.

## Actual

ESDIRK34 stops at t = 0.0027 and TR-BDF2 at t = 5e-6 on the reduction (before GlobeMode's workaround).

## Environment

- diffsol 0.13.2 as vendored in Rumoca v0.10.2 (`vendor/diffsol/`, rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`), driven through a custom SDIRK/RK plugin in GlobeMode (`globe-mode-exec`, `esdirk34` / `trbdf2`).
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- **Not verified against upstream diffsol main** (https://github.com/martinjrobins/diffsol). The vendored copy may differ from upstream. Reduce to a standalone diffsol program before filing.

## Workaround in GlobeMode

Yes (S021). The SDIRK problem is built with `ode_options.max_nonlinear_solver_failures = usize::MAX`. A runaway inside one step is stopped by diffsol's step-size reduction (x0.3 per failure) and `min_timestep`. A regression test checks that esdirk34 and trbdf2 finish and agree with BDF (1.3e-5 and 2e-6 at t = 0.0985).

## Notes

- Source: `doc/research/msl-sim-failures.md` (candidate 12, section "S021").
- The difference between SDIRK and BDF is from reading the vendored code at the rev above; not checked against upstream diffsol main [not verified].
