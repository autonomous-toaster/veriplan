# Scenario STE Prose

## Purpose

Apply a safe subset of STE prose rules (PronounAmbiguity, SentenceLength) to scenario step content, stripping the GIVEN/WHEN/THEN/AND scaffolding and code spans first, so ambiguous scenario assertions are flagged without false positives on legitimate structured steps.

## ADDED Requirements

### Requirement: Check scenario steps with safe STE subset

T1.1 SHALL check scenario step content with the safe STE subset BEFORE T2.1 SHALL report prose findings. T1.1 SHALL complete BEFORE T3.1 SHALL parse the scenario steps.

#### Scenario: Safe subset applied to scenario steps

- **GIVEN** a requirement with a scenario step "**THEN** the valve and the pump are connected, and it is faulty"
- **WHEN** T1.1 checks the step content
- **THEN** T1.1 SHALL report a PronounAmbiguity finding for "it"
- **AND** T2.1 SHALL emit it as a rephrase directive

### Requirement: Strip scenario scaffolding before checking

T1.2 SHALL strip the **GIVEN**/**WHEN**/**THEN**/**AND** markers and inline code spans from a scenario step BEFORE T1.1 SHALL check the step content. T1.2 SHALL complete BEFORE T1.1 SHALL check the step content.

#### Scenario: Scaffolding produces no noise

- **GIVEN** a scenario step "**THEN** the plan SHALL be marked VALID"
- **WHEN** T1.2 strips the markers and T1.1 checks the content
- **THEN** T1.1 SHALL NOT report a PassiveVoice finding
- **AND** T1.1 SHALL NOT report a OneInstructionPerSentence finding

### Requirement: Exclude noisy rules from scenario checking

T1.3 SHALL exclude PassiveVoice and OneInstructionPerSentence from the scenario STE subset BEFORE T1.1 SHALL check the step content. T1.3 SHALL complete BEFORE T1.1 SHALL check the step content.

#### Scenario: Legitimate state assertion not flagged

- **GIVEN** a scenario step "**THEN** the plan SHALL be marked VALID"
- **WHEN** T1.3 excludes PassiveVoice and T1.1 checks the content
- **THEN** T1.1 SHALL NOT report a PassiveVoice finding

### Requirement: Keep scenario prose findings advisory

T2.1 SHALL emit scenario prose findings as rephrase directives BEFORE T3.1 SHALL parse the scenario steps. T2.1 SHALL NOT mark the plan as blocking due to a scenario prose finding.

#### Scenario: Advisory only, never blocking

- **GIVEN** a scenario step with a PronounAmbiguity finding
- **WHEN** T2.1 emits the finding
- **THEN** T2.1 SHALL emit it as a rephrase directive
- **AND** the plan SHALL NOT be marked blocking due to that finding

### Requirement: Reuse parsed scenario steps

T3.1 SHALL parse scenario steps from a requirement body BEFORE T1.1 SHALL check the step content. T3.1 SHALL complete BEFORE T1.1 SHALL check the step content.

#### Scenario: Steps parsed from requirement body

- **GIVEN** a requirement body with a "#### Scenario:" block
- **WHEN** T3.1 parses the steps
- **THEN** T3.1 SHALL return the GIVEN/WHEN/THEN/AND steps
- **AND** T1.1 SHALL check each step's content
