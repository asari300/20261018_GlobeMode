# DRAFT (not filed) - A function input sized by an `Integer` input declared after it fails (ED019 `function shape proof`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

A function declares `input Real x[n]` before `input Integer n`. Rumoca rejects it when the function is called from a block used as a component:

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

With `input Integer n` first, or with `x[:]`, it compiles.

In real code this is `Controls.SetPoints.OccupancySchedule.getOutput` (`occupancy[nRow]` declared before `nRow`) in IBPSA, AixLib and Buildings. In a copy of IBPSA with `input Integer nRow` moved before `input Real occupancy[nRow]`, the ED019 of `IBPSA.Controls.SetPoints.Examples.OccupancySchedule` goes away (it stops later at a different ED019, `multi-output equation`, in the initial equation).

## Steps to reproduce

Compile the attached `25d-ed019-function-shape-proof-dimension-before-input.mo` (self-contained):

```bash
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M   # fails: the block as a component
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M0  # control: n first
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M1  # control: x[:]
rumoca compile 25d-ed019-function-shape-proof-dimension-before-input.mo -m Rep30.M2  # also fails: the block through extends
```

## Expected

All compile. Default values and bindings in a function are evaluated in dependency order, not declaration order, as long as they are acyclic, and dimensions may depend on inputs (MLS 3.6 sections 12.4.1, 12.4.4 and 12.4.5, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

`Rep30.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` and `M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 3 examples in the libraries run (`OccupancySchedule` in IBPSA, AixLib, Buildings).
- 25d also fails when the block is used through `extends` (`Rep30.M2`). With a model-level parameter `n` passed as the argument, the same function compiled during the reduction; not traced. [not verified]
