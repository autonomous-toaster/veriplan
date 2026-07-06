## Task Reference

| Task ID | Description |
|---------|-------------|
| T5.1 | Run both backends sequentially in compare mode |
| T5.2 | Build per-constraint comparison table |
| T5.3 | Print summary with match/mismatch counts and elapsed times |
| T5.4 | Exit 0 on agreement, warning on mismatch |

## ADDED Requirements

### Requirement: Compare mode runs both backends

T5.1 SHALL complete BEFORE T5.2 SHALL run. The `--compare` flag SHALL run both backends on the same plan.

#### Scenario: Compare mode activates

- **WHEN** user runs `veriplan check --compare`
- **THEN** T5.1 SHALL run both the spin binary backend and the spin-rs library backend on the same plan

### Requirement: Per-constraint comparison output

T5.2 SHALL complete BEFORE T5.3 SHALL run. The comparison SHALL produce a per-constraint diff table.

#### Scenario: Comparison table structure

- **WHEN** both backends have completed
- **THEN** T5.2 SHALL produce a table with columns: `Constraint | spin | spin-rs | Match?`

### Requirement: Summary statistics

T5.3 SHALL complete AFTER T5.2 SHALL run. The comparison SHALL include summary statistics.

#### Scenario: Summary output

- **WHEN** comparison mode runs
- **THEN** T5.3 SHALL print: `X/Y constraints match, Z mismatches` plus elapsed time for each backend

### Requirement: Exit code behavior

T5.4 SHALL complete AFTER T5.3 SHALL run. Exit code SHALL reflect agreement between backends.

#### Scenario: Agreement exits 0

- **WHEN** both backends agree on all constraints
- **THEN** T5.4 SHALL exit 0

#### Scenario: Mismatch is non-blocking

- **WHEN** backends disagree on any constraint
- **THEN** T5.4 SHALL exit with a warning but NOT block the commit (exit 0 in pre-commit mode)
