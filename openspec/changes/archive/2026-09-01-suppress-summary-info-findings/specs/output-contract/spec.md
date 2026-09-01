## ADDED Requirements

### Requirement: Task and scenario count diagnostics are not findings

T1.1 SHALL remove the `task_count` emission BEFORE T1.2 SHALL remove the `scenario_count` emission. The `findings[]` set SHALL NOT include a `task_count` or `scenario_count` item.

#### Scenario: Counts no longer surface as findings

- **GIVEN** a plan with N tasks and S scenarios
- **WHEN** the `task_count` and `scenario_count` emissions are removed
- **THEN** the `findings[]` SHALL NOT contain a `task_count` item
- **AND** the `findings[]` SHALL NOT contain a `scenario_count` item

### Requirement: Classifiability and diversity diagnostics are not findings

T1.3 SHALL remove the `constraint_diversity` emission BEFORE T1.4 SHALL remove the `classification_summary` emission. The `findings[]` set SHALL NOT include a `constraint_diversity` or `classification_summary` item.

#### Scenario: Distribution and formalizable counts are suppressed

- **GIVEN** a plan with formalizable requirements across multiple constraint categories
- **WHEN** the `constraint_diversity` and `classification_summary` emissions are removed
- **THEN** the `findings[]` SHALL NOT contain a `classification_summary` item
- **AND** the `findings[]` SHALL NOT contain a `constraint_diversity` item

### Requirement: Grounding summary removed and counts shown in verbose

T1.5 SHALL remove the `grounding_summary` emission BEFORE T1.6 SHALL add the scenario count to the verbose plan summary. The `findings[]` set SHALL NOT include a `grounding_summary` item. The verbose plan summary SHALL print the task, requirement, and scenario counts.

#### Scenario: Grounding summary is not a finding

- **GIVEN** a plan that passes the grounding check
- **WHEN** the `grounding_summary` emission is removed
- **THEN** the `findings[]` SHALL NOT contain a `grounding_summary` item

#### Scenario: Counts remain available under --verbose

- **GIVEN** a plan with N tasks, M requirements, and S scenarios
- **WHEN** the verbose plan summary is updated
- **THEN** the verbose output SHALL print the task and scenario counts
- **AND** the counts SHALL NOT appear as findings

### Requirement: Validation confirms no aggregate prose_other findings

T1.7 SHALL add a test asserting the `findings[]` has no aggregate `prose_other` summaries BEFORE T1.8 SHALL run `veriplan check` on the change. The change SHALL report VALID with no blockers.

#### Scenario: Default output is free of aggregate summary noise

- **GIVEN** a plan with several tasks, requirements, and scenarios
- **WHEN** T1.7 runs the assertion test
- **THEN** the `findings[]` SHALL NOT contain a `prose_other` finding with a count or summary message
- **AND** T1.8 SHALL report the change as VALID with no blockers
