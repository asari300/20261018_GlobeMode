# Formatter CRLF review

Checked 2026-11-03 at `e73af106c8b7b220a09e162d69798abfae0efcad` (fresh `origin/main`). Issue: [#19](https://github.com/asari300/20261018_GlobeMode/issues/19).

## Scope and evidence

This is the DFS `globe-mode-fmt/src/lib.rs` → `lexer.rs` branch, **limited to the CRLF comment-preservation path**, not a whole-file or crate audit. `layout.rs` was read only to trace output. See [ledger](../../../log/work/2026-11-03-s150-review-fmt-crlf.md) for per-file scope. No next branch was reviewed. Automatic `origin` insertion is intended behavior and excluded.

The block token retains its internal CRLF ([lexer.rs:69–82](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lexer.rs#L69-L82)); layout copies token text ([layout.rs:259](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/layout.rs#L259)). `format` selects CRLF and replaces each LF ([lib.rs:145–161](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lib.rs#L145-L161)), giving `\r\r\n` inside the block. The re-lex and strict comparison ([lib.rs:166–185](https://github.com/asari300/20261018_GlobeMode/blob/e73af106c8b7b220a09e162d69798abfae0efcad/src/Rust/mod/globe-mode-fmt/src/lib.rs#L166-L185)) reject the result. Temporary instrumentation of a copy printed `/* first\r\r\n  second */`; the original was not modified.

## Reproduce

From the repository root with rustc 1.99.1, default features only:

```sh
T=$(mktemp -d)
rustc --edition=2024 --crate-name globe_mode_fmt --crate-type lib src/Rust/mod/globe-mode-fmt/src/lib.rs -o "$T/libglobe_mode_fmt.rlib"
rustc --edition=2024 doc/sessions/S150-review-fmt-crlf/repro.rs --extern globe_mode_fmt="$T/libglobe_mode_fmt.rlib" -o "$T/repro"
"$T/repro"
rustc --edition=2024 --test --crate-name globe_mode_fmt src/Rust/mod/globe-mode-fmt/src/lib.rs -o "$T/tests"
"$T/tests"
```

The exact source and both controls are in [repro.rs](repro.rs); `insert_origin` and `format_html` are false, other options default. Observed:

```text
block_crlf: Err(FormatError { message: "整形でトークンが変わった（2 番目）。入力をそのまま残す", offset: 11 })
block_lf: Ok("model M\n\n  /* first\n  second */\n  Real x;\n\nend M;\n")
line_crlf: Ok("model M\r\n\r\n  // first\r\n  // second\r\n  Real x;\r\n\r\nend M;\r\n")
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Expected: return formatted text while preserving comment contents. Actual: error, so the API does not return a formatted string. The input is borrowed, not modified. This is not evidence of on-disk corruption.

## Limits

Direct rustc bypasses Cargo dependency resolution, workspace build scripts, optional `config`, CLI, wasm, integration tests, and CI. The controls are examples, not exhaustive. No product fix or PR is asserted. Issue and PR titles/searches were checked across open and closed items before publication; no duplicate found. [PR #17](https://github.com/asari300/20261018_GlobeMode/pull/17) addresses separate panics; [PR #18](https://github.com/asari300/20261018_GlobeMode/pull/18) is a historical review map, not this finding.
