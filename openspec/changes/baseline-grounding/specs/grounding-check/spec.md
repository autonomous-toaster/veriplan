## MODIFIED Requirements

### Requirement: Build Signature from PlanIR

T1.1 SHALL resolve the provenance of every requirement BEFORE T2.1 SHALL ground a body against the active tasks and the provenance tasks.

A requirement's provenance SHALL be the latest archived change whose delta contains the requirement and
whose task list defines every id the requirement body cites. The signature SHALL carry the active
change's tasks and, for each requirement, the tasks of that requirement's provenance change.

#### Scenario: Signature includes all tasks

- **GIVEN** a PlanIR with tasks "1.1" (description "Create project"), "1.2" (description "Add deps")
- **WHEN** T2.1 builds the signature
- **THEN** the signature SHALL contain constants "T1.1" and "T1.2"
- **AND** "T1.1" SHALL have aliases including "create project"

#### Scenario: Signature includes predicate definitions

- **GIVEN** a PlanIR with any tasks
- **WHEN** T2.1 builds the signature
- **THEN** the signature SHALL contain predicates: BEFORE, AFTER, CONCURRENTLY, IF_THEN, ALWAYS, AT_MOST_ONE
- **AND** each predicate SHALL have the correct argument slots

#### Scenario: Empty plan produces empty signature

- **GIVEN** a PlanIR with zero tasks and no archived changes
- **WHEN** T2.1 builds the signature
- **THEN** the signature SHALL have zero constants
- **AND** the signature SHALL still include all 6 predicate definitions

#### Scenario: A published body grounds against its provenance

- **GIVEN** a delta requirement citing two ids that only the archived change "model-scale" defines
- **WHEN** T1.1 resolves the provenance of the requirement
- **THEN** the provenance SHALL be "model-scale"
- **AND** T2.1 SHALL build constants for both ids

#### Scenario: The latest defining change owns the body

- **GIVEN** a requirement contained in two archived deltas, both defining the cited ids
- **WHEN** T1.1 resolves the provenance
- **THEN** the provenance SHALL be the later change in archive order

## ADDED Requirements

### Requirement: Record the resolved change on the requirement

T1.1 SHALL resolve the provenance of every requirement BEFORE T1.2 SHALL record the resolved change on the requirement.

A requirement whose provenance resolves SHALL carry the owning change name. A requirement whose
provenance does not resolve SHALL carry no owner, and the report SHALL state that provenance is
missing rather than that a cited id is absent.

#### Scenario: A resolved requirement carries its owner

- **GIVEN** a requirement whose body cites ids defined by one archived change
- **WHEN** T1.2 records the provenance
- **THEN** the requirement SHALL carry that change name

#### Scenario: An unresolved requirement reports missing provenance

- **GIVEN** a requirement whose name no archived delta contains
- **WHEN** T3.1 reports the provenance outcome
- **THEN** the report SHALL state that no archived change records the requirement
- **AND** the report SHALL NOT state that a cited id is absent

### Requirement: A constraint grounded only on history is historical

T2.1 SHALL ground a body against the active tasks and the provenance tasks BEFORE T2.2 SHALL classify a constraint with no live argument as historical.

A constraint whose every argument belongs to the provenance change SHALL be classified historical. A
historical constraint SHALL carry no formula and SHALL be reported with the owning change.

#### Scenario: A historical constraint carries no formula

- **GIVEN** a constraint whose two arguments are both defined by the provenance change
- **WHEN** T2.2 classifies the constraint
- **THEN** the constraint SHALL carry no formula
- **AND** the report SHALL name the owning change

#### Scenario: A live constraint stays modelled

- **GIVEN** a constraint whose two arguments belong to the active change
- **WHEN** T2.2 classifies the constraint
- **THEN** the constraint SHALL carry a formula
- **AND** the report SHALL NOT classify the constraint as historical

### Requirement: A mixed constraint is reported and not failed

T2.2 SHALL classify a constraint with no live argument as historical BEFORE T2.3 SHALL report a constraint that mixes a live task with history.

A constraint holding at least one argument from the active change and at least one from the provenance
change SHALL be classified historical, SHALL name the foreign ids, and SHALL NOT fail the plan.

#### Scenario: A mixed constraint names the foreign ids

- **GIVEN** a constraint citing one task of the active change and one task of the provenance change
- **WHEN** T2.3 reports the constraint
- **THEN** the report SHALL name the task that belongs to the provenance change
- **AND** the plan SHALL NOT be reported invalid for the mixing

#### Scenario: A split constraint becomes live

- **GIVEN** the same requirement reworded to cite only tasks of the active change
- **WHEN** T2.3 reports the constraint
- **THEN** the constraint SHALL NOT be classified historical
- **AND** the constraint SHALL carry a formula

### Requirement: A historical constraint is counted separately

T2.3 SHALL report a constraint that mixes a live task with history BEFORE T2.4 SHALL count a historical constraint separately.

The report SHALL carry a historical count, and the total SHALL equal the sum of the satisfied,
violated, unmodelled, unchecked and historical counts. A historical constraint SHALL NOT be counted as
satisfied.

#### Scenario: The historical count is visible

- **GIVEN** a plan holding one historical constraint and one live constraint
- **WHEN** T2.4 counts the results
- **THEN** the report SHALL state one historical constraint
- **AND** the satisfied count SHALL state one

#### Scenario: Historical alone yields no verdict

- **GIVEN** a plan whose only constraint is historical
- **WHEN** T2.4 reports the counts
- **THEN** the satisfied count SHALL be zero
- **AND** the verdict SHALL NOT be a pass

### Requirement: The report names the owning change

T3.1 SHALL report a requirement with no recorded provenance BEFORE T3.2 SHALL name the owning change in the report.

Every finding about a historical citation SHALL name the archived change that owns it, so the author
can tell a typo from a reference to completed work.

#### Scenario: The owning change is named

- **GIVEN** a historical citation whose provenance change is "model-scale"
- **WHEN** T3.2 composes the finding
- **THEN** the finding SHALL name "model-scale"

### Requirement: Both engines agree before the change lands

T4.1 SHALL run the strict validation BEFORE T4.2 SHALL run the plan check against both engines.

#### Scenario: The gates agree

- **GIVEN** a change whose constraints are all live
- **WHEN** T4.1 runs the strict validation
- **THEN** the validation SHALL report no error
- **AND** T4.2 SHALL report the same verdict from each engine

### Requirement: The corpus is re-run before the change lands

T4.2 SHALL run the plan check against both engines BEFORE T4.3 SHALL re-run the corpus and record the change.

#### Scenario: The corpus records what moved

- **GIVEN** the downstream corpus and this repository's own plans
- **WHEN** T4.3 re-runs the plan check
- **THEN** every plan whose verdict changed SHALL be recorded
- **AND** every constraint newly reported as historical SHALL be recorded
