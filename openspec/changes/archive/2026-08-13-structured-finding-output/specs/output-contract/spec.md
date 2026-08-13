## Purpose

Defines the unified `Finding` output contract for `veriplan check`: one canonical, machine-readable shape shared by convertibility, model-check, and prose findings, with stable `kind`/`op` identifiers, a machine-applicability (`fixability`) tier, consistent verbosity semantics across human and JSON rendering, and a `--fix` mode that applies only mechanically safe edits.

## ADDED Requirements

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
