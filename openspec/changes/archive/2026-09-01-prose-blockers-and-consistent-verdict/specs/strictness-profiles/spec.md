## MODIFIED Requirements

### Requirement: Prose severity mapping marks two rules as blockers in Strict

T3.1 SHALL map the two ambiguity-indicating prose rules to `blocker` in Strict BEFORE T3.2 SHALL keep Moderate and Lax advisory. In `Strict` mode, `OneInstructionPerSentence` and `PronounAmbiguity` SHALL be severity `blocker`. In `Moderate` mode, all prose rules SHALL be severity `warning`. In `Lax` mode, all prose rules SHALL be severity `info`.

#### Scenario: Strict marks the two rules as blockers

- **GIVEN** StrictnessProfile is `Strict`
- **WHEN** T3.1 maps a PronounAmbiguity finding and a PassiveVoice finding
- **THEN** the PronounAmbiguity finding SHALL be severity `blocker`
- **AND** the PassiveVoice finding SHALL be severity `warning`

#### Scenario: Moderate keeps prose as warnings

- **GIVEN** StrictnessProfile is `Moderate`
- **WHEN** T3.2 keeps prose advisory
- **THEN** the finding SHALL be severity `warning`
