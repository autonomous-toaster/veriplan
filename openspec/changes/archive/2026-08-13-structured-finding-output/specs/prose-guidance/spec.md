## MODIFIED Requirements

### Requirement: Report prose findings without blocking

T3.2 SHALL copy steve's structured fields onto each prose finding AFTER T3.1 SHALL extend the `ProseFinding` shape. Prose findings SHALL NEVER flip the plan status to Blocking.

#### Scenario: prose findings do not block a convertible plan

- **GIVEN** a plan that passes all structural and semantic checks but has one passive-voice requirement
- **WHEN** T4.4 tags the prose finding advisory
- **THEN** the report SHALL include a Finding for the passive requirement
- **AND** the plan status SHALL remain ConvertibleWithWarnings
- **AND** the plan SHALL NOT be marked Blocking

#### Scenario: prose Finding carries steve structured fields

- **GIVEN** a prose finding from steve's `SlopWord` rule with a deterministic replacement
- **WHEN** T3.2 copies the fields
- **THEN** the Finding SHALL carry the steve `fixability` tier (`local` for `SlopWord` with a replacement)
- **AND** SHALL carry the byte span and, when available, the official ASD-STE100 `ste_rule` number
- **AND** SHALL carry the deterministic `replacement`

## ADDED Requirements

### Requirement: SlopWord is included in the curated prose set

T3.3 SHALL add steve's `SlopWord` rule to the curated rule set AFTER T3.1 SHALL extend the `ProseFinding` shape.

#### Scenario: Slop word with replacement is machine-applicable

- **GIVEN** a requirement body containing the slop word "leverage"
- **WHEN** T3.3 runs steve's curated rules
- **THEN** a `SlopWord` Finding SHALL be produced with suggestion "replace \"leverage\" with \"use\""
- **AND** the Finding SHALL be marked machine-applicable with a deterministic `replacement`

#### Scenario: Slop word without replacement requires judgment

- **GIVEN** a requirement body containing a slop word with no plain replacement (e.g. "robust")
- **WHEN** T3.3 runs steve's curated rules
- **THEN** the `SlopWord` Finding SHALL be marked as requiring judgment (not machine-applicable)
- **AND** SHALL NOT carry a deterministic `replacement`
