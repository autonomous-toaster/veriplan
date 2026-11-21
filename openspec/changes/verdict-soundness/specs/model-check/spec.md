## MODIFIED Requirements

### Requirement: Run SPIN model checker

T2.5 SHALL fail a zero-transition search BEFORE T2.6 SHALL add the calibration canary.

A search that explores no transitions on a model holding at least one proctype SHALL NOT produce a validity verdict. The result SHALL carry `valid: null` and a `skip_reason` naming the empty search.

#### Scenario: SPIN available, model valid

- **GIVEN** the `spin` binary is on PATH and the Promela model is consistent
- **WHEN** T2.5 runs the search
- **THEN** T2.5 SHALL return success for all modelled LTL properties
- **AND** the plan SHALL be marked VALID

#### Scenario: SPIN available, ordering violation

- **GIVEN** a Promela model where task 1.2 can activate before task 1.1
- **WHEN** T2.5 runs the search
- **THEN** the search SHALL detect the violation
- **AND** the counterexample trail SHALL be parsed

#### Scenario: SPIN available, deadlock detected

- **GIVEN** a Promela model where tasks form a circular dependency (1.1 to 1.2 to 1.1)
- **WHEN** T2.5 runs the search
- **THEN** the search SHALL report an invalid end state
- **AND** the plan SHALL be marked INVALID

#### Scenario: An unexplored model is not valid

- **GIVEN** a generated model holding one proctype and one constraint
- **WHEN** T2.5 runs a search that stores one state and zero transitions
- **THEN** the result SHALL carry `valid: null`
- **AND** the `skip_reason` SHALL name the empty search

#### Scenario: A searched model keeps the verdict

- **GIVEN** a generated model whose search stores more states than one
- **WHEN** T2.5 runs the search
- **THEN** the result SHALL carry a validity verdict

## ADDED Requirements

### Requirement: A constraint is modelled, unmodelled or unchecked

T2.1 SHALL mark each constraint as modelled or unmodelled BEFORE T2.2 SHALL exclude an unmodelled constraint from the satisfied count.

Every translated constraint SHALL carry a modelling status. A constraint that carries no formula SHALL be marked unmodelled. A modelled constraint whose search exceeded the time budget SHALL be marked unchecked. `total_constraints` SHALL equal the satisfied count plus the violated count plus the unmodelled count plus the unchecked count.

#### Scenario: The count is self-describing

- **GIVEN** a plan holding three constraints, of which one carries no formula
- **WHEN** T2.1 marks every constraint
- **THEN** the report SHALL state one unmodelled constraint
- **AND** the satisfied count SHALL NOT include the unmodelled constraint

#### Scenario: A modelled constraint is counted

- **GIVEN** a plan holding one sequential constraint
- **WHEN** T2.2 counts the results
- **THEN** the satisfied count SHALL include the constraint
- **AND** the unmodelled count SHALL be zero

#### Scenario: A timed-out constraint is not satisfied

- **GIVEN** a plan holding one constraint whose search exceeds the time budget
- **WHEN** T2.7 reports the search outcome
- **THEN** the report SHALL state one unchecked constraint
- **AND** the satisfied count SHALL NOT include the constraint

### Requirement: An unchecked constraint is reported

T2.7 SHALL report a search timeout as unchecked BEFORE T3.1 SHALL compute scenario-step sources against the file.

A constraint whose search exceeded the time budget SHALL be counted as unchecked and SHALL be named in the report. A run in which no constraint was satisfied and no constraint was violated SHALL NOT produce a VALID verdict. The budget SHALL apply to every backend, including the in-process one: no backend SHALL run a search without a bound.

#### Scenario: Every search timed out

- **GIVEN** a plan holding seventeen constraints where every search exceeds the time budget
- **WHEN** T2.7 reports the outcomes
- **THEN** the report SHALL state seventeen unchecked constraints
- **AND** the verdict SHALL NOT be VALID

#### Scenario: The in-process backend is bounded

- **GIVEN** the in-process backend and a plan whose state space does not close within the budget
- **WHEN** T2.7 runs the search
- **THEN** the search SHALL stop at the budget
- **AND** the constraint SHALL be counted as unchecked
- **AND** the verdict SHALL NOT be VALID

#### Scenario: One timeout among satisfied constraints

- **GIVEN** a plan holding three constraints where one search exceeds the time budget
- **WHEN** T2.7 reports the outcomes
- **THEN** the report SHALL state one unchecked constraint
- **AND** the verdict SHALL name the unchecked count
