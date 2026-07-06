## Task Reference

| Task ID | Description |
|---------|-------------|
| T4.1 | Add `--checker` CLI flag to `check` subcommand |
| T4.2 | Add `--compare` CLI flag to `check` subcommand |
| T4.3 | Parse `VERIPLAN_CHECKER` env var with CLI override |
| T4.4 | Thread `CheckerBackend` through call chain |
| T4.5 | Validate `--checker` value and error on invalid input |
| T2.1 | Add `CheckerBackend` enum to `checker/mod.rs` |
| T2.2 | Add `checker_backend` parameter to `verify()` |
| T2.3 | Route to correct backend based on enum |
| T2.4 | Gate `require_spin()` behind `CheckerBackend::Spin` |

## ADDED Requirements

### Requirement: CLI flag selects backend

T4.1 SHALL complete BEFORE T4.4 SHALL run. The `--checker` flag SHALL accept `spin` or `spin-rs`.

#### Scenario: Default backend is spin

- **WHEN** user runs `veriplan check` without `--checker`
- **THEN** T2.3 SHALL route to the spin binary backend

#### Scenario: Explicit spin-rs selection

- **WHEN** user runs `veriplan check --checker spin-rs`
- **THEN** T2.3 SHALL route to the spin-rs library backend

#### Scenario: Invalid checker value

- **WHEN** user runs `veriplan check --checker invalid`
- **THEN** T4.5 SHALL exit with an error message listing valid options

### Requirement: Env var selects backend

T4.3 SHALL complete BEFORE T4.4 SHALL run. `VERIPLAN_CHECKER` SHALL select the backend. CLI flag SHALL override env var.

#### Scenario: Env var selects spin-rs

- **WHEN** `VERIPLAN_CHECKER=spin-rs` is set and no `--checker` flag is given
- **THEN** T2.3 SHALL route to the spin-rs library backend

#### Scenario: CLI flag overrides env var

- **WHEN** `VERIPLAN_CHECKER=spin-rs` is set and `--checker spin` is given
- **THEN** T2.3 SHALL route to the spin binary backend

### Requirement: Backend enum drives routing

T2.1 SHALL complete BEFORE T2.2 SHALL run. The `CheckerBackend` enum SHALL have `Spin` and `SpinRs` variants.

#### Scenario: Spin variant gates require_spin

- **WHEN** `CheckerBackend::Spin` is passed to `verify()`
- **THEN** T2.4 SHALL call `require_spin()` and T2.3 SHALL call `spin::run_spin_check()`

#### Scenario: SpinRs variant skips require_spin

- **WHEN** `CheckerBackend::SpinRs` is passed to `verify()`
- **THEN** T2.3 SHALL call `spin_rs::run_spin_rs_check()` without checking for external spin binary
