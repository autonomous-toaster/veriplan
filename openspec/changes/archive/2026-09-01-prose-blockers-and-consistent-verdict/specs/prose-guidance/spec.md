## MODIFIED Requirements

### Requirement: Report ambiguous prose as blockers in Strict

T3.1 SHALL map the two ambiguity-indicating prose rules to blockers in Strict BEFORE T1.1 SHALL derive the plan status from the flattened findings. In Strict mode, `OneInstructionPerSentence` and `PronounAmbiguity` findings SHALL be severity `blocker` and SHALL flip the plan status to Blocking. In Moderate and Lax modes, prose findings SHALL remain advisory and SHALL NOT flip the plan status. `PassiveVoice` and `Hedging` findings SHALL remain advisory in all profiles.

#### Scenario: Strict ambiguous prose flips the plan to Blocking

- **GIVEN** a plan that passes all structural and semantic checks but has a task description with two instructions
- **WHEN** T1.1 derives the plan status under Strict
- **THEN** the report SHALL include a Finding for the task with severity `blocker`
- **AND** the plan status SHALL be Blocking

#### Scenario: Moderate ambiguous prose stays advisory

- **GIVEN** a plan with a PronounAmbiguity finding under Moderate
- **WHEN** T3.2 keeps prose advisory
- **THEN** the finding SHALL be severity `warning`
- **AND** the plan status SHALL NOT be Blocking

### Requirement: Map the two ambiguity-indicating prose rules to blockers in Strict

T3.1 SHALL mark `OneInstructionPerSentence` and `PronounAmbiguity` as blockers in Strict BEFORE T3.2 SHALL keep the remaining rules advisory. `PassiveVoice`, `Hedging`, `SynonymConsistency`, `SentenceLength`, and `SlopWord` SHALL remain advisory in all profiles.

#### Scenario: Strict maps the two rules to blockers

- **GIVEN** StrictnessProfile is `Strict`
- **WHEN** T3.1 maps a PronounAmbiguity finding and a PassiveVoice finding
- **THEN** the PronounAmbiguity finding SHALL be severity `blocker`
- **AND** the PassiveVoice finding SHALL be severity `warning`

#### Scenario: Lax keeps all prose advisory

- **GIVEN** StrictnessProfile is `Lax`
- **WHEN** T3.2 keeps prose advisory
- **THEN** the finding SHALL be severity `info`
