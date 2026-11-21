# Model Check

## Purpose

Translate PlanIR into Promela models and run SPIN model checker to verify LTL properties. This is Phase 3 of the verification pipeline — it runs after rule translation and produces annotated counterexamples for the reporter.

## Requirements

### Requirement: Generate Promela model from PlanIR

T6.1 SHALL generate the Promela model BEFORE T6.4 SHALL run SPIN. T6.1 SHALL complete BEFORE T6.3 SHALL generate LTL properties.

#### Scenario: Single-phase Promela model

- **GIVEN** a PlanIR with 3 tasks (1.1, 1.2, 1.3) in one phase and one sequential constraint "T1.1 SHALL complete before T1.2"
- **WHEN** T6.1 runs
- **THEN** T6.1 SHALL produce a Promela file with:
  - `bool t1_1, t1_2, t1_3;` (state variables)
  - `active proctype task_t1_1()` with `do :: (1) -> t1_1 = done; break od`
  - `ltl p0 { G (active(t1_2) -> done(t1_1)) }` (sequential constraint)

#### Scenario: Multi-phase model with ordering

- **GIVEN** a PlanIR with Phase 1 (tasks 1.1-1.3) and Phase 2 (tasks 2.1-2.3) where T2.1 SHALL complete AFTER all Phase 1 tasks
- **WHEN** T6.1 runs
- **THEN** T6.2 SHALL add a guard: Phase 2 tasks can only activate AFTER all of t1_1, t1_2, t1_3 are done

#### Scenario: Exclusive constraint in Promela

- **GIVEN** exclusive constraint "At most one of T2.1 and T2.2 SHALL be active"
- **WHEN** T6.1 runs
- **THEN** T6.3 SHALL include: `ltl p1 { G ( !(t2_1 == 1 && t2_2 == 1) ) }`

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
