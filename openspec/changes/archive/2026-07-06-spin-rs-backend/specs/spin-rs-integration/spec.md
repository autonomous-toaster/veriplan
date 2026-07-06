## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add `spin-rs` git dependency to `Cargo.toml` |
| T1.2 | Create `checker/promela.rs` with extracted `generate_promela()` |
| T1.3 | Update `checker/spin.rs` to import from `promela.rs` |
| T1.4 | Add `pub mod promela;` to `checker/mod.rs` |
| T3.1 | Create `checker/spin_rs.rs` with `run_spin_rs_check()` |
| T3.2 | Implement Promela generation via `promela::generate_promela()` |
| T3.3 | Call `spin_rs::verify(promela_str)` |
| T3.4 | Map `CheckResult` to `VerificationResult` |
| T3.5 | Add `pub mod spin_rs;` to `checker/mod.rs` |

## ADDED Requirements

### Requirement: Shared Promela generation

T1.2 SHALL complete BEFORE T1.3 SHALL run. `generate_promela()` SHALL be moved from `checker/spin.rs` to `checker/promela.rs` and made public.

#### Scenario: Both backends import promela

- **WHEN** `checker/spin.rs` and `checker/spin_rs.rs` both need to generate Promela
- **THEN** T1.3 and T3.2 SHALL both call `promela::generate_promela(plan, constraints)`

### Requirement: spin-rs git dependency

T1.1 SHALL complete BEFORE T3.3 SHALL run. The `spin-rs` crate SHALL be added as a git dependency pinned to a specific commit.

#### Scenario: Cargo build succeeds

- **WHEN** `cargo build` is run
- **THEN** T1.1 SHALL have pinned spin-rs at a specific commit and compilation SHALL succeed

### Requirement: spin-rs checker module

T3.1 SHALL complete BEFORE T3.3 SHALL run. A new `checker/spin_rs.rs` module SHALL implement the spin-rs verification path.

#### Scenario: spin-rs verifies a plan

- **WHEN** `spin_rs::run_spin_rs_check(plan, plan_name, constraints, conv_report)` is called
- **THEN** T3.2 SHALL generate Promela, T3.3 SHALL call `spin_rs::verify()`, and T3.4 SHALL map the result

### Requirement: Result mapping

T3.4 SHALL complete BEFORE T3.5 SHALL run. The spin-rs module SHALL correctly map `CheckResult` to `VerificationResult`.

#### Scenario: No errors maps to valid

- **WHEN** `CheckResult.errors == 0`
- **THEN** `VerificationResult.valid` SHALL be `Some(true)`

#### Scenario: Errors found maps to invalid

- **WHEN** `CheckResult.errors > 0`
- **THEN** `VerificationResult.valid` SHALL be `Some(false)`

#### Scenario: Violations mapped correctly

- **WHEN** `CheckResult.violations` contains entries
- **THEN** each SHALL be mapped to a `Violation` struct with `constraint_id` from `property_name`, `state` from `description`
