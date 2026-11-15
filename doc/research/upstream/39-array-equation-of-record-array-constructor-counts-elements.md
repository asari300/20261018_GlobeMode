# DRAFT (not filed) - An array equation between an array of records and an array constructor of records (`aw = {R(1, 0), ...}`) counts one equation per element instead of one per field (ED008 on v0.10.2, ED001 unbalanced on v0.10.4)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S359): **bug** (the equation count; v0.10.2 stops earlier with ED008). With `R aw[3]` and two fields, `aw = {R(1, 0), R(2, 0), R(3, 0)};` is 6 scalar equations (MLS 3.7 section 10.6.1: equality of records is applied to each component-element; section 4.8: equations are counted after expanding to scalars of primitive types). Rumoca counts 3. The same model written with a `for` loop, or `aw = bw` between two arrays of records that are set another way, is counted right. omc 1.27.2 (`checkModel`) says "6 equation(s) and 6 variable(s)" for all of them.

Status (S359, 2026-11-16): **partly fixed in v0.10.4**. For the Modelica Standard Library's own `Complex` (an operator record) with a literal size (`Complex aw[3]`), v0.10.4 compiles (v0.10.2 gives ED008). For a plain record, and for `Complex` with a size that comes from a parameter array (the form of `Modelica.ComplexBlocks.ComplexMath.TransferFunction`), v0.10.4 still gives the ED001 below. Rumoca main was not built [not verified on main].

## Summary

A model with an array of records and one equation that sets the whole array from an array constructor (or an array comprehension) of record constructor calls is rejected:

```text
[ED001] unbalanced model: 3 equations, 6 unknowns (balance = -3)
```

In the flat model (`--emit flat-json`) the equation `aw = {R(1, 0), R(2, 0), R(3, 0)}` has `scalar_count` 3, and the record array is 6 scalar unknowns (`aw[1].re`, `aw[1].im`, ..., or the array variables `aw.re` and `aw.im` of size 3 when the size comes from a parameter array). A record array on both sides of the equation (`aw = bw`) is counted 6 in v0.10.4, so only the array constructor side is counted per element.

v0.10.2 reports `[ED008] unresolved Flat reference 'aw'` for the same models with a literal size; for a size from a parameter array it already gives the ED001 above.

In the Modelica Standard Library the pattern is `aw = {a[i]*(w)^(size(a,1)-i)*powerOfJ(size(a,1)-i) for i in 1:size(a,1)};` with `Complex aw[size(a,1)]` in `Modelica.ComplexBlocks.ComplexMath.TransferFunction`.

## Steps to reproduce

Self-contained, with a plain record (run it from a directory that holds only this file; the compiler also reads sibling `.mo` files):

```bash
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.A --emit dae-mo   # size 3, array constructor
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.B --emit dae-mo   # size from a parameter array, comprehension
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.C --emit dae-mo   # aw = bw, bw set by an array constructor
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.D --emit dae-mo   # control: a for loop
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.E --emit dae-mo   # control: aw = bw, bw set in a for loop
rumoca compile 39-array-equation-of-record-array-constructor-counts-elements.mo -m Rep39.F --emit dae-mo   # control: arrays of Real
```

With the Modelica Standard Library's `Complex` (4.1.0; `Complex.mo` alone is enough as the source root). Save as `ComplexForms.mo`:

```modelica
model L "size 3, array constructor"
  Complex aw[3];
equation
  aw = {Complex(1, 0), Complex(2, 0), Complex(3, 0)};
end L;

model P "size from a parameter array, comprehension"
  parameter Real a[:] = {1, 2, 3};
  Complex aw[size(a, 1)];
equation
  aw = {Complex(a[i], 0) for i in 1:size(a, 1)};
end P;
```

```bash
rumoca compile ComplexForms.mo -m L --source-root <path to MSL 4.1.0>/Complex.mo --emit dae-mo
rumoca compile ComplexForms.mo -m P --source-root <path to MSL 4.1.0>/Complex.mo --emit dae-mo
```

## Expected

All of `A`, `B`, `C`, `L`, `P` compile like the controls `D`, `E`, `F`: `aw = {R(1, 0), ...}` is one record equation per element, and each record equation is one scalar equation per field (6 for `A`, 12 for `C`).

## Actual

The first error line of `rumoca compile ... --emit dae-mo`:

| model | v0.10.2 | v0.10.4 |
| --- | --- | --- |
| `A` size 3, constructor | `ED008` unresolved Flat reference `aw` | `ED001` 3 equations, 6 unknowns |
| `B` size from `size(a, 1)`, comprehension | `ED001` 3 equations, 6 unknowns | `ED001` 3 equations, 6 unknowns |
| `C` `aw = bw`, `bw = {...}` | `ED008` unresolved Flat reference `bw` | `ED001` 9 equations, 12 unknowns |
| `D` for loop (control) | compiles | compiles |
| `E` `aw = bw`, `bw` in a for loop (control) | compiles | compiles |
| `F` arrays of `Real` (control) | compiles | compiles |
| `L` MSL `Complex`, size 3, constructor | `ED008` unresolved Flat reference `aw` | **compiles** |
| `P` MSL `Complex`, size from `size(a, 1)`, comprehension | `ED001` 3 equations, 6 unknowns | `ED001` 3 equations, 6 unknowns |

`E` balances in v0.10.4 only if `aw = bw` is counted 6, so the `ED001` of `C` (9 and 12) comes from the constructor equation alone.

omc 1.27.2, `checkModel`: `A` 6 equations and 6 variables, `B` 6 and 6, `C` 12 and 12, `D` 6 and 6; `L` and `P` (with the MSL's `Complex.mo`) 6 and 6.

## Re-check on Rumoca v0.10.4 (S359, 2026-11-16)

Done with the prebuilt CLI of each release (`rumoca 0.10.2`, `rumoca 0.10.4`), which is the table above. v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, published 2026-11-02) is still the latest release (`git ls-remote` and the releases API, 2026-11-16).

`L` is fixed in v0.10.4. `crates/rumoca-phase-flatten/src/operator_records.rs` and `operator_records/arrays.rs` exist in v0.10.4 and not in v0.10.2 (`276e82bd`); the latter handles "operator-record operators on vectors" and has a function that pairs the elements of an equation between two record vectors (`vector_equation_pairs`). That is probably why the operator record `Complex` is handled and a plain record is not [inferred; not traced]. A local `operator record Complex` without operators (fields only) gives the same `ED001` as the plain record, which fits.

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4). Rumoca main (`b55eafce`, 2026-11-13) was not built.
- Linux x86_64 (WSL2, Ubuntu). MSL 4.1.0 (only `Complex.mo`).

## Workaround in GlobeMode

None needed for GlobeMode's own checks: its DAE builder (`globe-mode-dae`) counts the equation by fields (6), covered by its own test (S351, `record_array_equation_counts_per_field`: the `for`, comprehension and array-expression forms count alike, 8 and 8; `aw = bw` 12 and 12). Its flat model keeps the element count (3), the number Rumoca's flat model has. The pinned Rumoca (v0.10.2) cannot be compared on these models: its balance check fails (GlobeMode's comparison, `diff-frontend`, S351: 0 of 2 small models for the `aw = {...}` and `aw = bw` forms; the for-loop form matches 2 of 2). Raising the pinned rev to v0.10.4 will change the result only for `Complex` with a literal size [the effect on `msl:diff-frontend` is not measured].

## Notes

- Not traced: where the count of 3 comes from. The flat model carries the equation as one residual `aw = {...}` with `scalar_count` 3.
- A separate limit found on the way, drafted separately as [41](41-ed019-record-array-member-slice-dimension-from-size-call.md) (S371): `Modelica.ComplexBlocks.Examples.ShowTransferFunction` stops on both versions with `[ED019] unsupported Flat semantic owner 'record-array member slice': reference scope is not an instantiated class occurrence`, at `sum(bw.re)` in `TransferFunction.mo:31`. The cause found in S371 is the dimension `size(a, 1)` of the record array (`Complex bw[size(b,1)]`), not the component: a field of the array (`aw.re`, `aw[1].re`) gives the ED019 on v0.10.2 and v0.10.4 as a top-level class too, and compiles when the size is a literal or an `Integer` parameter such as `n = 3`. (The first version of this note, S359, said that the same model compiled as the top-level class compiles; that did not hold for the forms tried in S371.) omc accepts all of them. So the failure of `ShowTransferFunction` in GlobeMode's comparison is not explained by the equation count alone.
- Source: GlobeMode S351 (`doc/sessions/S351-review6-fixes/解説書.md`, `log/work/2026-11-15-s351-review6-fixes.md`), MLS 3.7 https://specification.modelica.org/maint/3.7/arrays.html (section 10.6.1) and https://specification.modelica.org/maint/3.7/class-predefined-types-and-declarations.html (section 4.8, Definition 4.4), checked 2026-11-16. The section numbers and the quoted rules were read through a page summary, not read in full.
