## MODIFIED Requirements

### Requirement: Generate human-readable and JSON report

T5.3 SHALL generate the human-readable report AFTER T5.1 SHALL project all findings into one array.

#### Scenario: VALID report

- **GIVEN** a plan where all LTL properties pass
- **WHEN** T5.2 emits the report
- **THEN** T5.2 SHALL produce JSON with `{"plan": "veriplan-plan-verifier", "valid": true, "findings": []}`

#### Scenario: INVALID report with violations

- **GIVEN** a plan with 2 ordering violations and 1 exclusive violation
- **WHEN** T4.3 attaches Finding metadata
- **THEN** T4.3 SHALL emit each violation as a `Finding` with a suggested fix
- **AND** T5.2 SHALL produce JSON with all violations as `Finding`s in the `findings` array

#### Scenario: Blocking convertibility

- **GIVEN** a plan that failed convertibility check
- **WHEN** T5.3 emits the human report
- **THEN** T5.3 SHALL show the blockers as grouped `Finding`s per the output-contract grouping rule

#### Scenario: Global violation carries a suggested fix

- **GIVEN** a plan with a global-invariant violation
- **WHEN** T2.1 adds the fix arm
- **THEN** the `violation_global` Finding SHALL include a `suggestion`
- **AND** SHALL NOT be emitted without any suggested fix

#### Scenario: Fixed-time violation carries a suggested fix

- **GIVEN** a plan with a fixed-time-block violation
- **WHEN** T2.1 adds the fix arm
- **THEN** the `violation_fixed_time` Finding SHALL include a `suggestion`

## ADDED Requirements

### Requirement: Model-check violations attach to the Finding shape

T4.3 SHALL attach `kind`, `severity`, `location`, `message`, and `fixability` to each violation AFTER T1.1 SHALL define the `Finding` shape.

#### Scenario: Violation emits as a Finding

- **GIVEN** a plan with a sequential-order violation
- **WHEN** T4.3 attaches Finding metadata
- **THEN** the violation SHALL be a `Finding` with a `kind`, `severity`, `location`, and `fixability` tier
