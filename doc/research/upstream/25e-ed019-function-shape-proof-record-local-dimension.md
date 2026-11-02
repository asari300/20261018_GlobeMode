# DRAFT (not filed) - A record whose array dimension is a record component fails as a function's protected local (ED019 `function shape proof`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

A record with `parameter Integer nc = 2; parameter Real[nc] a` (also with `constant`) used as a function's protected local is rejected:

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

With a literal dimension, or with the same record as a model component, it compiles.

In real code this is MSL 4.1.0 `Media.R134a.R134a_ph` (`Common.EOSIdealCoeff`, `EOSResidualCoeff`, locals in `fid_R134a`, `fres_R134a`) and the ThermofluidStream copy. For the four AixLib examples and MSL `R134a1`, the error points at MSL `Media/R134a.mo` line 53 (`parameter Real[nc] a`). In a copy of MSL 4.1.0 with the record dimensions written as literals (`Real[5] a`, `Real[21] c, d, t, n`), MSL `R134a1` and `R134a2` get past this error (they stop later at a different ED008, `state_h`).

## Steps to reproduce

Compile the attached `25e-ed019-function-shape-proof-record-local-dimension.mo` (self-contained):

```bash
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M   # fails
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.MC  # fails: constant components
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M0  # control: literal dimension
rumoca compile 25e-ed019-function-shape-proof-record-local-dimension.mo -m Rep31.M1  # control: the record as a model component
```

## Expected

All compile. Array dimensions of function components may depend on constants and parameters of the component (MLS 3.6 section 12.4.5, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

`Rep31.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` and `M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 6 examples in the libraries run (AixLib expansion valves and compressors 4, ThermofluidStream `R134a1`/`R134a2` 2); MSL `R134a1`/`R134a2` also fail.
