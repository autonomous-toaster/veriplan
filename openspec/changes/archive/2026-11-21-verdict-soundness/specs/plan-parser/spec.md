## MODIFIED Requirements

### Requirement: Preserve source locations

T3.1 SHALL compute scenario-step sources against the file BEFORE T3.2 SHALL sweep the remaining constructors.

Every element in the plan model SHALL carry a source location whose line and byte offsets address the source file. A scenario step SHALL carry the line of the step inside the spec file, not an index inside the requirement body.

#### Scenario: Source location for task

- **GIVEN** a task on line 12 of `tasks.md`
- **WHEN** T3.1 parses the task
- **THEN** the task source SHALL report line 12

#### Scenario: Source location for requirement

- **GIVEN** a requirement heading on line 3 of a spec file
- **WHEN** T3.1 parses the requirement
- **THEN** the requirement source SHALL report line 3

#### Scenario: A step carries a file line

- **GIVEN** a requirement body that starts on line 5 of a spec file
- **WHEN** T3.1 parses a `THEN` step four lines into the body
- **THEN** the step source SHALL report line 9
