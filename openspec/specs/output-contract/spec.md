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

T5.2 SHALL emit the `findings[]` array in default JSON AFTER T5.1 SHALL project all findings into one array. Plan-level aggregate diagnostics (task/phase count, scenario count, classification summary, constraint distribution, grounding summary) SHALL NOT be part of the `findings[]` set.

#### Scenario: Blockers present in default JSON

- **GIVEN** a plan with 7 grounding multi-keyword blockers
- **WHEN** T5.2 runs `veriplan check <change> --format json` without `--verbose`
- **THEN** the JSON SHALL contain all 7 blockers in the `findings` array

#### Scenario: Same findings in both formats at default verbosity

- **GIVEN** a plan with blockers and violations
- **WHEN** T5.2 and T5.3 emit both formats at default verbosity
- **THEN** the set of `Finding`s described SHALL be identical between the two formats

#### Scenario: Aggregate diagnostics are not findings

- **GIVEN** a plan that passes convertibility
- **WHEN** T5.2 emits `findings[]` in default JSON
- **THEN** the array SHALL NOT contain `task_count`, `scenario_count`, `classification_summary`, `constraint_diversity`, or `grounding_summary` items

### Requirement: kind and op are curated enums

T1.5 SHALL split the prose kinds BEFORE T2.4 SHALL record the arity of every kind.

A prose finding SHALL take the `kind` from the steve rule that produced the finding.
`prose_other` SHALL remain only as the fallback for a rule without a curated kind. `KIND_NAMES`
SHALL record an arity for every kind: `plan` for a fact about the whole plan, and `site` for a
finding that names one place that needs one edit.

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

#### Scenario: A pronoun finding names the pronoun rule

- **GIVEN** a scenario step with an ambiguous pronoun
- **WHEN** T1.5 maps the steve rule to a kind
- **THEN** the finding SHALL report `prose_pronoun_ambiguity`
- **AND** the finding SHALL NOT report `prose_other`

#### Scenario: An unmapped rule falls back

- **GIVEN** a steve rule without a curated kind
- **WHEN** T1.5 maps the rule
- **THEN** the finding SHALL report `prose_other`

#### Scenario: A plan fact is not a site

- **GIVEN** a plan with no tasks
- **WHEN** T2.4 records the arity of `no_tasks`
- **THEN** the arity SHALL be `plan`

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

T1.4 SHALL replace substitution with summarisation BEFORE T2.5 SHALL scale the default human output.

A group line SHALL name one true member of the group and SHALL NOT describe the other members by
that member's location. Omitted members SHALL be counted after an explicit remainder marker, and
`--verbose` SHALL list every member one line each. The default output SHALL scale with the count: a
verdict at zero findings, full identity at one finding, and one line per kind and remedy at several
findings.

#### Scenario: Identical grounding blockers grouped

- **GIVEN** a plan with 7 identical grounding multi-keyword blockers
- **WHEN** T5.3 emits default human output
- **THEN** the output SHALL show one grouped entry "N× <kind>: <rephrase>" with a representative location
- **AND** `--verbose` SHALL expand all 7 with individual locations

#### Scenario: A group names a real member

- **GIVEN** a plan with seven `prose_pronoun_ambiguity` findings in three different deltas
- **WHEN** T1.4 emits the default human output
- **THEN** the group line SHALL show one member's path and line
- **AND** the group line SHALL show the count and an explicit remainder marker

#### Scenario: A singleton finding shows full identity

- **GIVEN** a plan with one `prose_pronoun_ambiguity` finding
- **WHEN** T2.5 formats the output
- **THEN** the output SHALL show the severity, the kind and the pointer
- **AND** the output SHALL show the element and the reason
- **AND** the output SHALL quote the offending span

#### Scenario: Verbose lists every member

- **GIVEN** a plan with seven findings of one kind
- **WHEN** T2.5 formats the output with `--verbose`
- **THEN** the output SHALL show seven lines
- **AND** every line SHALL carry a pointer

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

T1.1 SHALL carry a change-relative path on every finding BEFORE T2.1 SHALL derive the step line from the recorded byte span.

The `file` field SHALL be a path relative to the change directory, so two findings in different
spec deltas SHALL NOT share a location string. The `line`, `start` and `end` fields SHALL address
the source file for every finding kind. A scenario-step finding SHALL take the span from the step
block in the source file. A requirement-body finding SHALL report the body line rather than the
heading line. Single-file and stdin input SHALL keep the path the caller supplied.

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

#### Scenario: Two spec deltas do not share a location

- **GIVEN** a change with `specs/alpha/spec.md` and `specs/beta/spec.md`
- **WHEN** T1.1 attaches the path to each finding
- **THEN** the two location strings SHALL differ
- **AND** each path SHALL name the delta that owns the finding

#### Scenario: The fourth scenario reports the source line

- **GIVEN** a spec file with four scenarios whose fourth `THEN` step sits on line 26
- **WHEN** T2.1 derives the step line
- **THEN** the finding SHALL report line 26

#### Scenario: A step span addresses the file

- **GIVEN** a pronoun inside a `THEN` step on line 23 of a spec file
- **WHEN** T1.2 records the step span
- **THEN** `start` and `end` SHALL address that pronoun in the file

#### Scenario: A requirement body reports the body line

- **GIVEN** a requirement heading on line 12 with the body on line 14
- **WHEN** T3.1 anchors the requirement source
- **THEN** the finding SHALL report line 14
- **AND** the finding SHALL NOT report line 12

### Requirement: Pre-commit inherits prose blockers

T5.1 SHALL treat a Strict prose blocker as exit-1 in `--pre-commit` mode AFTER T1.2 SHALL make the exit code reflect prose blockers. A Strict prose blocker SHALL block a commit, consistent with other blockers.

#### Scenario: Pre-commit blocks on a prose blocker

- **GIVEN** a Strict-mode plan with a `blocker`-severity prose finding
- **WHEN** T5.1 runs in `--pre-commit` mode
- **THEN** the commit SHALL be blocked with exit code 1

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

### Requirement: A finding names its element and quotes its span

T1.3 SHALL add the `element` and `excerpt` fields BEFORE T2.2 SHALL carry the prose element into the finding.

Every finding SHALL name the element that owns the finding, such as a requirement or a scenario
step. A plan-scoped finding SHALL carry an empty element. The formatter SHALL render both fields and
SHALL NOT reconstruct either field from a location.

#### Scenario: A prose finding names the scenario

- **GIVEN** a pronoun finding inside the scenario `A client registers itself`
- **WHEN** T2.2 converts the prose finding
- **THEN** the finding SHALL carry the scenario name and the step kind in `element`
- **AND** the finding SHALL carry the offending span in `excerpt`

#### Scenario: A plan finding carries no element

- **GIVEN** a plan with no phase grouping
- **WHEN** T1.3 fills the element
- **THEN** `element` SHALL be empty

### Requirement: A singleton finding quotes its span

T1.3 SHALL add the element and excerpt fields BEFORE T2.3 SHALL quote the span of a singleton finding.

A report holding one finding SHALL print the quoted span. A report holding several findings SHALL
print the quoted span only where the group holds one member, so a digest line stays short.

#### Scenario: A lone blocker quotes the sentence

- **GIVEN** a report with one `prose_pronoun_ambiguity` finding
- **WHEN** T2.3 renders the finding
- **THEN** the output SHALL quote the sentence that holds the pronoun

### Requirement: advisory means not model-backed

T1.4 SHALL replace substitution with summarisation BEFORE T3.2 SHALL tie the advisory flag to the model.

The `advisory` flag SHALL describe whether a model formula backs the finding. The flag SHALL NOT
describe severity. A prose finding with severity `blocker` SHALL carry `advisory: true` and SHALL
still block the verdict.

#### Scenario: A strict prose blocker stays advisory

- **GIVEN** a strict run with one `prose_pronoun_ambiguity` finding
- **WHEN** T3.2 sets the advisory flag
- **THEN** `advisory` SHALL be true
- **AND** the severity SHALL be `blocker`
- **AND** the verdict SHALL NOT be `VALID`
