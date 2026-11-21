## MODIFIED Requirements

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

## ADDED Requirements

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
