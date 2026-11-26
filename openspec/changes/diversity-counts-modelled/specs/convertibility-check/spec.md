## MODIFIED Requirements

### Requirement: Check constraint diversity

T1.1 SHALL tally the modelled constraints BEFORE T1.4 SHALL pin the behaviour in tests.

The tally SHALL count only constraints that carry an LTL formula. A requirement whose body yields no
formula SHALL NOT enter the distribution, because the advisory SHALL describe what the model checks.
The INFO SHALL be emitted only when at least three modelled constraints share one category.

#### Scenario: Single-category plan

- **GIVEN** a plan where all 5 formalizable requirements are `SequentialOrder`
- **WHEN** T1.1 tallies the constraints
- **THEN** T1.4 SHALL emit info: "Constraint distribution: sequential(5). Consider adding exclusive or conditional constraints for stronger verification"

#### Scenario: An unmodelled requirement adds no diversity

- **GIVEN** a plan of three sequential requirements and one conditional body that cites no task ids
- **WHEN** T1.1 tallies the constraints
- **THEN** the distribution SHALL state `sequential(3)`
- **AND** T1.4 SHALL report `low_diversity`

#### Scenario: A modelled non-sequential constraint silences the INFO

- **GIVEN** a plan of three sequential requirements and one conditional body that cites two task ids
- **WHEN** T1.1 tallies the constraints
- **THEN** the distribution SHALL state `conditional(1)`
- **AND** T1.4 SHALL NOT report `low_diversity`

## ADDED Requirements

### Requirement: A non-sequential keyword names the argument it lacks

T1.2 SHALL name the missing argument BEFORE T1.3 SHALL rewrite the diversity hint.

A body that classifies as `Exclusive`, `Conditional` or `ConcurrentEvents` SHALL hold two task ids of
this change to be grounded. A body that cites fewer SHALL be reported as `PatternUngrounded`, and the
message SHALL name the argument the category lacks. A body with no temporal keyword SHALL keep the
generic message.

#### Scenario: A conditional body names the trigger and the consequent

- **GIVEN** the body "IF a client supplies a slug THEN Registry SHALL ignore the value" and no task id
- **WHEN** T1.2 classifies the body
- **THEN** the finding SHALL name a trigger task and a consequent task
- **AND** the finding SHALL NOT emit the generic message

#### Scenario: An exclusive body names the pair

- **GIVEN** the body "Registry SHALL persist AT MOST ONE slug source" and no task id
- **WHEN** T1.2 classifies the body
- **THEN** the finding SHALL name the pair the constraint keeps apart
- **AND** the finding SHALL NOT emit the generic message

#### Scenario: A concurrent body names two tasks of one phase

- **GIVEN** the body "Registry SHALL run CONCURRENTLY with the indexer" and no task id
- **WHEN** T1.2 classifies the body
- **THEN** the finding SHALL name two tasks of one concurrent phase
- **AND** the finding SHALL NOT emit the generic message

#### Scenario: A body with no temporal keyword keeps the generic message

- **GIVEN** the body "The system SHALL be robust" and no task id
- **WHEN** T1.2 classifies the body
- **THEN** the finding SHALL ask for a task id reference

### Requirement: The diversity hint names its precondition

T1.3 SHALL rewrite the diversity hint BEFORE T2.1 SHALL validate the change.

The fix hint of `low_diversity` SHALL cite a shape that names two task ids, for example
"T1.1 and T1.2 SHALL NOT run concurrently". The hint SHALL state that the line is advisory, and SHALL
warn that a constraint the plan does not need is worse than the line. The hint SHALL NOT suggest that
a temporal keyword alone satisfies the finding.

#### Scenario: The hint cites two task ids

- **GIVEN** a plan that reports `low_diversity`
- **WHEN** T1.3 rewrites the hint
- **THEN** the hint SHALL name two task ids
- **AND** the hint SHALL name one non-sequential shape

#### Scenario: The hint states the advisory standing

- **GIVEN** a plan that reports `low_diversity`
- **WHEN** T1.3 rewrites the hint
- **THEN** the hint SHALL state that the line is advisory
- **AND** the hint SHALL warn against a constraint the plan does not need
