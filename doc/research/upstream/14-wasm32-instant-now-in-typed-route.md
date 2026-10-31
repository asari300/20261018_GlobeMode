# DRAFT (not filed) - The typed simulation route calls std::time::Instant::now(), which panics on wasm32-unknown-unknown

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

`rumoca_sim::build_simulation` -> `check_prepared_initialization` -> `run_prepared_simulation` (the route the CLI uses) and `rk45::simulate` call `std::time::Instant::now()`. On `wasm32-unknown-unknown` that panics (`RuntimeError: unreachable`) because the platform has no clock. We would like either the use of `web-time`/`instant`, or a way to supply a clock (or no clock), so that the typed route can be used in the browser.

## Steps to reproduce

1. Build a crate that calls `rumoca_sim::build_simulation` for `wasm32-unknown-unknown` with wasm-bindgen.
2. Run it in Node v24 on any small model.

A small self-contained program is [not yet written]; GlobeMode's prototype is the origin of the observation.

## Expected

The route builds and runs (time limits would need a clock, but they can be optional).

## Actual

`RuntimeError: unreachable ... <std::time::Instant>::now ... build_simulation` (GlobeMode S012, Node v24.22.0).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Target: wasm32-unknown-unknown with wasm-bindgen, Node v24.22.0.

## Workaround in GlobeMode

Yes (S012, ADR-0014). GlobeMode uses the FMI-ME-style host (`rumoca_solver::fmi_me`: `lower_correlated_for_simulation_with_overrides` ... `run_to_stop`) with a BDF/RK plugin, which does not call `Instant::now()`. As a consequence the JIT choice `admitted_native_execution_backend` (`pub(crate)` at the rev) cannot be used and the route runs on the interpreter even natively.

## Notes

- Source: `doc/adr/0014-session-route-wasm.md`, `doc/research/msl-sim-failures.md`. A related request: making `admitted_native_execution_backend` public (or accepting a backend choice in the host route). Also for diffsol, `ClosureNoJac` is not public, so GlobeMode builds explicit RK with the Jacobian-carrying `rhs_implicit`. Those two are API-visibility requests and are not drafted separately.
- getrandom 0.3 (diffsol -> faer -> rand) also needs the `wasm_js` feature besides the cfg; this is configuration on the consumer's side, not a bug.
