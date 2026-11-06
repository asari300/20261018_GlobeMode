# DRAFT (not filed) - Rumoca rejects valid parameter expressions as `stateSelect` with EI032

Target: Rumoca (https://github.com/CogniPilot/rumoca)

Verdict (GlobeMode, S070): **Rumoca rejects valid Modelica.** `stateSelect` is declared as a parameter attribute, so any parameter expression is allowed. omc accepts both shapes.

## Summary

Rumoca evaluates the `stateSelect` attribute of a `Real` to an enumeration literal during instantiation and gives up with `[EI032] invalid value for type attribute 'stateSelect': <expr>` when the expression is not a literal it can fold. Two ordinary shapes fail:

- **A**: an if-expression over a parameter of another component, `T(stateSelect = if medium.preferredMediumStates then StateSelect.prefer else StateSelect.default)`. This is how IBPSA's conservation equation and its media (`IBPSA.Fluid.Interfaces.ConservationEquation`, `Media/Water.mo`, `Media/Air.mo`) choose states; it is also the pattern of `Modelica.Media` base properties.
- **B**: a parameter forwarded through a modification, `Joint j(stateSelect = ss)` where `Joint` declares `parameter StateSelect stateSelect` and uses it as `Real phi(stateSelect = stateSelect)`. MSL `Mechanics.MultiBody.Joints.Revolute` is built this way (its `stateSelect` parameter feeds `phi` and `w`); the failure shows when a parent supplies the value, as in the DeltaRobot library.

With literal values (`StateSelect.prefer`) and with the unmodified default of B both compile.

Counts in GlobeMode's run of seven public libraries: 235 examples (Buildings 87, IBPSA 61, AixLib about 80 by subtraction, DeltaRobot 7): 228 are shape A, 7 (DeltaRobot) shape B.

## Steps to reproduce

Compile the attached `19-ei032-statselect-parameter-expression.mo` (self-contained, no MSL):

```bash
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19A    # fails
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19B    # fails
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19A0   # literal value, compiles
rumoca compile 19-ei032-statselect-parameter-expression.mo -m Rep19B0   # compiles
```

## Expected

All four compile. MLS 3.6 section 4.8.1 "Real Type" (https://specification.modelica.org/maint/3.6/class-predefined-types-and-declarations.html, checked 2026-11-01) defines the attribute as `parameter StateSelect stateSelect = StateSelect.default;` next to `parameter RealType start;` etc., and section 4.8.7.1 says the `StateSelect` enumeration "is the type of the stateSelect attribute of the Real type". An attribute declared `parameter` takes a parameter expression (section 3.8.3 "Parameter Expressions", https://specification.modelica.org/maint/3.6/operators-and-expressions.html); a reference to a parameter, and an if-expression over parameters, are parameter expressions. The specification says nothing that restricts `stateSelect` to literals. (Rumoca's own error text cites section 4.4.4 of the specification, which is "Component Variability Prefixes" in 3.6; that section does not mention this.)

## Actual

```text
$ globe check 19-ei032-statselect-parameter-expression.mo --model Rep19A
...mo:22:24: error[GM-E3001] S3 instantiate: invalid value for type attribute `stateSelect`: if medium.preferredMediumStates then StateSelect.prefer else StateSelect.default
    note: rumoca: EI032
$ globe check 19-ei032-statselect-parameter-expression.mo --model Rep19B
...mo:38:37: error[GM-E3001] S3 instantiate: invalid value for type attribute `stateSelect`: stateSelect
    note: rumoca: EI032
$ rumoca compile ... -m Rep19A     ->  [EI032] invalid value for type attribute `stateSelect`: if medium.preferredMediumStates then ...
$ rumoca compile ... -m Rep19B     ->  [EI032] invalid value for type attribute `stateSelect`: stateSelect
```

OpenModelica 1.27.2: `checkModel(Rep19A)` and `checkModel(Rep19B)` both "completed successfully" (1 equation, 1 variable each).

## Re-check on Rumoca v0.10.4 (S115, 2026-11-03)

Unchanged: `Rep19A` and `Rep19B` give the same `[EI032] invalid value for type attribute 'stateSelect'` messages; `Rep19A0` and `Rep19B0` compile (exit 0). The library examples were not re-run with v0.10.4. [not verified]

## Environment

- Rumoca v0.10.2, git rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`; also the prebuilt `rumoca-linux-x86_64` v0.10.2 binary.
- Linux x86_64 (WSL2, Ubuntu).
- Counts: GlobeMode S063 classification, `doc/research/libraries-2026-11-01.md`: EI032 235 of 1,781 failures (228 are shape A, 7 DeltaRobot shape B). Libraries: IBPSA 4.0.0 (master `2adc9194`), Buildings 14.0.0, AixLib 3.0.1, DeltaRobot with MSL 4.0.0/4.1.0.
- Not yet re-checked against Rumoca main (0.10.3 at the time of writing). Re-check before filing.
- Re-checked on 2026-11-03 (S115) with the latest release, Rumoca v0.10.4 (tag commit `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`, released 2026-11-02, prebuilt `rumoca-linux-x86_64`, https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4), next to the v0.10.2 binary. Rumoca main was not built. [not verified on main]

## Workaround in GlobeMode

None. Recorded as a Rumoca limit; the examples stop at S3 (`GM-E3001`).

## Notes

- Where, in rev `276e82b` (names and line numbers only): `crates/rumoca-phase-instantiate/src/attributes.rs`: `parse_required_state_select` (about lines 607-630) tries `parse_state_select`, then `eval_state_select_expr_with_source_scope`, then once more after qualifying names; if all fail it raises `InvalidTypeAttribute` (`EI032`). The evaluator is in `crates/rumoca-eval-ast/src/eval_instantiate/component_params.rs` (`eval_state_select_expr_with_source_scope`, `eval_if_state_select`, about lines 71-135). It handles literals, references resolved through the modification environment, parenthesized expressions, arrays of equal elements, and if-expressions whose condition `eval_scoped_string_condition_with_depth` can decide. The two failing shapes (a dotted reference to a parameter of a sibling component in A; a reference through an intermediate modification in B) apparently are not resolved by it. [Inference from the outcome; the evaluator was not stepped through.]
- Since `stateSelect` only steers state selection, an alternative is to keep the unevaluated expression and evaluate it after parameter values are known, or to fall back to `StateSelect.default` (with a warning) when the value cannot be folded. [Suggestion only.]
- Shape B only fails when a parent supplies the value; `Rep19B0` and the unmodified `Joint` both compile.
