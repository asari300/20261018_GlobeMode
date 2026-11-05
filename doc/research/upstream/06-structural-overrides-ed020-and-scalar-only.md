# DRAFT (not filed) - set_structural_overrides fails with ED020 after a compile, ignores unknown names, and accepts scalars only

Target: Rumoca (https://github.com/CogniPilot/rumoca)

## Summary

Three limits of `Session::set_structural_overrides` (rumoca-compile), found while using it as the way to make parameter overrides reach inlined bindings (draft 05):

1. **ED020 after a compile.** Calling it on a session that has already compiled a model clears only the flatten and DAE caches; the synthetic override source is not added to the resolved tree's `source_map` (`session_impl.rs:1322-1328`), so the next compile fails with `DAE provenance references an unknown source` (ED020). The upstream unit tests place the overrides before the first compile, so they pass.
2. **Unknown names are silently ignored.** A mistyped parameter name produces no error or warning.
3. **Scalars only.** `StructuralOverride` is `Bool`, `Int` or `Real`; `root_modifications` is a private field of the session and `from_dotted` does not resolve subscripts to the target path (`session/structural_overrides.rs:36-77`). Overriding a whole array (`a = 5`) turns the array binding into a scalar literal and ends in ED020; overriding an element of an array that is passed down by a same-name modifier (`src.k[1] = 0.2` in the attached file) does not reach the inner binding.

## Steps to reproduce

1. Compile any model in a session, then call `set_structural_overrides` with any override and compile again: ED020 (item 1).
2. With the attached `06-structural-overrides-ed020-and-scalar-only.mo` (model `ArrayParameters`): override `src.k[1] = 0.2` and read `src.s` at t = 0.5 (item 3).

All steps go through the library API (no CLI). We read the code for items 2 and 3 but did not try passing an element name to the function [not verified].

## Expected

Overrides apply after a compile (or the API says that it is not supported); unknown names are reported; array elements and arrays are supported or rejected with a clear error.

## Actual

ED020 for item 1. Item 3: in GlobeMode S025 (2026-10-28) `src.k[1] = 0.2` left `src.i.s` switching at the old time (0.8) and `src.k[2] = 7` left `src.y` at 20, without any diagnostic.

## Re-check on Rumoca v0.10.4 (S162, 2026-11-05)

Not run: `Session::set_structural_overrides` is a Rust API of `rumoca-compile`; the CLI has no such option, and the Python `Model.with_params(recompile=True)` answers that structural re-instantiation "is the compile-API path and not yet available" (v0.10.4). Code reading only:

- `crates/rumoca-compile/src/session/structural_overrides.rs` is byte-identical in v0.10.2, v0.10.4 and main (`590ac739`); so are `session_snapshot.rs` and `session_impl_queries.rs`. `set_structural_overrides` still bumps the revision and clears only `query_state.flat` and `query_state.dae`; `from_dotted` and the three scalar variants are unchanged (items 2 and 3 stand).
- `session_impl.rs` changed (88 lines): v0.10.4 keeps the standard resolution together with its diagnostics (`resolved.builds.standard`) and reuses it for strict compiles. `set_structural_overrides` does not clear that cache, so the ED020 case of item 1 is plausibly unchanged. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode.
- Modelica Standard Library 4.1.0.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-05 (S162) by reading the source of the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4; still the latest release on 2026-11-05) against v0.10.2 and Rumoca main (`590ac739`, 2026-11-05). Nothing was built or run for this draft. [not verified by running]

## Workaround in GlobeMode

Yes. Items 1: clone the library-only session and set the overrides before the first compile of that clone. Items 2-3: GlobeMode rejects such overrides itself before conversion (`GM-E10006`), decided by the *shape of the name* only (ADR-0022). A modifier that renames (`final kk = k`) is missed.

## Notes

- Source: `doc/research/msl-sim-failures.md` ("Upstream candidates" 17 and 21, section "S025"). Reproduction files: `GlobeModeValidation.Simulation.ArrayParameters` and `InlinedParameters` in GlobeMode.
- These three may be one issue or three; we leave that to the maintainers.
