# Output Contract

## Purpose

Defines the unified `Finding` output contract for `veriplan check`: one canonical, machine-readable shape shared by convertibility, model-check, and prose findings, with stable `kind`/`op` identifiers, a machine-applicability (`fixability`) tier, consistent verbosity semantics across human and JSON rendering, and a `--fix` mode that applies only mechanically safe edits.

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Define the shared `Finding` shape |
| T1.2 | Define the `Kind` enum |
| T1.3 | Map check values to kinds |
| T2.2 | Key the check off the non-formalizable subtype |
| T4.1 | Attach Finding metadata to convertibility items |
| T4.2 | Populate `op`/`replacement` on grounding findings |
| T4.3 | Attach Finding metadata to model-check violations |
| T5.1 | Project all findings into one array |
| T5.2 | Emit the `findings[]` array in default JSON |
| T5.3 | Emit the human form, grouped by kind |
| T6.1 | Add the `--fix` flag |
| T6.2 | Apply only machine-applicable findings |
| T8.2 | Assert kind stability across strictness profiles |

## Requirements

### Requirement: Every finding has a canonical shape

T1.1 SHALL define the shared `Finding` shape BEFORE T4.1 SHALL attach it to convertibility items.

#### Scenario: Convertibility blocker is a Finding

- **GIVEN** a plan with a grounding multi-keyword blocker
- **WHEN** T4.1 attaches the Finding shape
- **THEN** the blocker SHALL appear with `kind`, `severity`, `location`, `message`, and a `fixability` tier
- **AND** the shape SHALL NOT differ structurally from a model-check violation

#### Scenario: Model-check violation is a Finding

- **GIVEN** a plan with a sequential-order model-check violation
- **WHEN** T4.3 attaches the Finding shape
- **THEN** the violation SHALL use the same shape as a convertibility blocker

### Requirement: Findings always appear in default output

T5.2 SHALL emit the `findings[]` array in default JSON AFTER T5.1 SHALL project all findings into one array.

#### Scenario: Blockers present in default JSON

- **GIVEN** a plan with 7 grounding multi-keyword blockers
- **WHEN** T5.2 runs `veriplan check <change> --format json` without `--verbose`
- **THEN** the JSON SHALL contain all 7 blockers in the `findings` array

#### Scenario: Same findings in both formats at default verbosity

- **GIVEN** a plan with blockers and violations
- **WHEN** T5.2 and T5.3 emit both formats at default verbosity
- **THEN** the set of `Finding`s described SHALL be identical between the two formats

### Requirement: kind and op are curated enums

T1.2 SHALL define the `Kind` enum BEFORE T1.3 SHALL map check values to kinds.

#### Scenario: kind stable across strictness profiles

- **GIVEN** a `pattern_ungrounded` requirement
- **WHEN** T8.2 runs veriplan under Strict, Moderate, and Lax strictness
- **THEN** the `kind` SHALL remain `pattern_ungrounded` in all three
- **AND** the `severity` SHALL differ as defined by the strictness profile

#### Scenario: non_formalizable subtypes are distinct kinds

- **GIVEN** a `bare_capability`, a `vague_action`, and a `vague_quality` requirement
- **WHEN** T2.2 keys the check off the subtype
- **THEN** each SHALL have a distinct `kind`
- **AND** each SHALL carry its own targeted `message` and `suggestion`

### Requirement: Deterministic edits carry a structured replacement

T4.2 SHALL populate `op` and `replacement` on a grounding multi-keyword Finding AFTER T1.1 SHALL define the `Op` enum.

#### Scenario: Grounding multi-keyword split carries replacement

- **GIVEN** a requirement matching both AFTER and BEFORE keywords
- **WHEN** T4.2 emits the Finding
- **THEN** the Finding SHALL have `op: split_requirement`
- **AND** `replacement` SHALL contain the concrete split requirement bodies

#### Scenario: Vague action carries no replacement

- **GIVEN** a `vague_action` requirement whose rewrite requires intent
- **WHEN** veriplan emits the Finding
- **THEN** the Finding SHALL have `op: replace_body`
- **AND** SHALL NOT carry a structured `replacement`
- **AND** SHALL be marked `fixability` other than machine-applicable

### Requirement: Human output groups findings by kind at default verbosity

T5.3 SHALL group identical findings by `kind` in default human output AFTER T5.1 SHALL project all findings into one array.

#### Scenario: Identical grounding blockers grouped

- **GIVEN** a plan with 7 identical grounding multi-keyword blockers
- **WHEN** T5.3 emits default human output
- **THEN** the output SHALL show one grouped entry "N× <kind>: <rephrase>" with a representative location
- **AND** `--verbose` SHALL expand all 7 with individual locations

### Requirement: --fix applies only machine-applicable findings

T6.2 SHALL apply edits only to Findings whose `op` is machine-applicable AFTER T6.1 SHALL add the `--fix` flag.

#### Scenario: --fix applies only mechanical ops

- **GIVEN** a plan with a `split_requirement` (structural) and a `replace_body` (judgment) Finding
- **WHEN** T6.2 runs `veriplan check --fix`
- **THEN** T6.2 SHALL leave the `split_requirement` finding as a suggestion (not auto-applied)
- **AND** T6.2 SHALL revalidate the plan after applying any machine-applicable edits

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

### Requirement: Pre-commit inherits prose blockers

T5.1 SHALL treat a Strict prose blocker as exit-1 in `--pre-commit` mode AFTER T1.2 SHALL make the exit code reflect prose blockers. A Strict prose blocker SHALL block a commit, consistent with other blockers.

#### Scenario: Pre-commit blocks on a prose blocker

- **GIVEN** a Strict-mode plan with a `blocker`-severity prose finding
- **WHEN** T5.1 runs in `--pre-commit` mode
- **THEN** the commit SHALL be blocked with exit code 1
