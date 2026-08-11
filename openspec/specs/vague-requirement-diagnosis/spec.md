# Vague Requirement Diagnosis

## Purpose

Diagnose why a non-formalizable requirement is not verifiable (bare capability, vague action, or vague quality) and emit targeted, pedagogical fixes instead of the generic "does not match any temporal category" message. This maps to the MIL-STD-498 "Testable" evaluation criterion: a vague requirement is one for which no objective test can be defined.

## Task Reference

| T ID | Description |
|------|-------------|
| T1.1 | Classify a non-formalizable requirement as BareCapability |
| T1.2 | Classify a non-formalizable requirement as VagueAction |
| T1.3 | Classify a non-formalizable requirement as VagueQuality |
| T2.1 | Identify a requirement as non-formalizable |
| T2.2 | Classify a temporal requirement as its temporal category |
| T3.1 | Report a blocker |
| T3.2 | Emit the generic blocker for an undiagnosed requirement |

## Requirements

### Requirement: Diagnose bare capability

T1.1 SHALL classify a non-formalizable requirement as BareCapability BEFORE T2.1 SHALL identify any requirement as non-formalizable. T1.1 SHALL complete BEFORE T3.1 SHALL report a blocker.

#### Scenario: Bare capability produces targeted fix

- **GIVEN** a requirement "T1.1 SHALL be executed." that references a task and contains no vague word
- **WHEN** T1.1 classifies it
- **THEN** T1.1 SHALL return diagnosis BareCapability
- **AND** T3.1 SHALL emit a blocker with fix "add a temporal relation to another task (e.g. 'T1.1 SHALL complete BEFORE T1.2 SHALL start'), or remove it if it merely re-states the task"

### Requirement: Diagnose vague action

T1.2 SHALL classify a non-formalizable requirement as VagueAction BEFORE T2.1 SHALL identify any requirement as non-formalizable. T1.2 SHALL complete BEFORE T3.1 SHALL report a blocker.

#### Scenario: Vague action produces targeted fix

- **GIVEN** a requirement "T1.1 SHALL be done quickly." that references a task and contains a vague adverb
- **WHEN** T1.2 classifies it
- **THEN** T1.2 SHALL return diagnosis VagueAction
- **AND** T3.1 SHALL emit a blocker with fix "define it measurably (e.g. 'within 200ms'), or add a temporal relation to another task"

### Requirement: Diagnose vague quality

T1.3 SHALL classify a non-formalizable requirement as VagueQuality BEFORE T2.1 SHALL identify any requirement as non-formalizable. T1.3 SHALL complete BEFORE T3.1 SHALL report a blocker.

#### Scenario: Vague quality produces targeted fix

- **GIVEN** a requirement "The system SHALL be robust." with no task references and a vague adjective
- **WHEN** T1.3 classifies it
- **THEN** T1.3 SHALL return diagnosis VagueQuality
- **AND** T3.1 SHALL emit a blocker with fix "reference a task with a temporal relation, or define 'robust' via a measurable criterion or standard"

### Requirement: Preserve safety boundary for temporal requirements

T2.2 SHALL classify a requirement containing a temporal keyword as its temporal category BEFORE T2.1 SHALL identify any requirement as non-formalizable. T2.2 SHALL complete BEFORE T1.1 SHALL classify a bare capability.

#### Scenario: Temporal requirement never diagnosed as vague

- **GIVEN** the requirement "T1.1 SHALL be done quickly BEFORE T1.2 SHALL start."
- **WHEN** T2.2 evaluates it
- **THEN** T2.2 SHALL classify it as SequentialOrder
- **AND** no vague diagnosis SHALL be returned

### Requirement: Fall back to generic blocker for undiagnosed requirements

T3.2 SHALL emit the generic blocker for an undiagnosed requirement BEFORE T3.1 SHALL finalize the report. T3.2 SHALL complete BEFORE T3.1 SHALL finalize the report.

#### Scenario: Undiagnosed requirement keeps generic message

- **GIVEN** a requirement "The migration SHALL happen." with no task references and no vague word
- **WHEN** T3.2 evaluates it
- **THEN** T3.2 SHALL emit the blocker "does not match any temporal category"
