# pre-commit-mode

## Purpose

Defines the `--pre-commit` flag for `veriplan check` that provides sane exit code semantics for pre-commit hook usage: missing SPIN is non-blocking, only real violations block commits.

## Task Reference

| T ID | Description |
|------|-------------|
| T1.1 | Add --pre-commit flag to check subcommand in main.rs |
| T1.2 | Implement pre-commit exit code semantics (blockers → 1, missing SPIN → 0) |
| T1.3 | Add SPIN-missing graceful fallback in pre-commit mode |
| T1.4 | Detect $PRE_COMMIT env var for concise output format |

## Requirements

### Requirement: Pre-commit exit code semantics

T1.2 SHALL ALWAYS produce the following exit codes when `--pre-commit` is active:

- Exit 0 when all constraints are satisfied (plan is valid)
- Exit 0 when SPIN is not available but convertibility passes (with a warning to stderr)
- Exit 0 when only warnings are found (no blockers, no violations)
- Exit 1 when convertibility blockers are found
- Exit 1 when SPIN violations are found

T1.2 SHALL ALWAYS produce exit code 2 (hard failure) when `--pre-commit` is NOT active and SPIN is missing — preserving existing behavior.

#### Scenario: Valid plan with SPIN available

- **WHEN** T1.2 processes a plan that passes both convertibility and SPIN model checking
- **THEN** T1.2 SHALL exit with code 0

#### Scenario: Valid plan without SPIN

- **WHEN** T1.2 processes a plan that passes convertibility but SPIN is not installed
- **AND** T1.2 is running in pre-commit mode
- **THEN** T1.2 SHALL print a warning to stderr about missing SPIN and exit with code 0

#### Scenario: Convertibility blockers found

- **WHEN** T1.2 processes a plan with convertibility blockers
- **AND** T1.2 is running in pre-commit mode
- **THEN** T1.2 SHALL exit with code 1 regardless of whether SPIN is available

#### Scenario: SPIN violations found

- **WHEN** T1.2 processes a plan that passes convertibility but has SPIN violations
- **AND** T1.2 is running in pre-commit mode
- **THEN** T1.2 SHALL exit with code 1

### Requirement: SPIN-missing graceful fallback in pre-commit mode

T1.3 SHALL ALWAYS treat a missing SPIN binary as a non-blocking condition when `--pre-commit` is active. The check SHALL proceed with convertibility-only verification and print a message to stderr indicating that full verification requires SPIN installation.

#### Scenario: SPIN not found in pre-commit mode

- **WHEN** T1.3 detects that `spin` is not on PATH
- **AND** `--pre-commit` is active
- **THEN** T1.3 SHALL print "SPIN not found — skipping model checking. Install SPIN for full verification." to stderr
- **AND** T1.3 SHALL proceed with convertibility check only and exit 0 if it passes

#### Scenario: SPIN not found in normal mode

- **WHEN** T1.3 detects that `spin` is not on PATH
- **AND** `--pre-commit` is NOT active
- **THEN** T1.3 SHALL exit with code 2 (preserving existing hard-failure behavior)

### Requirement: Pre-commit output format

T1.4 SHALL ALWAYS produce concise, action-oriented output when the `PRE_COMMIT` environment variable is set to `1`. The output SHALL include:

- One line per change showing status (✓ valid, ✗ blocked)
- Blocker details only when blockers exist
- A "skip with: VERIPLAN_SKIP=1 git commit" hint when the commit is blocked

#### Scenario: Running under pre-commit framework

- **WHEN** T1.4 detects `PRE_COMMIT=1` in the environment
- **THEN** T1.4 SHALL format output concisely: one status line per change, blocker details only, and a skip hint
