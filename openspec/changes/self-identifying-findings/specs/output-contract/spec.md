## MODIFIED Requirements

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

## ADDED Requirements

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
