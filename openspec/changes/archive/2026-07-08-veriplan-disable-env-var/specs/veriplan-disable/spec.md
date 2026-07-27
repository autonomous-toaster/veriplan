## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add VERIPLAN_DISABLE check in main() before Cli::parse() |
| T1.2 | Add unit test for truthy/falsy value parsing |
| T1.3 | Run veriplan check on the change to validate |

## ADDED Requirements

### Requirement: Disable before argument parsing

T1.1 SHALL complete BEFORE T1.2 SHALL start.

#### Scenario: Disable check runs before CLI parsing

- **WHEN** `VERIPLAN_DISABLE=1` is set
- **THEN** veriplan SHALL print a warning to stderr and SHALL exit 0 before parsing any CLI arguments

### Requirement: Truthy semantics

T1.2 SHALL complete BEFORE T1.3 SHALL start.

#### Scenario: Truthy values disable

- **WHEN** `VERIPLAN_DISABLE` is set to `1`, `true`, or `yes`
- **THEN** veriplan SHALL be disabled

#### Scenario: Falsy values do not disable

- **WHEN** `VERIPLAN_DISABLE` is set to `0`, `false`, `no`, or empty string
- **THEN** veriplan SHALL NOT be disabled

#### Scenario: Unset does not disable

- **WHEN** `VERIPLAN_DISABLE` is not set
- **THEN** veriplan SHALL NOT be disabled

### Requirement: Change passes self-check

T1.3 SHALL ALWAYS pass.

#### Scenario: veriplan validates its own spec

- **WHEN** `veriplan check` is run on this change
- **THEN** it SHALL report all constraints satisfied
