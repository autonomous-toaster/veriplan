## ADDED Requirements

### Requirement: A word of a task description is not an alias

T4.4 SHALL relocate the init table rule BEFORE T4.5 SHALL stop treating one word as an alias.

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
