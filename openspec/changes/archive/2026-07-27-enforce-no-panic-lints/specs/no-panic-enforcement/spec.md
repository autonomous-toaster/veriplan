## ADDED Requirements

### Requirement: Lint-level panic prohibition

`Cargo.toml` SHALL contain a `[lints.clippy]` section denying `unwrap_used`, `expect_used`, and `panic` BEFORE any code is merged that introduces new violations.

#### Scenario: Lint rules present in Cargo.toml
- **WHEN** `cargo clippy` runs
- **THEN** the `unwrap_used`, `expect_used`, and `panic` lints SHALL be denied at the workspace level

### Requirement: Existing violations eliminated

All existing `.unwrap()` calls in the codebase SHALL be replaced with explicit error handling (propagation via `?`, match with fallback, or explanatory `.expect()`) BEFORE the lint rules can pass.

#### Scenario: Annotator no-unwrap
- **WHEN** `cargo clippy` runs on `src/annotator/mod.rs`
- **THEN** line 32 SHALL NOT contain `.unwrap()`

#### Scenario: BFS evaluator no-unwrap
- **WHEN** `cargo clippy` runs on `src/checker/bfs.rs`
- **THEN** line 276 SHALL NOT contain `.unwrap()`

#### Scenario: LSP handler no-unwrap
- **WHEN** `cargo clippy` runs on files matching `src/lsp/*.rs`
- **THEN** all `.unwrap()` calls on RwLock read/write results SHALL be replaced

#### Scenario: LSP transport no-unwrap
- **WHEN** `cargo clippy` runs on `src/lsp/transport.rs`
- **THEN** line 41 SHALL NOT contain `.unwrap()`

### Requirement: UTF-8-safe truncation

The `truncate` function SHALL NOT panic when slicing strings containing multi-byte UTF-8 characters SHALL use char-boundary-aware slicing via `char_indices()`.

#### Scenario: Truncation with multi-byte characters
- **WHEN** `truncate("café au lait → chaud", 15)` is called
- **THEN** it SHALL return a string with length ≤ 18 bytes that is the longest prefix ending on a char boundary within the limit

#### Scenario: Duplicate truncate functions consolidated
- **WHEN** the codebase is searched for `fn truncate`
- **THEN** there SHALL be exactly one shared implementation in a utility module

### Requirement: CI enforcement

The Justfile SHALL contain a `check-lint-rules` recipe that verifies the three lint rules are present in `Cargo.toml` BEFORE each CI run.

#### Scenario: Justfile recipe exists
- **WHEN** `just check-lint-rules` is run
- **THEN** it SHALL exit with code 0 if all three lint rules are present and denied in `Cargo.toml`

#### Scenario: Missing lint rule detected
- **WHEN** a lint rule is removed from `Cargo.toml`
- **THEN** `just check-lint-rules` SHALL exit with code 1 and print which rule is missing
