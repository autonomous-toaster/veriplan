## MODIFIED Requirements

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

## ADDED Requirements

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
