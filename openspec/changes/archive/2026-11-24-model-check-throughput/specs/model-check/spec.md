# Spec Delta

## ADDED Requirements

### Requirement: A failure block is emitted only for a task a claim reads

T1.1 SHALL collect the failure atoms a formula names BEFORE T1.2 SHALL emit a failure flag.

The generator SHALL emit a task's failure flag and the nondeterministic failure block only when some constraint's formula names that task's atom. The decision SHALL be a walk over the formula's atoms, not a search of the rendered formula. A task no formula names SHALL carry neither the flag nor the block.

#### Scenario: An unreferenced task carries neither

- **GIVEN** a plan of three tasks and one constraint that names the failure atom of the second
- **WHEN** T1.2 emits the model
- **THEN** the model SHALL carry a failure flag for the second task
- **AND** the model SHALL carry no failure flag for the first or the third

#### Scenario: A failure atom is not matched as a prefix

- **GIVEN** a plan holding tasks 1.1 and 1.10, and a constraint naming only the failure atom of 1.10
- **WHEN** T1.1 collects the atoms
- **THEN** task 1.1 SHALL NOT be named
- **AND** task 1.10 SHALL be named

#### Scenario: A referenced task keeps its failure block

- **GIVEN** a plan holding one conditional constraint over a failure flag
- **WHEN** T1.2 emits the body of the task that carries the flag
- **THEN** the failure block SHALL be emitted before the completion flag
- **AND** the search SHALL still refute a plan that does not recover

### Requirement: The state count does not follow the unnamed tasks

T1.2 SHALL emit the flags per task BEFORE T1.3 SHALL measure the state count.

The state count of a generated model SHALL grow with the tasks a claim names and SHALL NOT grow with the tasks a claim does not name. The measurement SHALL use a claim that holds in every reachable state, because a claim false near the root stops the search early and measures refutation depth instead of the space.

#### Scenario: Sixteen unnamed tasks are nearly free

- **GIVEN** an eight-task chain and a twenty-four-task chain, each naming the same one failure flag
- **WHEN** T1.3 measures the state count of each
- **THEN** the twenty-four-task count SHALL stay under four times the eight-task count

#### Scenario: A refuting claim does not stand in for a count probe

- **GIVEN** two claims of the same shape, one false near the root and one holding in every reachable state
- **WHEN** T1.3 measures a state count
- **THEN** the refuting claim SHALL be reported as violated
- **AND** the closing claim SHALL be reported as holding
- **AND** the count fixture SHALL be the closing claim

### Requirement: A stopped search reports its own cause and its own cost

T2.1 SHALL name the cause of a stopped search BEFORE T2.2 SHALL state the states that claim used.

A claim the external backend stopped on the wall-clock budget SHALL NOT carry the state-budget reason. The claim SHALL carry a reason naming the wall-clock stop, the time allowed and the time used. A claim the in-process backend stopped SHALL carry the states used and the states allowed.

#### Scenario: A wall-clock stop is not a state-budget stop

- **GIVEN** a plan whose search the external backend stops at the wall-clock budget
- **WHEN** T2.1 reports the outcome
- **THEN** the claim SHALL carry a wall-clock reason
- **AND** the claim SHALL NOT carry the state-budget reason

#### Scenario: A stopped claim reports the time it was allowed

- **GIVEN** a plan holding three constraints where one search exceeds the wall-clock budget
- **WHEN** T2.1 reports the outcome
- **THEN** the report SHALL state the time allowed for that claim
- **AND** the report SHALL state the time the claim used

#### Scenario: The engine's per-claim numbers reach the report

- **GIVEN** the in-process backend stopping a named claim at the share given to that claim
- **WHEN** T2.2 relays the outcome
- **THEN** the summary SHALL state the claim name, the states used and the states allowed
- **AND** the JSON output SHALL carry both numbers for that claim

### Requirement: A budget the caller set is honoured or refused

T2.2 SHALL report the share the engine received BEFORE T2.3 SHALL refuse an unusable budget.

A state budget the caller set SHALL be used as set, or refused with a message naming the value. A budget of zero, or a value that is not a count, SHALL NOT be replaced by a positive default. The report SHALL NOT state a run total lower than the states explored.

#### Scenario: Zero is refused, not raised

- **GIVEN** a caller setting the state budget to zero
- **WHEN** T2.3 reads the budget
- **THEN** the run SHALL state that the budget cannot be honoured
- **AND** the run SHALL NOT explore with a positive budget the caller did not ask for

#### Scenario: A typo is refused, not replaced

- **GIVEN** a caller setting the state budget to a value that is not a count
- **WHEN** T2.3 reads the budget
- **THEN** the run SHALL name the unreadable value
- **AND** the run SHALL NOT fall back to the default silently

#### Scenario: The share is reported, not a total

- **GIVEN** a caller setting a budget below the number of claims
- **WHEN** T2.2 reports a stopped claim
- **THEN** the report SHALL state the share that claim was given
- **AND** the report SHALL NOT state a run total lower than the states explored

### Requirement: The validation runs before the plan check

T5.1 SHALL complete BEFORE T5.2 SHALL run.

The change SHALL be validated by the specification gate and then by the plan check, in that order, and both SHALL be clean before the change lands.

#### Scenario: The specification gate runs first

- **GIVEN** a change ready to validate
- **WHEN** T5.1 runs the strict validation
- **THEN** the validation SHALL report no warning
- **AND** the change SHALL then be checked by the plan gate

#### Scenario: Both backends agree

- **GIVEN** a change whose constraints are all modelled
- **WHEN** T5.2 runs the plan check against each backend
- **THEN** both backends SHALL report the same verdict
- **AND** both SHALL report the same violated count

### Requirement: The corpus scale is re-measured before it is recorded

T5.2 SHALL complete BEFORE T5.3 SHALL re-measure the corpus scale.

Every state count and every elapsed time recorded in the change SHALL come from a fresh run, and SHALL name the model shape and the backend that produced the number.

#### Scenario: The recorded numbers are fresh

- **GIVEN** a change carrying measurements taken before the fix of the change
- **WHEN** T5.3 rewrites them
- **THEN** every number SHALL come from a run made after the fix
- **AND** the record SHALL name the model shape the number was taken from

#### Scenario: The requirement left unsatisfied is recorded

- **GIVEN** a requirement this change does not make true
- **WHEN** T5.3 records the outcome
- **THEN** the record SHALL name the requirement
- **AND** the record SHALL name the engine issue that closes the requirement
