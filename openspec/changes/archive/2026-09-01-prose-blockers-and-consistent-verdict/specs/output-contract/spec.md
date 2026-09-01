## ADDED Requirements

### Requirement: Verdict derives from the flattened findings

T1.1 SHALL derive the plan status from the flattened `findings[]` set BEFORE T1.2 SHALL make the exit code reflect prose blockers. The status SHALL NOT be Blocking when any finding has severity `blocker`. The status SHALL be derived from the same finding set that is printed, so the label and the list SHALL NOT contradict.

#### Scenario: Prose blocker prevents a VALID verdict

- **GIVEN** a plan whose model-check result is valid but which has a `blocker`-severity prose finding
- **WHEN** T1.1 derives the plan status
- **THEN** the status SHALL NOT be "✓ VALID"
- **AND** the status SHALL reflect the presence of the `blocker` finding

#### Scenario: No blockers yields a VALID verdict

- **GIVEN** a plan with no findings of severity `blocker`
- **WHEN** T1.1 derives the plan status
- **THEN** the status SHALL be "✓ VALID"

### Requirement: Blocker-severity findings live in report.blockers

T2.1 SHALL route a finding with severity `blocker` into `report.blockers` BEFORE T1.1 SHALL derive the plan status. A finding whose severity is `blocker` SHALL NOT be bucketed into `report.info` or `report.warnings`.

#### Scenario: Prose blocker reaches the blockers list

- **GIVEN** a Strict-mode prose finding with severity `blocker`
- **WHEN** T2.1 attaches the finding to the report
- **THEN** the finding SHALL appear in `report.blockers`
- **AND** the finding SHALL NOT appear in `report.info`

## MODIFIED Requirements

### Requirement: Finding coordinates are file-absolute

T4.1 SHALL compute the snippet start offset in the source file BEFORE T4.2 SHALL add the offset to the reported line and byte offsets. The `line`, `start`, and `end` fields SHALL be offsets into the source file, not into a snippet. A task-description finding SHALL report the task's actual line in `tasks.md` and the column within that line.

#### Scenario: Task finding reports its real line

- **GIVEN** a task description on line 31 of `tasks.md` with a prose finding
- **WHEN** T4.2 adds the snippet offset
- **THEN** the finding SHALL report `line` 31
- **AND** `start`/`end` SHALL be file-absolute byte offsets pointing into line 31 of the file

#### Scenario: Snippet-relative offsets are not reported

- **GIVEN** a prose finding whose steve offsets are relative to a single-line snippet
- **WHEN** T4.2 adds the snippet offset
- **THEN** the reported `start`/`end` SHALL include the snippet's start offset in the file
- **AND** SHALL NOT be the raw snippet-relative values

### Requirement: Coordinate contract is documented in the spec

T4.3 SHALL document the file-absolute coordinate contract in the spec AFTER T4.2 SHALL add the snippet offset. The spec SHALL state that `line`, `start`, and `end` are file-absolute offsets.

#### Scenario: Spec states file-absolute semantics

- **GIVEN** the `output-contract` spec after the coordinate change
- **WHEN** T4.3 documents the contract
- **THEN** the spec SHALL state that `line`, `start`, and `end` are file-absolute

## ADDED Requirements

### Requirement: Pre-commit inherits prose blockers

T5.1 SHALL treat a Strict prose blocker as exit-1 in `--pre-commit` mode AFTER T1.2 SHALL make the exit code reflect prose blockers. A Strict prose blocker SHALL block a commit, consistent with other blockers.

#### Scenario: Pre-commit blocks on a prose blocker

- **GIVEN** a Strict-mode plan with a `blocker`-severity prose finding
- **WHEN** T5.1 runs in `--pre-commit` mode
- **THEN** the commit SHALL be blocked with exit code 1

### Requirement: Validation covers verdict and coordinate behavior

T6.1 SHALL add a test asserting the verdict is not "✓ VALID" when a `blocker`-severity prose finding is present BEFORE T6.3 SHALL run `veriplan check` on the change. T6.2 SHALL add a test asserting a task-description prose finding reports the task's real line and file byte offsets BEFORE T6.3 SHALL run `veriplan check` on the change.

#### Scenario: Verdict test guards the no-contradiction invariant

- **GIVEN** a plan with a `blocker`-severity prose finding and a valid model-check result
- **WHEN** T6.1 runs the verdict test
- **THEN** the test SHALL assert the status is not "✓ VALID"

#### Scenario: Coordinate test guards file-absolute offsets

- **GIVEN** a task-description prose finding
- **WHEN** T6.2 runs the coordinate test
- **THEN** the test SHALL assert the finding reports the task's real line and file byte offsets
