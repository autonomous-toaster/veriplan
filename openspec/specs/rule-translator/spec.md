# Rule Translator

## Purpose

Translate classified requirements into LTL (Linear Temporal Logic) formulas for model checking. This is Phase 2 of the verification pipeline — it runs after convertibility checking and before model checking.

## Requirements

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

T1.1 SHALL test sequential order BEFORE T1.2 SHALL narrow the time reference.

`classify` SHALL test `is_sequential` before `is_fixed_time`. A requirement body that cites a task id and contains ` before `, ` after `, `complete before` or `only after` SHALL classify as `SequentialOrder`. The generated formula SHALL be `[](active_<later> -> done_<earlier>)`, with the earlier task taken from statement position.

#### Scenario: Task A before task B

- **GIVEN** requirement: "T1.1 SHALL complete before T1.2"
- **WHEN** T1.1 tests the classification of the requirement
- **THEN** T1.1 SHALL produce LTL: `[] (active_t1_2 -> done_t1_1)`

#### Scenario: Task A must finish before B starts

- **GIVEN** requirement: "T1.2 MUST NOT start until T1.1 completes"
- **WHEN** T1.1 tests the classification of the requirement
- **THEN** T1.1 SHALL produce LTL: `[] (active_t1_2 -> done_t1_1)`

#### Scenario: A task-citing ordering statement reaches the sequential arm

- **GIVEN** a body that cites two task ids and uses BEFORE
- **WHEN** T1.1 tests the classification of the body
- **THEN** the category SHALL be `SequentialOrder`
- **AND** the formula SHALL reference an `active_` atom and a `done_` atom

#### Scenario: Fixed time no longer shadows sequential order

- **GIVEN** a body that cites two task ids
- **WHEN** T1.1 tests the classification of the body
- **THEN** the category SHALL NOT be `FixedTime` on the strength of a digit in a task id

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

T1.4 SHALL remove the placeholder formula BEFORE T1.5 SHALL reject a self-referential relation.

A category without an encoding SHALL produce no formula. `FixedTime` and `Global` SHALL NOT emit a placeholder formula. The constraint SHALL carry the reason the category is not modelled, and the reason SHALL reach the report.

#### Scenario: Always available

- **GIVEN** requirement: "Rollback SHALL be available throughout deployment"
- **WHEN** T1.4 translates the requirement
- **THEN** T1.4 SHALL produce no formula
- **AND** the constraint SHALL carry a reason naming the absent encoding

#### Scenario: An unencoded category is reported

- **GIVEN** a plan holding one `FixedTime` constraint
- **WHEN** T1.4 translates the plan
- **THEN** the report SHALL name the constraint as unmodelled
- **AND** the constraint SHALL NOT appear in the satisfied count

### Requirement: Flag unverifiable requirements

T5.8 SHALL mark NonFormalizable requirements AFTER T4.4 SHALL complete classification. T5.8 SHALL complete AFTER T5.1 SHALL map RFC 2119 strength.

#### Scenario: Non-formalizable flagged

- **GIVEN** requirement: "The system SHALL be user-friendly"
- **WHEN** T4.4 runs
- **THEN** T4.4 SHALL classify as NonFormalizable
- **AND** T5.8 SHALL NOT produce an LTL formula
- **AND** T4.9 SHALL include it in the feedback report as "unverifiable — human review required"

### Requirement: Classify a time reference as a clock

T1.2 SHALL narrow the time reference BEFORE T1.3 SHALL add the seam regression test.

`is_time_ref` SHALL match a clock literal or a duration literal. A bare digit SHALL NOT count as a time reference, so a task id alone SHALL NOT satisfy the time clause of `is_fixed_time`.

#### Scenario: A clock literal is a time reference

- **GIVEN** a body containing `09:30` or `2 minutes`
- **WHEN** T1.2 tests the time reference of the body
- **THEN** T1.2 SHALL match the time reference

#### Scenario: A task id is not a time reference

- **GIVEN** a body containing `T4.7` and no clock or duration literal
- **WHEN** T1.2 tests the time reference of the body
- **THEN** T1.2 SHALL NOT match the time reference

### Requirement: Reject a self-referential temporal relation

T1.5 SHALL reject a self-referential relation BEFORE T2.1 SHALL mark each constraint as modelled or unmodelled.

A temporal relation whose two argument slots resolve to one task SHALL NOT be accepted. The relation SHALL be reported, and no formula SHALL be emitted for the relation.

#### Scenario: One task ordered against itself

- **GIVEN** a body reading "T1.1 SHALL write the header BEFORE T1.1 SHALL seal it"
- **WHEN** T1.5 resolves the relation arguments
- **THEN** T1.5 SHALL reject the relation
- **AND** the finding SHALL name the repeated task id

### Requirement: Report a requirement body with more than one constraint clause

T1.6 SHALL report an unmodelled clause BEFORE T2.1 SHALL mark each constraint as modelled or unmodelled.

A constraint clause is a sentence that begins with a task id reference. A body holding more than one clause SHALL produce a finding that names each clause and states which clause was modelled. The second clause SHALL NOT be dropped without a finding.

#### Scenario: A second clause is named

- **GIVEN** a body holding two sentences that each begin with a task id
- **WHEN** T1.6 inspects the body
- **THEN** the finding SHALL name both clauses
- **AND** the finding SHALL state that only the first clause was modelled

#### Scenario: A single clause produces no finding

- **GIVEN** a body holding one sentence that begins with a task id and one explanatory sentence
- **WHEN** T1.6 inspects the body
- **THEN** the body SHALL produce no finding
