## ADDED Requirements

### Requirement: steve accepts exclusion ranges for prose-zone scoping

T1.1 SHALL add steve's exclusion-range builder method BEFORE T3.4 SHALL use it for provenance-preserving scoping. The builder method SHALL accept a set of excluded line ranges (1-based, inclusive start/end). Findings produced for text wholly inside an excluded range SHALL NOT be emitted. A finding spanning the boundary between an excluded and an included region SHALL be attributed to the included region.

#### Scenario: exclude scenario scaffolding lines

- **GIVEN** a steve builder configured with an exclusion range covering a `**THEN**` step line "the plan SHALL be marked VALID"
- **WHEN** steve checks a document containing that line
- **THEN** no PassiveVoice finding SHALL be emitted for "SHALL be marked VALID"
- **AND** findings on lines outside the exclusion range SHALL still be emitted normally

#### Scenario: boundary-spanning finding kept in included region

- **GIVEN** an exclusion range that ends at line 41 and an included requirement body starting at line 42
- **WHEN** a finding spans from line 40 into line 43
- **THEN** the finding SHALL be attributed to the included region and emitted
- **AND** its start line SHALL be reported as the first line of the included region

### Requirement: steve supports configurable max sentence length

T1.2 SHALL add steve's configurable max-sentence-length builder method BEFORE T4.1 SHALL build a curated `Ste` per artifact. The method SHALL allow the maximum sentence length (in words) to be configured on the steve builder, independent of the fixed `TextKind` defaults (20 for Procedural, 25 for Descriptive). When set, the SentenceLength rule SHALL use the configured value.

#### Scenario: configure a longer limit for OpenSpec spec prose

- **GIVEN** a steve builder with a configured max sentence length of 30 words
- **WHEN** steve checks a 25-word requirement body sentence
- **THEN** no SentenceLength finding SHALL be emitted
- **AND** a 35-word requirement body sentence SHALL still emit a SentenceLength finding

#### Scenario: default unchanged when not configured

- **GIVEN** a steve builder with no explicit max sentence length
- **WHEN** steve checks text in `TextKind::Procedural`
- **THEN** the SentenceLength rule SHALL use the default of 20 words
