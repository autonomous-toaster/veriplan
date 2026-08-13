# Convertibility Check

## Purpose

Verify that a plan specification is structurally sound and can be converted into formal LTL properties for model checking. This is Phase 1 of the verification pipeline — it runs before rule translation and model checking.

## Requirements

### Requirement: Check task structure

T4.1 SHALL validate task structure BEFORE T4.2 SHALL check requirement references.

#### Scenario: All tasks have unique IDs

- **GIVEN** a PlanIR with tasks "1.1", "1.2", "2.1", "2.2"
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL pass the task structure check

#### Scenario: Duplicate task ID

- **GIVEN** a PlanIR with two tasks both ID "1.3"
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL emit a blocking error: "Duplicate task ID 1.3 at tasks.md:12 and tasks.md:21"
- **AND** T4.9 SHALL mark the plan not convertible

#### Scenario: Empty task list

- **GIVEN** a PlanIR with zero tasks
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL emit a blocking error: "No tasks found in plan"
- **AND** T4.9 SHALL mark the plan not convertible

### Requirement: Check requirement structure

T4.1 SHALL verify task references BEFORE T3.2 SHALL ground requirement statements. T4.1 SHALL complete BEFORE T3.2 SHALL run.

#### Scenario: SHALL references existing task

- **GIVEN** a requirement "T1.1 SHALL complete before T1.2" and tasks exist for "1.1" and "1.2"
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL pass the requirement reference check

#### Scenario: SHALL references non-existent task

- **GIVEN** a requirement "T99 SHALL run before T1.2" but no task T99 exists
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL emit a blocking error: "Requirement references non-existent task ID: T99"
- **AND** T5.1 SHALL mark the plan not convertible

#### Scenario: No formalizable requirements

- **GIVEN** all requirements lack temporal category (e.g., "System SHALL be robust", "Code SHALL be clean")
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL emit a blocking error: "No requirements are classifiable into a temporal category — cannot generate LTL properties"
- **AND** T5.1 SHALL mark the plan not convertible

#### Scenario: No RFC 2119 keyword

- **GIVEN** a requirement paragraph with no SHALL/MUST/SHOULD/MAY/MUST NOT
- **WHEN** T4.1 runs
- **THEN** T4.1 SHALL emit a warning: "Requirement 'Build order' has no RFC 2119 keyword"

### Requirement: Classify SHALL into temporal categories

T3.4 SHALL classify temporal categories AFTER T3.2 SHALL ground requirement statements. T3.4 SHALL complete BEFORE T5.1 SHALL translate to LTL.

#### Scenario: Sequential ordering classification

- **GIVEN** a SHALL statement: "T1.1 SHALL complete before T1.2"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL classify as `SequentialOrder`

#### Scenario: Exclusive classification

- **GIVEN**: "At most one deployment SHALL be active at a time"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL classify as `Exclusive`

#### Scenario: Conditional classification

- **GIVEN**: "IF smoke tests fail THEN rollback SHALL trigger"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL classify as `Conditional`

#### Scenario: Concurrent classification

- **GIVEN**: "Monitoring and deployment SHALL run concurrently"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL classify as `Concurrent`

#### Scenario: Global invariant classification

- **GIVEN**: "Rollback SHALL be available throughout the deployment window"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL classify as `Global`

#### Scenario: Non-formalizable SHALL

- **GIVEN**: "The system SHALL handle errors gracefully"
- **WHEN** T3.4 runs
- **THEN** T3.4 SHALL NOT match any category
- **AND** T3.3 SHALL flag it as "unverifiable — human review required"

### Requirement: Check scenario completeness

T4.6 SHALL check scenario completeness BEFORE T4.9 SHALL produce the report.

#### Scenario: Complete scenario passes

- **GIVEN** a scenario with WHEN: "deploy is triggered" and THEN: "T1.2 SHALL block the deploy"
- **WHEN** T4.6 runs
- **THEN** T4.6 SHALL pass the scenario structure check

#### Scenario: Scenario missing THEN

- **GIVEN** a scenario with only WHEN and no THEN
- **WHEN** T4.6 runs
- **THEN** T4.6 SHALL emit a warning: "Scenario 'Rapid deploy' missing THEN step at specs/deploy/spec.md:45"

### Requirement: Check constraint diversity

T4.7 SHALL inspect category distribution AFTER T4.4 SHALL classify all requirements. T4.6 SHALL complete scenario completeness BEFORE T4.7 SHALL inspect category distribution.

#### Scenario: Single-category plan

- **GIVEN** a plan where all 5 formalizable requirements are `SequentialOrder`
- **WHEN** T4.7 runs
- **THEN** T4.7 SHALL emit info: "Constraint distribution: SequentialOrder(5). Consider adding exclusive or conditional constraints for stronger verification"

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
