## MODIFIED Requirements

### Requirement: Finding coordinates are file-absolute

T3.2 SHALL sweep the remaining constructors BEFORE T3.3 SHALL add the coordinate tests.

The `line`, `start` and `end` fields SHALL be offsets into the source file for every finding kind. A scenario-step finding SHALL report the step line inside the spec file. No finding constructor SHALL report a snippet-relative pair as a file coordinate.

#### Scenario: Task finding reports its real line

- **GIVEN** a task description on line 31 of `tasks.md` with a prose finding
- **WHEN** T3.2 computes the coordinate of the finding
- **THEN** the finding SHALL report `line` 31
- **AND** `start` and `end` SHALL be file-absolute byte offsets pointing into line 31

#### Scenario: Snippet-relative offsets are not reported

- **GIVEN** a finding whose source is a snippet inside a longer element
- **WHEN** T3.2 computes the coordinate of the finding
- **THEN** the reported line SHALL be the line in the source file
- **AND** the reported line SHALL NOT be the offset inside the snippet

#### Scenario: A scenario-step finding reports the step line

- **GIVEN** a `THEN` step on line 9 of a spec file whose requirement body starts on line 5
- **WHEN** T3.2 computes the source of the step
- **THEN** the finding SHALL report line 9
- **AND** the finding SHALL NOT report line 5

#### Scenario: A prose finding reports the task line

- **GIVEN** a task description carrying a prose finding
- **WHEN** T3.3 tests the coordinate of the finding
- **THEN** the finding SHALL report the task line in `tasks.md`

## ADDED Requirements

### Requirement: The verdict names the engine and the modelling split

T2.3 SHALL name the checker backend BEFORE T2.4 SHALL add the backend to the JSON output.

The human verdict SHALL name the backend that produced the result. The JSON output SHALL carry the backend name and the modelled, unmodelled, unchecked and violated counts. Two runs differing only in backend SHALL NOT produce indistinguishable output.

#### Scenario: The human verdict names the backend

- **GIVEN** a plan checked with the `spin` backend
- **WHEN** T2.3 formats the verdict
- **THEN** the output SHALL name `spin`
- **AND** the output SHALL state the unmodelled count

#### Scenario: The JSON carries the split

- **GIVEN** a plan holding three constraints, of which one is unmodelled and one is unchecked
- **WHEN** T2.4 formats the JSON result
- **THEN** the JSON SHALL carry the backend name
- **AND** the JSON SHALL carry a distinct unmodelled count
- **AND** the JSON SHALL carry a distinct unchecked count
