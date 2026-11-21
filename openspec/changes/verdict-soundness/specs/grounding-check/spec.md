## MODIFIED Requirements

### Requirement: Build Signature from PlanIR

T4.1 SHALL include archived task constants BEFORE T4.2 SHALL report an archived task owner.

The signature SHALL carry one constant per task of the active change and one constant per task of every archived change. A constant from an archived change SHALL record the name of the change that defines the constant.

#### Scenario: Signature includes all tasks

- **GIVEN** a plan holding N tasks
- **WHEN** T4.1 builds the signature
- **THEN** the signature SHALL hold N task constants
- **AND** every constant SHALL carry the type `task_id`

#### Scenario: Signature includes predicate definitions

- **GIVEN** a plan holding tasks
- **WHEN** T4.1 builds the signature
- **THEN** the signature SHALL hold the six predicate definitions
- **AND** every definition SHALL declare the argument slots of the predicate

#### Scenario: Empty plan produces empty signature

- **GIVEN** a plan holding no task
- **WHEN** T4.1 builds the signature
- **THEN** the signature SHALL hold no constant

#### Scenario: An archived constant grounds a published requirement

- **GIVEN** a change whose requirement cites a task id defined by an archived change
- **WHEN** T4.1 builds the signature
- **THEN** the signature SHALL hold a constant for the cited task id
- **AND** the requirement SHALL ground against the constant

#### Scenario: A change-local constant keeps the change name

- **GIVEN** a change whose requirement cites a task id of the change itself
- **WHEN** T4.1 builds the signature
- **THEN** the constant SHALL record the active change name

## ADDED Requirements

### Requirement: Report a cited task id owned by an archived change

T4.2 SHALL name the owning change BEFORE T4.3 SHALL document the archive policy.

A task id that the active change does not define and an archived change does define SHALL produce a finding that names the owning change. The finding SHALL NOT instruct the author to add a task id reference.

#### Scenario: A foreign task id names the owner

- **GIVEN** a requirement body citing `T2.6` where the active change defines no such task and an archived change defines the task
- **WHEN** T4.2 evaluates the grounding result of the body
- **THEN** the finding SHALL name the archived change
- **AND** the finding SHALL NOT ask for an additional task id reference

#### Scenario: An unknown task id stays a grounding failure

- **GIVEN** a requirement body citing a task id that no change defines
- **WHEN** T4.2 evaluates the grounding result of the body
- **THEN** the finding SHALL report an absent task id

### Requirement: A word of a task description is not an alias

T4.2 SHALL report an archived task owner BEFORE T4.5 SHALL stop treating one word as an alias.

A task description SHALL NOT contribute a single word as an alias of the task. Only the whole description and its leading phrases SHALL act as aliases. A requirement that cites two task ids SHALL ground on the two cited tasks, not on tasks whose descriptions happen to share a word.

#### Scenario: A shared word does not capture a slot

- **GIVEN** a requirement citing T5.15 and T5.16 where T5.1's description shares one word with the requirement
- **WHEN** T4.5 builds the signature aliases
- **THEN** the shared word SHALL NOT name T5.1
- **AND** the two slots SHALL fill from T5.15 and T5.16

#### Scenario: A whole description still grounds

- **GIVEN** a requirement that names no task id and repeats a task's whole description
- **WHEN** T4.5 builds the signature aliases
- **THEN** the description SHALL remain an alias of that task
