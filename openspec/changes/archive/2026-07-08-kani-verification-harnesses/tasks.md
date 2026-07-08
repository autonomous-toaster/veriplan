## Phase 1: Setup

- [x] 1.1 Add Kani as a dev-dependency in Cargo.toml with `kani = "0.60"` and configure `[package.metadata.kani]` flags
- [x] 1.2 Create `kani-harnesses/` directory and verify `cargo kani --version` works in the project

## Phase 2: Write Independent Harnesses [concurrent]

- [x] 2.1 Write `src/kani_harnesses/bfs_evaluator.rs` with harnesses that prove the BFS evaluator is broken on `[]` and `<>` patterns
- [x] 2.2 Write `src/kani_harnesses/naming_convention.rs` with harnesses that verify variable name consistency
- [x] 2.3 Write `src/kani_harnesses/translator.rs` with harnesses that verify LTL syntactic validity
- [x] 2.4 Write `src/kani_harnesses/promela_generator.rs` with harnesses that verify Promela structural invariants
- [x] 2.5 Write `src/kani_harnesses/convertibility.rs` with harnesses that verify severity invariants

## Phase 3: Fix BFS Evaluator

- [x] 3.1 Fix `evaluate_ltl` in `src/checker/bfs.rs` to handle `[] ( condition )` patterns
- [x] 3.2 Fix `evaluate_ltl` to handle `<> condition` patterns
- [x] 3.3 Run unit tests to confirm the fix works (218 tests pass)

## Phase 4: Run All Harnesses

- [x] 4.1 Run `cargo kani --harness verify_bracket_equals_g` and confirm pass (9/9 BFS harnesses verified, ~1-18s each)
- [ ] 4.2 Run `cargo kani --harness verify_classify_sequential` and confirm pass (string ops hit Kani loop unwind limits, needs higher --unwind)
- [ ] 4.3 Run `cargo kani --harness verify_promela_balanced_braces` and confirm pass (string-heavy, needs optimization)
- [ ] 4.4 Run `cargo kani --harness verify_check_tasks_soundness` and confirm pass (string-heavy, needs optimization)

## Phase 5: CI Integration

- [x] 5.1 Add `cargo kani` step to Justfile CI configuration (runs BFS harnesses with --unwind 20)
- [x] 5.2 Verify BFS harnesses pass: 9/9 verified (1-18s each, --unwind 20)
