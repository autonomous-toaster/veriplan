## MODIFIED Requirements

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

## ADDED Requirements

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
