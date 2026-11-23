## MODIFIED Requirements

### Requirement: Generate LTL for sequential order

T1.1 SHALL match a task-prefixed id beside a sequential keyword BEFORE T1.2 SHALL return the earlier task first for both keywords.

`classify` SHALL test `is_sequential` before `is_fixed_time`. A requirement body that cites a task
id and contains ` before `, ` after `, `complete before` or `only after` SHALL classify as
`SequentialOrder`. The generated formula SHALL be `[](active_<later> -> done_<earlier>)`. The
earlier task SHALL come from the keyword: a BEFORE body names the earlier task first, and an AFTER
body names the earlier task second. Statement position alone SHALL NOT decide the direction.

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

#### Scenario: A grammar-shaped body reaches the pair finder

- **GIVEN** the body "T1.1 SHALL complete BEFORE T2.1 SHALL complete"
- **WHEN** T1.1 matches the keyword beside the task id
- **THEN** the pair SHALL be found
- **AND** the earlier task SHALL be T1.1

#### Scenario: The AFTER body names the earlier task second

- **GIVEN** the body "T2.1 SHALL complete AFTER T1.1 SHALL complete"
- **WHEN** T1.2 returns the pair
- **THEN** the earlier task SHALL be T1.1
- **AND** the formula SHALL be `[] (active_t2_1 -> done_t1_1)`

## ADDED Requirements

### Requirement: The positional fallback follows the keyword

T1.2 SHALL return the earlier task first for both keywords BEFORE T1.3 SHALL gate the positional fallback on the keyword.

When the pair finder cannot name a pair, the fallback SHALL keep statement position for a BEFORE
body and SHALL reverse statement position for an AFTER body. A body that carries no sequential
keyword SHALL NOT reach the sequential arm.

#### Scenario: An AFTER body with three cited tasks reverses the fallback

- **GIVEN** the body "T3.1 SHALL complete AFTER T1.1 SHALL complete BEFORE T2.1 SHALL complete"
- **WHEN** T1.3 resolves the fallback direction
- **THEN** the earlier task SHALL NOT be the first cited task

#### Scenario: A BEFORE body keeps the fallback direction

- **GIVEN** the body "T1.1 SHALL complete BEFORE T2.1 SHALL complete BEFORE T3.1 SHALL complete"
- **WHEN** T1.3 resolves the fallback direction
- **THEN** the earlier task SHALL be the first cited task

### Requirement: The two unit tests use the requirement grammar

T2.1 SHALL rewrite the two pair-finder tests with grammar-shaped bodies BEFORE T2.2 SHALL assert that the two orderings produce one formula.

A pair-finder test SHALL use a body that the requirement grammar permits, with a `T`-prefixed id
and an RFC 2119 keyword. A test over a sentence the grammar forbids SHALL NOT stand in for a body
the product produces.

#### Scenario: The pair-finder tests cite task-prefixed ids

- **GIVEN** the pair-finder suite
- **WHEN** T2.1 rewrites the two tests
- **THEN** each test body SHALL carry a `T` prefix
- **AND** each test body SHALL carry an RFC 2119 keyword

### Requirement: A BEFORE body and the AFTER twin produce one formula

T2.2 SHALL assert that the two orderings produce one formula BEFORE T3.1 SHALL re-run the three-run fixture.

Two bodies that state one ordering SHALL translate to one formula. The fixture SHALL record three
verdicts: a BEFORE body over a forward chain, the AFTER twin over a forward chain, and the AFTER
twin over a reversed chain. The first two SHALL be VALID and the third SHALL be INVALID.

#### Scenario: The two twins agree

- **GIVEN** the bodies "T1.1 SHALL complete BEFORE T2.1 SHALL complete" and "T2.1 SHALL complete AFTER T1.1 SHALL complete"
- **WHEN** T2.2 translates both bodies
- **THEN** both bodies SHALL produce one formula

#### Scenario: The reversed chain fails the AFTER twin

- **GIVEN** the AFTER twin over a chain that runs T2.1 then T1.1
- **WHEN** T3.1 re-runs the fixture
- **THEN** the verdict SHALL be INVALID
