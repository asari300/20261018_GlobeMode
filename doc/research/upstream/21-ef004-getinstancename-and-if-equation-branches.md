# DRAFT (not filed) - Rumoca: EF004 "unsupported equation form" for `getInstanceName()` in a parameter binding, and for an if-equation with a vector product over a component array

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **both are Rumoca limits on valid Modelica**, reproduced with small self-contained models. They are two different causes behind one code (EF004); file them as two issues, or as one issue with two parts. omc accepts all the models.

## Summary

EF004 is Rumoca's catch-all "unsupported equation form" in the flatten phase. In GlobeMode's run of seven public libraries it is the third most common failure code (217 examples). Two causes cover the bulk:

- **A. `getInstanceName()` outside an equation or algorithm** (119 examples: ThermofluidStream 93, Buildings 11, IBPSA 8, AixLib 7). `final parameter String name = getInstanceName();` fails with `[EF004] unsupported equation form: getInstanceName() requires a model/block instance scope`. The same call inside an `assert` in an `equation` or `initial equation` section works. The message says "equation form", but the failing place is a declaration binding.
- **B. A dynamic if-equation whose branches look different in "scalar row count"** (Chemical, 41 examples). In `Chemical.Processes.*` (`Processes.mo` line 1174) the first branch is `h_rear_mix*(substrates.n_flow*ones(nS)) + products.n_flow*products.state_rearwards.h = 0;`, a scalar equation, and the else branch is `h_rear_mix = 0;`. `substrates` is a component array, so `substrates.n_flow` is an array of `Real`. With one element all is fine; with `nS` of 2 or 0 Rumoca reports `[EF004] unsupported equation form: dynamic if-equation branches have different scalar row counts`. Outside an if-equation the same equation is counted as `n` equations instead of 1 (error `ED001 unbalanced model: 4 equations, 3 unknowns`), so the root cause looks like a wrong scalar count for `array_field * vector` when the array is a field of a component array.

## Steps to reproduce

Compile the attached `21-ef004-getinstancename-and-if-equation-branches.mo` (self-contained, no MSL):

```bash
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A     # A: fails
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A2    # assert in equation: compiles
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21A3    # assert in initial equation: compiles
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B     # B, n = 2: fails
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B3    # B, n = 3: fails
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B1    # B, n = 1: compiles
rumoca compile 21-ef004-getinstancename-and-if-equation-branches.mo -m Rep21B4    # B outside an if: ED001 (4 equations, 3 unknowns)
```

## Expected

A. All three compile. MLS 3.6 section 3.7.4.6 "getInstanceName" (https://specification.modelica.org/maint/3.6/operators-and-expressions.html, checked 2026-11-01): "Returns a string with the name of the model/block that is simulated, appended with the fully qualified name of the instance in which this function is called." and "If this function is not called inside a model or block (e.g., the function is called in a function or in a constant of a package), the return value is not specified." A binding of a parameter inside a model is "inside a model"; ThermofluidStream does exactly that (8 declarations of the form `final parameter String name = getInstanceName();`). [The spec does not say which variability the result has; it is used as a parameter binding, and omc accepts it.]

B. All compile and balance: `hMix*(s.n_flow*ones(n))` is the product of a scalar and the scalar `Vector*Vector` (MLS 3.6 section 10.6.4 "Multiplication of Matrices and Vectors", https://specification.modelica.org/maint/3.6/arrays.html, checked 2026-11-01: Table 10.11, m-vector times m-vector has size "Scalar", `c := sum_k a[k]*b[k]`), so the equation has one row for every `n`. With n = 3 the model has 5 equations in 5 unknowns (omc: `Rep21B3` 5 equations, 5 variables).

## Actual

```text
$ globe check 21-ef004-getinstancename-and-if-equation-branches.mo --model Rep21A
...mo:8:33: unsupported[GM-U0001] S4 接続: unsupported equation form: getInstanceName() requires a model/block instance scope
    note: rumoca: EF004
$ globe check ... --model Rep21B
...mo:62:6: unsupported[GM-U0001] S4 接続: unsupported equation form: dynamic if-equation branches have different scalar row counts
    note: rumoca: EF004
$ globe check ... --model Rep21B4
...: error[GM-E5002] S5 方程式数と変数数: unbalanced model: 4 equations, 3 unknowns (balance = 1)
    note: rumoca: ED001
$ rumoca compile ... -m Rep21A   ->  [EF004] unsupported equation form: getInstanceName() requires a model/block instance scope
$ rumoca compile ... -m Rep21B   ->  [EF004] unsupported equation form: dynamic if-equation branches have different scalar row counts
```

OpenModelica 1.27.2: `checkModel` on `Rep21A`, `Rep21B`, `Rep21B3` and `Rep21B4` all complete successfully (`Rep21B4`: 3 equations, 3 variables; `Rep21B`: 4 and 4).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged for both parts: `Rep21A` gives `[EF004] ... getInstanceName() requires a model/block instance scope`; `Rep21B` and `Rep21B3` give `[EF004] ... dynamic if-equation branches have different scalar row counts`; `Rep21B4` gives `[ED001] unbalanced model: 4 equations, 3 unknowns`. `Rep21A2`, `Rep21A3` and `Rep21B1` compile (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 classification, `doc/research/libraries-2026-11-01.md`: EF004 217 of 1,781 failures (A: 119; B: Chemical 41, 41 of Chemical's 43 GM-U cases, the other 2 being ED020). Libraries: ThermofluidStream 1.4.1 (`final parameter String name = getInstanceName();` in `Idealized/...` and others; 17 uses in all), Chemical 2.0.0 (`Chemical.Examples.HydrogenCombustion` stops at `Processes.mo:1174`), IBPSA 4.0.0 (`Fluid/Sources/Boundary_pT.mo` uses it in asserts), Buildings 14.0.0, AixLib 3.0.1.
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as Rumoca limits; both stop at S4 and are counted as `GM-U0001` (unsupported by Rumoca v0.10.2).

## Notes

- Where, in rev `276e82b` (names and line numbers only):
  - A: `crates/rumoca-phase-flatten/src/ast_lower.rs`, `lower_get_instance_name_call` (about lines 1124-1145) fails when `LoweringContext::instance_name` is `None`; the contexts built for declaration bindings in `crates/rumoca-phase-flatten/src/variables.rs` (about lines 432, 530, 564) set `instance_name: None`, while the equation path supplies it. [Reading only.]
  - B: `crates/rumoca-phase-flatten/src/equations/conditional_and_eval.rs`, `create_conditional_equation_from_simple` (about lines 141-172), compares `infer_simple_equation_scalar_count` (`crates/rumoca-phase-flatten/src/equations/shape_inference.rs`, about line 456) between branches. The count for `hMix*(s.n_flow*ones(n))` apparently becomes `n` instead of 1 when `s` is an array of components (a plain `Real v[n]` and `v*ones(n)` gives 1). [Inference from the outcomes; the function was not stepped through.]
- The B reproduction was reduced from `Chemical.Processes.Reaction` (`substrates`/`products` are connector arrays `[nS]`/`[nP]`); a first attempt with connectors and flow variables was unbalanced for unrelated reasons, so the attached model uses a plain record array.
- Chemical's other failure, `EF016` (21 examples: `invalid call of function Chemical.Interfaces.Definition: named argument MM does not match any input`), is a different cause and is not part of this draft.
