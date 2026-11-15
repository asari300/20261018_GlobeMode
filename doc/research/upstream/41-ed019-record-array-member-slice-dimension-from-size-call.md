# DRAFT (not filed) - A field of a record array (`aw.re`, `aw[1].re`, `aw[i].re`) stops with ED019 `record-array member slice` when the dimension of the array is computed with `size(...)` (e.g. `Complex aw[size(a, 1)]`)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S371): **bug**. A field of an element of a record array is a plain component reference, and `a.m` on an array of records is a slice that "returns the array of components `{a[1].m, ...}`" (MLS 3.7 section 10.6.9: "When the member access operator is applied to a record array, it is interpreted as constructing an array by selecting the member in each record"). Rumoca accepts these references when the array has a literal size or a size given by an `Integer` parameter such as `n = 3`, and stops with `[ED019] unsupported Flat semantic owner 'record-array member slice': reference scope is not an instantiated class occurrence` when the dimension is computed with `size(...)` (also `ndims(...)` and `integer(...)`). The flat model is built; the error comes when the DAE is built. omc 1.27.2 (`checkModel`) accepts all of these models, including the Modelica Standard Library's `Modelica.ComplexBlocks.Examples.ShowTransferFunction`, which Rumoca cannot compile for this reason.

Status (S371, 2026-11-16): reproduces on v0.10.2 (the pin of GlobeMode) and v0.10.4 (the latest release). Rumoca main (`b55eafce`) was not built; its source still has the message (see Notes).

## Summary

A model with `R aw[size(a, 1)]` (`a` a parameter array, `R` a record with two `Real` fields) and any reference to a field of `aw` is rejected:

```text
[ED019] unsupported Flat semantic owner `record-array member slice`:
  reference scope is not an instantiated class occurrence
```

The three kinds of reference all give it: the slice `sum(aw.re)`, the field of one element `aw[1].re`, and the field of an element in an equation `aw[i].re = a[i]` (the error points at the first such reference). It does not depend on the model being used as a component: the top-level class and a component of another model give the same error.

What decides it is how the dimension is written. With the same model body, `R aw[3]`, `R aw[n]` with `parameter Integer n = 3`, `constant Integer n = 3`, `n = 2 + 1`, `n = max(1, 3)`, `n = min(3, 4)`, `n = abs(-3)`, `n = div(7, 2)` and `n = mod(7, 4)` compile, while `R aw[size(a, 1)]` (`a` a parameter, a constant or a fixed-size parameter array), `size({1, 2, 3}, 1)`, `size(ones(3), 1)`, `n = size(a, 1)` (an `Integer` parameter), `ndims({1, 2, 3}) + 2` and `integer(3.2)` do not. Those variants were found with models like `A` below with only the dimension changed; the `.mo` has `A`, `E` (the failing forms) and `F`, `G` (controls).

Without any reference to a field (only the `for` loop with `aw[i] = R(a[i], 0)`), the same model gives the `ED001` of draft 39 (3 equations, 6 unknowns).

The flat model (`--emit flat-json`) is produced for both. For a size that works, the variables are `aw[1].re`, `aw[1].im`, ... and `instance_relations` has one `Aggregate` instance per element (`indices` `[1]`, `[2]`, `[3]`), each with a `Class` instance of `R`. For `size(a, 1)` the variables are `aw.re` and `aw.im` with `dims` `[3]` and there is one `Aggregate` instance (`indices` `[]`) and one `Class` instance of `R` for the whole array. `--emit dae-json` and `--emit dae-mo` stop with the ED019.

In the Modelica Standard Library the pattern is `Complex bw[size(b,1)]; Complex aw[size(a,1)];` with `bSum = Complex(sum(bw.re), sum(bw.im));` in `Modelica.ComplexBlocks.ComplexMath.TransferFunction` (line 31).

## Steps to reproduce

Self-contained, with a plain record (run it from a directory that holds only this file; the compiler also reads sibling `.mo` files):

```bash
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.A --emit dae-mo   # size(a, 1), slice sum(aw.re)
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.B --emit dae-mo   # size(a, 1), field of one element aw[1].re
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.C --emit dae-mo   # A as a component
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.D --emit dae-mo   # size(a, 1), aw[i].re = ... in a for loop
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.E --emit dae-mo   # n = size(a, 1) (Integer parameter), slice
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.F --emit dae-mo   # control: literal size 3
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.G --emit dae-mo   # control: n = 3
rumoca compile 41-ed019-record-array-member-slice-dimension-from-size-call.mo -m Rep41.H --emit dae-mo   # control: arrays of Real sized by size(a, 1)
```

With the Modelica Standard Library's `Complex` (4.1.0; `Complex.mo` alone is enough as the source root). Save as `ComplexSlice.mo`:

```modelica
model Q "size from size(a, 1), the field of all elements read as a slice"
  parameter Real a[:] = {1, 2, 3};
  Complex aw[size(a, 1)];
  Real s;
equation
  for i in 1:size(a, 1) loop
    aw[i] = Complex(a[i], 0);
  end for;
  s = sum(aw.re);
end Q;

model K "control: literal size 3"
  Complex aw[3];
  Real s;
equation
  for i in 1:3 loop
    aw[i] = Complex(i, 0);
  end for;
  s = sum(aw.re);
end K;
```

```bash
rumoca compile ComplexSlice.mo -m Q --source-root <path to MSL 4.1.0>/Complex.mo --emit dae-mo
rumoca compile ComplexSlice.mo -m K --source-root <path to MSL 4.1.0>/Complex.mo --emit dae-mo
```

The Modelica Standard Library example (`<MSL root>` is the folder that holds `Modelica 4.1.0`):

```bash
rumoca compile "Modelica 4.1.0/package.mo" -m Modelica.ComplexBlocks.Examples.ShowTransferFunction --source-root <MSL root> --emit dae-mo
```

## Expected

`A` to `E` compile like the controls `F`, `G`, `H`: the dimension `size(a, 1)` of `a = {1, 2, 3}` is 3, the same array as `R aw[3]`, and `aw.re` is the array `{aw[1].re, aw[2].re, aw[3].re}` (MLS 3.7 section 10.6.9). `Q` compiles like `K`, and so does `Modelica.ComplexBlocks.Examples.ShowTransferFunction`.

## Actual

The first error line of `rumoca compile ... --emit dae-mo` (the same on v0.10.2 and v0.10.4):

| model | result | first error at |
| --- | --- | --- |
| `A` `size(a, 1)`, `sum(aw.re)` | `ED019` record-array member slice | `aw.re` of `sum(aw.re)` (21:13) |
| `B` `size(a, 1)`, `aw[1].re` | `ED019` record-array member slice | `aw[1].re` (36:9) |
| `C` `A` as a component | `ED019` record-array member slice | `aw.re` of `sum(aw.re)` (21:13) |
| `D` `size(a, 1)`, `aw[i].re = a[i]` | `ED019` record-array member slice | `aw[i].re` (54:7) |
| `E` `n = size(a, 1)`, `sum(aw.re)` | `ED019` record-array member slice | `aw.re` of `sum(aw.re)` (72:13) |
| `F` literal size 3 (control) | compiles | |
| `G` `n = 3` (control) | compiles | |
| `H` arrays of `Real` sized by `size(a, 1)` (control) | compiles | |
| `Q` MSL `Complex`, `size(a, 1)`, `sum(aw.re)` | `ED019` record-array member slice | |
| `K` MSL `Complex`, literal size 3 (control) | compiles | |
| MSL `ShowTransferFunction` | `ED019` record-array member slice | `TransferFunction.mo:31:22`, `bw.re` of `sum(bw.re)` |

The `行:列` are in the `.mo` of this folder. The message of the MSL example is the same text:

```text
[ED019] unsupported Flat semantic owner `record-array member slice`:
  reference scope is not an instantiated class occurrence
  ...TransferFunction.mo:31:22
  30 |   aw = {a[i]*(w)^(size(a,1)-i)*powerOfJ(size(a,1)-i) for i in 1:size(a,1)};
  31 |   bSum = Complex(sum(bw.re), sum(bw.im));
```

omc 1.27.2, `checkModel` (with `loadModel(Modelica, {"4.1.0"})` for the last three): `A` 7 equations and 7 variables, `B` 7 and 7, `C` 7 and 7, `D` 6 and 6, `E` 7 and 7, `F` 7 and 7, `G` 7 and 7, `H` 7 and 7; `Q` 7 and 7, `K` 7 and 7; `Modelica.ComplexBlocks.Examples.ShowTransferFunction` 29 and 29.

## Re-check on Rumoca v0.10.4 (S371, 2026-11-16)

Done with the prebuilt CLI of each release (`rumoca 0.10.2`, `rumoca 0.10.4`), which is the table above. v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, published 2026-11-02) is still the latest release (the releases API, 2026-11-16). No model of the `.mo` changes between the two versions.

The other Modelica Standard Library example with the same message in GlobeMode's funnel file of 2026-10-31 (v0.10.2, 566 examples; 2 of them have it) is `Modelica.Electrical.Batteries.Examples.ShowImpedance`, at `cellData.rcData[k].R` in `Electrical/Batteries/Utilities/Impedance.mo:12`. v0.10.2 stops with the ED019; v0.10.4 gets past it (`--emit dae-json` exits 0; `--emit dae-mo` then stops in code generation with `EC002` for an enumeration, which is a different limit). That is a record array inside a parameter record (`parameter RCData rcData[nRC]` in `Electrical/Batteries/ParameterRecords/TransientData/CellData.mo`, `nRC` an `Integer` parameter), not the `size(...)` dimension of this draft; why v0.10.2 stops there and v0.10.4 does not is not traced.

## Environment

- Rumoca v0.10.2 (prebuilt CLI `rumoca 0.10.2`; GlobeMode pins rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`) and v0.10.4 (prebuilt `rumoca 0.10.4`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4). Rumoca main (`b55eafce`, 2026-11-13) was not built.
- Linux x86_64 (WSL2, Ubuntu). MSL 4.1.0. omc 1.27.2.

## Workaround in GlobeMode

None needed in GlobeMode's own front end: its flat model (`globe-mode-flat`) expands a record array whose size it can evaluate into elements (`aw[1].re`, ...) and keeps the field references, so the MSL example flattens (S337; `doc/adr/0039-native-cst-parser.md`). In the checks that go through Rumoca (`globe check`, `msl:funnel`) the example stops at S5 and is classified `GM-U0001` (unsupported, not an error) by its Rumoca code `ED019` (`doc/msl-funnel.md`, row 109). GlobeMode's flat model and Rumoca's differ here: Rumoca keeps one variable `transferFunction.aw.re` of size 3 where GlobeMode has `transferFunction.aw[1].re`, ... (the names differ in GlobeMode's comparison, `diff-frontend`; ADR-0039, S334 and S337).

## Notes

- Not traced: why the dimension changes the result. The cause seems to be in the construction of the DAE: in Rumoca main (`b55eafce`, source read through the GitHub contents API, not built) the message "reference scope is not an instantiated class occurrence" is returned by `component_ancestry` in `crates/rumoca-phase-dae/src/construction/analysis/record_array_fields.rs` when the kind of the scope instance is not `Class`. Which instance is the scope in `A` is not known. The regression test of the member slice in main (`crates/rumoca/tests/suite_core/record_array_member_slice_test.rs`, `pin[:].v` of `Pin pin[3]`) has no `size(...)` dimension [inferred from reading the file; not run].
- Of the 2 MSL examples with this message in the funnel file, 1 has the `size(...)` form and still stops on v0.10.4 (`ShowTransferFunction`; `Complex bw[size(b,1)]`). Only that example was checked on both versions; other library examples were not looked at [not verified]. GlobeMode's comparison of its flat model with Rumoca's (`diff-frontend`, S351) had `ShowTransferFunction` at 0 of 1 for the equation count; the ED019 comes before the count is made. Draft 39 (the equation count of record arrays) reaches that example only after this is fixed.
- `D` also shows that the reference does not need to be a slice: a field of an element on the left side of an equation gives the same error, so the name `member slice` is wider than the slices of MLS section 10.6.9.
- Source: GlobeMode S359 (found with the `ShowTransferFunction` of draft 39; its note said the same model as a top-level class compiles, which is not so for these forms: `A` and its top-level use fail alike) and S371 (this draft). MLS 3.7 https://specification.modelica.org/maint/3.7/arrays.html (section 10.6.9 Slice Operation; the text of the section was read in the page, 2026-11-16). Rumoca main source: https://github.com/CogniPilot/rumoca/blob/b55eafce/crates/rumoca-phase-dae/src/construction/analysis/record_array_fields.rs (read 2026-11-16).
