# DRAFT (not filed) - A protected `Real[:]` sized only by assigning a call result fails (ED019 `axis 1 is variable-size`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

In a function, `protected Real[:] X;` is sized only by `X := g(x)` (also with the binding `= g(x)`). Rumoca rejects it:

```text
[ED019] unsupported Flat semantic owner `function shape proof`: `X`: axis 1 is variable-size and neither a binding nor an assignment sizes it
```

With `Real[2] X` it compiles.

In real code this is MSL 4.1.0 `Media.Air.ReferenceMoistAir.Xsaturation` (`MassFraction[:] X; X := massFractionSaturation(state)`) and the ThermofluidStream copy.

## Steps to reproduce

Compile the attached `25h-ed019-function-shape-proof-flexible-array-from-call.mo` (self-contained):

```bash
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M   # fails
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M0  # control: X[2]
rumoca compile 25h-ed019-function-shape-proof-flexible-array-from-call.mo -m Rep34.M1  # also fails: binding instead of assignment
```

## Expected

All compile. A protected array with `:` may be sized by an assignment (MLS 3.6 section 12.4.5, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function shape proof`: `X`: axis 1 is variable-size and neither a binding nor an assignment sizes it
```

`Rep34.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged: `Rep34.M` and `Rep34.M1` give the same `[ED019] ... function shape proof: 'X': axis 1 is variable-size and neither a binding nor an assignment sizes it`; `Rep34.M0` compiles (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 2 examples in the libraries run (ThermofluidStream `MoistAir`, `MoistAir2`); MSL `ReferenceAir.MoistAir`, `MoistAir2` also fail.
- The exact message text is from GlobeMode's notes (S083); copy the full text from the CLI when re-checking on Rumoca main.
