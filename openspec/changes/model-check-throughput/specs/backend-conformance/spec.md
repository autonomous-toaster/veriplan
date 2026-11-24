# Spec Delta

## ADDED Requirements

### Requirement: A stopped backend is not a conforming backend

T3.1 SHALL exclude an undecided claim from the tally BEFORE T3.2 SHALL name the backend that stopped.

A claim SHALL count as agreeing only when both backends decided it. A claim either backend left undecided SHALL be reported as not compared, and SHALL NOT be counted as a match or as a mismatch. The comparison SHALL name the backend that stopped, the claim the backend stopped on, and the cause.

#### Scenario: An undecided claim is not agreement

- **GIVEN** a claim the external backend decided and the in-process backend left undecided
- **WHEN** T3.1 builds the comparison
- **THEN** the claim SHALL NOT be counted as matching
- **AND** the comparison SHALL state how many claims were not compared

#### Scenario: The stopped backend is named

- **GIVEN** a comparison in which one backend stopped at the budget
- **WHEN** T3.2 prints the comparison
- **THEN** the output SHALL name that backend
- **AND** the output SHALL name the claim the backend stopped on and the cause

### Requirement: The in-process engine carries a pinned gate

T4.1 SHALL pin the in-process engine's state count BEFORE T4.2 SHALL fail the gate on the exponential shape.

A regression test SHALL assert the state count and the elapsed time of the in-process engine on a generated plan of fixed shape. The test SHALL be separate from the engine-independent state-space bound, so an engine regression fails a test about the engine and not a test about the model.

#### Scenario: The gate pins both numbers

- **GIVEN** a generated chain of twenty tasks
- **WHEN** T4.1 runs the gate
- **THEN** the test SHALL assert a state count
- **AND** the test SHALL assert an elapsed time

### Requirement: The gate measures a claim blind to a flag

T4.3 SHALL record the liveness product BEFORE T4.4 SHALL measure a blind claim.

A model SHALL be measured in which a failure flag exists because one claim names the flag, under a different claim that cannot see the flag. A single-claim model SHALL NOT stand in for this model, because the generator emits a flag only when some claim names the flag, so a flag no claim can see does not exist in a model carrying one claim.

#### Scenario: A blind claim is measured

- **GIVEN** a chain whose first two tasks carry a failure flag, under a claim that names neither
- **WHEN** T4.4 measures the product
- **THEN** the record SHALL hold the exact state count for each task count
- **AND** the record SHALL state that a single-claim liveness fixture cannot measure a blind claim

#### Scenario: A single-claim model cannot show the projection

- **GIVEN** a model carrying one claim that names the failure flag
- **WHEN** T4.4 is considered for that model
- **THEN** the claim SHALL see every flag the model carries
- **AND** the record SHALL NOT present the model as a measurement of the projection

### Requirement: The gate measures the liveness path too

T4.1 SHALL pin the safety count BEFORE T4.3 SHALL measure the liveness product.

A model whose only claims carry a liveness operator SHALL be measured by the gate as well as a model whose claims are safety claims. The gate SHALL record the exact count for each task count, not an upper bound.

#### Scenario: The liveness product is measured

- **GIVEN** a chain whose only claim carries a liveness operator
- **WHEN** T4.3 runs the gate
- **THEN** the test SHALL assert a state count for the liveness product
- **AND** the test SHALL assert an elapsed time for the liveness product

#### Scenario: The liveness product is recorded exactly

- **GIVEN** an engine that projects safety claims and not liveness claims
- **WHEN** T4.3 records the product
- **THEN** the record SHALL hold the exact state count for each task count
- **AND** a change in the count in either direction SHALL fail the gate

#### Scenario: The invisible bit is priced

- **GIVEN** two chains that differ only by the failure flag, the branch under the flag, and a claim that names the flag
- **WHEN** T4.3 measures both
- **THEN** the chain holding the flag SHALL explore twice the states of the chain without the flag
- **AND** the test SHALL state that the ratio reaching one is the signal that the engine projects liveness claims

#### Scenario: The gate fails on the exponential shape

- **GIVEN** a model carrying a failure branch on every task
- **WHEN** T4.2 runs the gate
- **THEN** the test SHALL fail
- **AND** the failure message SHALL state the task count at which the count stopped growing linearly
