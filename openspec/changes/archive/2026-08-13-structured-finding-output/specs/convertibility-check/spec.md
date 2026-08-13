## MODIFIED Requirements

### Requirement: Produce AI feedback report

T4.1 SHALL attach `kind`, `severity`, `location`, `message`, and `fixability` to each blocker AFTER T1.1 SHALL define the `Finding` shape.

#### Scenario: Blocking-only report

- **GIVEN** a plan with 2 blocking issues (non-existent task reference + no formalizable requirements)
- **WHEN** T4.1 attaches Finding metadata
- **THEN** T4.1 SHALL list both blockers with source locations
- **AND** each Finding SHALL carry a `kind` and `fixability` tier
- **AND** veriplan SHALL mark the plan `blocking`

#### Scenario: Warnings-only report

- **GIVEN** a plan with no blockers but 2 warnings (missing RFC 2119 keyword, low constraint diversity)
- **WHEN** T4.1 attaches Finding metadata
- **THEN** T4.1 SHALL list both warnings as `Finding`s
- **AND** veriplan SHALL mark the plan `convertible_with_warnings`

#### Scenario: Clean plan

- **GIVEN** a plan with no blockers and no warnings
- **WHEN** veriplan emits the report
- **THEN** veriplan SHALL indicate "Plan is convertible"
- **AND** SHALL mark the plan `convertible`

#### Scenario: Grounding blocker in report

- **GIVEN** a plan where T3.2 found an ungroundable requirement
- **WHEN** T4.1 attaches Finding metadata
- **THEN** T4.1 SHALL include the grounding blocker as a `Finding` with source location
- **AND** veriplan SHALL mark the plan `blocking`

#### Scenario: Non-formalizable subtypes are distinct Findings

- **GIVEN** a `bare_capability` requirement and a `vague_action` requirement
- **WHEN** T2.2 keys the check off the subtype
- **THEN** each SHALL appear as a distinct `kind`
- **AND** each SHALL carry its own targeted `message` and `suggestion`

## ADDED Requirements

### Requirement: Convertibility findings carry structured replacement

T4.1 SHALL attach a structured `replacement` to a convertibility Finding AFTER T1.1 SHALL define the `Op` enum.

#### Scenario: Duplicate task id is machine-applicable

- **GIVEN** a plan with duplicate task ID `1.3`
- **WHEN** T4.1 attaches Finding metadata
- **THEN** the `duplicate_task_id` Finding SHALL be marked machine-applicable (`fixability` other than needs-judgment)
- **AND** SHALL carry an `op` for the rename

#### Scenario: Bad task reference requires judgment

- **GIVEN** a requirement referencing non-existent task `T99`
- **WHEN** T4.1 attaches Finding metadata
- **THEN** the `bad_task_reference` Finding SHALL be marked as requiring judgment (not machine-applicable)
- **AND** SHALL carry a `suggestion` that names valid task IDs
