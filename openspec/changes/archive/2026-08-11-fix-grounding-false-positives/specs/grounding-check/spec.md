## MODIFIED Requirements

### Requirement: Ground requirement statements against Signature

T3.2 SHALL ground each requirement's SHALL statement AFTER T2.1 SHALL build the Signature. T3.2 SHALL complete BEFORE T3.4 SHALL classify temporal categories. T3.2 SHALL skip requirements with strength `May` — they are informational and do not need grounding.

#### Scenario: Explicit task IDs ground with high confidence

- **GIVEN** a requirement "T1.1 SHALL complete BEFORE T1.2" and a Signature with tasks "T1.1", "T1.2"
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Grounded`
- **AND** confidence SHALL be >= 0.8

#### Scenario: NL aliases ground with moderate confidence

- **GIVEN** a requirement "The migration SHALL complete before testing SHALL start" and a Signature where "T2.1" has alias "migration" and "T2.2" has alias "testing"
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Grounded`
- **AND** the grounded atoms SHALL reference "T2.1" and "T2.2"

#### Scenario: Vague NL with no task reference is ungroundable

- **GIVEN** a requirement "The system SHALL be user-friendly" and any Signature
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Ungroundable`
- **AND** T5.1 SHALL emit a blocking error: "Requirement '...' is ungroundable — no matching task or predicate found"

#### Scenario: Task ID present but no predicate keyword

- **GIVEN** a requirement "T6.7 MAY provide a built-in BFS explorer" and a Signature with constant "T6.7"
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Ungroundable`
- **AND** the error SHALL say "no matching predicate keyword found" rather than "no matching task or predicate found"
- **AND** the error SHALL list the valid predicate keywords: BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE

#### Scenario: MAY requirement is skipped

- **GIVEN** a requirement with strength `May` (e.g., "T6.7 MAY provide a built-in BFS explorer")
- **WHEN** T3.2 processes the requirement
- **THEN** T3.2 SHALL skip grounding for this requirement
- **AND** T3.2 SHALL NOT emit any blocker or warning for it

#### Scenario: Ambiguous match with low confidence

- **GIVEN** a requirement "The setup step SHALL complete before the migration" and a Signature where "T1.1" has alias "setup" and "T2.1" has alias "migration"
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Grounded`
- **AND** confidence SHALL be >= 0.8 (both arguments found via aliases)

### Requirement: Report grounding results in convertibility report

T5.1 SHALL include grounding results AFTER T3.2 SHALL complete. T5.1 SHALL produce rephrase directives for ungroundable requirements. T5.1 SHALL NOT include grounding results for MAY requirements — they are informational and were skipped.

#### Scenario: Ungroundable requirement produces blocker

- **GIVEN** a requirement that T3.2 marked as `Ungroundable`
- **WHEN** T5.1 produces the report
- **THEN** the report SHALL include a blocker: "Requirement '...' is ungroundable — no matching task or predicate found"
- **AND** the report SHALL include a rephrase directive: "Add a task ID reference (e.g., 'T5.1') or a known predicate keyword (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)"
- **AND** the plan SHALL be marked `blocking`

#### Scenario: Ungroundable with task ID but no predicate keyword

- **GIVEN** a requirement that T3.2 marked as `Ungroundable` because no predicate keyword matched, but task IDs are present
- **WHEN** T5.1 produces the report
- **THEN** the report SHALL include a blocker: "Requirement '...' is ungroundable — no matching predicate keyword found (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)"
- **AND** the report SHALL include a rephrase directive: "Add a temporal keyword to the requirement statement"
- **AND** the plan SHALL be marked `blocking`

#### Scenario: MAY requirement produces no grounding output

- **GIVEN** a requirement with strength `May` that T3.2 skipped
- **WHEN** T5.1 produces the report
- **THEN** the report SHALL NOT include any grounding-related items for this requirement

#### Scenario: Ambiguous requirement produces blocker by default

- **GIVEN** a requirement that T3.2 marked as `Ambiguous` (confidence < 0.8)
- **WHEN** T5.1 produces the report with default strictness
- **THEN** the report SHALL include a blocker: "Requirement '...' is ambiguous — low confidence grounding"
- **AND** the report SHALL include close match suggestions

#### Scenario: Ambiguous requirement downgraded to warning with relaxed strictness

- **GIVEN** a requirement that T3.2 marked as `Ambiguous`
- **WHEN** T5.1 produces the report with `Moderate` or `Lax` strictness
- **THEN** the report SHALL include a warning instead of a blocker
- **AND** the plan SHALL NOT be marked `blocking`

#### Scenario: All requirements grounded produces no grounding warnings

- **GIVEN** all requirements are `Grounded` with confidence >= 0.8
- **WHEN** T5.1 produces the report
- **THEN** the report SHALL NOT include any grounding-related blockers or warnings
