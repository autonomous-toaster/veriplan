## ADDED Requirements

### Requirement: Run curated steve rules on OpenSpec prose

T1.1 SHALL run steve's curated rule set against the prose zones of an OpenSpec plan AFTER T0.1 SHALL parse the plan into PlanIR. The curated rules SHALL be limited to PassiveVoice, PronounAmbiguity, Hedging, OneInstructionPerSentence, SynonymConsistency, and SentenceLength. All other steve rules SHALL be disabled.

#### Scenario: steve runs only curated rules

- **GIVEN** an OpenSpec plan with requirement prose containing a passive sentence "A .md file path SHALL be resolved"
- **WHEN** T1.1 runs the steve prose check
- **THEN** a PassiveVoice finding SHALL be produced for that sentence
- **AND** no DictionaryNotApprovedWord findings SHALL be produced for words like "shall", "create", or "verify"
- **AND** no NounCluster findings SHALL be produced for "**GIVEN**"/"**WHEN**"/"**THEN**" scenario steps

#### Scenario: steve skips non-prose zones

- **GIVEN** a requirement whose body is followed by `#### Scenario:` scaffolding with `- **GIVEN**`, `- **WHEN**`, `- **THEN**` list items
- **WHEN** T1.1 runs the steve prose check
- **THEN** the scenario scaffolding lines SHALL NOT be included in steve prose input
- **AND** inline code spans such as `` `Grounded` `` and predicate keywords such as `BEFORE`, `AFTER`, `CONCURRENTLY` SHALL NOT be treated as prose

#### Scenario: THEN-step passive phrases are not flagged

- **GIVEN** a scenario with a `**THEN**` step "the plan SHALL be marked VALID"
- **WHEN** T1.1 runs the steve prose check
- **THEN** NO PassiveVoice finding SHALL be produced for "SHALL be marked VALID"
- **AND** the finding SHALL NOT appear because the `**THEN**` step is scenario scaffolding, not requirement-body prose

### Requirement: Scope steve rules per artifact type

T1.2 SHALL apply a different curated steve rule set per artifact type. For `spec.md` files, the full curated set SHALL run on requirement body paragraphs. For `tasks.md`, only OneInstructionPerSentence and Hedging SHALL run on task descriptions. For `design.md` and `proposal.md`, only PassiveVoice, PronounAmbiguity, and Hedging SHALL run.

#### Scenario: spec.md gets the full curated set

- **GIVEN** a `spec.md` with a requirement body "The response SHALL be a Location" (passive) and a task description elsewhere
- **WHEN** T1.2 applies rules to the `spec.md`
- **THEN** PassiveVoice SHALL fire on the requirement body
- **AND** task-description-only rules SHALL NOT be applied to `spec.md` prose

#### Scenario: tasks.md gets the minimal set

- **GIVEN** a `tasks.md` whose task description contains two instructions in one sentence and no hedging
- **WHEN** T1.2 applies rules to the `tasks.md`
- **THEN** OneInstructionPerSentence SHALL fire on the task description
- **AND** PassiveVoice SHALL NOT fire on task descriptions

#### Scenario: design and proposal get the light set

- **GIVEN** a `design.md` and a `proposal.md`
- **WHEN** T1.2 applies rules to both
- **THEN** PassiveVoice, PronounAmbiguity, and Hedging SHALL be active
- **AND** OneInstructionPerSentence SHALL NOT be active

### Requirement: Map steve finding severity from StrictnessProfile

T1.3 SHALL map each steve finding to a veriplan severity based on the active StrictnessProfile. In `Strict` mode, PassiveVoice and OneInstructionPerSentence SHALL be hard findings and the rest soft. In `Moderate` mode, all curated rules SHALL be soft. In `Lax` mode, all curated rules SHALL be info.

#### Scenario: Strict profile maps to hard/soft

- **GIVEN** StrictnessProfile is `Strict`
- **WHEN** T1.3 maps a PassiveVoice finding and a PronounAmbiguity finding
- **THEN** the PassiveVoice finding SHALL be severity `blocker`
- **AND** the PronounAmbiguity finding SHALL be severity `warning`

#### Scenario: Lax profile maps to info

- **GIVEN** StrictnessProfile is `Lax`
- **WHEN** T1.3 maps any curated steve finding
- **THEN** the finding SHALL be severity `info`

### Requirement: Report prose findings without blocking

T1.4 SHALL include prose-guidance findings in the convertibility report as rephrase directives, but prose findings SHALL NEVER contribute a blocker that flips the plan status to Blocking. A plan with only prose findings SHALL NOT be marked Blocking.

#### Scenario: prose findings do not block a convertible plan

- **GIVEN** a plan that passes all structural and semantic checks but has one passive-voice requirement
- **WHEN** T1.4 builds the report in `Strict` mode
- **THEN** the report SHALL include a rephrase directive for the passive requirement
- **AND** the plan status SHALL remain ConvertibleWithWarnings
- **AND** the plan SHALL NOT be marked Blocking
