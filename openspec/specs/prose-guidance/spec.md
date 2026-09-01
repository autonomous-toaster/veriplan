# Prose Guidance

## Purpose

Run curated steve prose rules over OpenSpec prose zones, scoped per artifact, advisory-only.

## Task Reference

| T ID | Description |
|------|-------------|
| T3.1 | select prose zones per artifact |
| T3.2 | exclude scenario scaffolding |
| T3.3 | exclude inline code spans |
| T4.1 | run steve's curated rule set |
| T4.2 | apply per-artifact rule subsets |
| T4.3 | map steve finding severity |
| T5.1 | emit prose findings as rephrase directives |
## Requirements

### Requirement: Run curated steve rules on OpenSpec prose

T3.1 SHALL select prose zones per artifact BEFORE T4.1 SHALL run steve's curated rule set. The curated rules SHALL be limited to PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, and SentenceLength. All other steve rules SHALL be disabled.

#### Scenario: steve runs only curated rules

- **GIVEN** an OpenSpec plan with requirement prose containing a passive sentence "A .md file path SHALL be resolved"
- **WHEN** T4.1 runs the steve prose check
- **THEN** a PassiveVoice finding SHALL be produced for that sentence
- **AND** no DictionaryNotApprovedWord findings SHALL be produced for words like "shall", "create", or "verify"
- **AND** no NounCluster findings SHALL be produced for "**GIVEN**"/"**WHEN**"/"**THEN**" scenario steps

#### Scenario: steve skips non-prose zones

- **GIVEN** a requirement whose body is followed by `#### Scenario:` scaffolding with `- **GIVEN**`, `- **WHEN**`, `- **THEN**` list items
- **WHEN** T4.1 runs the steve prose check
- **THEN** the scenario scaffolding lines SHALL NOT be included in steve prose input
- **AND** inline code spans such as `` `Grounded` `` and predicate keywords such as `BEFORE`, `AFTER`, `CONCURRENTLY` SHALL NOT be treated as prose

#### Scenario: THEN-step passive phrases are not flagged

- **GIVEN** a scenario with a `**THEN**` step "the plan SHALL be marked VALID"
- **WHEN** T4.1 runs the steve prose check
- **THEN** NO PassiveVoice finding SHALL be produced for "SHALL be marked VALID"
- **AND** the finding SHALL NOT appear because the `**THEN**` step is scenario scaffolding, not requirement-body prose

### Requirement: Scope steve rules per artifact type

T3.2 SHALL exclude scenario scaffolding BEFORE T4.2 SHALL apply per-artifact rule subsets. For `spec.md` files, the full curated set SHALL run on requirement body paragraphs. For `tasks.md`, only OneInstructionPerSentence and Hedging SHALL run on task descriptions. For `design.md` and `proposal.md`, only PassiveVoice, PronounAmbiguity, and Hedging SHALL run.

#### Scenario: spec.md gets the full curated set

- **GIVEN** a `spec.md` with a requirement body "The response SHALL be a Location" (passive) and a task description elsewhere
- **WHEN** T4.2 applies rules to the `spec.md`
- **THEN** PassiveVoice SHALL fire on the requirement body
- **AND** task-description-only rules SHALL NOT be applied to `spec.md` prose

#### Scenario: tasks.md gets the minimal set

- **GIVEN** a `tasks.md` whose task description contains two instructions in one sentence and no hedging
- **WHEN** T4.2 applies rules to the `tasks.md`
- **THEN** OneInstructionPerSentence SHALL fire on the task description
- **AND** PassiveVoice SHALL NOT fire on task descriptions

#### Scenario: design and proposal get the light set

- **GIVEN** a `design.md` and a `proposal.md`
- **WHEN** T4.2 applies rules to both
- **THEN** PassiveVoice, PronounAmbiguity, and Hedging SHALL be active
- **AND** OneInstructionPerSentence SHALL NOT be active

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

### Requirement: Report ambiguous prose as blockers in Strict

T3.2 SHALL copy steve's structured fields onto each prose finding AFTER T3.1 SHALL extend the `ProseFinding` shape. T3.1 SHALL map the two ambiguity-indicating prose rules to blockers in Strict BEFORE T1.1 SHALL derive the plan status from the flattened findings. In Strict mode, `OneInstructionPerSentence` and `PronounAmbiguity` findings SHALL be severity `blocker` and SHALL flip the plan status to Blocking. In Moderate and Lax modes, prose findings SHALL remain advisory and SHALL NOT flip the plan status. `PassiveVoice` and `Hedging` findings SHALL remain advisory in all profiles.

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

#### Scenario: prose Finding carries steve structured fields

- **GIVEN** a prose finding from steve's `SlopWord` rule with a deterministic replacement
- **WHEN** T3.2 copies the fields
- **THEN** the Finding SHALL carry the steve `fixability` tier (`local` for `SlopWord` with a replacement)
- **AND** SHALL carry the byte span and, when available, the official ASD-STE100 `ste_rule` number
- **AND** SHALL carry the deterministic `replacement`

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
