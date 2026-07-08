## Phase 1: LTL AST Types

- [x] 1.1 Define `LtlFormula` and `LtlCondition` enums in `src/ir/ltl.rs` with all variants (Always, Eventually, Atom, Not, And, Or, Implies, Iff)
- [x] 1.2 Add `ltl_to_string()` function that serializes `LtlFormula` to the same string format the current `generate_ltl()` produces
- [x] 1.3 Add round-trip tests: generate LtlFormula, serialize, verify output matches expected format (6 tests pass)

## Phase 2: Refactor LTL Generation

- [x] 2.1 Refactor `generate_ltl()` in `src/translator/mod.rs` to return `Option<LtlFormula>` instead of `Option<String>`
- [x] 2.2 Update all callers of `generate_ltl()` to handle the new return type
- [x] 2.3 Run `cargo test` to confirm all tests pass with the new return type (222 tests pass)

## Phase 3: Refactor BFS Evaluator

- [x] 3.1 Refactor `evaluate_ltl()` in `src/checker/bfs.rs` to take `&LtlFormula` instead of `&str`
- [x] 3.2 Implement evaluation by structural induction: match on LtlFormula and LtlCondition variants
- [x] 3.3 Remove all `strip_prefix()` and `split_once()` string-parsing logic from the evaluator
- [x] 3.4 Run `cargo test` to confirm all tests pass

## Phase 4: Promela AST Types

- [ ] 4.1 Define `PromelaModel`, `PromelaVar`, `PromelaProcess`, `PromelaStmt`, `PromelaBranch`, `PromelaLtl` types in `src/ir/promela.rs`
- [ ] 4.2 Add `promela_to_string()` function that serializes `PromelaModel` to valid Promela source
- [ ] 4.3 Add structural invariant tests: balanced braces, matching do/od, matching if/fi

## Phase 5: Refactor Promela Generation

- [ ] 5.1 Refactor `generate_promela()` in `src/checker/promela.rs` to return `PromelaModel` instead of `String`
- [ ] 5.2 Update all callers of `generate_promela()` to handle the new return type
- [ ] 5.3 Run `cargo test` to confirm all tests pass

## Phase 6: Kani Harness Updates

- [x] 6.1 Update BFS evaluator Kani harnesses to use `LtlFormula` directly (no string parsing)
- [x] 6.2 Add Kani harness for `ltl_to_string()` round-trip: generate → serialize → verify format
- [x] 6.3 Add Kani harness for `evaluate_ltl()` structural induction: verify all LtlFormula variants
- [x] 6.4 Run all Kani harnesses and confirm they pass (10/10, 0.8-3.8s each)
