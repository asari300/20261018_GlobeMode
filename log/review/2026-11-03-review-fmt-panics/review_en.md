# Formatter panic fixes: review and reproduction

| Item | Value |
| --- | --- |
| Date | 2026-11-03 |
| Branch | `feature/s137-review-fmt-panics` |
| Base | `5726edf62f6c`, with three source/test changes |
| Artifacts | [Japanese](review_jp.md), [slides](review.html), [reproduction](repro.rs) |

## Purpose and findings

Document two already-verified fixes, not a new audit. In the lexer, `!é` previously sliced through a UTF-8 character while trying a two-byte operator. It now compares byte prefixes and returns a `FormatError` at byte 0. In layout, a two-output function call with the first tuple output omitted, `(, x) := f();`, and width 5 previously reached a nonempty-argument assertion. Empty argument nodes now take the non-flat rendering path; the reproduction checks the output and idempotence.

## Reproduce

Run from the repository root in this worktree with the fixes present. Requires Git, tar, `rustc` with edition 2024 and a C linker. The baseline archive is read from the local Git object database; it never overwrites the worktree. Each old case is run separately so the first panic cannot hide the second.

```sh
set -eu
T=$(mktemp -d)
BASE=e73af106c8b7b220a09e162d69798abfae0efcad
CRATE=src/Rust/mod/globe-mode-fmt
REPRO=log/review/2026-11-03-review-fmt-panics/repro.rs
# Before: extract only the committed source, without changing the worktree.
git archive "$BASE" "$CRATE/src" | tar -x -C "$T"
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib "$T/$CRATE/src/lib.rs" -o "$T/libglobe_mode_fmt.rlib"
rustc --edition=2024 "$REPRO" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/repro"
set +e
"$T/repro" unicode; printf 'before unicode exit=%s\n' "$?"
"$T/repro" tuple; printf 'before tuple exit=%s\n' "$?"
set -e
# After: compile the modified worktree source.
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib "$CRATE/src/lib.rs" -o "$T/libglobe_mode_fmt.rlib"
rustc --edition=2024 "$REPRO" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/repro"
"$T/repro" unicode
"$T/repro" tuple
# Default-feature unit and integration tests without Cargo.
rustc --edition=2024 --test --crate-name globe_mode_fmt "$CRATE/src/lib.rs" -o "$T/unit"
"$T/unit"
CARGO_MANIFEST_DIR="$PWD/$CRATE" rustc --edition=2024 --test "$CRATE/tests/format.rs" --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/format-tests"
"$T/format-tests"
rm -rf "$T"
```

Observed after the worktree move and merge, using main `e73af106` as baseline, with `rustc 1.99.1 (58cf8f9c3 2026-10-13)`:

```text
before unicode exit=101  (lexer: end byte index 2 is not a char boundary)
before tuple exit=101    (layout: 引数は空でない)
unicode: error at byte 0 (no panic)
tuple: formatted and idempotent (no panic)
test result: ok. 7 passed; 0 failed
test result: ok. 11 passed; 0 failed
```

The parent and independent reviewer also reported 18 tests passing; that is distinct from the post-move rerun above (7 unit + 11 integration). These are direct `rustc` default-feature tests, not Cargo or full-workspace verification. Cargo, optional features, clippy and CI remain unverified. The reproduction does not establish full Modelica semantic validation.

## Next

The overall Mermaid diagram was created separately in S136. Cargo, CI and browser rendering of the HTML remain unverified.
