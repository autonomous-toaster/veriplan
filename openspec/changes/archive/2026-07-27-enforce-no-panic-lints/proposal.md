## Why

A run of `veriplan check` on the shield project panicked at runtime because `truncate()` slices a string at a byte boundary that falls inside a multi-byte UTF-8 character. Panics are forbidden by cargo lint rules — this violation went undetected because no lint rule enforces the prohibition, and the Kani harness that proves `check_classifiability` is panic-free only models ASCII strings.

Without lint-level enforcement, any `unwrap()`, `expect()`, or `panic!()` introduced in the future will compile without warning, making the codebase vulnerable to runtime panics that erode the model-checking guarantees the tool is built on.

## What Changes

- Add `[lints.clippy]` to `Cargo.toml` denying `unwrap_used`, `expect_used`, and `panic`
- Copy `check-lint-rules` Justfile recipe from shield to veriplan
- Replace all `.unwrap()` calls in the codebase with proper error handling (`?`, `.ok()`, `.expect()` with explanatory messages, or explicit match)
- Fix the `truncate()` function family (6 copies, 5 broken) to handle multi-byte UTF-8 safely — the original trigger for this change
- Add a shared `truncate` utility to eliminate duplication

## Capabilities

### New Capabilities

- `no-panic-enforcement`: Lint-level and CI-level enforcement that panics (via `unwrap`, `expect`, `panic!()`) are denied at compile time

### Modified Capabilities

- (none — no spec-level behavior changes)

## Impact

- **Cargo.toml**: new `[lints.clippy]` section
- **Justfile**: new `check-lint-rules` recipe
- **12 call sites across 4 files**: `.unwrap()` replaced with proper error handling
- **6 `truncate` functions**: consolidated into a shared, UTF-8-safe utility
- **Kani harnesses**: `verify_check_classifiability_no_panic` should be updated to model non-ASCII strings (separate task)
