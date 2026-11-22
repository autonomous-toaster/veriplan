## MODIFIED Requirements

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
- **AND** the total SHALL equal the sum of all four statuses
- **AND** the satisfied count SHALL NOT include the constraint
