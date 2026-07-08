## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Modify `extract_shall_statement()` to return only first paragraph |
| T1.2 | Update existing tests for multi-paragraph bodies |
| T1.3 | Run cargo test to verify no regressions |
| T2.1 | Add multi-keyword pre-check after grounder returns |
| T2.2 | Emit `grounding_ambiguous_multi_keyword` blocker |
| T2.3 | Add unit tests for multi-keyword detection |
| T3.1 | Propagate grounding failure status to verification result |
| T3.2 | Skip BFS model checker for PatternUngrounded requirements |
| T3.3 | Add integration test for grounding-failure skip |
| T4.1 | Update `suggest_fix()` for Exclusive category |
| T4.2 | Update `suggest_fix()` for Conditional category |
| T4.3 | Update `suggest_fix()` for SequentialOrder category |
| T4.4 | Update tests for new message formats |
| T5.1 | Add `--change` alias in cli.rs |
| T5.2 | Verify `--change` works identically to positional arg |

## ADDED Requirements

### Requirement: Parser extracts only the first paragraph for classification

T1.1 SHALL ALWAYS extract only the first paragraph from the requirement body as the `statement` field. The first paragraph is the text prior to the first blank line or `####` heading. Subsequent body paragraphs SHALL be excluded from classification but MAY remain in the requirement's source text for documentation.

#### Scenario: Single paragraph body

- **WHEN** the requirement body contains only one paragraph with a SHALL sentence
- **THEN** `extract_shall_statement()` SHALL return that paragraph unchanged (T1.1)

#### Scenario: Multi-paragraph body with prose

- **WHEN** the requirement body contains a first paragraph with SHALL sentences followed by a blank line and a second paragraph with explanatory prose
- **THEN** `extract_shall_statement()` SHALL return only the first paragraph (T1.1)
- **AND** the second paragraph SHALL NOT affect the requirement's classification category

#### Scenario: Body with scenario heading

- **WHEN** the requirement body contains a first paragraph followed by a `#### Scenario:` heading
- **THEN** `extract_shall_statement()` SHALL return only the text prior to the scenario heading (T1.1)

### Requirement: Multi-keyword statements produce a clear grounding error

T2.1 SHALL ALWAYS detect when a requirement statement matches more than one temporal predicate keyword in the grounding check. T2.2 SHALL ALWAYS emit a `grounding_ambiguous_multi_keyword` blocker with a message identifying the conflicting keywords and instructing the user to split into separate requirements.

The six temporal predicate keywords are: BEFORE, AFTER, CONCURRENTLY, IF...THEN, ALWAYS, AT MOST ONE.

#### Scenario: Statement with BEFORE and ALWAYS

- **WHEN** a requirement statement contains both `BEFORE` and `ALWAYS` keywords
- **THEN** the grounding check SHALL emit a blocker with message containing "GROUNDING AMBIGUITY" and both "BEFORE" and "ALWAYS" (T2.2)

#### Scenario: Statement with only one keyword

- **WHEN** a requirement statement contains only one temporal keyword
- **THEN** the grounding check SHALL NOT emit a multi-keyword ambiguity error (T2.1)

### Requirement: BFS model checker skips requirements with grounding failures

T3.1 SHALL ALWAYS propagate grounding failure status to the verification result. T3.2 SHALL ALWAYS skip the BFS model checker for any requirement whose grounding status indicates failure.

Grounding failure statuses are `Ungroundable` and `Ambiguous`.

#### Scenario: Grounding failed, BFS skipped

- **WHEN** a requirement has `Ungroundable` or `Ambiguous` grounding status
- **THEN** the BFS model checker SHALL NOT run on that requirement (T3.2)
- **AND** the verification result SHALL indicate the requirement was skipped due to grounding failure

#### Scenario: Grounding passed, BFS runs normally

- **WHEN** a requirement has `Grounded` grounding status
- **THEN** the BFS model checker SHALL run normally on that requirement (T3.2)

### Requirement: Error messages reference actual detected keywords

T4.1 SHALL ALWAYS update the `Exclusive` category hint in `suggest_fix()` to reference the actual detected keyword. T4.2 SHALL ALWAYS update the `Conditional` category hint to reference the actual detected keyword. T4.3 SHALL ALWAYS update the `SequentialOrder` category hint to reference the actual detected keyword.

The current hints assume specific keywords like "AT MOST ONE" or "IF...THEN" even when those keywords are not present in the statement.

#### Scenario: Exclusive from "only one" in body text

- **WHEN** a requirement is classified as `Exclusive` because body text contains "only one"
- **THEN** the suggested fix SHALL say "body text contains 'only one'" rather than referencing assumed keywords (T4.1)

#### Scenario: Conditional from "if" in body text

- **WHEN** a requirement is classified as `Conditional` because body text contains "if"
- **THEN** the suggested fix SHALL say "body text contains 'if'" rather than referencing assumed keywords (T4.2)

### Requirement: CLI accepts --change as alias

T5.1 SHALL ALWAYS accept `--change <name>` as an alias for the positional `[CHANGE]` argument in `veriplan check`.

#### Scenario: --change flag used

- **WHEN** a user runs `veriplan check --change my-change`
- **THEN** veriplan SHALL treat `my-change` as the change name, identical to `veriplan check my-change` (T5.1)
