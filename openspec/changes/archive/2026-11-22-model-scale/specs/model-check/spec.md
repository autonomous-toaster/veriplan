## MODIFIED Requirements

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

## ADDED Requirements

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
