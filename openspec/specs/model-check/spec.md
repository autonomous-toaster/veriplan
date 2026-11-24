# Model Check

## Purpose

Translate PlanIR into Promela models and run SPIN model checker to verify LTL properties. This is Phase 3 of the verification pipeline — it runs after rule translation and produces annotated counterexamples for the reporter.

## Requirements

### Requirement: Generate Promela model from PlanIR

T1.1 SHALL emit the completion flag last BEFORE T1.2 SHALL place the completion flag past the failure block.

A task's completion flag SHALL be emitted after the task is complete: after the deactivation
statement, and after the failure block when a failure block is emitted. A successor's guard reads
that flag, so publishing the flag earlier lets the successor interleave with a predecessor that has
not finished.

#### Scenario: Single-phase Promela model

- **GIVEN** a PlanIR with three tasks (1.1, 1.2, 1.3) in one phase and one sequential constraint "T1.1 SHALL complete before T1.2"
- **WHEN** T1.1 runs
- **THEN** T1.1 SHALL produce a Promela file with the three state variables
- **AND** each proctype body SHALL activate, deactivate, then publish completion
- **AND** the file SHALL carry one LTL claim per constraint

#### Scenario: Multi-phase model with ordering

- **GIVEN** a PlanIR with Phase 1 (tasks 1.1 to 1.3) and Phase 2 (tasks 2.1 to 2.3)
- **WHEN** T1.1 runs
- **THEN** T1.1 SHALL guard the first task of Phase 2 on the completion of the last task of Phase 1

#### Scenario: Exclusive constraint in Promela

- **GIVEN** an exclusive constraint over two tasks
- **WHEN** T1.1 runs
- **THEN** the generated model SHALL carry a claim of the form "not both active"

#### Scenario: The failure status is decided before completion is published

- **GIVEN** a plan holding one conditional constraint over a failure flag
- **WHEN** T1.2 emits the body of the task that carries the flag
- **THEN** the failure block SHALL be emitted before the completion flag

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

### Requirement: Built-in BFS fallback (Deprecated)

T6.7 MAY provide a built-in BFS explorer for plans with ≤20 tasks. (Deprecated: BFS fallback was removed — T6.4 fails hard when SPIN is missing.)

#### Scenario: SPIN not available, model valid

- **GIVEN** `spin` binary is not installed and a plan with 4 tasks and 2 LTL properties
- **WHEN** T6.4 fails to find SPIN
- **THEN** T6.4 SHALL emit a hard error: "SPIN binary not found on PATH"
- **AND** the plan SHALL be marked NOT CONVERTIBLE

### Requirement: Annotate counterexample with source locations

T7.1 SHALL map each counterexample to source locations AFTER T6.6 SHALL parse the trail file. T7.1 SHALL complete BEFORE T7.2 SHALL map each violating state.

#### Scenario: Ordering violation annotated

- **GIVEN** a SPIN trail showing task 1.2 active before task 1.1 done
- **WHEN** T7.1 runs
- **THEN** T7.1 SHALL produce:
  - Violated requirement: `specs/deploy/spec.md:16` (sequential constraint)
  - Violating task: `tasks.md:13` (task 1.2)
  - State: `t1_1=0, t1_2=1`

#### Scenario: Multiple violations

- **GIVEN** a plan with a sequential violation AND an exclusive constraint violation
- **WHEN** T7.1 runs
- **THEN** T7.1 SHALL list both violations with source locations for each

### Requirement: Generate human-readable and JSON report

T5.3 SHALL generate the human-readable report AFTER T5.1 SHALL project all findings into one array.

#### Scenario: VALID report

- **GIVEN** a plan where all LTL properties pass
- **WHEN** T5.2 emits the report
- **THEN** T5.2 SHALL produce JSON with `{"plan": "veriplan-plan-verifier", "valid": true, "findings": []}`

#### Scenario: INVALID report with violations

- **GIVEN** a plan with 2 ordering violations and 1 exclusive violation
- **WHEN** T4.3 attaches Finding metadata
- **THEN** T4.3 SHALL emit each violation as a `Finding` with a suggested fix
- **AND** T5.2 SHALL produce JSON with all violations as `Finding`s in the `findings` array

#### Scenario: Blocking convertibility

- **GIVEN** a plan that failed convertibility check
- **WHEN** T5.3 emits the human report
- **THEN** T5.3 SHALL show the blockers as grouped `Finding`s per the output-contract grouping rule

#### Scenario: Global violation carries a suggested fix

- **GIVEN** a plan with a global-invariant violation
- **WHEN** T2.1 adds the fix arm
- **THEN** the `violation_global` Finding SHALL include a `suggestion`
- **AND** SHALL NOT be emitted without any suggested fix

#### Scenario: Fixed-time violation carries a suggested fix

- **GIVEN** a plan with a fixed-time-block violation
- **WHEN** T2.1 adds the fix arm
- **THEN** the `violation_fixed_time` Finding SHALL include a `suggestion`

### Requirement: Model-check violations attach to the Finding shape

T4.3 SHALL attach `kind`, `severity`, `location`, `message`, and `fixability` to each violation AFTER T1.1 SHALL define the `Finding` shape.

#### Scenario: Violation emits as a Finding

- **GIVEN** a plan with a sequential-order violation
- **WHEN** T4.3 attaches Finding metadata
- **THEN** the violation SHALL be a `Finding` with a `kind`, `severity`, `location`, and `fixability` tier

### Requirement: A constraint is modelled, unmodelled or unchecked

T2.4 SHALL count a historical constraint separately BEFORE T4.1 SHALL run the strict validation.

Every translated constraint SHALL carry a modelling status: modelled, unmodelled, unchecked or
historical. A constraint whose arguments all belong to history SHALL be marked historical and SHALL
carry no formula. `total_constraints` SHALL equal the satisfied count plus the violated count plus the
unmodelled count plus the unchecked count plus the historical count.

#### Scenario: The count is self-describing

- **GIVEN** a plan holding three constraints, of which one carries no formula
- **WHEN** T2.4 marks every constraint
- **THEN** the report SHALL state one unmodelled constraint
- **AND** the satisfied count SHALL NOT include the unmodelled constraint

#### Scenario: A modelled constraint is counted

- **GIVEN** a plan holding one sequential constraint between live tasks
- **WHEN** T2.4 counts the results
- **THEN** the satisfied count SHALL include the constraint
- **AND** the unmodelled count SHALL be zero

#### Scenario: A timed-out constraint is not satisfied

- **GIVEN** a plan holding one constraint whose search exceeds the time budget
- **WHEN** T2.4 reports the search outcome
- **THEN** the report SHALL state one unchecked constraint
- **AND** the satisfied count SHALL NOT include the constraint

#### Scenario: A historical constraint is a fourth count

- **GIVEN** a plan holding one constraint whose arguments all belong to history
- **WHEN** T2.4 counts the results
- **THEN** the report SHALL state one historical constraint
- **AND** the total SHALL equal the sum of the satisfied, violated, unmodelled, unchecked and historical counts
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

### Requirement: A successor starts only after its predecessor finished

T1.3 SHALL assert the predecessor guard BEFORE T1.4 SHALL assert the linear state space.

Two tasks chained in a sequential phase SHALL NOT be active at the same time. The completion flag
that unblocks the successor SHALL be published after the predecessor deactivates, so no state exists
in which both tasks are active.

#### Scenario: The two activations do not overlap

- **GIVEN** two tasks chained in one sequential phase
- **WHEN** T1.3 explores the generated model
- **THEN** no reachable state SHALL hold both activation flags set
- **AND** the assertion SHALL fail against a generator that publishes completion early

### Requirement: A sequential plan has a linear state space

T1.4 SHALL assert the linear state space BEFORE T2.1 SHALL update the exclusive fixture.

The state space of a generated model SHALL grow linearly in the number of tasks when the plan is a
linear chain. A regression test SHALL assert an upper bound on that state count, measured by an
engine-independent oracle rather than by the in-process engine, so an engine regression cannot fail
a test about the model.

#### Scenario: Twenty tasks stay under a thousand states

- **GIVEN** a plan of twenty tasks in one sequential phase with one ordering constraint
- **WHEN** T1.4 runs the model check
- **THEN** the search SHALL explore fewer than one thousand states

#### Scenario: A reopened window fails the assertion

- **GIVEN** a generator that publishes the completion flag before the deactivation
- **WHEN** T1.4 runs the same twenty-task plan
- **THEN** the assertion SHALL fail

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
