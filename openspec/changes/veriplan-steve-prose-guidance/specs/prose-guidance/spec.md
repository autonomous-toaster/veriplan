## ADDED Requirements

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

### Requirement: Map steve finding severity from StrictnessProfile

T3.3 SHALL exclude inline code spans BEFORE T4.3 SHALL map steve finding severity. The mapping SHALL use StrictnessProfile: `Strict` marks PassiveVoice and OneInstructionPerSentence hard and the rest soft; `Moderate` marks all soft; `Lax` marks all info.

#### Scenario: Strict profile maps to hard/soft

- **GIVEN** StrictnessProfile is `Strict`
- **WHEN** T4.3 maps a PassiveVoice finding and a PronounAmbiguity finding
- **THEN** the PassiveVoice finding SHALL be severity `blocker`
- **AND** the PronounAmbiguity finding SHALL be severity `warning`

#### Scenario: Lax profile maps to info

- **GIVEN** StrictnessProfile is `Lax`
- **WHEN** T4.3 maps any curated steve finding
- **THEN** the finding SHALL be severity `info`

### Requirement: Report prose findings without blocking

T4.1 SHALL run steve's curated rule set BEFORE T5.1 SHALL emit prose findings as rephrase directives. Prose findings SHALL NEVER contribute a blocker that flips the plan status to Blocking. A plan with only prose findings SHALL NOT be marked Blocking.

#### Scenario: prose findings do not block a convertible plan

- **GIVEN** a plan that passes all structural and semantic checks but has one passive-voice requirement
- **WHEN** T5.1 builds the report in `Strict` mode
- **THEN** the report SHALL include a rephrase directive for the passive requirement
- **AND** the plan status SHALL remain ConvertibleWithWarnings
- **AND** the plan SHALL NOT be marked Blocking
