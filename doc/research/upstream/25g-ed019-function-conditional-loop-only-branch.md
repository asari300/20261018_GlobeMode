# DRAFT (not filed) - A function `if` branch that holds only a `for` loop fails (ED019 `function conditional`, `splineDerivatives`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

Status (S115, 2026-11-03): **fixed in Rumoca v0.10.4.** `Rep33.M` compiles with the v0.10.4 release binary. Do not file; this draft is kept as a record.

## Summary

A function defines its `Real` array output element by element in a `for` loop, and a later `if b then for ... loop d[i] := 0; end for; end if;` overwrites elements. Rumoca rejects it:

```text
[ED019] unsupported Flat semantic owner `function conditional`: `f` requires assignments or nested conditionals in every checked branch
```

With `d := y` instead of the first loop, or with an assignment in the branch, it compiles.

In real code this is `Utilities.Math.Functions.splineDerivatives` ("Ensure monotonicity") in IBPSA, AixLib and Buildings. It is the same check as draft 25 (`isMonotonic`, `Boolean` output), reached with a `Real` array output; file the two together or one right after the other.

## Steps to reproduce

Compile the attached `25g-ed019-function-conditional-loop-only-branch.mo` (self-contained):

```bash
rumoca compile 25g-ed019-function-conditional-loop-only-branch.mo -m Rep33.M   # fails
rumoca compile 25g-ed019-function-conditional-loop-only-branch.mo -m Rep33.M0  # control: d := y first
rumoca compile 25g-ed019-function-conditional-loop-only-branch.mo -m Rep33.M1  # control: assignment in the branch
```

## Expected

All three compile. Assignments, `if` statements and `for` statements are ordinary statements of a function's algorithm section (MLS 3.6 sections 11.2 and 12.2, https://specification.modelica.org/maint/3.6/statements-and-algorithm-sections.html and https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function conditional`: `f` requires assignments or nested conditionals in every checked branch
```

`Rep33.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` and `M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Fixed: `Rep33.M` compiles (`Compilation successful!`, exit 0); `Rep33.M0` and `Rep33.M1` still compile. With v0.10.2 `Rep33.M` still gives ED019 `function conditional`. Same fix as draft 25. The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 25 examples in the libraries run (IBPSA 8, AixLib 9, Buildings 8).
- The message above is written with the reproduction's function name `f`; in the libraries it names `splineDerivatives`. Copy the exact text from the CLI when re-checking on Rumoca main. [not verified]
- Where: see draft 25 (`validate_conditional_branch_shape` in `function_conditionals.rs`, rev `276e82b`). [not verified]
