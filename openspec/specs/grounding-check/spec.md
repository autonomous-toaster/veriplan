# Grounding Check

## Purpose

Verify that each requirement's SHALL statement references known task IDs using known predicate keywords. This is a spec quality gate that runs before temporal classification — it catches vague NL references like "the migration step" instead of explicit task IDs like "T2.1".

## Requirements

### Requirement: Build Signature from PlanIR

T2.1 SHALL build a Signature from PlanIR tasks BEFORE T3.2 SHALL ground requirement statements. T2.1 SHALL complete BEFORE T3.2 SHALL run.

#### Scenario: Signature includes all tasks

- **GIVEN** a PlanIR with tasks "1.1" (description "Create project"), "1.2" (description "Add deps")
- **WHEN** T2.1 builds the Signature
- **THEN** the Signature SHALL contain constants "T1.1" and "T1.2"
- **AND** "T1.1" SHALL have aliases including "create project"
- **AND** "T1.2" SHALL have aliases including "add deps"

#### Scenario: Signature includes predicate definitions

- **GIVEN** a PlanIR with any tasks
- **WHEN** T2.1 builds the Signature
- **THEN** the Signature SHALL contain predicates: BEFORE, AFTER, CONCURRENTLY, IF_THEN, ALWAYS, AT_MOST_ONE
- **AND** each predicate SHALL have the correct argument slots

#### Scenario: Empty plan produces empty signature

- **GIVEN** a PlanIR with zero tasks
- **WHEN** T2.1 builds the Signature
- **THEN** the Signature SHALL have zero constants
- **AND** the Signature SHALL still include all 6 predicate definitions

### Requirement: Ground requirement statements against Signature

T3.2 SHALL ground each requirement's SHALL statement AFTER T2.1 SHALL build the Signature. T3.2 SHALL complete BEFORE T3.4 SHALL classify temporal categories.

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

#### Scenario: Ambiguous match with low confidence

- **GIVEN** a requirement "The setup step SHALL complete before the migration" and a Signature where "T1.1" has alias "setup" and "T2.1" has alias "migration"
- **WHEN** T3.2 grounds the statement
- **THEN** the grounding result SHALL have status `Grounded`
- **AND** confidence SHALL be >= 0.8 (both arguments found via aliases)

### Requirement: Report grounding results in convertibility report

T5.1 SHALL include grounding results AFTER T3.2 SHALL complete. T5.1 SHALL produce rephrase directives for ungroundable requirements.

#### Scenario: Ungroundable requirement produces blocker

- **GIVEN** a requirement that T3.2 marked as `Ungroundable`
- **WHEN** T5.1 produces the report
- **THEN** the report SHALL include a blocker: "Requirement '...' is ungroundable — no matching task or predicate found"
- **AND** the report SHALL include a rephrase directive: "Add a task ID reference (e.g., 'T5.1') or a known predicate keyword (BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE)"
- **AND** the plan SHALL be marked `blocking`

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
