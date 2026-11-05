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

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Still present in the source (the wasm32 target was not built here): in Rumoca v0.10.4 and in main (`590ac739`), `crates/rumoca-sim/src/diffsol.rs` (lines 202 and 208) and `crates/rumoca-sim/src/rk45.rs` (lines 134 and 139) still `use std::time::Instant;` and call `Instant::now()` on the typed route. `crates/rumoca-solver/src/runtime/timeout.rs` already switches to the `instant` crate for `wasm32` (`cfg(all(target_arch = "wasm32", target_os = "unknown"))` in `Cargo.toml`), so only `rumoca-sim` is left. [not verified by building for wasm32]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) by reading the source of the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05) against v0.10.2 and Rumoca main (`590ac739`, 2026-11-05). Nothing was built or run for this draft. [not verified by running]
- Target: wasm32-unknown-unknown with wasm-bindgen, Node v24.22.0.

## Workaround in GlobeMode

Yes (S012, ADR-0014). GlobeMode uses the FMI-ME-style host (`rumoca_solver::fmi_me`: `lower_correlated_for_simulation_with_overrides` ... `run_to_stop`) with a BDF/RK plugin, which does not call `Instant::now()`. As a consequence the JIT choice `admitted_native_execution_backend` (`pub(crate)` at the rev) cannot be used and the route runs on the interpreter even natively.

## Notes

- Source: `doc/adr/0014-session-route-wasm.md`, `doc/research/msl-sim-failures.md`. A related request: making `admitted_native_execution_backend` public (or accepting a backend choice in the host route). Also for diffsol, `ClosureNoJac` is not public, so GlobeMode builds explicit RK with the Jacobian-carrying `rhs_implicit`. Those two are API-visibility requests and are not drafted separately.
- getrandom 0.3 (diffsol -> faer -> rand) also needs the `wasm_js` feature besides the cfg; this is configuration on the consumer's side, not a bug.
