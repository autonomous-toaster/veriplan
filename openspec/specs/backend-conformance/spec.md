# backend-conformance Specification

## Purpose

Decide whether a checker backend can be trusted with a plan verdict: that it refutes a canary it
should refute, that it produces no verdict at all rather than a wrong one when a construct is beyond
it, that the two backends agree before a change lands, and that the in-process engine's cost is
pinned by a measurement rather than by a feeling.

## Requirements

### Requirement: A backend refutes the calibration canary

T2.6 SHALL add the canary to the generated model set BEFORE T3.1 SHALL compute scenario-step sources against the file.

Every backend SHALL refute the calibration canary before the result of the backend is used for a plan. The canary SHALL carry one claim per formula shape the translator emits, and every claim SHALL be false in a reachable state. A backend that reports any canary claim satisfied SHALL NOT produce a plan verdict.

#### Scenario: A sound backend refutes the canary

- **GIVEN** a backend able to explore a model holding nested non-deterministic choice
- **WHEN** T2.6 runs the canary through the backend
- **THEN** the backend SHALL report every canary claim violated
- **AND** the plan result SHALL carry a validity verdict

#### Scenario: A backend that inverts liveness fails the canary

- **GIVEN** a backend that refutes the safety claim but reports the liveness claim satisfied
- **WHEN** T2.6 runs the canary through the backend
- **THEN** the canary SHALL be considered not refuted
- **AND** the plan result SHALL NOT carry a validity verdict

#### Scenario: An unsound backend refutes nothing

- **GIVEN** a backend that stores one state and zero transitions
- **WHEN** T2.6 runs the canary through the backend
- **THEN** the backend SHALL NOT report the canary violated
- **AND** the plan result SHALL NOT carry a validity verdict

### Requirement: An unverified engine produces no verdict

T2.5 SHALL fail the empty search BEFORE T2.6 SHALL add the canary.

A backend that cannot refute the canary SHALL produce `valid: null`. The result SHALL name the canary in `skip_reason`. Pre-commit mode SHALL treat the unverified result as non-blocking.

#### Scenario: An unverified run is not a failure verdict

- **GIVEN** a backend that reports the canary satisfied
- **WHEN** the plan is checked in normal mode
- **THEN** the result SHALL carry `valid: null`
- **AND** the `skip_reason` SHALL name the calibration canary

#### Scenario: Pre-commit accepts an unverified run

- **GIVEN** a backend that reports the canary satisfied
- **WHEN** the plan is checked in pre-commit mode
- **THEN** the exit code SHALL be zero
- **AND** the output SHALL state that the plan is unverified

### Requirement: The strict gate runs before the change lands

T5.1 SHALL run the strict validation BEFORE T5.2 SHALL run the strict plan check.

Both gates SHALL run before the change lands. The plan check SHALL state the modelled split.

#### Scenario: Both gates run

- **GIVEN** a change ready to land
- **WHEN** T5.1 runs `openspec validate`
- **THEN** the validation SHALL report no error
- **AND** the plan check SHALL state the modelled split

#### Scenario: The plan check reports a modelled split

- **GIVEN** a change carrying modelled and unmodelled constraints
- **WHEN** T5.2 runs `veriplan check`
- **THEN** the output SHALL state the unmodelled count
- **AND** the output SHALL name the backend

### Requirement: The downstream corpus is recorded

T5.2 SHALL run the strict plan check BEFORE T5.3 SHALL record the downstream violations.

A downstream corpus that passed under placeholder formulas SHALL be re-checked with the repaired translator, and the violated constraints SHALL be recorded in the change.

#### Scenario: The downstream corpus is re-checked

- **GIVEN** a downstream corpus that passed under placeholder formulas
- **WHEN** T5.2 re-checks the corpus
- **THEN** the violated constraints SHALL be recorded in the change
- **AND** the recorded count SHALL distinguish violated from unmodelled

### Requirement: The scale measurement is recorded

T2.2 SHALL record the new state counts BEFORE T2.3 SHALL close the model-shape issue.

The number of tasks a plan can carry before the model stops closing SHALL be asserted by a test
rather than recorded in prose, so a regression fails the suite instead of ageing a comment.

#### Scenario: The scale measurement is recorded

- **GIVEN** plans of two, four, six and eight tasks
- **WHEN** T2.2 measures the elapsed time of each
- **THEN** the growth SHALL be stated in the change
- **AND** the largest plan that closes within the budget SHALL be named

#### Scenario: The recorded numbers are re-measured, not adjusted

- **GIVEN** a scale section carrying numbers from the previous model shape
- **WHEN** T2.2 rewrites the section
- **THEN** every number SHALL come from a fresh measurement
- **AND** the section SHALL name the model shape the numbers were taken from

### Requirement: Both engines agree before the change lands

T3.1 SHALL run the strict validation BEFORE T3.2 SHALL run the full plan check.

The change SHALL be validated by the specification gate and then by the plan check, in that order,
and both SHALL be clean before the change lands.

#### Scenario: The specification gate runs first

- **GIVEN** a change ready to validate
- **WHEN** T3.1 runs the strict validation
- **THEN** the validation SHALL report no error
- **AND** the change SHALL then be checked by the plan gate

#### Scenario: Both engines give the same verdict

- **GIVEN** a change whose constraints are all modelled
- **WHEN** T3.2 runs the plan check against each backend
- **THEN** both backends SHALL report the same verdict
- **AND** both SHALL report the same violated count

### Requirement: The corpus scale is re-measured

T3.2 SHALL run the full plan check BEFORE T3.3 SHALL re-measure the corpus scale.

The largest plan that closes within the default budget SHALL be measured again after the change and
recorded, so the next reader inherits a measurement rather than an estimate.

#### Scenario: The largest closing plan is named

- **GIVEN** linear chains of increasing size
- **WHEN** T3.3 measures each against the default budget
- **THEN** the largest chain that closes SHALL be named
- **AND** the state count of that chain SHALL be recorded

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
