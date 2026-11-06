# DRAFT (not filed) - Rumoca cannot compile a function whose `if` branch assigns and then loops (ED019 `function conditional`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S072): **unsupported valid Modelica** (Rumoca reports it as unsupported, not as an error in the model). omc accepts it.

Status (S115, 2026-11-03): **fixed in Rumoca v0.10.4.** `Rep25.M` compiles with the v0.10.4 release binary. Do not file; this draft is kept as a record.

## Summary

A function with a `Boolean` output whose `if`/`else` has, in one branch, an assignment followed by a `for` loop is rejected when the DAE is built:

```text
[ED019] unsupported Flat semantic owner `function conditional`: `isIncreasing` requires assignments or nested conditionals in every checked branch
```

The same shape with a `Real` output compiles, and so does the `Boolean` function with the loop moved out of the `if`.

In real code this is `IBPSA.Utilities.Math.Functions.isMonotonic` (called from `splineDerivatives`, which also fails with the same message), with copies in Buildings and AixLib. It is the largest group of ED019 in GlobeMode's run of seven public libraries: 44 of 125 ED019 failures have this message (`splineDerivatives`, `isMonotonic` and others), 12 have `function shape proof`, 12 `compact dependent-domain transition`, the rest smaller groups.

## Steps to reproduce

Compile the attached `25-ed019-function-conditional-boolean-loop.mo` (self-contained):

```bash
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M    # fails
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M0   # control: Real output
rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M1   # control: loop outside the if
```

## Expected

All three compile. Assignments, `if` statements and `for` statements are ordinary statements of a function's algorithm section (MLS 3.6 sections 11.2 and 12.2, https://specification.modelica.org/maint/3.6/statements-and-algorithm-sections.html and https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02). The specification does not restrict which statements a branch may contain.

## Actual

```text
$ globe check 25-ed019-function-conditional-boolean-loop.mo --model Rep25.M
25-ed019-function-conditional-boolean-loop.mo:17:16: unsupported[GM-U0001] S5 方程式数と変数数: unsupported Flat semantic owner `function conditional`: `isIncreasing` requires assignments or nested conditionals in every checked branch
    note: rumoca: ED019
Rep25.M: NG（S5 方程式数と変数数 で停止）

$ rumoca compile 25-ed019-function-conditional-boolean-loop.mo -m Rep25.M
  [ED019] unsupported Flat semantic owner `function conditional`: ...
```

`Rep25.M0` and `Rep25.M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all three "completed successfully".

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Fixed: `Rep25.M` compiles (`Compilation successful!`, exit 0); `Rep25.M0` and `Rep25.M1` still compile. With v0.10.2 `Rep25.M` still gives ED019 `function conditional`. The library examples (`isMonotonic`, `splineDerivatives`) were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 run (GitHub Actions run 41250636800), `doc/research/libraries-2026-11-01.md`: ED019 125 of 1,781 failures. Library: IBPSA 4.0.0 (master `2adc9194`), `IBPSA/Utilities/Math/Functions/isMonotonic.mo`; reproduced with a model that calls `isMonotonic` and with `IBPSA.Airflow.Multizone.BaseClasses.Examples.WindPressureProfile`.
- Not yet re-checked against Rumoca main. Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-dae/src/construction/analysis/function_conditionals.rs`, `validate_conditional_branch_shape` (from about line 496); the error is raised at about line 549 for any branch statement that is neither an assignment nor a nested `if` matched to a plan. Why the `Real` output takes another path was not traced. [not verified]
- `splineDerivatives` (array `Real` output) fails with the same message; reduced in S083 to draft `25g` (a branch that holds only a `for` loop). File 25g with this issue or right after it.
- The other ED019 messages (`function shape proof`, `axis 1 is variable-size`) are different shapes; GlobeMode split them into drafts `25b` to `25h` (S096), one shape per draft.
