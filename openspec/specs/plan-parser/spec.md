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

T1.2 SHALL record a byte span for every scenario step BEFORE T2.1 SHALL derive the step line from the recorded span.

The parser SHALL take each step span from the step block in the source file. The parser SHALL NOT
derive a step line by index arithmetic over the joined requirement body. The body is a
concatenation of sibling blocks, so an index in the joined text does not address a line in the
file. Measured with v0.5.0, the reported line drifts by one line for each preceding scenario.

#### Scenario: Source location for task

- **GIVEN** a task on line 12 of `tasks.md`
- **WHEN** T2.1 derives the line of the task source
- **THEN** the task source SHALL report line 12

#### Scenario: Source location for requirement

- **GIVEN** a requirement heading on line 3 of a spec file with the statement on line 5
- **WHEN** T2.1 derives the line of the requirement source
- **THEN** the requirement source SHALL report line 5
- **AND** the requirement source SHALL report the body line rather than the heading line

#### Scenario: A step carries a file line

- **GIVEN** a requirement body that starts on line 5 of a spec file
- **WHEN** T2.1 derives the line of a `THEN` step four lines into the body
- **THEN** the step source SHALL report line 9

#### Scenario: The fourth scenario does not drift

- **GIVEN** a spec file with four scenarios
- **WHEN** T2.1 derives the line of each `THEN` step
- **THEN** every step source SHALL report the line of the step in the source file
- **AND** no step source SHALL report a line one greater for each preceding scenario

#### Scenario: A step span addresses the file

- **GIVEN** a `THEN` step on line 23 of a spec file
- **WHEN** T1.2 records the span
- **THEN** `start_byte` SHALL point inside line 23
- **AND** `end_byte` SHALL point inside line 23

### Requirement: A requirement body source is the body line

T2.1 SHALL derive the step line from the recorded span BEFORE T3.1 SHALL anchor a requirement-body source at the body line.

The body line SHALL be the first line that carries the requirement statement. The heading line
SHALL describe the heading only, so a prose finding in a body paragraph SHALL report the body line.

#### Scenario: A body finding reports the body line

- **GIVEN** a requirement heading on line 12 with the statement on line 14
- **WHEN** T3.1 anchors the requirement source
- **THEN** the requirement source SHALL report line 14
- **AND** the requirement source SHALL NOT report line 12

### Requirement: Coordinate tests cover the joined body

T3.1 SHALL anchor a requirement-body source at the body line BEFORE T4.1 SHALL test four scenarios and two same-named deltas.

The coordinate suite SHALL hold a spec file with four scenarios, so a per-scenario drift fails a
test instead of ageing in a report. The suite SHALL hold a change with two files named `spec.md`,
so a basename location fails a test.

#### Scenario: The four-scenario fixture fails on drift

- **GIVEN** a spec file with four scenarios
- **WHEN** T4.1 runs the coordinate suite
- **THEN** every step SHALL report the line of the step in the file

#### Scenario: The two-delta fixture fails on a basename

- **GIVEN** a change with two spec deltas
- **WHEN** T4.1 runs the coordinate suite
- **THEN** every finding SHALL carry a path that names the delta
