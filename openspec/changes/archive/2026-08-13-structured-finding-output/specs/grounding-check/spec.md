## MODIFIED Requirements

### Requirement: Report grounding results in convertibility report

T4.2 SHALL attach `kind`, `severity`, `location`, `message`, and `fixability` to each grounding result AFTER T3.2 SHALL complete grounding.

#### Scenario: Ungroundable requirement produces blocker

- **GIVEN** a requirement that T3.2 marked as `Ungroundable`
- **WHEN** T4.2 attaches Finding metadata
- **THEN** the report SHALL include a Finding with `kind` for the ungroundable case
- **AND** the report SHALL include a suggestion naming valid task IDs or predicate keywords
- **AND** the plan SHALL be marked `blocking`

#### Scenario: Ambiguous requirement produces blocker by default

- **GIVEN** a requirement that T3.2 marked as `Ambiguous` (confidence < 0.8)
- **WHEN** T4.2 attaches Finding metadata with default strictness
- **THEN** the report SHALL include a Finding with `kind` for the ambiguous case
- **AND** the report SHALL include close match suggestions

#### Scenario: Ambiguous requirement downgraded to warning with relaxed strictness

- **GIVEN** a requirement that T3.2 marked as `Ambiguous`
- **WHEN** veriplan produces the report with `Moderate` or `Lax` strictness
- **THEN** the report SHALL include a warning instead of a blocker
- **AND** the plan SHALL NOT be marked `blocking`

#### Scenario: All requirements grounded produces no grounding warnings

- **GIVEN** all requirements are `Grounded` with confidence >= 0.8
- **WHEN** veriplan produces the report
- **THEN** the report SHALL NOT include any grounding-related blockers or warnings

## ADDED Requirements

### Requirement: Multi-keyword grounding carries a split replacement

T4.2 SHALL emit a `grounding_multi_keyword` Finding with `op: split_requirement` AFTER T3.2 SHALL classify a requirement as matching multiple temporal keywords.

#### Scenario: Multi-keyword finding proposes a split

- **GIVEN** a requirement body matching both AFTER and BEFORE, containing two constraint clauses
- **WHEN** T4.2 produces the report
- **THEN** the Finding SHALL have `op: split_requirement`
- **AND** `replacement` SHALL contain two split bodies, one per constraint clause
- **AND** the Finding SHALL be marked `structural` (not auto-applied by `--fix`)
