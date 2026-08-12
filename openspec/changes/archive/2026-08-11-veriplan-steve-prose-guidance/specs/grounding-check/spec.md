## MODIFIED Requirements

### Requirement: Correlate prose findings with grounding outcomes

T5.3 SHALL correlate steve style findings with grounding outcomes AFTER T5.1 SHALL emit prose findings. A requirement with both a style finding AND an `Ungroundable` or `Ambiguous` outcome SHALL get ONE combined directive instead of separate ones.

#### Scenario: passive requirement also ungrounded produces combined directive

- **GIVEN** a requirement "A .md file path SHALL be resolved relative to the current working directory" whose steve prose check produced a PassiveVoice finding AND whose grounding outcome is `Ungroundable`
- **WHEN** T5.3 correlates the findings
- **THEN** the report SHALL emit ONE rephrase directive that both notes the passive voice and names a task ID, e.g. "This requirement is passive AND ungrounded — name the agent as a task ID, e.g. 'T1.2 SHALL resolve ...'"
- **AND** the report SHALL NOT emit separate standalone passive and ungrounded directives for that requirement

#### Scenario: active requirement with grounding success produces no combined directive

- **GIVEN** a requirement "T1.2 SHALL resolve a .md file path BEFORE T1.5 dispatches to the parser" with no steve style finding and a `Grounded` outcome
- **WHEN** T5.3 correlates the findings
- **THEN** the report SHALL NOT emit a combined directive for that requirement

### Requirement: Run correlation in all strictness modes

T5.4 SHALL run the prose/grounding correlation in all StrictnessProfile modes AFTER T4.3 SHALL map steve finding severity. In `Lax` mode, when a requirement is `Ungroundable` (normally info-only) and also has a steve style finding, the combined directive SHALL still be emitted at info severity.

#### Scenario: correlation emits in Lax mode

- **GIVEN** StrictnessProfile is `Lax` and a requirement that is `Ungroundable` with a PassiveVoice finding
- **WHEN** T5.4 runs the correlation
- **THEN** a combined rephrase directive SHALL be emitted at severity `info`
- **AND** the plan SHALL NOT be marked Blocking
