## MODIFIED Requirements

### Requirement: Build Signature from PlanIR

T1.1 SHALL read the archived task lists BEFORE T1.2 SHALL build the signature from both sources.

The signature SHALL carry one constant per task of the active change and one constant per task of
every archived change. A task id that both define SHALL resolve to the active change's constant, so a
live reference never binds to history.

#### Scenario: Signature includes all tasks

- **GIVEN** a PlanIR with tasks "1.1" (description "Create project"), "1.2" (description "Add deps")
- **WHEN** T1.2 builds the Signature
- **THEN** the Signature SHALL contain constants "T1.1" and "T1.2"
- **AND** "T1.1" SHALL have aliases including "create project"
- **AND** "T1.2" SHALL have aliases including "add deps"

#### Scenario: Signature includes predicate definitions

- **GIVEN** a PlanIR with any tasks
- **WHEN** T1.2 builds the Signature
- **THEN** the Signature SHALL contain predicates: BEFORE, AFTER, CONCURRENTLY, IF_THEN, ALWAYS, AT_MOST_ONE
- **AND** each predicate SHALL have the correct argument slots

#### Scenario: Empty plan produces empty signature

- **GIVEN** a PlanIR with zero tasks and no archived changes
- **WHEN** T1.2 builds the Signature
- **THEN** the Signature SHALL have zero constants
- **AND** the Signature SHALL still include all 6 predicate definitions

#### Scenario: An archived task contributes a constant

- **GIVEN** a plan holding no task 2.6, and an archived change whose tasks include 2.6
- **WHEN** T1.1 reads the archived task lists
- **THEN** T1.2 SHALL build a constant for T2.6
- **AND** T1.3 SHALL record the archived change as the owner of T2.6

#### Scenario: The active change wins a shared id

- **GIVEN** a plan holding task 2.6, and an archived change whose tasks include 2.6
- **WHEN** T1.2 builds the Signature
- **THEN** the constant T2.6 SHALL be the active change's task
- **AND** the owner of T2.6 SHALL be the active change

## ADDED Requirements

### Requirement: Report a task id owned by an archived change

T1.3 SHALL record the owning change BEFORE T2.1 SHALL report a cited id owned by an archived change.

A task id that the active change does not define and an archived change does define SHALL be
reported with the owning change named. The finding SHALL NOT instruct the author to add a task id
reference, because the author cited a real task.

#### Scenario: A historical citation names its owner

- **GIVEN** a requirement body citing T2.6, which only an archived change defines
- **WHEN** T2.1 evaluates the grounding result of the body
- **THEN** the finding SHALL name the archived change that owns T2.6
- **AND** the finding SHALL NOT ask for a task id reference

#### Scenario: An unknown id stays a grounding failure

- **GIVEN** a requirement body citing a task id no change defines
- **WHEN** T2.1 evaluates the grounding result of the body
- **THEN** the finding SHALL report an absent task id

### Requirement: Report a constraint grounded only on archived tasks

T2.2 SHALL name the owning change BEFORE T2.3 SHALL report a requirement grounded only on archived tasks.

A constraint whose every argument comes from an archived change SHALL be reported as grounded on
history. The constraint SHALL still be modelled, and the report SHALL NOT count it as unmodelled.

#### Scenario: A historical ordering is named

- **GIVEN** a requirement body whose two task ids are defined only by archived changes
- **WHEN** T2.3 inspects the grounding outcome
- **THEN** the report SHALL name the archived changes
- **AND** the constraint SHALL still carry a formula

#### Scenario: A live ordering is not flagged

- **GIVEN** a requirement body citing two tasks of the active change
- **WHEN** T2.3 inspects the grounding outcome
- **THEN** the report SHALL NOT name an archived change

### Requirement: Both engines agree before the change lands

T3.1 SHALL run the strict validation BEFORE T3.2 SHALL run the plan check against both engines.

The change SHALL be validated by the specification gate, then checked by both engines, then the
corpus SHALL be re-run and the result recorded.

#### Scenario: The gates agree

- **GIVEN** a change whose constraints are all modelled
- **WHEN** T3.1 runs the strict validation
- **THEN** the validation SHALL report no error
- **AND** T3.2 SHALL report the same verdict from each engine

### Requirement: The corpus is re-run before the change lands

T3.2 SHALL run the plan check against both engines BEFORE T3.3 SHALL re-run the corpus and record the change.

Any change in grounding findings caused by the archive entering the signature SHALL be recorded
rather than absorbed, and citations that were ungroundable SHALL be gone.

#### Scenario: The corpus is re-run

- **GIVEN** the downstream corpus and this repository's own plan
- **WHEN** T3.3 re-runs the plan check
- **THEN** any change in grounding findings SHALL be recorded
- **AND** findings that were ungroundable citations SHALL be gone
