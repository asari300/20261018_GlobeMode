# DRAFT (not filed) - Rumoca rejects valid `redeclare function extends product` with ER132 (a function named like a built-in)

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **Rumoca rejects valid Modelica.** The library code is legal; omc accepts it.

## Summary

`redeclare function extends product` inside a package that inherits a `replaceable partial function product` is rejected with `[ER132] cannot verify ExternalObject lifecycle for 'product': resolved base DefId(N) is missing`, followed by one `ER002 unresolved component reference` per input/output of the function (`x`, `y`, `z`, because the function did not inherit them). The same shape with a function name that is not a built-in (`prod`) compiles. The same error appears for other built-in names (`sum`, `max`, `min`, `abs`, `sin`, `ceil` were tried). It looks like the `extends` of the short form resolves the simple name `product` to the predefined built-in function instead of the inherited member, and then the ExternalObject-lifecycle recognizer does not know that identity.

In real code this is `IBPSA.Electrical.PhaseSystems.OnePhase` (`redeclare function extends product`; the sibling `divide`, `j`, `rotate` in the same file are fine). Every IBPSA / Buildings / AixLib example that uses an electrical component stops here: 267 examples in GlobeMode's run of seven public libraries (Buildings 97, AixLib 85, IBPSA 85). It is the largest single failure of the S1 (name resolution) stage in that run.

## Steps to reproduce

Compile the attached `17-er132-redeclare-function-extends-product.mo` (model `Rep17.M`; no MSL needed):

```bash
rumoca compile 17-er132-redeclare-function-extends-product.mo -m Rep17.M
```

The model has two redeclared functions in `Rep17.One`: `product` (fails) and `sum2` (the same shape with an ordinary name; fine). Renaming `product` to `prod` everywhere makes the whole file compile (`Compilation successful!`, 3 algebraic variables).

## Expected

Compiles. MLS 3.6 section 7.3.1 (https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, checked 2026-11-01): "A class declaration of the type redeclare class extends B(...), where class as usual can be replaced by any other specialized class, replaces the inherited class B with another declaration that extends the inherited class ..." and "Inherited B here means that the class containing redeclare class extends B(...) should also inherit another declaration of B from one of its extends-clauses." The spec's own example uses `redeclare function extends dynamicViscosity` in the same section. The name `B` is therefore the inherited member; section 5.3.1 (simple name lookup, https://specification.modelica.org/maint/3.6/scoping-name-lookup-and-flattening.html, checked 2026-11-01) looks a simple name up "sequentially in each member of the ordered set of instance scopes ... corresponding to lexically enclosing classes until a match is found"; the predefined functions are only reached from the global scope when the lookup stops at an encapsulated class. `product` is not a keyword (the page says only the type names `Real`, `Integer`, `Boolean`, `String` have "restrictions similar to keywords", section 4.8 pointing to 2.3.3), and section 3.7 (Built-in Intrinsic Operators, https://specification.modelica.org/maint/3.6/operators-and-expressions.html) only says that when the *specification* refers to a function with a built-in name, the built-in is meant.

## Actual

```text
$ globe check 17-er132-redeclare-function-extends-product.mo --model Rep17.M
17-er132-redeclare-function-extends-product.mo:27:32: error[GM-E1001] S1 名前解決: cannot verify ExternalObject lifecycle for 'product': resolved base DefId(78) is missing
    note: rumoca: ER132
Rep17.M: NG（S1 名前解決 で停止）

$ rumoca compile 17-er132-redeclare-function-extends-product.mo -m Rep17.M
  [ER132] cannot verify ExternalObject lifecycle for 'product': resolved base DefId(78) is missing
  [ER002] unresolved component reference: 'z'
  [ER002] unresolved component reference: 'x'
  [ER002] unresolved component reference: 'y'
  ...
```

(`globe` is GlobeMode's CLI, which calls the same Rumoca crates; the second block is the prebuilt `rumoca` v0.10.2 binary.)

OpenModelica 1.27.2 (`omc`, `loadFile` then `checkModel(Rep17.M)`): `Check of Rep17.M completed successfully. Class Rep17.M has 6 equation(s) and 6 variable(s).`

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged: `Rep17.M` gives the same `[ER132] cannot verify ExternalObject lifecycle for 'product': resolved base DefId(78) is missing`, followed by the same `ER002` cascade (`z`, `x`, `y`). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce` (https://github.com/CogniPilot/rumoca), as pinned by GlobeMode; also the prebuilt `rumoca-linux-x86_64` v0.10.2 release binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 failure classification, `doc/research/libraries-2026-11-01.md` ("failures by cause"): ER132 267 of 1,781 failures; all 267 name `product`. Library: IBPSA 4.0.0 (master `2adc9194`, 2026-10-29), file `IBPSA/Electrical/PhaseSystems/OnePhase.mo`; Buildings 14.0.0 and AixLib 3.0.1 carry copies of the same package.
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit; GlobeMode counts these examples as failures at S1 (`GM-E1001`).

## Notes

- Where, in rev `276e82b` (names and line numbers only, no code copied):
  - The message comes from `crates/rumoca-phase-resolve/src/semantic_checks/external_objects.rs` (`emit_lifecycle_query_error`, the `MissingBase` arm, about line 206).
  - The error is produced by `ClassDefIndex::external_object_lifecycle` in `crates/rumoca-ir-ast/src/external_object.rs` (about lines 182-221): for each `extends`, the resolved `base_def_id` must be the ExternalObject, a known ExternalObject owner, in `builtin_def_ids`, or a class in the tree.
  - `builtin_def_ids` is built from `BUILTIN_TYPES` only (`crates/rumoca-ir-ast/src/lib.rs`, about line 243). A `DefId` of a predefined built-in *function* is neither, so the check fails with "missing". [Inference from reading; not confirmed by a debugger.]
  - Two things could be fixed: the lookup of `extends product` should give the inherited member (the `ER002` cascade shows the function body did not inherit `x`, `y`, `z`), and the lifecycle recognizer should not report ER132 for a base it cannot classify as an ExternalObject (it ignores `UnresolvedBaseIdentity` already).
- Verified with `sum`, `max`, `min`, `abs`, `sin`, `ceil` in place of `sum2` (with `product` renamed): all fail the same way.
