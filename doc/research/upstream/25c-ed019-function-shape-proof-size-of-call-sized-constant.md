# DRAFT (not filed) - An `Integer` local bound to `size()` of a package constant whose size comes from a call fails (ED019 `function shape proof`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

A function declares `protected Integer n = size(c, 1) - 1`, where the package constant `c[:]` is bound to a call whose output size depends on an input (`output Real p[n + 1]`). Rumoca rejects it:

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

With a literal binding of `c`, or with `size(c, 1)` used without the `Integer` local, it compiles.

In real code this is MSL 4.1.0 `Modelica.Media.Incompressible.TableBased.specificEntropy` (`npol = size(poly_Cp, 1) - 1`, with `poly_Cp = Polynomials.fitting(...)`), and the ThermofluidStream copy. For MSL `Incompressible.Examples.TestGlycol` the CLI points at `Math/Polynomials.mo` line 137 (`output Real p[n + 1]` of `fitting`); a model that only calls `Glycol47.specificEntropy` gives the same message.

## Steps to reproduce

Compile the attached `25c-ed019-function-shape-proof-size-of-call-sized-constant.mo` (self-contained):

```bash
rumoca compile 25c-ed019-function-shape-proof-size-of-call-sized-constant.mo -m Rep29.M   # fails
rumoca compile 25c-ed019-function-shape-proof-size-of-call-sized-constant.mo -m Rep29.M0  # control: literal binding
rumoca compile 25c-ed019-function-shape-proof-size-of-call-sized-constant.mo -m Rep29.M1  # control: no Integer local
rumoca compile 25c-ed019-function-shape-proof-size-of-call-sized-constant.mo -m Rep29.M2  # another message: local not named like the constant
```

## Expected

All of `M`, `M0` and `M1` compile. Array dimensions of function components may depend on inputs and on constants (MLS 3.6 section 12.4.5, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function shape proof`: dependent extent is not an exact Integer expression over proven shape axes
```

`Rep29.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` and `M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 2 examples in the libraries run (`TestGlycol`, `InverseIncompressible_sh_T`); MSL's own 2 examples also fail.
- 25c gives the target message only when the `Integer` local has the same name as the constant the call's argument reads through an alias (`nc = n`, local `n`), as in MSL (`npolHeatCapacity = npol`, local `npol`). With another local name (`Rep29.M2`) the message is `dependent extents cannot call runtime function`. S091 checked the names with eight variants (the callee's input `n` or `m`, the package constant `n` or `q` with `nc` bound to it, the local `n`, `m`, `q` or `k`): the target message comes out exactly when the local has the name of the package constant, whatever the callee's input is called. So the binding `nc = n` seems to be resolved in the scope of the function `f`, where `n` is the local `Integer`, instead of the package `P` where it is written (MLS 3.6 section 5.3). This reading is an inference from the messages; the code was not read. [not verified]
