# Plan Parser

## Purpose

Parse OpenSpec plan files (tasks.md and specs/**/*.md) into a PlanIR intermediate representation. This is the entry point of the verification pipeline — it runs before convertibility checking.

## Requirements

### Requirement: Parse real OpenSpec directory structure

T3.2 SHALL locate the change directory BEFORE T3.9 SHALL parse the plan.

#### Scenario: Parse change with multiple capabilities

- **GIVEN** a change directory with `tasks.md` at root and `specs/parser/spec.md`, `specs/checker/spec.md`
- **WHEN** T3.9 parses
- **THEN** T3.9 SHALL return tasks from `tasks.md` and requirements + scenarios from both spec files
- **AND** each element SHALL carry a `SourceLocation` with its file path

#### Scenario: Missing tasks.md

- **GIVEN** a change directory with no `tasks.md`
- **WHEN** T3.2 runs
- **THEN** it SHALL return an error: "No tasks.md found in change"

#### Scenario: Missing spec directory

- **GIVEN** a change directory with no `specs/` subdirectory
- **WHEN** T3.2 runs
- **THEN** it SHALL return an error: "No specs/ directory found in change"

### Requirement: Parse tasks with N.M numbering

T3.3 SHALL parse tasks BEFORE T3.9 SHALL build PlanIR. T3.3 SHALL complete BEFORE T3.4 SHALL group tasks into phases.

#### Scenario: Parse task list with sections

- **GIVEN** a `tasks.md` with:

  ```markdown
  ## Phase 1: Scaffolding
  - [x] 1.1 Create project
  - [ ] 1.2 Add dependencies
  ## Phase 2: Core
  - [ ] 2.1 Implement parser
  ```

- **WHEN** T3.3 and T3.4 run together
- **THEN** T3.9 SHALL return 3 tasks with IDs "1.1", "1.2", "2.1", correct checked status, and phase assignments "Phase 1: Scaffolding" and "Phase 2: Core"

#### Scenario: Non-N.M task format

- **GIVEN** a task item with no N.M prefix: "- [ ] Some task"
- **WHEN** T3.3 runs
- **THEN** T3.3 SHALL assign an auto-generated ID based on position
- **AND** it SHALL emit a warning: "Task 'Some task' has no N.M ID — using auto-ID"

#### Scenario: Duplicate task IDs

- **GIVEN** two tasks with the same ID "1.3"
- **WHEN** T3.3 runs
- **THEN** T3.3 SHALL emit a warning: "Duplicate task ID 1.3"

### Requirement: Parse requirements with RFC 2119 keywords

T3.5 SHALL extract requirements from spec files BEFORE T3.6 SHALL classify RFC 2119 strength. T3.5 SHALL complete BEFORE T3.7 SHALL parse scenario blocks.

#### Scenario: Extract requirement with SHALL

- **GIVEN** a `spec.md` with:

  ```markdown
  ### Requirement: Build order
  The system SHALL ensure T1.1 completes before T1.2.
  ```

- **WHEN** T3.5 runs
- **THEN** T3.5 SHALL return a requirement with id "Build order", statement "The system SHALL ensure T1.1 completes before T1.2.", strength `Must`, and source location

#### Scenario: SHOULD requirement

- **GIVEN**: `The system SHOULD display a progress bar during deploy`
- **WHEN** T3.5 runs
- **THEN** T3.6 SHALL classify strength as `Should`

#### Scenario: MUST NOT requirement

- **GIVEN**: `The system MUST NOT deploy without passing smoke tests`
- **WHEN** T3.5 runs
- **THEN** T3.6 SHALL classify strength as `MustNot`

#### Scenario: MAY requirement

- **GIVEN**: `The system MAY cache build artifacts for reuse`
- **WHEN** T3.5 runs
- **THEN** T3.6 SHALL classify strength as `May`

#### Scenario: Requirement with no RFC 2119 keyword

- **GIVEN**: `The system does some thing`
- **WHEN** T3.5 runs
- **THEN** T3.5 SHALL emit a warning: "Requirement '...' has no RFC 2119 keyword (SHALL/MUST/SHOULD/MAY/MUST NOT)"

### Requirement: Parse Scenarios with GIVEN/WHEN/THEN

T3.7 SHALL extract scenario blocks AFTER T3.5 SHALL complete requirement extraction. T3.7 SHALL complete BEFORE T3.8 SHALL associate scenarios with requirements.

#### Scenario: Extract full scenario

- **GIVEN** a scenario block with name "Build before deploy", a GIVEN step "the build has not completed", a WHEN step "T3.5 detects a deploy trigger", and a THEN step "T3.8 SHALL link the scenario to its requirement"
- **WHEN** T3.7 runs
- **THEN** T3.7 SHALL return a scenario with name "Build before deploy", three steps of kinds Given/When/Then, and each step's text and source location

#### Scenario: Scenario missing THEN

- **GIVEN** a scenario with WHEN but no THEN
- **WHEN** T3.7 runs
- **THEN** T3.7 SHALL emit a warning: "Scenario 'Build before deploy' has no THEN step"

#### Scenario: THEN without SHALL

- **GIVEN** a THEN step that doesn't contain SHALL/MUST/SHOULD/MAY
- **WHEN** T3.7 runs
- **THEN** T3.7 SHALL emit a warning: "THEN step in scenario 'Build before deploy' has no RFC 2119 keyword"

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
