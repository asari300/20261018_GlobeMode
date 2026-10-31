# DRAFT (not filed) - SDIRK refreshes the Jacobian at the state y instead of the stage unknown, which silently accepts a wrong trajectory

Target: diffsol (https://github.com/martinjrobins/diffsol)

## Summary

The stage equation of the SDIRK solver is `F(x) = M x - h f(phi + c x)` (`op/sdirk.rs`). When the Jacobian is refreshed, `ode_solver/runge_kutta.rs:637` and `ode_solver/sdirk.rs:261,277` (`jacobian_updates`) call `reset_jacobian(op, &state.y, t)`, i.e. they pass the *state* `y` where the stage unknown `x` is expected (a code comment says "x can be anything"). `jacobian_inplace` then evaluates `f'` at `phi + c y`, which is about 1.4 times y_n. For a right-hand side with an exponential (a Zener diode) the Jacobian at that point is 1e15 instead of 5e5. The Newton update `r / (1 - h gamma J)` and the embedded error estimate (which uses the same matrix) both collapse by about 1e12, so unconverged stages and too-large steps are accepted without any error.

## Steps to reproduce

MSL form: `Modelica.Electrical.Analog.Examples.OvervoltageProtection` with an ESDIRK34 plugin (GlobeMode `--solver esdirk34`, before GlobeMode's workaround), tolerance 1e-7 (GlobeMode default in `msl-sim`).

No `.mo` reduction is attached: the minimal reduction (a stiff exponential) is not written yet. A standalone diffsol program is [not yet written].

## Expected

`CL.v(0.4) = -0.0006` V (BDF and the reference result).

## Actual

`CL.v(0.4) = -1.64` V from the Zener breakdown at t = 0.0184. At the accepted point t = 0.0184, `CL.v = 5.41`, the derivative is -2.8e5, and after the stage evaluation Newton moves to `CL.v = -3839` and then "converges" (GlobeMode S022).

## Environment

- diffsol 0.13.2 as vendored in Rumoca v0.10.2 (`vendor/diffsol/`, rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`), driven through a custom SDIRK/RK plugin in GlobeMode (`globe-mode-exec`, `esdirk34` / `trbdf2`).
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- **Not verified against upstream diffsol main** (https://github.com/martinjrobins/diffsol). The vendored copy may differ from upstream. Reduce to a standalone diffsol program before filing.

## Workaround in GlobeMode

Yes (S022). The Jacobian closure in `globe-mode-exec/src/rumoca/plugins/rk.rs` ignores the point diffsol passes and uses the accepted state at the start of the step (`anchor`). The matrix is one step old inside a step, which SDIRK reuses anyway. After the change esdirk34 gives `CL.v(0.4) = -0.00061562` (BDF: -0.00061563), and the `HeatingRectifier` and `DiodeSwitching` cases improve too.

## Notes

- Source: `doc/research/msl-sim-failures.md` (candidate 14, section "S022"). The tolerance 1e-8 happens to give the correct value (32 s versus 0.15 s after the workaround): tightening only compensates.
- Related: 10 (cumulative Newton failures). Both are in `runge_kutta.rs`; they can be one issue if the maintainers prefer.
