# DRAFT (not filed) - The scalar equation count of `tau0 = mu0 * fn` is the number of elements of the table that built an external object (ED001 unbalanced model)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S331): **bug** (the equation count). The equation is a scalar equation in the Modelica Language Specification, and it is counted as many equations as the table has elements.

Status (S331, 2026-11-15): **fixed in Rumoca v0.10.4.** The counts below are right with the v0.10.4 release binary (`rumoca 0.10.4`, the latest release on 2026-11-15) and wrong with v0.10.2 (the rev GlobeMode pins). Do not file; this draft is kept as a record, like 25g and 26. Rumoca main was not built [not verified on main]. The change that fixed it was not identified [not verified].

## Summary

A model builds an external object from a matrix parameter (`table = mu_pos`) and reads a scalar from it through a function whose result is a `Real` (`getTable1DValueNoDer2(tableID, 1, 0)`). The size of the matrix is carried into the size of the scalar parameter `mu0`, and the equation `tau0 = mu0 * cgeo * fn` is counted as `size(mu_pos)` scalar equations instead of one. With `mu_pos = [0, 0.5; 1, 1; 2, 3]` (3 x 2) the equation counts as 6; with a 1 x 2 table it counts as 2.

The model then fails the balance check:

```text
[ED001] unbalanced model: 7 equations, 2 unknowns (balance = 5)
```

Writing the call inline in the equation (`getTable1DValueNoDer2(tableID, 1, 0) * fn`), or a `mu0` that does not come from the table (`mu0 = 2`, `mu0 = f(1, 0)` with an ordinary function), counts 1.

In the Modelica Standard Library the pattern is in `Modelica.Mechanics.Rotational.Components.Clutch`, `Brake`, `OneWayClutch` (`final parameter Real mu0 = getTable1DValueNoDer2(tableID, 1, 0)` and `tau0 = mu0*cgeo*fn`) and in the components that contain them (`LossyGear`, `BearingFriction` use the same table call), and in `Modelica.Mechanics.Translational.Components.Brake` (`f0 = mu0*cgeo*fn`).

## Steps to reproduce

Self-contained (the file declares its own external object): the equation counts in the flat model.

```bash
# run it from a directory that holds only this file (the compiler also reads sibling .mo files)
rumoca compile 36-flat-equation-count-from-external-object-table-size.mo -m Rep36.A --emit flat-json   # 2 x 2 table: scalar_count of the equations
rumoca compile 36-flat-equation-count-from-external-object-table-size.mo -m Rep36.B --emit flat-json   # 3 x 2 table
rumoca compile 36-flat-equation-count-from-external-object-table-size.mo -m Rep36.C --emit flat-json   # control: the call inline in the equation
rumoca compile 36-flat-equation-count-from-external-object-table-size.mo -m Rep36.D --emit flat-json   # control: mu0 is a plain parameter
```

With the Modelica Standard Library (4.1.0), the balance error. Save as `W4.mo`:

```modelica
model W4
  import Modelica.Blocks.Types.{ExternalCombiTable1D, Extrapolation, Smoothness};
  import Modelica.Blocks.Tables.Internal.getTable1DValueNoDer2;
  parameter Real mu_pos[:, 2] = [0, 0.5; 1, 1; 2, 3];
  Real fn;
  Real tau0;
  final parameter ExternalCombiTable1D tableID = ExternalCombiTable1D(
    tableName = "NoName", fileName = "NoName", table = mu_pos, columns = {2},
    smoothness = Smoothness.LinearSegments, extrapolation = Extrapolation.LastTwoPoints,
    verboseRead = false);
  final parameter Real mu0 = getTable1DValueNoDer2(tableID, 1, 0);
equation
  fn = time;
  tau0 = mu0 * fn;
end W4;
```

```bash
rumoca compile W4.mo -m W4 --source-root <path to MSL 4.1.0>
```

`final` and `parameter` on `mu0` are not needed (`Real mu0 = ...` gives the same count).

## Expected

`fn = time` and `tau0 = mu0 * fn` are two scalar equations for two unknowns (`fn`, `tau0`). All of `tau0`, `mu0`, `fn` are scalars, and the result of a function with a `Real` output is a scalar whatever the arguments are. The model is balanced (MLS 3.7, section 4.8 "Balanced Models", https://specification.modelica.org/maint/3.7/class-predefined-types-and-declarations.html, checked 2026-11-15; the section number was read, the counting rule was not re-read in full). The counts: A and B give `[1, 1]`.

## Actual

`scalar_count` of the two equations of the flat model (`--emit flat-json`), from the self-contained file:

| model | table | Rumoca v0.10.2 | Rumoca v0.10.4 |
| --- | --- | --- | --- |
| A | 2 x 2 | `[1, 4]` | `[1, 1]` |
| B | 3 x 2 | `[1, 6]` | `[1, 1]` |
| C | 3 x 2, call inline | `[1, 1]` | `[1, 1]` |
| D | no table | `[1, 1]` | `[1, 1]` |
| W4 (MSL) | 3 x 2 | `[1, 6]`, `ED001` 7 equations, 2 unknowns | `[1, 1]`, compiles |

`rumoca compile` of the self-contained file itself stops at `[ED019] unsupported Flat semantic owner function value type: Table.t` for A and B in both versions (a different limit, on the constructor of the toy external object; the MSL's `ExternalCombiTable1D` does not hit it). Only the flat dump is compared.

## Impact

Ten MSL 4.1.0 examples with the friction and gear tables. `rumoca compile` (no `--emit`):

| example | v0.10.2 | v0.10.4 |
| --- | --- | --- |
| `Modelica.Mechanics.Rotational.Examples.OneWayClutch` | ED001, 44 equations, 43 unknowns | compiles |
| `...OneWayClutchDisengaged` | ED001, 47 / 46 | compiles |
| `...LossyGearDemo1` | ED001, 84 / 76 | compiles |
| `...LossyGearDemo2` | ED001, 108 / 100 | compiles |
| `...LossyGearDemo3` | ED001, 75 / 67 | compiles |
| `...SimpleGearShift` | ED001, 90 / 88 | compiles |
| `...Friction` | ED001, 104 / 102 | compiles |
| `...CoupledClutches` | ED001, 115 / 112 | compiles |
| `...HeatLosses` | ED001, 269 / 248 | compiles |
| `Modelica.Mechanics.Translational.Examples.Brake` | ED001, 75 / 73 | compiles |

The sum of `scalar_count` in the flat model of v0.10.4 equals the equation count of GlobeMode's own flattener in all ten (for example `LossyGearDemo1` 69 and 69, `HeatLosses` 239 and 239; v0.10.2 gave 77 and 260).

## Environment

- Rumoca v0.10.2 (rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`, the prebuilt `rumoca-linux-x86_64`) and v0.10.4 (https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4, published 2026-11-02, the latest release when checked on 2026-11-15; Rumoca main `b55eafce`, 2026-11-13, was not built).
- Linux x86_64 (WSL2, Ubuntu). MSL 4.1.0.

## Workaround in GlobeMode

None needed on GlobeMode's side: the count is wrong only in Rumoca's flat model. GlobeMode's own flattener (`globe-mode-flat`) counts 1 (S330 compared the two with `cargo xtask diff-frontend`: 10 of 350 examples differed in the equation count, all of them this case). The pinned Rumoca (v0.10.2) still counts wrong, so `globe check` through Rumoca reports `ED001` for the nine examples above. Raising the pinned rev to v0.10.4 (planned at the start of the next version) removes them [the effect on `msl:funnel` is not measured].

## Notes

- Where: S330 suspected `infer_simple_equation_scalar_count` in `rumoca-phase-flatten/src/equations/shape_inference.rs`, which takes the shape of an unknown right-hand side from the call chain of the parameter binding. Not traced further [not verified].
- A function call whose output is a scalar `Real` should give the shape `[]` whatever the shapes of its arguments are; the fix may have been exactly that.
