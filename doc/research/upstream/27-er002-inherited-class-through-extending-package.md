# DRAFT (not filed) - A class inherited into a package by `extends` cannot be used through that package's name (ER002)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S083): **bug** (valid Modelica rejected at name resolution). omc accepts it.

## Summary

If package `Q` is `extends P` and `P` declares `model B`, then `Q.B` (or `A.B` with `package A = Q`) as a component type is rejected:

```text
[ER002] unresolved type reference: 'Q.B'
```

`P.B` works, and so does `A.B` when the alias is `replaceable package A = Q`. A function inherited the same way (`Q.f(time)`) also resolves.

This was seen while reducing other reproductions (S080, S083): a model with `package Medium = Modelica.Media.Incompressible.Examples.Glycol47; Medium.BaseProperties medium;` gives ER002 for `Medium.BaseProperties` with MSL 4.1.0, while `replaceable package Medium = ...` resolves. No example of GlobeMode's library runs stops on this shape (S091): in run 41365960542 (2026-11-02) there are 68 ER002 (AixLib 67, Buildings 1), and none is an `unresolved type reference` to an inherited class. The only type references are to `Modelica_DeviceDrivers`, which is not fetched; the rest are `unresolved function call: 'specificEntropyOfpTX'` (39, AixLib, not traced), `unresolved component reference` (18) and `unresolved import` (10).

## Steps to reproduce

Compile the attached `27-er002-inherited-class-through-extending-package.mo` (self-contained):

```bash
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M    # fails: Q.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M1   # fails: package A = Q; A.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M0   # control: P.B
rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M2   # control: replaceable package A = Q
```

## Expected

All four compile. The elements of a class include the elements inherited through `extends` (MLS 3.6 section 7.1, https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-02), and in a composite name `Q.B` the rest of the name is looked up among the elements of `Q` (MLS 3.6 section 5.3.2, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-02).

## Actual

```text
$ globe check 27-er002-inherited-class-through-extending-package.mo --model Rep35.M
... error[GM-E1001] S1 名前解決: unresolved type reference: 'Q.B'
Rep35.M: NG（S1 名前解決 で停止）

$ rumoca compile 27-er002-inherited-class-through-extending-package.mo -m Rep35.M
  [ER002] unresolved type reference: 'Q.B'
```

`Rep35.M0` and `Rep35.M2` compile in both. OpenModelica 1.27.2 (`loadFile`, `checkModel`): all four "completed successfully".

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Not yet re-checked against Rumoca main. Re-check before filing.

## Workaround in GlobeMode

None. Write `replaceable package Medium = ...` (or use the declaring package) in models that use an inherited nested class.

## Notes

- Also fails with the package qualified from the root (`Rep35.Q.B`), with `P` and `Q` outside the enclosing package, with a constant added to `Q`, and with `extends P(k = 2)`.
- MSL's own `Modelica.Media.Examples.SimpleLiquidWater` uses a non-replaceable alias with a modifier (`package Medium = Water.ConstantPropertyLiquidWater(SpecificEnthalpy(max = 1e6))`) and passes; why the modifier changes the lookup was not traced. [not verified]
