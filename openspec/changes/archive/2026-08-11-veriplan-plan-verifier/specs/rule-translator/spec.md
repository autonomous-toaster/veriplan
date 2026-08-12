## ADDED Requirements

### Requirement: Map RFC 2119 strength to constraint strictness

T5.1 SHALL map RFC 2119 strength BEFORE T5.2 SHALL generate LTL. T5.1 SHALL complete AFTER T4.4 SHALL classify all requirements.

#### Scenario: MUST maps to hard
- **GIVEN** a requirement with strength `Must`
- **WHEN** T5.1 maps strictness
- **THEN** T5.1 SHALL mark it as a hard constraint — violation blocks the plan

#### Scenario: SHOULD maps to soft
- **GIVEN** a requirement with strength `Should`
- **WHEN** T5.1 maps strictness
- **THEN** T5.1 SHALL mark it as a soft constraint — violation is flagged but does not block

#### Scenario: MAY maps to informational
- **GIVEN** a requirement with strength `May`
- **WHEN** T5.1 maps strictness
- **THEN** T5.1 SHALL skip it during model checking — informational only

#### Scenario: MUST NOT maps to hard prohibition
- **GIVEN** a requirement with strength `MustNot`
- **WHEN** T5.1 maps strictness
- **THEN** T5.1 SHALL treat it as a hard constraint inverted — the condition MUST be false

### Requirement: Generate LTL for sequential order

T5.2 SHALL generate sequential LTL BEFORE T6.1 SHALL build the Promela model. T5.2 SHALL complete BEFORE T6.1 SHALL start.

#### Scenario: Task A before task B
- **GIVEN** requirement: "T1.1 SHALL complete before T1.2"
- **WHEN** T5.2 runs
- **THEN** T5.2 SHALL produce LTL: `G (active(T1.1) → done(T1.2))`

#### Scenario: Task A must finish before B starts
- **GIVEN** requirement: "T1.2 MUST NOT start until T1.1 completes"
- **WHEN** T5.2 runs
- **THEN** T5.2 SHALL produce LTL: `G (started(T1.2) → done(T1.1))`

### Requirement: Generate LTL for exclusive constraints

T5.3 SHALL generate exclusive LTL AFTER T5.2 SHALL complete sequential generation. T5.2 AND T5.3 SHALL NOT reference the same task in conflicting categories.

#### Scenario: Pairwise exclusion
- **GIVEN** requirement: "At most one of T2.1, T2.2, T2.3 SHALL be active at a time"
- **WHEN** T5.3 runs
- **THEN** T5.3 SHALL produce: `G (¬(active(2.1) ∧ active(2.2)) ∧ ¬(active(2.2) ∧ active(2.3)) ∧ ¬(active(2.1) ∧ active(2.3)))`

#### Scenario: MUST NOT concurrent
- **GIVEN** requirement: "T2.1 and T2.2 MUST NOT run concurrently"
- **WHEN** T5.3 runs
- **THEN** T5.3 SHALL produce: `G (¬(active(2.1) ∧ active(2.2)))`

### Requirement: Generate LTL for conditional constraints

T5.4 SHALL generate conditional LTL AFTER T5.2 SHALL complete sequential generation. T5.4 SHALL proceed normally even when T5.2 produces no output.

#### Scenario: Conditional rollback
- **GIVEN** requirement: "IF smoke tests fail THEN rollback SHALL trigger"
- **WHEN** T5.4 runs
- **THEN** T5.4 SHALL produce: `G (failed(1.4) → F active(1.5))`

### Requirement: Generate LTL for concurrent constraints

T5.5 SHALL generate concurrent LTL AFTER T5.3 SHALL complete exclusive LTL generation.

#### Scenario: Parallel execution
- **GIVEN** requirement: "T3.1 and T3.2 SHALL run concurrently"
- **WHEN** T5.5 runs
- **THEN** T5.5 SHALL produce: `G (active(3.1) ↔ active(3.2))`

### Requirement: Generate LTL for global invariants

T5.6 SHALL generate global LTL for any requirement that SHALL ALWAYS hold. T5.6 SHALL run AFTER T4.4 SHALL classify the requirement as Global.

#### Scenario: Always available
- **GIVEN** requirement: "Rollback SHALL be available throughout deployment"
- **WHEN** T5.6 runs
- **THEN** T5.6 SHALL produce: `G (available(rollback))`

### Requirement: Flag unverifiable requirements

T5.8 SHALL mark NonFormalizable requirements AFTER T4.4 SHALL complete classification. T5.8 SHALL complete AFTER T5.1 SHALL map RFC 2119 strength.

#### Scenario: Non-formalizable flagged
- **GIVEN** requirement: "The system SHALL be user-friendly"
- **WHEN** T4.4 runs
- **THEN** T4.4 SHALL classify as NonFormalizable
- **AND** T5.8 SHALL NOT produce an LTL formula
- **AND** T4.9 SHALL include it in the feedback report as "unverifiable — human review required"
