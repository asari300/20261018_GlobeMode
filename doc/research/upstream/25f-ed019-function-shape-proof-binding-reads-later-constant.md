# DRAFT (not filed) - A protected `b[:]` whose binding reads a constant declared after it fails (ED019 `axis 1 is variable-size`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **valid Modelica that Rumoca rejects** (ED019, reported as unsupported). omc accepts it. Split out of draft 25 in S096 (one shape per draft); the other ED019 shapes are drafts 25 and 25b to 25h.

## Summary

In a function, `protected constant Real b[:] = {c, 2}` is declared before `constant Real c`. Rumoca rejects it:

```text
[ED019] unsupported Flat semantic owner `function shape proof`: `b`: axis 1 is variable-size and neither a binding nor an assignment sizes it
```

With `c` declared first, or with a literal binding, it compiles.

In real code this is `Media.Steam.temperature_ph` (`b[:]` reads `TMean` and `TSD`, which are declared after it) in IBPSA, AixLib and Buildings.

## Steps to reproduce

Compile the attached `25f-ed019-function-shape-proof-binding-reads-later-constant.mo` (self-contained):

```bash
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M   # fails
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M0  # control: declaration order
rumoca compile 25f-ed019-function-shape-proof-binding-reads-later-constant.mo -m Rep32.M1  # control: literal binding
```

## Expected

All compile. Bindings in a function are evaluated in dependency order, not declaration order, as long as they are acyclic (MLS 3.6 section 12.4.4, https://specification.modelica.org/maint/3.6/functions.html, checked 2026-11-02).

## Actual

```text
[ED019] unsupported Flat semantic owner `function shape proof`: `b`: axis 1 is variable-size and neither a binding nor an assignment sizes it
```

`Rep32.M` fails in both `globe check` (`unsupported[GM-U0001]`, note `rumoca: ED019`) and the Rumoca v0.10.2 CLI with this message; `M0` and `M1` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`) accepts every model in the file (checked 2026-11-02).

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode's libraries run of 2026-11-02 (GitHub Actions run 41365960542, MSL 4.1.0), `*-funnel.json`.
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Recorded as a Rumoca limit (`GM-U0001`, "unsupported").

## Notes

- Counts: 3 examples in the libraries run (`SteamTemperatureEnthalpyInversion` in IBPSA, AixLib, Buildings).
- The exact message text is from GlobeMode's notes (S083); the table there writes it as `` `b`: axis 1 is variable-size ... ``. Copy the full text from the CLI when re-checking on Rumoca main.
