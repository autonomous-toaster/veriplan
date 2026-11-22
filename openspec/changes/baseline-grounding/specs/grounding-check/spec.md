## MODIFIED Requirements

### Requirement: Build Signature from PlanIR

T1.1 SHALL thread the project root into the grounding check BEFORE T1.2 SHALL scan the archived task lists.

The signature SHALL be built from the active change's tasks alone. A task id that this change does
not define SHALL NOT ground, whatever the archive contains.

#### Scenario: Signature includes all tasks

- **GIVEN** a PlanIR with tasks "1.1" (description "Create project"), "1.2" (description "Add deps")
- **WHEN** T1.1 provides filesystem context to the grounding check
- **THEN** the Signature SHALL contain constants "T1.1" and "T1.2"
- **AND** "T1.1" SHALL have aliases including "create project"

#### Scenario: Signature includes predicate definitions

- **GIVEN** a PlanIR with any tasks
- **WHEN** the Signature is built
- **THEN** the Signature SHALL contain predicates: BEFORE, AFTER, CONCURRENTLY, IF_THEN, ALWAYS, AT_MOST_ONE
- **AND** each predicate SHALL have the correct argument slots

#### Scenario: Empty plan produces empty signature

- **GIVEN** a PlanIR with zero tasks
- **WHEN** the Signature is built
- **THEN** the Signature SHALL have zero constants
- **AND** the Signature SHALL still include all 6 predicate definitions

#### Scenario: An archived task is not a constant

- **GIVEN** a requirement citing a task id that only an archived change defines
- **WHEN** T1.2 scans the archived task lists
- **THEN** the Signature SHALL NOT contain a constant for that id
- **AND** the requirement SHALL NOT ground on it

## ADDED Requirements

### Requirement: Report a cited id owned by an archived change

T1.2 SHALL scan the archived task lists BEFORE T1.3 SHALL build an id-to-owners map.

A cited id that this change does not define and that archived changes do define SHALL be reported
with the owning changes named. Only archived changes holding a spec for the requirement's capability
SHALL be scanned.

#### Scenario: A historical citation names its owners

- **GIVEN** a requirement in capability C citing an id that archived changes A and B define
- **WHEN** T1.3 builds the map
- **THEN** the map SHALL associate the id with the changes A and B
- **AND** the finding SHALL name both

#### Scenario: A capability nobody archived yields no owners

- **GIVEN** a requirement in a capability no archived change has touched
- **WHEN** T1.3 builds the map
- **THEN** the map SHALL be empty
- **AND** the finding SHALL be the existing absent-task report

### Requirement: The provenance map precedes the diagnosis

T1.3 SHALL build an id-to-owners map BEFORE T2.1 SHALL report a cited id that only archives define.

#### Scenario: No diagnosis without provenance

- **GIVEN** a requirement citing an id that the active change does not define
- **WHEN** T2.1 reports the finding
- **THEN** the map SHALL already hold the owners of that id
- **AND** the finding SHALL use them rather than reporting an absent id

### Requirement: Report an id that many changes define

T2.1 SHALL report a cited id that only archives define BEFORE T2.2 SHALL report how many archived changes define it.

An id that more than one archived change defines SHALL be reported with the count as well as the
names, because an id twenty-two changes share does not identify a task.

#### Scenario: A shared id is reported with its count

- **GIVEN** a requirement citing an id that twenty-two archived changes define
- **WHEN** T2.2 reports the finding
- **THEN** the finding SHALL state the number of archived changes that define the id
- **AND** the finding SHALL NOT ask for a task id reference

#### Scenario: A uniquely defined id is reported without a count

- **GIVEN** a requirement citing an id that exactly one archived change defines
- **WHEN** T2.2 reports the finding
- **THEN** the finding SHALL name that change
- **AND** the finding SHALL NOT state a count

### Requirement: Both engines agree before the change lands

T3.1 SHALL run the strict validation BEFORE T3.2 SHALL run the plan check against both engines.

#### Scenario: The gates agree

- **GIVEN** a change whose constraints are all modelled
- **WHEN** T3.1 runs the strict validation
- **THEN** the validation SHALL report no error
- **AND** T3.2 SHALL report the same verdict from each engine

### Requirement: The corpus is re-run before the change lands

T3.2 SHALL run the plan check against both engines BEFORE T3.3 SHALL re-run the corpus and record the change.

#### Scenario: The corpus is unchanged

- **GIVEN** the downstream corpus and this repository's own plan
- **WHEN** T3.3 re-runs the plan check
- **THEN** any change in grounding findings SHALL be recorded
- **AND** no constraint SHALL change its verdict
