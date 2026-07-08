## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add Kani dev-dependency and configure Cargo.toml |
| T1.2 | Create kani-harnesses directory |
| T2.1 | Write BFS evaluator proof harnesses |
| T2.2 | Write naming convention proof harnesses |
| T2.3 | Write translator proof harnesses |
| T2.4 | Write Promela generator proof harnesses |
| T2.5 | Write convertibility check proof harnesses |
| T3.1 | Fix BFS evaluator to handle `[]` patterns |
| T3.2 | Fix BFS evaluator to handle `<>` patterns |
| T3.3 | Run BFS harness to confirm fix works |
| T4.1 | Run naming convention harnesses |
| T4.2 | Run translator harnesses |
| T4.3 | Run Promela generator harnesses |
| T4.4 | Run convertibility check harnesses |
| T5.1 | Add CI step to run Kani harnesses |
| T5.2 | Verify CI passes on a PR branch |

## ADDED Requirements

### Requirement: Setup before implementation

T1.1 SHALL complete BEFORE T2.1 SHALL start.

#### Scenario: Kani is available before harness work begins

- **WHEN** T1.1 completes
- **THEN** `cargo kani --version` SHALL succeed in the project

### Requirement: Bug-first verification order

T2.1 SHALL complete BEFORE T3.1 SHALL be applied.

#### Scenario: Failing harness proves the bug exists

- **WHEN** T2.1 writes a harness that expects `evaluate_ltl` to fail on `[]` patterns
- **THEN** `cargo kani --harness verify_unrecognized_patterns_silently_pass` SHALL report FAILURE before T3.1 runs

#### Scenario: Passing harness proves the fix works

- **WHEN** T3.3 runs the same harness after T3.1 and T3.2 complete
- **THEN** `cargo kani --harness verify_unrecognized_patterns_silently_pass` SHALL report SUCCESS

### Requirement: Independent harnesses run concurrently

T2.1, T2.2, T2.3, T2.4, and T2.5 SHALL run CONCURRENTLY.

#### Scenario: Naming harnesses verify variable consistency

- **WHEN** T2.2 writes `kani-harnesses/naming_convention.rs`
- **THEN** `cargo kani --harness verify_naming_consistency` SHALL pass

#### Scenario: Translator harnesses verify LTL validity

- **WHEN** T2.3 writes `kani-harnesses/translator.rs`
- **THEN** `cargo kani --harness verify_generated_ltl_is_valid` SHALL pass

#### Scenario: Promela harnesses verify structural invariants

- **WHEN** T2.4 writes `kani-harnesses/promela_generator.rs`
- **THEN** `cargo kani --harness verify_promela_balanced_braces` SHALL pass

#### Scenario: Convertibility harnesses verify severity invariants

- **WHEN** T2.5 writes `kani-harnesses/convertibility.rs`
- **THEN** `cargo kani --harness verify_check_tasks_soundness` SHALL pass

### Requirement: All harnesses pass before CI integration

T2.1, T3.3, T4.1, T4.2, T4.3, and T4.4 SHALL complete BEFORE T5.1 SHALL be configured.

#### Scenario: CI runs all Kani harnesses

- **WHEN** T5.1 adds a `cargo kani` step to CI
- **THEN** the CI job SHALL run all harnesses and SHALL report SUCCESS

### Requirement: Harnesses always pass in CI

T5.1 SHALL ALWAYS pass on every code change.

#### Scenario: Regression is caught by CI

- **WHEN** a pull request modifies `src/checker/bfs.rs`, `src/translator/mod.rs`, or `src/checker/promela.rs`
- **THEN** the Kani CI job SHALL run and SHALL report SUCCESS
