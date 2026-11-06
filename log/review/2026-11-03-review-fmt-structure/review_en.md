# S152 formatter structure review

Baseline: `e73af106c8b7b220a09e162d69798abfae0efcad` (`origin/main` at worktree creation). Branch `feature/s152-review-fmt-structure`. Independently rerun and checked against the specification.

## Findings

### 1. [Issue #20](https://github.com/asari300/20261018_GlobeMode/issues/20): Valid `redeclare model extends M(...)` rejected

`src/Rust/mod/globe-mode-fmt/src/structure.rs:127-145` calls `scan_to` at the token after the opening `(`. At `:243-255`, a closing `)` decrements depth below zero before the stop predicate is checked, so even `M(x = 2)` is rejected as an extra closing parenthesis. `repro.rs` contains complete inherited `replaceable model M` and redeclaration packages: the unmodified `extends M` control succeeds, but the valid modifier returns `Err(FormatError { message: "閉じ括弧が多い", offset: 145 })`. Expected: successful formatting. Modelica 3.6 §7.3.1 permits an optional class modification on `redeclare class extends` (https://specification.modelica.org/maint/3.6/inheritance-modification-and-redeclaration.html, accessed 2026-11-03). The same section gives `long-class-specifier: extends IDENT [class-modification] … end IDENT` and requires an inherited replaceable class, as in this example. No omc or downloaded Rumoca CLI was found; independent compiler semantic verification of this exact example remains unverified.

### 2. [Issue #21](https://github.com/asari300/20261018_GlobeMode/issues/21): Different block end accepted as matching

`structure.rs:18-21,86-105,114-116` records every block only as `Ctx::Block`, then accepts every keyword in `BLOCK_END` for any block. In `repro.rs`, valid `if ... end if;` succeeds, but the malformed `if ... end for;` also returns `Ok` and formatted text. Expected `Err` without changing the document: `src/Rust/mod/globe-mode-fmt/src/lib.rs:144-147` and `doc/format-style.md:23` promise rejection of mismatched `end`. There is no established contract of complete syntax validation; this finding is limited to the explicit promise to reject mismatched `end`, and the demonstrated effect is a successful formatting result for this malformed document. This does not claim a compiler accepts it.

## Scope and verification

- DFS: `src/Rust/mod/globe-mode-fmt/src/lib.rs:144-187` → `structure.rs:1-341`: static inspection of class headers, nested/short classes, sections, block handling, scanning and error branches; tests are samples, not exhaustive. Downstream modules were not audited.
- Run at repository root:

```sh
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib src/Rust/mod/globe-mode-fmt/src/lib.rs -o /tmp/libs152_fmt.rlib
rustc --edition=2024 log/review/2026-11-03-review-fmt-structure/repro.rs --extern globe_mode_fmt=/tmp/libs152_fmt.rlib -o /tmp/s152_repro
/tmp/s152_repro
rustc --edition=2024 --test --crate-name globe_mode_fmt src/Rust/mod/globe-mode-fmt/src/lib.rs -o /tmp/s152_fmt_tests
/tmp/s152_fmt_tests
```

Reproducer: control cases `Ok`, modifier `Err` at byte 145, mismatched block `Ok`; assertions passed. Default-feature direct rustc tests: 7 passed. `cargo test -p globe-mode-fmt --offline` failed before build: workspace git dependency `rumoca` revision `276e82bd` unavailable offline. Cargo feature resolution, integration tests, config feature and whole workspace were not tested.

- Existing Issue #19 CRLF, PR #17 two panics, and intentional `origin` insertion are not findings. Issues and PRs listed with `gh` (all states, limit 200); no matching titles found. No duplicate found; Issues #20 and #21 were created and their bodies and open states read back. No product code was changed.
- No new DFS branch was pursued.

Documentation checks: `check_docs.py` checked 3,005 relative links with no broken links; Playwright Chromium `render_slides.py` checked four slides each at desktop light, desktop dark, and mobile with zero issues.
